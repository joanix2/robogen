//! Display-only density projection, kept separate from exportable geometry.

use crate::{topology::part_operations, Project};
use robogen_cad::{build_part_mesh, Mesh, ResolvedTopologyKernel};
use robogen_domain::{Length, PartId};
use robogen_ir::SolidGeometry;
use std::collections::BTreeMap;

impl Project {
    pub fn diagnostic_mesh(
        &self,
        part_id: PartId,
        threshold: f64,
        reference: bool,
    ) -> Result<Mesh, robogen_cad::CadError> {
        use robogen_cad::{CadKernel, NativeCadKernel};
        let Some(part) = self
            .snapshot
            .model
            .parts
            .values()
            .find(|part| part.id == part_id)
        else {
            return Ok(Mesh::default());
        };
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(robogen_cad::CadError::InvalidSolid {
                span: part.source_span,
                reason: "Invalid diagnostic threshold".into(),
            });
        }
        let mut resolved = BTreeMap::new();
        for operation in part_operations(part) {
            let Some(diagnostic) = self.snapshot.mass_diagnostics.get(&operation.id) else {
                continue;
            };
            let densities = if reference {
                &diagnostic.reference_densities
            } else {
                &diagnostic.densities
            };
            let spacing: [Length; 3] = std::array::from_fn(|axis| {
                Length::from_metres(
                    diagnostic.dimensions[axis].metres() / diagnostic.cells[axis] as f64,
                )
            });
            let cube = NativeCadKernel.solid(&SolidGeometry {
                span: diagnostic.span,
                operation: robogen_ir::SolidOperation::Box { size: spacing },
            })?;
            let mut mesh = Mesh::default();
            let mut shared_vertices = BTreeMap::new();
            let corners: Vec<[usize; 3]> = cube
                .vertices
                .iter()
                .map(|vertex| std::array::from_fn(|axis| usize::from(vertex.position[axis] > 0.0)))
                .collect();
            for (index, density) in densities.iter().enumerate() {
                if *density <= 0.0 || *density < threshold {
                    continue;
                }
                let position = [
                    index / (diagnostic.cells[1] * diagnostic.cells[2]),
                    index / diagnostic.cells[2] % diagnostic.cells[1],
                    index % diagnostic.cells[2],
                ];
                let color =
                    if diagnostic.regions[index] == robogen_topopt::simp::CellRegion::Preserve {
                        [75, 215, 120]
                    } else {
                        [
                            45,
                            (130.0 + 95.0 * density) as u8,
                            (235.0 - 80.0 * density) as u8,
                        ]
                    };
                for triangle in &cube.triangles {
                    let face = triangle.indices.map(|index| corners[index as usize]);
                    let hidden = (0..3).any(|axis| {
                        let side = face[0][axis];
                        if !face.iter().all(|corner| corner[axis] == side) {
                            return false;
                        }
                        let mut neighbor = position;
                        if side == 0 {
                            if position[axis] == 0 {
                                return false;
                            }
                            neighbor[axis] -= 1;
                        } else {
                            if position[axis] + 1 == diagnostic.cells[axis] {
                                return false;
                            }
                            neighbor[axis] += 1;
                        }
                        let neighbor_index = (neighbor[0] * diagnostic.cells[1] + neighbor[1])
                            * diagnostic.cells[2]
                            + neighbor[2];
                        densities[neighbor_index] > 0.0 && densities[neighbor_index] >= threshold
                    });
                    if hidden {
                        continue;
                    }
                    let indices = face.map(|corner| {
                        let node: [usize; 3] =
                            std::array::from_fn(|axis| position[axis] + corner[axis]);
                        *shared_vertices.entry(node).or_insert_with(|| {
                            let index = mesh.vertices.len() as u32;
                            mesh.vertices.push(robogen_cad::Vertex {
                                position: std::array::from_fn(|axis| {
                                    (diagnostic.origin[axis].metres()
                                        + node[axis] as f64 * spacing[axis].metres())
                                        as f32
                                }),
                            });
                            index
                        })
                    });
                    mesh.triangles.push(robogen_cad::Triangle {
                        indices,
                        color: Some(color),
                    });
                }
            }
            resolved.insert(operation.id, mesh);
        }
        if resolved.is_empty() {
            return Ok(Mesh::default());
        }
        build_part_mesh(
            &self.snapshot.model,
            part,
            &ResolvedTopologyKernel { meshes: &resolved },
        )
    }
}
