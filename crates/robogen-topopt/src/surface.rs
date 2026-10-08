use std::collections::{BTreeMap, BTreeSet};

use fast_surface_nets::{
    SurfaceNetsBuffer,
    ndshape::{RuntimeShape, Shape},
    surface_nets,
};
use robogen_domain::Length;
use thiserror::Error;

use crate::simp::{BeamOptimization, BeamRequest};

const SUBDIVISIONS: u32 = 2;

#[derive(Debug)]
pub struct BeamSurface {
    pub request: BeamRequest,
    pub solver_backend: String,
    pub surface: ReconstructedSurface,
    pub filtered_material_volume: Volume,
}

impl BeamOptimization {
    pub fn reconstruct_surface(
        &self,
        threshold: f64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<BeamSurface, SurfaceError> {
        let grid = DensityGrid {
            dimensions: self.request.beam.dimensions,
            cells: self.request.beam.cells,
            densities: &self.densities,
        };
        let surface = SurfaceNetsReconstructor.reconstruct(&grid, threshold, cancelled)?;
        let domain_volume = grid
            .dimensions
            .iter()
            .map(|length| length.metres())
            .product::<f64>();
        let volume =
            domain_volume * self.densities.iter().sum::<f64>() / self.densities.len() as f64;
        if !volume.is_finite() || volume <= 0.0 {
            return Err(SurfaceError::InvalidInput);
        }
        if cancelled() {
            return Err(SurfaceError::Cancelled);
        }
        Ok(BeamSurface {
            request: self.request.clone(),
            solver_backend: self.solver_backend.clone(),
            surface,
            filtered_material_volume: Volume(volume),
        })
    }
}

pub struct DensityGrid<'a> {
    pub dimensions: [Length; 3],
    pub cells: [usize; 3],
    pub densities: &'a [f64],
}

#[derive(Debug, Clone, Copy)]
pub struct Volume(f64);

impl Volume {
    pub fn cubic_metres(self) -> f64 {
        self.0
    }
}

#[derive(Debug)]
pub struct ReconstructedSurface {
    vertices: Vec<[Length; 3]>,
    triangles: Vec<[u32; 3]>,
    pub threshold: f64,
    pub backend: &'static str,
    pub sampling_subdivisions: u32,
    pub components: usize,
    pub enclosed_volume: Volume,
}

impl ReconstructedSurface {
    pub fn vertices(&self) -> &[[Length; 3]] {
        &self.vertices
    }
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }
}

#[derive(Debug, Error)]
pub enum SurfaceError {
    #[error("Invalid density grid, threshold or resource budget")]
    InvalidInput,
    #[error("No surface exists at the selected density threshold")]
    Empty,
    #[error("Surface reconstruction cancelled")]
    Cancelled,
    #[error("Invalid reconstructed surface: {0}")]
    InvalidMesh(&'static str),
}

pub trait SurfaceReconstructor: Send + Sync {
    fn reconstruct(
        &self,
        grid: &DensityGrid<'_>,
        threshold: f64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReconstructedSurface, SurfaceError>;
}

pub struct SurfaceNetsReconstructor;

impl SurfaceReconstructor for SurfaceNetsReconstructor {
    fn reconstruct(
        &self,
        grid: &DensityGrid<'_>,
        threshold: f64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReconstructedSurface, SurfaceError> {
        validate_grid(grid, threshold, cancelled)?;
        let size = grid.cells.map(|count| count as u32 + 4);
        let shape = RuntimeShape::<u32, 3>::new(size);
        let mut samples = vec![threshold as f32; shape.size() as usize];
        for axial in 0..grid.cells[0] {
            if cancelled() {
                return Err(SurfaceError::Cancelled);
            }
            for lateral in 0..grid.cells[1] {
                for vertical in 0..grid.cells[2] {
                    let source = (axial * grid.cells[1] + lateral) * grid.cells[2] + vertical;
                    let destination = shape.linearize([
                        axial as u32 + 2,
                        lateral as u32 + 2,
                        vertical as u32 + 2,
                    ]);
                    samples[destination as usize] = (threshold - grid.densities[source]) as f32;
                }
            }
        }
        let refined_size = size.map(|count| SUBDIVISIONS * (count - 1) + 1);
        let refined_shape = RuntimeShape::<u32, 3>::new(refined_size);
        let mut refined = vec![0.0; refined_shape.size() as usize];
        for index in 0..refined_shape.size() {
            if index % 1024 == 0 && cancelled() {
                return Err(SurfaceError::Cancelled);
            }
            let position = refined_shape.delinearize(index);
            let lower = position.map(|coordinate| coordinate / SUBDIVISIONS);
            let mut value = 0.0;
            for corner in 0..8 {
                let mut weight = 1.0;
                let neighbour = std::array::from_fn(|axis| {
                    let upper = (corner >> axis) & 1;
                    let fraction = (position[axis] % SUBDIVISIONS) as f32 / SUBDIVISIONS as f32;
                    weight *= if upper == 0 { 1.0 - fraction } else { fraction };
                    (lower[axis] + upper).min(size[axis] - 1)
                });
                value += weight * samples[shape.linearize(neighbour) as usize];
            }
            refined[index as usize] = value;
        }
        let mut output = SurfaceNetsBuffer::default();
        surface_nets(
            &refined,
            &refined_shape,
            [0; 3],
            refined_size.map(|count| count - 1),
            &mut output,
        );
        if cancelled() {
            return Err(SurfaceError::Cancelled);
        }
        if output.indices.is_empty() {
            return Err(SurfaceError::Empty);
        }
        let spacing: [f64; 3] =
            std::array::from_fn(|axis| grid.dimensions[axis].metres() / grid.cells[axis] as f64);
        let vertices: Vec<[Length; 3]> = output
            .positions
            .iter()
            .map(|position| {
                std::array::from_fn(|axis| {
                    Length::from_metres(
                        (f64::from(position[axis]) / f64::from(SUBDIVISIONS) - 1.5) * spacing[axis],
                    )
                })
            })
            .collect();
        let triangles: Vec<[u32; 3]> = output
            .indices
            .chunks_exact(3)
            .map(|face| [face[0], face[1], face[2]])
            .collect();
        let (components, volume) =
            validate_mesh(&vertices, &triangles, grid.dimensions, cancelled)?;
        Ok(ReconstructedSurface {
            vertices,
            triangles,
            threshold,
            backend: "fast-surface-nets-0.2.1",
            sampling_subdivisions: SUBDIVISIONS,
            components,
            enclosed_volume: Volume(volume),
        })
    }
}

fn validate_grid(
    grid: &DensityGrid<'_>,
    threshold: f64,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), SurfaceError> {
    if grid.cells.iter().any(|count| *count == 0 || *count > 64)
        || grid.cells.iter().product::<usize>() > 2048
        || grid.cells.iter().product::<usize>() != grid.densities.len()
        || grid
            .dimensions
            .iter()
            .any(|length| !length.metres().is_finite() || length.metres() <= 0.0)
        || grid
            .densities
            .iter()
            .any(|density| !density.is_finite() || !(0.0..=1.0).contains(density))
        || !threshold.is_finite()
        || !(0.5..1.0).contains(&threshold)
    {
        return Err(SurfaceError::InvalidInput);
    }
    if cancelled() {
        return Err(SurfaceError::Cancelled);
    }
    Ok(())
}

/// Exact boundary of thresholded cells. No interpolation or smoothing.
pub struct VoxelBoundaryReconstructor;

impl SurfaceReconstructor for VoxelBoundaryReconstructor {
    fn reconstruct(
        &self,
        grid: &DensityGrid<'_>,
        threshold: f64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReconstructedSurface, SurfaceError> {
        validate_grid(grid, threshold, cancelled)?;
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let mut indices = BTreeMap::new();
        let spacing: [f64; 3] =
            std::array::from_fn(|axis| grid.dimensions[axis].metres() / grid.cells[axis] as f64);
        let [nx, ny, nz] = grid.cells;
        let occupied = |p: [usize; 3]| grid.densities[(p[0] * ny + p[1]) * nz + p[2]] >= threshold;
        let mut count = 0;
        for x in 0..nx {
            for y in 0..ny {
                for z in 0..nz {
                    if cancelled() {
                        return Err(SurfaceError::Cancelled);
                    }
                    let point = [x, y, z];
                    if !occupied(point) {
                        continue;
                    }
                    count += 1;
                    for axis in 0..3 {
                        for positive in [false, true] {
                            let mut neighbour = point;
                            let boundary = if positive {
                                point[axis] + 1 == grid.cells[axis]
                            } else {
                                point[axis] == 0
                            };
                            if !boundary {
                                if positive {
                                    neighbour[axis] += 1;
                                } else {
                                    neighbour[axis] -= 1;
                                }
                                if occupied(neighbour) {
                                    continue;
                                }
                            }
                            // Cyclic axes give u cross v = outward positive normal.
                            let u = (axis + 1) % 3;
                            let v = (axis + 2) % 3;
                            let mut face = [0; 4];
                            for (corner, [du, dv]) in
                                [[0, 0], [1, 0], [1, 1], [0, 1]].into_iter().enumerate()
                            {
                                let mut position = point;
                                position[axis] += usize::from(positive);
                                position[u] += du;
                                position[v] += dv;
                                face[corner] = *indices.entry(position).or_insert_with(|| {
                                    let index = vertices.len() as u32;
                                    vertices.push(std::array::from_fn(|a| {
                                        Length::from_metres(position[a] as f64 * spacing[a])
                                    }));
                                    index
                                });
                            }
                            if !positive {
                                face.reverse();
                            }
                            triangles
                                .extend([[face[0], face[1], face[2]], [face[0], face[2], face[3]]]);
                        }
                    }
                }
            }
        }
        if triangles.is_empty() {
            return Err(SurfaceError::Empty);
        }
        let (components, volume) =
            validate_mesh(&vertices, &triangles, grid.dimensions, cancelled)?;
        let exact_volume = count as f64 * spacing.iter().product::<f64>();
        if (volume / exact_volume - 1.0).abs() > 1e-9 {
            return Err(SurfaceError::InvalidMesh("voxel boundary volume mismatch"));
        }
        Ok(ReconstructedSurface {
            vertices,
            triangles,
            threshold,
            backend: "robogen-voxel-boundary-v1",
            sampling_subdivisions: 1,
            components,
            enclosed_volume: Volume(volume),
        })
    }
}

#[derive(Debug)]
pub struct BeamSolid {
    pub request: BeamRequest,
    pub solver_backend: String,
    pub surface: ReconstructedSurface,
    /// Recomputed on binary cells with void elements removed, not on densities.
    pub analysis: crate::fem::FemResult,
    pub volume_fraction: f64,
    /// A threshold can violate the original volume budget. Such a candidate
    /// must not be published as satisfying the optimisation constraints.
    pub within_volume_budget: bool,
}

#[derive(Debug, Error)]
pub enum BeamSolidError {
    #[error(transparent)]
    Surface(#[from] SurfaceError),
    #[error(transparent)]
    Fem(#[from] crate::fem::FemError),
    #[error("Density field violates its preserved/void regions")]
    Regions,
}

impl BeamOptimization {
    pub fn reconstruct_solid(
        &self,
        threshold: f64,
        solver: &dyn crate::linear::SparseLinearSolver,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<BeamSolid, BeamSolidError> {
        use crate::simp::CellRegion;
        let grid = DensityGrid {
            dimensions: self.request.beam.dimensions,
            cells: self.request.beam.cells,
            densities: &self.densities,
        };
        validate_grid(&grid, threshold, cancelled)?;
        if !self.request.regions.is_empty()
            && (self.request.regions.len() != self.densities.len()
                || self.request.regions.iter().zip(&self.densities).any(
                    |(role, density)| match role {
                        CellRegion::Design => false,
                        CellRegion::Preserve => *density != 1.0,
                        CellRegion::Void => *density != 0.0,
                    },
                ))
        {
            return Err(BeamSolidError::Regions);
        }
        let solid: Vec<_> = self
            .densities
            .iter()
            .map(|density| *density >= threshold)
            .collect();
        let model = crate::fem::BeamModel::new(self.request.beam.clone())?;
        let analysis = model.solve_solid_mask(&solid, solver, cancelled)?;
        let surface = VoxelBoundaryReconstructor.reconstruct(&grid, threshold, cancelled)?;
        let volume_fraction =
            solid.iter().filter(|value| **value).count() as f64 / solid.len() as f64;
        Ok(BeamSolid {
            request: self.request.clone(),
            solver_backend: solver.backend_id().to_owned(),
            surface,
            analysis,
            volume_fraction,
            within_volume_budget: volume_fraction <= self.request.options.volume_fraction + 1e-10,
        })
    }
}

fn validate_mesh(
    vertices: &[[Length; 3]],
    triangles: &[[u32; 3]],
    dimensions: [Length; 3],
    cancelled: &dyn Fn() -> bool,
) -> Result<(usize, f64), SurfaceError> {
    let scale = dimensions
        .iter()
        .map(|length| length.metres())
        .fold(0.0, f64::max);
    let tolerance = scale * 1e-6;
    if vertices.iter().any(|point| {
        (0..3).any(|axis| {
            !point[axis].metres().is_finite()
                || point[axis].metres() < -tolerance
                || point[axis].metres() > dimensions[axis].metres() + tolerance
        })
    }) {
        return Err(SurfaceError::InvalidMesh(
            "non-finite or out-of-domain vertex",
        ));
    }
    let mut edges: BTreeMap<(u32, u32), Vec<(usize, bool)>> = BTreeMap::new();
    let mut links = vec![Vec::new(); vertices.len()];
    let mut face_volumes = Vec::with_capacity(triangles.len());
    for (index, face) in triangles.iter().enumerate() {
        if cancelled() {
            return Err(SurfaceError::Cancelled);
        }
        if face.iter().any(|vertex| *vertex as usize >= vertices.len()) {
            return Err(SurfaceError::InvalidMesh("invalid vertex index"));
        }
        let points = face.map(|vertex| vertices[vertex as usize].map(Length::metres));
        let first = subtract(points[1], points[0]);
        let second = subtract(points[2], points[0]);
        let normal = cross(first, second);
        let area_squared = dot(normal, normal);
        if !area_squared.is_finite() || area_squared <= scale.powi(4) * 1e-24 {
            return Err(SurfaceError::InvalidMesh("degenerate triangle"));
        }
        face_volumes.push(dot(points[0], cross(points[1], points[2])) / 6.0);
        for corner in 0..3 {
            let start = face[corner];
            let end = face[(corner + 1) % 3];
            edges
                .entry((start.min(end), start.max(end)))
                .or_default()
                .push((index, start < end));
            links[start as usize].push((end, face[(corner + 2) % 3]));
        }
    }
    let mut adjacency = vec![Vec::new(); triangles.len()];
    for incidents in edges.values() {
        if incidents.len() != 2 || incidents[0].1 == incidents[1].1 {
            return Err(SurfaceError::InvalidMesh(
                "open, non-manifold or inconsistently oriented edge",
            ));
        }
        adjacency[incidents[0].0].push(incidents[1].0);
        adjacency[incidents[1].0].push(incidents[0].0);
    }
    for link in links {
        if cancelled() {
            return Err(SurfaceError::Cancelled);
        }
        let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (first, second) in link {
            neighbours.entry(first).or_default().push(second);
            neighbours.entry(second).or_default().push(first);
        }
        if neighbours.is_empty() || neighbours.values().any(|adjacent| adjacent.len() != 2) {
            return Err(SurfaceError::InvalidMesh("unused or non-manifold vertex"));
        }
        let Some(&start) = neighbours.keys().next() else {
            return Err(SurfaceError::InvalidMesh("empty vertex link"));
        };
        let mut visited = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(vertex) = pending.pop() {
            if visited.insert(vertex) {
                pending.extend(&neighbours[&vertex]);
            }
        }
        if visited.len() != neighbours.len() {
            return Err(SurfaceError::InvalidMesh("disconnected vertex link"));
        }
    }
    let mut seen = vec![false; triangles.len()];
    let mut components = 0;
    let mut volume = 0.0;
    for seed in 0..triangles.len() {
        if seen[seed] {
            continue;
        }
        components += 1;
        let mut pending = vec![seed];
        while let Some(face) = pending.pop() {
            if cancelled() {
                return Err(SurfaceError::Cancelled);
            }
            if seen[face] {
                continue;
            }
            seen[face] = true;
            volume += face_volumes[face];
            pending.extend(&adjacency[face]);
        }
    }
    if !volume.is_finite() || volume <= 0.0 {
        return Err(SurfaceError::InvalidMesh("nonpositive signed volume"));
    }
    Ok((components, volume))
}

fn subtract(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| first[axis] - second[axis])
}
fn cross(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}
fn dot(first: [f64; 3], second: [f64; 3]) -> f64 {
    first
        .iter()
        .zip(second)
        .map(|(first, second)| first * second)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_voxel_boundary_retains_domain_faces_and_volume() -> Result<(), SurfaceError> {
        let mut densities = vec![1.0; 27];
        densities[13] = 0.0; // enclosed cavity, correctly oriented inner shell
        let grid = DensityGrid {
            dimensions: [0.03, 0.06, 0.09].map(Length::from_metres),
            cells: [3; 3],
            densities: &densities,
        };
        let surface = VoxelBoundaryReconstructor.reconstruct(&grid, 0.5, &|| false)?;
        assert_eq!(surface.components, 2); // boundary components, one solid
        assert!(
            (surface.enclosed_volume.cubic_metres() / (0.01 * 0.02 * 0.03 * 26.0) - 1.0).abs()
                < 1e-12
        );
        for axis in 0..3 {
            assert_eq!(
                surface
                    .vertices
                    .iter()
                    .map(|point| point[axis].metres())
                    .fold(f64::INFINITY, f64::min),
                0.0
            );
            assert!(
                (surface
                    .vertices
                    .iter()
                    .map(|point| point[axis].metres())
                    .fold(0.0, f64::max)
                    - grid.dimensions[axis].metres())
                .abs()
                    < 1e-12
            );
        }
        assert!(matches!(
            VoxelBoundaryReconstructor.reconstruct(&grid, 0.5, &|| true),
            Err(SurfaceError::Cancelled)
        ));
        Ok(())
    }

    #[test]
    fn voxel_boundary_rejects_edge_and_corner_contacts() {
        for last in [3, 7] {
            let mut densities = [0.0; 8];
            densities[0] = 1.0;
            densities[last] = 1.0;
            let grid = DensityGrid {
                dimensions: [0.02; 3].map(Length::from_metres),
                cells: [2; 3],
                densities: &densities,
            };
            assert!(matches!(
                VoxelBoundaryReconstructor.reconstruct(&grid, 0.5, &|| false),
                Err(SurfaceError::InvalidMesh(_))
            ));
        }
    }

    #[test]
    fn full_anisotropic_grid_has_closed_oriented_surface() -> Result<(), SurfaceError> {
        let grid = DensityGrid {
            dimensions: [0.08, 0.04, 0.02].map(Length::from_metres),
            cells: [8, 4, 2],
            densities: &[1.0; 64],
        };
        let mesh = SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| false)?;
        assert_eq!(mesh.components, 1);
        assert!(mesh.enclosed_volume.cubic_metres() > 0.5 * 0.08 * 0.04 * 0.02);
        assert!(mesh.enclosed_volume.cubic_metres() < 0.08 * 0.04 * 0.02);
        for axis in 0..3 {
            let minimum = mesh
                .vertices
                .iter()
                .map(|point| point[axis].metres())
                .fold(f64::INFINITY, f64::min);
            let maximum = mesh
                .vertices
                .iter()
                .map(|point| point[axis].metres())
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(minimum.abs() < 1e-8);
            assert!((maximum - grid.dimensions[axis].metres()).abs() < 1e-8);
        }
        Ok(())
    }

    #[test]
    fn separate_islands_and_threshold_change_are_reported() -> Result<(), SurfaceError> {
        let mut densities = vec![0.0; 6 * 3 * 3];
        densities[(3 + 1) * 3 + 1] = 1.0;
        densities[(4 * 3 + 1) * 3 + 1] = 1.0;
        let grid = DensityGrid {
            dimensions: [0.06, 0.03, 0.03].map(Length::from_metres),
            cells: [6, 3, 3],
            densities: &densities,
        };
        let base = SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| false)?;
        let high = SurfaceNetsReconstructor.reconstruct(&grid, 0.8, &|| false)?;
        assert_eq!(base.components, 2);
        assert_eq!(high.components, 2);
        assert!(high.enclosed_volume.cubic_metres() < base.enclosed_volume.cubic_metres());
        assert!(matches!(
            SurfaceNetsReconstructor.reconstruct(&grid, 0.49, &|| false),
            Err(SurfaceError::InvalidInput)
        ));
        let calls = std::cell::Cell::new(0);
        assert!(matches!(
            SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| {
                calls.set(calls.get() + 1);
                calls.get() > 10
            }),
            Err(SurfaceError::Cancelled)
        ));
        Ok(())
    }

    #[test]
    fn mesh_validator_rejects_open_reversed_and_degenerate_meshes() -> Result<(), SurfaceError> {
        let dimensions = [0.04; 3].map(Length::from_metres);
        let grid = DensityGrid {
            dimensions,
            cells: [4; 3],
            densities: &[1.0; 64],
        };
        let mesh = SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| false)?;
        let mut triangles = mesh.triangles.clone();
        triangles.pop();
        assert!(validate_mesh(&mesh.vertices, &triangles, dimensions, &|| false).is_err());
        let mut triangles = mesh.triangles.clone();
        triangles[0].swap(0, 1);
        assert!(validate_mesh(&mesh.vertices, &triangles, dimensions, &|| false).is_err());
        let mut triangles = mesh.triangles.clone();
        triangles[0][1] = triangles[0][0];
        assert!(validate_mesh(&mesh.vertices, &triangles, dimensions, &|| false).is_err());
        Ok(())
    }

    #[test]
    fn empty_invalid_and_cancelled_are_explicit() {
        let mut grid = DensityGrid {
            dimensions: [0.01; 3].map(Length::from_metres),
            cells: [1; 3],
            densities: &[0.0],
        };
        assert!(matches!(
            SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| false),
            Err(SurfaceError::Empty)
        ));
        assert!(matches!(
            SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| true),
            Err(SurfaceError::Cancelled)
        ));
        grid.densities = &[f64::NAN];
        assert!(matches!(
            SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| false),
            Err(SurfaceError::InvalidInput)
        ));
        grid.cells = [usize::MAX, 1, 1];
        assert!(matches!(
            SurfaceNetsReconstructor.reconstruct(&grid, 0.5, &|| false),
            Err(SurfaceError::InvalidInput)
        ));
    }
}
