//! Source compilation and construction of a prepared project snapshot.

use crate::{
    manifest::load_manifest,
    projection::{build_all_meshes, build_instance_interfaces, solve_constraints},
    topology::part_operations,
    OptimizationState, Project, ProjectIoError, ProjectManifest, ProjectSnapshot,
};
use robogen_domain::{Diagnostic, PartId, Severity};
use robogen_ir::{lower, SemanticModel, SolidGeometry};
use std::{collections::BTreeMap, path::Path};

pub fn compile_source(source: &str) -> Result<SemanticModel, Vec<Diagnostic>> {
    let parsed = robogen_dsl::parse(source);
    let mut diagnostics = parsed.diagnostics;
    let Some(module) = parsed.module else {
        return Err(diagnostics);
    };
    let lowered = lower(&module);
    diagnostics.extend(lowered.diagnostics);
    let mut model = lowered.model;
    solve_constraints(&mut model, &mut diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        Err(diagnostics)
    } else {
        Ok(model)
    }
}

impl Project {
    pub fn from_source(source: impl Into<String>) -> Result<Self, Vec<Diagnostic>> {
        Self::from_source_controlled(source.into(), 0, &|| false, &|_, _| {})
    }

    pub(super) fn from_source_controlled(
        source: String,
        revision: u64,
        cancelled: &dyn Fn() -> bool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Self, Vec<Diagnostic>> {
        let parsed = robogen_dsl::parse(&source);
        let mut diagnostics = parsed.diagnostics;
        let Some(ast) = parsed.module else {
            return Err(diagnostics);
        };
        let lowered = lower(&ast);
        diagnostics.extend(lowered.diagnostics);
        let mut model = lowered.model;
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(diagnostics);
        }
        solve_constraints(&mut model, &mut diagnostics);
        let (meshes, preparation) =
            build_all_meshes(&model, revision, &mut diagnostics, cancelled, progress);
        let instance_interfaces = build_instance_interfaces(&model, cancelled, &mut diagnostics);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(diagnostics);
        }
        let optimization_states = preparation
            .keys()
            .map(|id| (*id, OptimizationState::NotCalculated))
            .collect();
        Ok(Self {
            source,
            ast,
            overrides: BTreeMap::new(),
            snapshot: ProjectSnapshot {
                model,
                meshes,
                topology: BTreeMap::new(),
                mass_diagnostics: BTreeMap::new(),
                preparation,
                instance_interfaces,
                optimization_states,
            },
        })
    }

    pub fn open(path: &Path) -> Result<(ProjectManifest, Self), ProjectIoError> {
        let (manifest, source) = load_manifest(path)?;
        let project = Self::from_source(source).map_err(ProjectIoError::Diagnostics)?;
        Ok((manifest, project))
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn snapshot(&self) -> &ProjectSnapshot {
        &self.snapshot
    }

    pub fn topology_operations(
        &self,
        part: PartId,
    ) -> Vec<&robogen_ir::topology::TopologySpec<SolidGeometry>> {
        self.snapshot
            .model
            .parts
            .values()
            .find(|item| item.id == part)
            .map(part_operations)
            .unwrap_or_default()
    }
}
