use crate::linear::{DofId, LinearError, MatrixEntry, SparseLinearSolver, SymmetricSystem};
use robogen_domain::{Force, Length, Pressure};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Beam {
    pub dimensions: [Length; 3],
    pub cells: [usize; 3],
    pub young_modulus: Pressure,
    pub poisson_ratio: f64,
    pub end_force: [Force; 3],
}

#[derive(Debug, Clone)]
pub struct GridDefinition {
    pub dimensions: [Length; 3],
    pub cells: [usize; 3],
    pub young_modulus: Pressure,
    pub poisson_ratio: f64,
}

/// Revision-local node index, X slowest and Z fastest; never persist this ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone)]
pub struct NodeConstraint {
    pub node: NodeId,
    pub fixed: [bool; 3],
}

#[derive(Debug, Clone)]
pub struct NodalForce {
    pub node: NodeId,
    pub force: [Force; 3],
}

#[derive(Debug, Clone)]
pub struct BoundaryConditions {
    pub constraints: Vec<NodeConstraint>,
    pub forces: Vec<NodalForce>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Compliance(f64);
impl Compliance {
    pub fn joules(self) -> f64 {
        self.0
    }
    pub(crate) fn sum(values: impl Iterator<Item = Self>) -> Self {
        Self(values.map(|c| c.0).sum())
    }
}

#[derive(Debug)]
pub struct GridResult {
    /// Grid nodes; absent nodes are zero placeholders, not solved DOFs.
    pub displacements: Vec<[Length; 3]>,
    pub compliance: Compliance,
    pub relative_residual: f64,
    pub(crate) element_energies: Vec<f64>,
}

#[derive(Debug)]
pub struct FemResult {
    pub grid: GridResult,
    pub mean_tip_displacement: [Length; 3],
}
impl std::ops::Deref for FemResult {
    type Target = GridResult;
    fn deref(&self) -> &Self::Target {
        &self.grid
    }
}

#[derive(Debug, Error)]
pub enum FemError {
    #[error("Invalid grid, density, interpolation or resource budget")]
    InvalidInput,
    #[error("Invalid boundary conditions, zero net load or load on a constrained DOF")]
    Boundary,
    #[error("Boundary conditions leave unconstrained rigid-body modes")]
    RigidModes,
    #[error("Solid mask removes a clamped or loaded interface")]
    MissingInterface,
    #[error("Solid mask is not connected through cell faces")]
    Disconnected,
    #[error("Computation cancelled")]
    Cancelled,
    #[error(transparent)]
    Linear(#[from] LinearError),
    #[error("Non-finite FEM energy or failed energy balance")]
    Energy,
}

pub struct GridModel {
    definition: GridDefinition,
    stiffness: [[f64; 24]; 24],
    elements: Vec<[usize; 8]>,
    node_count: usize,
}

impl GridModel {
    pub fn new(definition: GridDefinition) -> Result<Self, FemError> {
        let cells = definition.cells;
        if cells.iter().any(|n| *n == 0 || *n > 64)
            || cells.iter().product::<usize>() > 2048
            || definition
                .dimensions
                .iter()
                .any(|l| !l.metres().is_finite() || l.metres() <= 0.0)
            || !definition.young_modulus.pascals().is_finite()
            || definition.young_modulus.pascals() <= 0.0
            || !definition.poisson_ratio.is_finite()
            || !(-0.99..0.49).contains(&definition.poisson_ratio)
        {
            return Err(FemError::InvalidInput);
        }
        let node_count = cells.iter().map(|n| n + 1).product();
        let spacing = std::array::from_fn(|a| definition.dimensions[a].metres() / cells[a] as f64);
        let stiffness = hex_stiffness(
            spacing,
            definition.young_modulus.pascals(),
            definition.poisson_ratio,
        );
        if stiffness.iter().flatten().any(|v| !v.is_finite()) {
            return Err(FemError::InvalidInput);
        }
        let mut elements = Vec::new();
        for x in 0..cells[0] {
            for y in 0..cells[1] {
                for z in 0..cells[2] {
                    elements.push(std::array::from_fn(|corner| {
                        ((x + (corner & 1)) * (cells[1] + 1) + y + ((corner >> 1) & 1))
                            * (cells[2] + 1)
                            + z
                            + ((corner >> 2) & 1)
                    }));
                }
            }
        }
        Ok(Self {
            definition,
            stiffness,
            elements,
            node_count,
        })
    }
    pub fn definition(&self) -> &GridDefinition {
        &self.definition
    }
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    pub fn node_id(&self, position: [usize; 3]) -> Option<NodeId> {
        if (0..3).any(|a| position[a] > self.definition.cells[a]) {
            return None;
        }
        Some(NodeId(
            (position[0] * (self.definition.cells[1] + 1) + position[1])
                * (self.definition.cells[2] + 1)
                + position[2],
        ))
    }

    pub fn solve(
        &self,
        densities: &[f64],
        penalty: f64,
        stiffness_floor: f64,
        boundary: &BoundaryConditions,
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<GridResult, FemError> {
        if densities.len() != self.element_count()
            || densities
                .iter()
                .any(|r| !r.is_finite() || !(0.0..=1.0).contains(r))
            || !penalty.is_finite()
            || !(1.0..=5.0).contains(&penalty)
            || !stiffness_floor.is_finite()
            || !(1e-9..=0.1).contains(&stiffness_floor)
        {
            return Err(FemError::InvalidInput);
        }
        let scales: Vec<_> = densities
            .iter()
            .map(|r| stiffness_floor + (1.0 - stiffness_floor) * r.powf(penalty))
            .collect();
        self.solve_scales(&scales, boundary, solver, cancelled)
    }

    pub fn solve_solid_mask(
        &self,
        solid: &[bool],
        boundary: &BoundaryConditions,
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<GridResult, FemError> {
        if cancelled() {
            return Err(FemError::Cancelled);
        }
        if solid.len() != self.element_count() {
            return Err(FemError::InvalidInput);
        }
        let Some(seed) = solid.iter().position(|v| *v) else {
            return Err(FemError::MissingInterface);
        };
        let [_, ny, nz] = self.definition.cells;
        let mut seen = vec![false; solid.len()];
        let mut pending = vec![seed];
        while let Some(i) = pending.pop() {
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            if seen[i] {
                continue;
            }
            seen[i] = true;
            let p = [i / (ny * nz), i / nz % ny, i % nz];
            for axis in 0..3 {
                for positive in [false, true] {
                    let mut q = p;
                    if positive {
                        if q[axis] + 1 == self.definition.cells[axis] {
                            continue;
                        }
                        q[axis] += 1;
                    } else {
                        if q[axis] == 0 {
                            continue;
                        }
                        q[axis] -= 1;
                    }
                    let j = (q[0] * ny + q[1]) * nz + q[2];
                    if solid[j] && !seen[j] {
                        pending.push(j);
                    }
                }
            }
        }
        if solid
            .iter()
            .zip(seen)
            .any(|(active, visited)| *active && !visited)
        {
            return Err(FemError::Disconnected);
        }
        let scales: Vec<_> = solid
            .iter()
            .map(|active| if *active { 1.0 } else { 0.0 })
            .collect();
        self.solve_scales(&scales, boundary, solver, cancelled)
    }

    fn restrains_rigid_modes(&self, fixed: &[[bool; 3]]) -> bool {
        // Rank of prescribed components of u = translation + rotation cross r.
        // Coordinates are nondimensionalized by the domain's largest length.
        let [_, ny, nz] = self.definition.cells;
        let scale = self
            .definition
            .dimensions
            .iter()
            .map(|l| l.metres())
            .fold(0.0, f64::max);
        let mut basis: Vec<[f64; 6]> = Vec::new();
        for (node, axes) in fixed.iter().enumerate() {
            let p = [
                node / ((ny + 1) * (nz + 1)),
                node / (nz + 1) % (ny + 1),
                node % (nz + 1),
            ];
            let [x, y, z] = std::array::from_fn(|a| {
                p[a] as f64 / self.definition.cells[a] as f64
                    * self.definition.dimensions[a].metres()
                    / scale
            });
            let rows = [
                [1.0, 0.0, 0.0, 0.0, z, -y],
                [0.0, 1.0, 0.0, -z, 0.0, x],
                [0.0, 0.0, 1.0, y, -x, 0.0],
            ];
            for axis in 0..3 {
                if !axes[axis] {
                    continue;
                }
                let mut row = rows[axis];
                for vector in &basis {
                    let dot: f64 = row.iter().zip(vector).map(|(a, b)| a * b).sum();
                    for i in 0..6 {
                        row[i] -= dot * vector[i];
                    }
                }
                let norm = row.iter().fold(0.0_f64, |norm, v| norm.hypot(*v));
                if norm > 1e-10 {
                    basis.push(row.map(|v| v / norm));
                }
                if basis.len() == 6 {
                    return true;
                }
            }
        }
        false
    }

    fn solve_scales(
        &self,
        scales: &[f64],
        boundary: &BoundaryConditions,
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<GridResult, FemError> {
        if cancelled() {
            return Err(FemError::Cancelled);
        }
        if boundary.constraints.is_empty()
            || boundary.forces.is_empty()
            || boundary.constraints.len() > self.node_count * 3
            || boundary.forces.len() > self.node_count * 3
        {
            return Err(FemError::Boundary);
        }
        let mut active = vec![false; self.node_count];
        for (element, scale) in self.elements.iter().zip(scales) {
            if *scale > 0.0 {
                for node in element {
                    active[*node] = true;
                }
            }
        }
        let mut fixed = vec![[false; 3]; self.node_count];
        let mut forces = vec![[0.0; 3]; self.node_count];
        for constraint in &boundary.constraints {
            let node = constraint.node.0;
            if node >= self.node_count || !constraint.fixed.iter().any(|v| *v) {
                return Err(FemError::Boundary);
            }
            if !active[node] {
                return Err(FemError::MissingInterface);
            }
            for axis in 0..3 {
                fixed[node][axis] |= constraint.fixed[axis];
            }
        }
        for load in &boundary.forces {
            let node = load.node.0;
            if node >= self.node_count || load.force.iter().any(|f| !f.newtons().is_finite()) {
                return Err(FemError::Boundary);
            }
            if !active[node] {
                return Err(FemError::MissingInterface);
            }
            for axis in 0..3 {
                forces[node][axis] += load.force[axis].newtons();
            }
        }
        if !self.restrains_rigid_modes(&fixed) {
            return Err(FemError::RigidModes);
        }
        let mut mapping = vec![[None; 3]; self.node_count];
        let mut rhs = Vec::new();
        for node in 0..self.node_count {
            for axis in 0..3 {
                let force = forces[node][axis];
                if !force.is_finite() || (fixed[node][axis] && force != 0.0) {
                    return Err(FemError::Boundary);
                }
                if active[node] && !fixed[node][axis] {
                    mapping[node][axis] = Some(rhs.len());
                    rhs.push(force);
                }
            }
        }
        if rhs.is_empty() || rhs.len() > 8000 || rhs.iter().all(|v| *v == 0.0) {
            return Err(FemError::Boundary);
        }
        let mut lower = Vec::with_capacity(self.element_count() * 300);
        for (element, scale) in self.elements.iter().zip(scales) {
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            if *scale == 0.0 {
                continue;
            }
            for local_row in 0..24 {
                let Some(row) = mapping[element[local_row / 3]][local_row % 3] else {
                    continue;
                };
                for local_column in 0..24 {
                    let Some(column) = mapping[element[local_column / 3]][local_column % 3] else {
                        continue;
                    };
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
        let displacements: Vec<[Length; 3]> = mapping
            .iter()
            .map(|node| {
                std::array::from_fn(|axis| {
                    node[axis].map_or(Length::ZERO, |dof| {
                        Length::from_metres(solution.values[dof])
                    })
                })
            })
            .collect();
        let compliance: f64 = rhs.iter().zip(&solution.values).map(|(f, u)| f * u).sum();
        let mut element_energies = Vec::with_capacity(self.element_count());
        for (element, scale) in self.elements.iter().zip(scales) {
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            if *scale == 0.0 {
                element_energies.push(0.0);
                continue;
            }
            let local: [f64; 24] =
                std::array::from_fn(|dof| displacements[element[dof / 3]][dof % 3].metres());
            let mut energy = 0.0;
            for (row, u) in local.iter().enumerate() {
                for (column, v) in local.iter().enumerate() {
                    energy += u * self.stiffness[row][column] * v;
                }
            }
            element_energies.push(energy);
        }
        let total_energy: f64 = element_energies
            .iter()
            .zip(scales)
            .map(|(e, s)| e * s)
            .sum();
        if !compliance.is_finite()
            || compliance <= 0.0
            || !total_energy.is_finite()
            || (total_energy - compliance).abs() / compliance > 1e-6
            || element_energies.iter().any(|e| !e.is_finite() || *e < 0.0)
        {
            return Err(FemError::Energy);
        }
        Ok(GridResult {
            displacements,
            compliance: Compliance(compliance),
            relative_residual: solution.relative_residual,
            element_energies,
        })
    }
}

pub struct BeamModel {
    beam: Beam,
    grid: GridModel,
    boundary: BoundaryConditions,
}
impl BeamModel {
    pub fn new(beam: Beam) -> Result<Self, FemError> {
        let grid = GridModel::new(GridDefinition {
            dimensions: beam.dimensions,
            cells: beam.cells,
            young_modulus: beam.young_modulus,
            poisson_ratio: beam.poisson_ratio,
        })?;
        if beam.end_force.iter().any(|f| !f.newtons().is_finite())
            || beam.end_force.iter().all(|f| f.newtons() == 0.0)
            || beam.cells[0] * (beam.cells[1] + 1) * (beam.cells[2] + 1) * 3 > 8000
        {
            return Err(FemError::InvalidInput);
        }
        let mut boundary = BoundaryConditions {
            constraints: Vec::new(),
            forces: Vec::new(),
        };
        let [nx, ny, nz] = beam.cells;
        for y in 0..=ny {
            for z in 0..=nz {
                boundary.constraints.push(NodeConstraint {
                    node: NodeId(y * (nz + 1) + z),
                    fixed: [true; 3],
                });
                let weight = face_weight(y, ny) * face_weight(z, nz);
                boundary.forces.push(NodalForce {
                    node: NodeId((nx * (ny + 1) + y) * (nz + 1) + z),
                    force: beam
                        .end_force
                        .map(|f| Force::from_newtons(f.newtons() * weight)),
                });
            }
        }
        Ok(Self {
            beam,
            grid,
            boundary,
        })
    }
    pub fn beam(&self) -> &Beam {
        &self.beam
    }
    pub fn grid_definition(&self) -> &GridDefinition {
        self.grid.definition()
    }
    pub fn boundary_conditions(&self) -> &BoundaryConditions {
        &self.boundary
    }
    pub fn element_count(&self) -> usize {
        self.grid.element_count()
    }
    pub fn solve(
        &self,
        densities: &[f64],
        penalty: f64,
        stiffness_floor: f64,
        solver: &dyn SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<FemResult, FemError> {
        self.grid
            .solve(
                densities,
                penalty,
                stiffness_floor,
                &self.boundary,
                solver,
                cancelled,
            )
            .map(|result| self.with_tip(result))
    }
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
        let layer = self.beam.cells[1] * self.beam.cells[2];
        if solid[..layer]
            .iter()
            .chain(&solid[solid.len() - layer..])
            .any(|s| !s)
        {
            return Err(FemError::MissingInterface);
        }
        self.grid
            .solve_solid_mask(solid, &self.boundary, solver, cancelled)
            .map(|result| self.with_tip(result))
    }
    pub(crate) fn with_tip(&self, grid: GridResult) -> FemResult {
        let [nx, ny, nz] = self.beam.cells;
        let mut tip = [0.0; 3];
        for y in 0..=ny {
            for z in 0..=nz {
                let node = (nx * (ny + 1) + y) * (nz + 1) + z;
                let weight = face_weight(y, ny) * face_weight(z, nz);
                for (axis, value) in tip.iter_mut().enumerate() {
                    *value += weight * grid.displacements[node][axis].metres();
                }
            }
        }
        FemResult {
            grid,
            mean_tip_displacement: tip.map(Length::from_metres),
        }
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
    fn arbitrary_axes_and_directional_supports_pass_exact_patch() -> Result<(), FemError> {
        for axis in 0..3 {
            let definition = GridDefinition {
                dimensions: [0.03, 0.04, 0.05].map(Length::from_metres),
                cells: [3, 2, 2],
                young_modulus: Pressure::from_pascals(70e9),
                poisson_ratio: 0.3,
            };
            let model = GridModel::new(definition.clone())?;
            let mut boundary = BoundaryConditions {
                constraints: Vec::new(),
                forces: Vec::new(),
            };
            let [nx, ny, nz] = definition.cells;
            for x in 0..=nx {
                for y in 0..=ny {
                    for z in 0..=nz {
                        let p = [x, y, z];
                        let node = model.node_id(p).ok_or(FemError::InvalidInput)?;
                        let fixed = p.map(|c| c == 0);
                        if fixed.iter().any(|v| *v) {
                            boundary.constraints.push(NodeConstraint { node, fixed });
                        }
                        if p[axis] == definition.cells[axis] {
                            let u = (axis + 1) % 3;
                            let v = (axis + 2) % 3;
                            let weight = face_weight(p[u], definition.cells[u])
                                * face_weight(p[v], definition.cells[v]);
                            boundary.forces.push(NodalForce {
                                node,
                                force: std::array::from_fn(|a| {
                                    Force::from_newtons(if a == axis { 10.0 * weight } else { 0.0 })
                                }),
                            });
                        }
                    }
                }
            }
            let result = model.solve(&[1.0; 12], 3.0, 1e-6, &boundary, &FaerSolver, &|| false)?;
            let area = definition.dimensions[(axis + 1) % 3].metres()
                * definition.dimensions[(axis + 2) % 3].metres();
            let extension = 10.0 * definition.dimensions[axis].metres() / (70e9 * area);
            assert!((result.compliance.joules() / (10.0 * extension) - 1.0).abs() < 1e-10);
            for load in &boundary.forces {
                assert!(
                    (result.displacements[load.node.0][axis].metres() / extension - 1.0).abs()
                        < 1e-10
                );
            }
        }
        Ok(())
    }

    #[test]
    fn general_boundaries_reject_invalid_nodes_lost_interfaces_and_rigid_modes()
    -> Result<(), FemError> {
        let adapter = BeamModel::new(beam([4, 2, 2], [0.0, 0.0, -1.0]))?;
        let model = GridModel::new(adapter.grid_definition().clone())?;
        let boundary = adapter.boundary_conditions().clone();
        let solve = |conditions: &BoundaryConditions| {
            model.solve(&[1.0; 16], 3.0, 1e-6, conditions, &FaerSolver, &|| false)
        };
        let mut invalid = boundary.clone();
        invalid.constraints.truncate(1);
        assert!(matches!(solve(&invalid), Err(FemError::RigidModes)));
        invalid = boundary.clone();
        invalid.forces[0].node = NodeId(usize::MAX);
        assert!(matches!(solve(&invalid), Err(FemError::Boundary)));
        invalid = boundary.clone();
        invalid.forces[0].node = NodeId(0);
        assert!(matches!(solve(&invalid), Err(FemError::Boundary)));
        invalid = boundary.clone();
        invalid.forces[0].force[0] = Force::from_newtons(f64::NAN);
        assert!(matches!(solve(&invalid), Err(FemError::Boundary)));
        let mut mask = [true; 16];
        mask[15] = false;
        assert!(matches!(
            model.solve_solid_mask(&mask, &boundary, &FaerSolver, &|| false),
            Err(FemError::MissingInterface)
        ));
        assert_eq!(model.node_id([5, 0, 0]), None);
        Ok(())
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
