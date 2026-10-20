//! Constraint resolution and CAD/preparation snapshot projection.

use crate::topology::part_operations;
use robogen_cad::{build_part_mesh_cancellable, CadKernel, Mesh};
use robogen_constraints::{ConstraintSolver, NativeConstraintSolver};
use robogen_domain::{Diagnostic, ObjectInstanceId, PartId};
use robogen_ir::{Feature, SemanticModel};
use std::collections::BTreeMap;

pub(super) fn solve_constraints(model: &mut SemanticModel, diagnostics: &mut Vec<Diagnostic>) {
    for sketch in model.sketches.values_mut() {
        match NativeConstraintSolver.solve(sketch) {
            Ok(report) => *sketch = report.sketch,
            Err(error) => diagnostics.push(Diagnostic::error(
                "E310",
                error.to_string(),
                sketch.source_span,
            )),
        }
    }
}

pub(super) fn build_all_meshes(
    model: &SemanticModel,
    _revision: u64,
    diagnostics: &mut Vec<Diagnostic>,
    cancelled: &dyn Fn() -> bool,
    progress: &dyn Fn(usize, usize),
) -> (BTreeMap<PartId, Mesh>, BTreeMap<PartId, [Mesh; 3]>) {
    let mut meshes = BTreeMap::new();
    let mut preparation = BTreeMap::new();
    let total = model.parts.len();
    progress(0, total);
    for (index, part) in model.parts.values().enumerate() {
        if !part_operations(part).is_empty() {
            let mut layers: [Mesh; 3] = std::array::from_fn(|_| Mesh::default());
            for feature in &part.features {
                if let Feature::Solid { geometry, .. } = feature {
                    match robogen_cad::topology_preview(geometry, cancelled) {
                        Ok(projected) => {
                            for (layer, mesh) in layers.iter_mut().zip(projected) {
                                layer.append(&mesh);
                            }
                        }
                        Err(error) => diagnostics.push(Diagnostic::error(
                            "E320",
                            error.to_string(),
                            part.source_span,
                        )),
                    }
                }
            }
            preparation.insert(part.id, layers);
        } else {
            match build_part_mesh_cancellable(model, part, &robogen_cad::NativeCadKernel, cancelled)
            {
                Ok(mesh) => {
                    meshes.insert(part.id, mesh);
                }
                Err(error) => diagnostics.push(Diagnostic::error(
                    "E320",
                    error.to_string(),
                    part.source_span,
                )),
            }
        }
        progress(index + 1, total);
        if cancelled() {
            diagnostics.push(Diagnostic::error(
                "E321",
                "Preparation cancelled",
                part.source_span,
            ));
            break;
        }
    }
    (meshes, preparation)
}

pub(super) fn build_instance_interfaces(
    model: &SemanticModel,
    cancelled: &dyn Fn() -> bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<ObjectInstanceId, Mesh> {
    let mut interfaces = BTreeMap::new();
    for instance in model.instances.values() {
        let mut combined = Mesh::default();
        for geometry in instance.interfaces.values() {
            match robogen_cad::NativeCadKernel.solid_cancellable(geometry, cancelled) {
                Ok(mesh) => combined.append(&mesh),
                Err(error) => {
                    diagnostics.push(Diagnostic::error("E320", error.to_string(), instance.span))
                }
            }
        }
        if !combined.triangles.is_empty() {
            interfaces.insert(instance.id, combined);
        }
    }
    interfaces
}
