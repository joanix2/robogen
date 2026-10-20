//! Parameter overrides and dependency-aware synchronous rebuilding.

use crate::{
    dependencies::{dependencies_for, quantity_expr},
    projection::{build_all_meshes, solve_constraints},
    DirtyNode, OptimizationState, Project, ProjectSnapshot, RebuildReport,
};
use robogen_cad::{build_part_mesh, ResolvedTopologyKernel};
use robogen_domain::{Diagnostic, Quantity, Severity, SourceSpan};
use robogen_dsl::Declaration;
use robogen_ir::lower;
use std::collections::BTreeMap;

impl Project {
    pub fn set_parameter(
        &mut self,
        name: impl Into<String>,
        value: Quantity,
    ) -> Result<(), Diagnostic> {
        let name = name.into();
        let Some(parameter) = self.snapshot.model.parameters.get(&name) else {
            return Err(Diagnostic::error(
                "E300",
                format!("unknown parameter `{name}`"),
                SourceSpan::default(),
            ));
        };
        if parameter.value.kind() != value.kind() {
            return Err(Diagnostic::error(
                "E301",
                format!(
                    "parameter `{name}` expects {:?}, found {:?}",
                    parameter.value.kind(),
                    value.kind()
                ),
                parameter.span,
            ));
        }
        self.overrides.insert(name, value);
        Ok(())
    }

    pub fn rebuild(&mut self) -> Result<RebuildReport, Vec<Diagnostic>> {
        let dirty = dependencies_for(&self.ast, self.overrides.keys());
        let mut ast = self.ast.clone();
        for declaration in &mut ast.declarations {
            if let Declaration::Parameter(parameter) = declaration {
                if let Some(value) = self.overrides.get(&parameter.name).copied() {
                    parameter.value = quantity_expr(value, parameter.value.span);
                }
            }
        }
        let lowered = lower(&ast);
        let mut diagnostics = lowered.diagnostics;
        let mut model = lowered.model;
        solve_constraints(&mut model, &mut diagnostics);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(diagnostics);
        }

        if !model.topology_operations().is_empty() {
            let (meshes, preparation) =
                build_all_meshes(&model, 0, &mut diagnostics, &|| false, &|_, _| {});
            if !diagnostics.is_empty() {
                return Err(diagnostics);
            }
            self.invalidate_topology();
            let optimization_states = preparation
                .keys()
                .map(|id| {
                    (
                        *id,
                        self.snapshot
                            .optimization_states
                            .get(id)
                            .copied()
                            .unwrap_or(OptimizationState::NotCalculated),
                    )
                })
                .collect();
            self.snapshot = ProjectSnapshot {
                model,
                meshes,
                preparation,
                optimization_states,
                topology: BTreeMap::new(),
                mass_diagnostics: BTreeMap::new(),
            };
            return Ok(RebuildReport {
                rebuilt: dirty.into_iter().collect(),
            });
        }
        let resolved = BTreeMap::new();
        let mut meshes = self.snapshot.meshes.clone();
        for node in &dirty {
            if let DirtyNode::Part(name) = node {
                if let Some(part) = model.parts.get(name) {
                    match build_part_mesh(
                        &model,
                        part,
                        &ResolvedTopologyKernel { meshes: &resolved },
                    ) {
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
            }
        }
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(diagnostics);
        }
        self.snapshot = ProjectSnapshot {
            model,
            meshes,
            topology: BTreeMap::new(),
            mass_diagnostics: BTreeMap::new(),
            preparation: BTreeMap::new(),
            optimization_states: BTreeMap::new(),
        };
        Ok(RebuildReport {
            rebuilt: dirty.into_iter().collect(),
        })
    }
}
