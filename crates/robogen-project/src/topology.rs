//! Topology operation discovery, execution and accepted report conversion.

use crate::{ExecutionSettings, TopologyCaseReport, TopologyReport};
use robogen_cad::Mesh;
use robogen_domain::{Diagnostic, OptimizationId};
use robogen_ir::{Feature, Part, SemanticModel, SolidGeometry};
use robogen_topopt::{execution::execute_topology, linear::FaerSolver, TopologyRequest};
use std::collections::BTreeMap;

pub(super) fn part_operations(
    part: &Part,
) -> Vec<&robogen_ir::topology::TopologySpec<SolidGeometry>> {
    part.features
        .iter()
        .flat_map(|feature| match feature {
            Feature::Solid { geometry, .. } => geometry.topology_operations(),
            _ => Vec::new(),
        })
        .collect()
}

pub(super) fn resolve_topology(
    model: &SemanticModel,
    operations: Vec<&robogen_ir::topology::TopologySpec<SolidGeometry>>,
    settings: &ExecutionSettings,
    revision: u64,
    diagnostics: &mut Vec<Diagnostic>,
    cancelled: &dyn Fn() -> bool,
    progress: &dyn Fn(usize, usize),
) -> (
    BTreeMap<OptimizationId, Mesh>,
    BTreeMap<OptimizationId, TopologyReport>,
) {
    let mut resolved = BTreeMap::new();
    let mut reports = BTreeMap::new();
    let total = operations.len();
    for (index, specification) in operations.into_iter().enumerate() {
        if resolved.contains_key(&specification.id) {
            continue;
        }
        if cancelled() {
            diagnostics.push(Diagnostic::error(
                "E321",
                "build cancelled",
                specification.span,
            ));
            return (resolved, reports);
        }
        let issues = specification.preparation_issues();
        if !issues.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E347",
                format!("Incomplete topology preparation: {}", issues.join("; ")),
                specification.span,
            ));
            continue;
        }
        let Some(material) = model
            .materials
            .values()
            .find(|material| Some(material.id) == specification.material)
        else {
            diagnostics.push(Diagnostic::error(
                "E330",
                "topology material is missing",
                specification.span,
            ));
            continue;
        };
        let request = TopologyRequest {
            specification: specification.clone(),
            material: material.clone(),
            document_revision: revision,
        };
        match execute_topology(&request, settings, &FaerSolver, cancelled, &mut |_| {
            progress(index, total)
        }) {
            Ok(result) => {
                reports.insert(
                    specification.id,
                    TopologyReport {
                        name: specification.name.clone(),
                        span: specification.span,
                        material: material.id,
                        document_revision: revision,
                        settings: settings.clone(),
                        solver_backend: result.solid.solver_backend.clone(),
                        origin: result.origin,
                        domain_cells: result.domain_cells,
                        iterations: result.optimization.history.len().saturating_sub(1),
                        volume_fraction: result.solid.volume_fraction,
                        mass: result.mass,
                        cases: result
                            .solid
                            .analyses
                            .iter()
                            .zip(&result.max_displacements)
                            .map(|(case, (_, displacement))| TopologyCaseReport {
                                id: case.id,
                                span: case.span,
                                compliance: case.analysis.compliance,
                                relative_residual: case.analysis.relative_residual,
                                max_displacement: *displacement,
                            })
                            .collect(),
                    },
                );
                let mesh = Mesh {
                    vertices: result
                        .solid
                        .surface
                        .vertices()
                        .iter()
                        .map(|point| robogen_cad::Vertex {
                            position: std::array::from_fn(|axis| {
                                (point[axis].metres() + result.origin[axis].metres()) as f32
                            }),
                        })
                        .collect(),
                    triangles: result
                        .solid
                        .surface
                        .triangles()
                        .iter()
                        .map(|indices| robogen_cad::Triangle {
                            indices: *indices,
                            color: None,
                        })
                        .collect(),
                };
                resolved.insert(specification.id, mesh);
            }
            Err(error) => diagnostics.push(Diagnostic::error(
                "E330",
                format!(
                    "topology `{}`: {}: {}; no optimized geometry published",
                    specification.name, error.diagnostic.code, error.diagnostic.message
                ),
                error.diagnostic.span,
            )),
        }
        progress(index + 1, total);
    }
    (resolved, reports)
}
