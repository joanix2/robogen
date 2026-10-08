use robogen_domain::{Force, Length, Pressure};
use thiserror::Error;

use crate::linear::{DofId, LinearError, MatrixEntry, SparseLinearSolver, SymmetricSystem};

#[derive(Debug, Clone)]
pub struct Beam {
    pub dimensions: [Length; 3],
    pub cells: [usize; 3],
    pub young_modulus: Pressure,
    pub poisson_ratio: f64,
    pub end_force: [Force; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct Compliance(f64);

impl Compliance {
    pub fn joules(self) -> f64 {
        self.0
    }
}

#[derive(Debug)]
pub struct FemResult {
    /// Grid nodes; absent nodes in a solid-mask analysis are zero placeholders.
    pub displacements: Vec<[Length; 3]>,
    pub compliance: Compliance,
    pub relative_residual: f64,
    pub mean_tip_displacement: [Length; 3],
    pub(crate) element_energies: Vec<f64>,
}

#[derive(Debug, Error)]
pub enum FemError {
    #[error("Invalid beam, density, interpolation or resource budget")]
    InvalidInput,
    #[error("Solid mask removes a clamped or loaded interface cell")]
    MissingInterface,
    #[error("Solid mask is not connected through cell faces to the clamp")]
    Disconnected,
    #[error("Computation cancelled")]
    Cancelled,
    #[error(transparent)]
    Linear(#[from] LinearError),
    #[error("Non-finite FEM energy or failed energy balance")]
    Energy,
}

pub struct BeamModel {
    beam: Beam,
    stiffness: [[f64; 24]; 24],
    elements: Vec<[usize; 8]>,
    rhs: Vec<f64>,
    fixed_nodes: usize,
    node_count: usize,
}

impl BeamModel {
    pub fn new(beam: Beam) -> Result<Self, FemError> {
        let cells = beam.cells;
        if cells.iter().any(|count| *count == 0 || *count > 64)
            || cells.iter().product::<usize>() > 2048
            || beam
                .dimensions
                .iter()
                .any(|length| !length.metres().is_finite() || length.metres() <= 0.0)
            || !beam.young_modulus.pascals().is_finite()
            || beam.young_modulus.pascals() <= 0.0
            || !beam.poisson_ratio.is_finite()
            || !(-0.99..0.49).contains(&beam.poisson_ratio)
            || beam
                .end_force
                .iter()
                .any(|force| !force.newtons().is_finite())
            || beam.end_force.iter().all(|force| force.newtons() == 0.0)
        {
            return Err(FemError::InvalidInput);
        }
        let fixed_nodes = (cells[1] + 1) * (cells[2] + 1);
        let node_count = (cells[0] + 1) * fixed_nodes;
        if (node_count - fixed_nodes) * 3 > 8000 {
            return Err(FemError::InvalidInput);
        }
        let spacing =
            std::array::from_fn(|axis| beam.dimensions[axis].metres() / cells[axis] as f64);
        let stiffness = hex_stiffness(spacing, beam.young_modulus.pascals(), beam.poisson_ratio);
        if stiffness.iter().flatten().any(|value| !value.is_finite()) {
            return Err(FemError::InvalidInput);
        }
        let mut elements = Vec::new();
        for axial in 0..cells[0] {
            for lateral in 0..cells[1] {
                for vertical in 0..cells[2] {
                    elements.push(std::array::from_fn(|corner| {
                        ((axial + (corner & 1)) * (cells[1] + 1) + lateral + ((corner >> 1) & 1))
                            * (cells[2] + 1)
                            + vertical
                            + ((corner >> 2) & 1)
                    }));
                }
            }
        }
        let mut rhs = vec![0.0; (node_count - fixed_nodes) * 3];
        for lateral in 0..=cells[1] {
            for vertical in 0..=cells[2] {
                let node = cells[0] * fixed_nodes + lateral * (cells[2] + 1) + vertical;
                let weight = face_weight(lateral, cells[1]) * face_weight(vertical, cells[2]);
                for axis in 0..3 {
                    rhs[3 * (node - fixed_nodes) + axis] = beam.end_force[axis].newtons() * weight;
                }
            }
        }
        Ok(Self {
            beam,
            stiffness,
            elements,
            rhs,
            fixed_nodes,
            node_count,
        })
    }

    pub fn beam(&self) -> &Beam {
        &self.beam
    }
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn solve(
        &self,
        densities: &[f64],
        penalty: f64,
        stiffness_floor: f64,
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<FemResult, FemError> {
        if densities.len() != self.elements.len()
            || densities
                .iter()
                .any(|density| !density.is_finite() || !(0.0..=1.0).contains(density))
            || !penalty.is_finite()
            || !(1.0..=5.0).contains(&penalty)
            || !stiffness_floor.is_finite()
            || !(1e-9..=0.1).contains(&stiffness_floor)
        {
            return Err(FemError::InvalidInput);
        }
        let scales: Vec<_> = densities
            .iter()
            .map(|density| stiffness_floor + (1.0 - stiffness_floor) * density.powf(penalty))
            .collect();
        self.solve_scales(&scales, solver, cancelled)
    }

    /// Reanalyse the exact union of solid Hex8 cells. Void cells and unused
    /// degrees of freedom are absent, with no SIMP stiffness floor.
    /// The beam's complete clamp and loaded end faces must remain present.
    pub fn solve_solid_mask(
        &self,
        solid: &[bool],
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<FemResult, FemError> {
        if cancelled() {
            return Err(FemError::Cancelled);
        }
        if solid.len() != self.element_count() {
            return Err(FemError::InvalidInput);
        }
        let [nx, ny, nz] = self.beam.cells;
        let layer = ny * nz;
        if solid[..layer]
            .iter()
            .chain(&solid[(nx - 1) * layer..])
            .any(|value| !value)
        {
            return Err(FemError::MissingInterface);
        }
        let mut visited = vec![false; solid.len()];
        let mut pending = vec![0];
        while let Some(index) = pending.pop() {
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            if visited[index] {
                continue;
            }
            visited[index] = true;
            let point = [index / layer, index / nz % ny, index % nz];
            for axis in 0..3 {
                for positive in [false, true] {
                    let mut other = point;
                    if positive {
                        if other[axis] + 1 == self.beam.cells[axis] {
                            continue;
                        }
                        other[axis] += 1;
                    } else {
                        if other[axis] == 0 {
                            continue;
                        }
                        other[axis] -= 1;
                    }
                    let neighbour = (other[0] * ny + other[1]) * nz + other[2];
                    if solid[neighbour] && !visited[neighbour] {
                        pending.push(neighbour);
                    }
                }
            }
        }
        if solid
            .iter()
            .zip(visited)
            .any(|(active, seen)| *active && !seen)
        {
            return Err(FemError::Disconnected);
        }
        let scales: Vec<_> = solid
            .iter()
            .map(|active| if *active { 1.0 } else { 0.0 })
            .collect();
        self.solve_scales(&scales, solver, cancelled)
    }

    fn solve_scales(
        &self,
        scales: &[f64],
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<FemResult, FemError> {
        let mut active = vec![false; self.node_count];
        for (element, scale) in self.elements.iter().zip(scales) {
            if *scale > 0.0 {
                for node in element {
                    active[*node] = true;
                }
            }
        }
        let mut mapping = vec![None; self.node_count];
        let mut rhs = Vec::new();
        for node in self.fixed_nodes..self.node_count {
            if active[node] {
                mapping[node] = Some(rhs.len());
                rhs.extend_from_slice(
                    &self.rhs[(node - self.fixed_nodes) * 3..(node - self.fixed_nodes + 1) * 3],
                );
            }
        }
        let mut lower = Vec::with_capacity(self.elements.len() * 300);
        for (element, scale) in self.elements.iter().zip(scales) {
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            if *scale == 0.0 {
                continue;
            }
            for local_row in 0..24 {
                let row_node = element[local_row / 3];
                let Some(base) = mapping[row_node] else {
                    continue;
                };
                let row = base + local_row % 3;
                for local_column in 0..24 {
                    let column_node = element[local_column / 3];
                    let Some(base) = mapping[column_node] else {
                        continue;
                    };
                    let column = base + local_column % 3;
                    if row >= column {
                        lower.push(MatrixEntry {
                            row: DofId(row),
                            column: DofId(column),
                            value: scale * self.stiffness[local_row][local_column],
                        });
                    }
                }
            }
        }
        if cancelled() {
            return Err(FemError::Cancelled);
        }
        let solution = solver.solve(
            &SymmetricSystem {
                lower,
                rhs: rhs.clone(),
            },
            1e-7,
        )?;
        if cancelled() {
            return Err(FemError::Cancelled);
        }
        let mut displacements = vec![[Length::ZERO; 3]; self.node_count];
        for (node, displacement) in displacements.iter_mut().enumerate().skip(self.fixed_nodes) {
            if let Some(base) = mapping[node] {
                *displacement =
                    std::array::from_fn(|axis| Length::from_metres(solution.values[base + axis]));
            }
        }
        let compliance = rhs
            .iter()
            .zip(&solution.values)
            .map(|(force, displacement)| force * displacement)
            .sum::<f64>();
        let mut element_energies = Vec::with_capacity(self.elements.len());
        for (element, scale) in self.elements.iter().zip(scales) {
            if *scale == 0.0 {
                element_energies.push(0.0);
                continue;
            }
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            let local: [f64; 24] =
                std::array::from_fn(|dof| displacements[element[dof / 3]][dof % 3].metres());
            let mut energy = 0.0;
            for (row, displacement) in local.iter().enumerate() {
                for (column, other) in local.iter().enumerate() {
                    energy += displacement * self.stiffness[row][column] * other;
                }
            }
            element_energies.push(energy);
        }
        let total_energy: f64 = element_energies
            .iter()
            .zip(scales)
            .map(|(energy, scale)| energy * scale)
            .sum();
        if !compliance.is_finite()
            || compliance <= 0.0
            || !total_energy.is_finite()
            || (total_energy - compliance).abs() / compliance > 1e-6
            || element_energies
                .iter()
                .any(|energy| !energy.is_finite() || *energy < 0.0)
        {
            return Err(FemError::Energy);
        }
        let mut tip = [0.0; 3];
        for lateral in 0..=self.beam.cells[1] {
            for vertical in 0..=self.beam.cells[2] {
                let node = self.beam.cells[0] * self.fixed_nodes
                    + lateral * (self.beam.cells[2] + 1)
                    + vertical;
                let weight = face_weight(lateral, self.beam.cells[1])
                    * face_weight(vertical, self.beam.cells[2]);
                for (axis, value) in tip.iter_mut().enumerate() {
                    *value += weight * displacements[node][axis].metres();
                }
            }
        }
        Ok(FemResult {
            displacements,
            compliance: Compliance(compliance),
            relative_residual: solution.relative_residual,
            mean_tip_displacement: tip.map(Length::from_metres),
            element_energies,
        })
    }
}

fn face_weight(index: usize, count: usize) -> f64 {
    if index == 0 || index == count {
        0.5 / count as f64
    } else {
        1.0 / count as f64
    }
}

fn hex_stiffness(spacing: [f64; 3], young: f64, poisson: f64) -> [[f64; 24]; 24] {
    let lambda = young * poisson / ((1.0 + poisson) * (1.0 - 2.0 * poisson));
    let shear = young / (2.0 * (1.0 + poisson));
    let mut constitutive = [[0.0; 6]; 6];
    for (row, values) in constitutive.iter_mut().enumerate() {
        for (column, value) in values.iter_mut().enumerate() {
            *value = if row < 3 && column < 3 {
                lambda + if row == column { 2.0 * shear } else { 0.0 }
            } else if row == column {
                shear
            } else {
                0.0
            };
        }
    }
    let mut stiffness = [[0.0; 24]; 24];
    let gauss = 1.0 / 3.0_f64.sqrt();
    let determinant = spacing.iter().product::<f64>() / 8.0;
    for point in 0..8 {
        let position: [f64; 3] = std::array::from_fn(|axis| {
            if point & (1 << axis) == 0 {
                -gauss
            } else {
                gauss
            }
        });
        let mut strain = [[0.0; 24]; 6];
        for corner in 0..8 {
            let sign: [f64; 3] =
                std::array::from_fn(|axis| if corner & (1 << axis) == 0 { -1.0 } else { 1.0 });
            let gradient: [f64; 3] = std::array::from_fn(|axis| {
                let other = (axis + 1) % 3;
                let last = (axis + 2) % 3;
                sign[axis]
                    * (1.0 + sign[other] * position[other])
                    * (1.0 + sign[last] * position[last])
                    / (4.0 * spacing[axis])
            });
            let base = 3 * corner;
            strain[0][base] = gradient[0];
            strain[1][base + 1] = gradient[1];
            strain[2][base + 2] = gradient[2];
            strain[3][base] = gradient[1];
            strain[3][base + 1] = gradient[0];
            strain[4][base + 1] = gradient[2];
            strain[4][base + 2] = gradient[1];
            strain[5][base] = gradient[2];
            strain[5][base + 2] = gradient[0];
        }
        for (row, values) in stiffness.iter_mut().enumerate() {
            for (column, value) in values.iter_mut().enumerate() {
                for first in 0..6 {
                    for second in 0..6 {
                        *value += strain[first][row]
                            * constitutive[first][second]
                            * strain[second][column]
                            * determinant;
                    }
                }
            }
        }
    }
    stiffness
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear::FaerSolver;

    fn beam(cells: [usize; 3], force: [f64; 3]) -> Beam {
        Beam {
            dimensions: [0.12, 0.02, 0.02].map(Length::from_metres),
            cells,
            young_modulus: Pressure::from_pascals(70e9),
            poisson_ratio: 0.0,
            end_force: force.map(Force::from_newtons),
        }
    }

    #[test]
    fn axial_patch_matches_exact_solution() -> Result<(), FemError> {
        let model = BeamModel::new(beam([4, 2, 2], [10.0, 0.0, 0.0]))?;
        let result = model.solve(
            &vec![1.0; model.element_count()],
            3.0,
            1e-6,
            &FaerSolver,
            &|| false,
        )?;
        let exact = 10.0 * 0.12 / (70e9 * 0.02 * 0.02);
        assert!((result.mean_tip_displacement[0].metres() / exact - 1.0).abs() < 1e-10);
        assert!((result.compliance.joules() / (10.0 * exact) - 1.0).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn binary_reanalysis_removes_void_dofs_and_rejects_lost_load_paths() -> Result<(), FemError> {
        let model = BeamModel::new(beam([6, 3, 2], [0.0, 0.0, -1.0]))?;
        let full = model.solve(&[1.0; 36], 3.0, 1e-6, &FaerSolver, &|| false)?;
        let mut mask = vec![true; 36];
        let binary = model.solve_solid_mask(&mask, &FaerSolver, &|| false)?;
        assert!((binary.compliance.joules() / full.compliance.joules() - 1.0).abs() < 1e-12);
        // Retain a one-cell-wide rail and both full end plates. Some interior
        // nodes now touch only void cells and must leave the stiffness system.
        for x in 1..5 {
            for y in 1..3 {
                for z in 0..2 {
                    mask[(x * 3 + y) * 2 + z] = false;
                }
            }
        }
        let reduced = model.solve_solid_mask(&mask, &FaerSolver, &|| false)?;
        assert!(reduced.compliance.joules() > full.compliance.joules());
        assert!(reduced.relative_residual < 1e-7);
        assert_eq!(
            reduced.displacements[(3 * 4 + 3) * 3 + 1],
            [Length::ZERO; 3]
        );
        assert!(
            reduced
                .element_energies
                .iter()
                .zip(&mask)
                .all(|(energy, active)| *active || *energy == 0.0)
        );
        assert!(matches!(
            model.solve_solid_mask(&mask, &FaerSolver, &|| true),
            Err(FemError::Cancelled)
        ));
        mask[3 * 6..4 * 6].fill(false);
        assert!(matches!(
            model.solve_solid_mask(&mask, &FaerSolver, &|| false),
            Err(FemError::Disconnected)
        ));
        mask[0] = false;
        assert!(matches!(
            model.solve_solid_mask(&mask, &FaerSolver, &|| false),
            Err(FemError::MissingInterface)
        ));
        assert!(matches!(
            model.solve_solid_mask(&[], &FaerSolver, &|| false),
            Err(FemError::InvalidInput)
        ));
        Ok(())
    }

    #[test]
    fn bending_refinement_approaches_beam_reference() -> Result<(), FemError> {
        let exact = 0.12_f64.powi(3) / (3.0 * 70e9 * (0.02 * 0.02_f64.powi(3) / 12.0));
        let mut errors = Vec::new();
        for cells in [[6, 2, 2], [24, 4, 4]] {
            let model = BeamModel::new(beam(cells, [0.0, 0.0, -1.0]))?;
            let result = model.solve(
                &vec![1.0; model.element_count()],
                3.0,
                1e-6,
                &FaerSolver,
                &|| false,
            )?;
            errors.push((result.mean_tip_displacement[2].metres().abs() / exact - 1.0).abs());
            assert!(result.relative_residual < 1e-7);
        }
        assert!(errors[1] < errors[0], "{errors:?}");
        assert!(errors[1] < 0.1, "{errors:?}");
        Ok(())
    }

    #[test]
    fn element_rigid_modes_have_zero_energy() {
        let spacing = [0.03, 0.02, 0.01];
        let stiffness = hex_stiffness(spacing, 70e9, 0.3);
        for mode in 0..6 {
            let displacement: [f64; 24] = std::array::from_fn(|dof| {
                let position: [f64; 3] =
                    std::array::from_fn(|axis| (((dof / 3) >> axis) & 1) as f64 * spacing[axis]);
                if mode < 3 {
                    if dof % 3 == mode { 1.0 } else { 0.0 }
                } else {
                    let axis = mode - 3;
                    if dof % 3 == (axis + 1) % 3 {
                        -position[(axis + 2) % 3]
                    } else if dof % 3 == (axis + 2) % 3 {
                        position[(axis + 1) % 3]
                    } else {
                        0.0
                    }
                }
            });
            for row in &stiffness {
                let force: f64 = row
                    .iter()
                    .zip(displacement)
                    .map(|(value, displacement)| value * displacement)
                    .sum();
                assert!(force.abs() < 1e-6, "rigid mode {mode}: {force}");
            }
        }
    }

    #[test]
    fn constant_strain_energy_matches_isotropic_elasticity() {
        let spacing = [0.03, 0.02, 0.01];
        let young = 70e9;
        let poisson = 0.3;
        let stiffness = hex_stiffness(spacing, young, poisson);
        let strain = [0.01, 0.02, -0.005, 0.008, -0.004, 0.006];
        let displacements: [f64; 24] = std::array::from_fn(|dof| {
            let position: [f64; 3] =
                std::array::from_fn(|axis| (((dof / 3) >> axis) & 1) as f64 * spacing[axis]);
            match dof % 3 {
                0 => {
                    strain[0] * position[0]
                        + 0.5 * strain[3] * position[1]
                        + 0.5 * strain[5] * position[2]
                }
                1 => {
                    0.5 * strain[3] * position[0]
                        + strain[1] * position[1]
                        + 0.5 * strain[4] * position[2]
                }
                _ => {
                    0.5 * strain[5] * position[0]
                        + 0.5 * strain[4] * position[1]
                        + strain[2] * position[2]
                }
            }
        });
        let mut energy = 0.0;
        for row in 0..24 {
            for column in 0..24 {
                assert!((stiffness[row][column] - stiffness[column][row]).abs() < 1e-6);
                energy += displacements[row] * stiffness[row][column] * displacements[column];
            }
        }
        let shear = young / (2.0 * (1.0 + poisson));
        let lambda = young * poisson / ((1.0 + poisson) * (1.0 - 2.0 * poisson));
        let expected = spacing.iter().product::<f64>()
            * (lambda * strain[..3].iter().sum::<f64>().powi(2)
                + 2.0 * shear * strain[..3].iter().map(|value| value * value).sum::<f64>()
                + shear * strain[3..].iter().map(|value| value * value).sum::<f64>());
        assert!((energy / expected - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_inputs_and_cancellation_are_errors() -> Result<(), FemError> {
        assert!(BeamModel::new(beam([usize::MAX, 1, 1], [1.0, 0.0, 0.0])).is_err());
        let model = BeamModel::new(beam([2, 1, 1], [1.0, 0.0, 0.0]))?;
        assert!(matches!(
            model.solve(&[1.0; 2], 3.0, 1e-6, &FaerSolver, &|| true),
            Err(FemError::Cancelled)
        ));
        assert!(matches!(
            model.solve(&[f64::NAN; 2], 3.0, 1e-6, &FaerSolver, &|| false),
            Err(FemError::InvalidInput)
        ));
        Ok(())
    }
}
