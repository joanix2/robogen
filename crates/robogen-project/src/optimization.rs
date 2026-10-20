//! Explicit topology execution and invalidation of derived results.

use crate::{
    topology::{part_operations, resolve_topology},
    ExecutionSettings, MassDiagnostic, OptimizationState, Project, TopologyObjective,
};
use robogen_cad::{build_part_mesh_cancellable, ResolvedTopologyKernel};
use robogen_domain::{Diagnostic, PartId, SourceSpan};
use robogen_topopt::TopologyRequest;

impl Project {
    pub fn optimize_part(
        &mut self,
        part_id: PartId,
        settings: &ExecutionSettings,
        revision: u64,
        cancelled: &dyn Fn() -> bool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<(), Vec<Diagnostic>> {
        self.optimize_part_with_diagnostics(
            part_id,
            settings,
            revision,
            cancelled,
            progress,
            &|_| {},
        )
    }

    pub(super) fn optimize_part_with_diagnostics(
        &mut self,
        part_id: PartId,
        settings: &ExecutionSettings,
        revision: u64,
        cancelled: &dyn Fn() -> bool,
        progress: &dyn Fn(usize, usize),
        diagnostic: &dyn Fn(&MassDiagnostic),
    ) -> Result<(), Vec<Diagnostic>> {
        let part = self
            .snapshot
            .model
            .parts
            .values()
            .find(|part| part.id == part_id)
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    "E330",
                    "Unknown optimization part",
                    SourceSpan::default(),
                )]
            })?;
        let operations = part_operations(part);
        if operations.is_empty() {
            return Err(vec![Diagnostic::error(
                "E330",
                "Part has no topology declaration",
                part.source_span,
            )]);
        }
        if operations
            .iter()
            .any(|spec| spec.objective == TopologyObjective::MinimizeMass)
        {
            if operations.len() != 1 {
                return Err(vec![Diagnostic::error(
                    "E348",
                    "Mass diagnostics currently require one topology operation per part",
                    part.source_span,
                )]);
            }
            let specification = operations[0];
            let issues = specification.preparation_issues();
            if !issues.is_empty() {
                return Err(vec![Diagnostic::error(
                    "E347",
                    format!("Incomplete topology preparation: {}", issues.join("; ")),
                    specification.span,
                )]);
            }
            let material = self
                .snapshot
                .model
                .materials
                .values()
                .find(|material| Some(material.id) == specification.material)
                .ok_or_else(|| {
                    vec![Diagnostic::error(
                        "E330",
                        "Topology material is missing",
                        specification.span,
                    )]
                })?;
            let request = TopologyRequest {
                specification: specification.clone(),
                material: material.clone(),
                document_revision: revision,
            };
            let result =
                robogen_topopt::mass::diagnose_mass(&request, settings, cancelled, &mut |event| {
                    progress(
                        event.history.len().min(settings.max_iterations),
                        settings.max_iterations,
                    );
                    diagnostic(event);
                })
                .map_err(|error| {
                    vec![Diagnostic::error(
                        "E330",
                        format!("{}: {}", error.diagnostic.code, error.diagnostic.message),
                        request.specification.span,
                    )]
                })?;
            if cancelled() {
                return Err(vec![Diagnostic::error(
                    "E345",
                    "Mass calculation cancelled",
                    request.specification.span,
                )]);
            }
            self.snapshot.topology.remove(&result.id);
            self.snapshot.mass_diagnostics.insert(result.id, result);
            self.snapshot.meshes.remove(&part_id);
            self.snapshot
                .optimization_states
                .insert(part_id, OptimizationState::Diagnostic);
            return Ok(());
        }
        let mut diagnostics = Vec::new();
        let (resolved, reports) = resolve_topology(
            &self.snapshot.model,
            operations,
            settings,
            revision,
            &mut diagnostics,
            cancelled,
            progress,
        );
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        let mesh = build_part_mesh_cancellable(
            &self.snapshot.model,
            part,
            &ResolvedTopologyKernel { meshes: &resolved },
            cancelled,
        )
        .map_err(|error| {
            vec![Diagnostic::error(
                "E320",
                error.to_string(),
                part.source_span,
            )]
        })?;
        if cancelled() {
            return Err(vec![Diagnostic::error(
                "E321",
                "Optimization cancelled",
                part.source_span,
            )]);
        }
        self.snapshot.meshes.insert(part_id, mesh);
        self.snapshot.topology.extend(reports);
        self.snapshot
            .optimization_states
            .insert(part_id, OptimizationState::Calculated);
        Ok(())
    }
    pub(super) fn invalidate_topology(&mut self) {
        self.snapshot.topology.clear();
        self.snapshot.mass_diagnostics.clear();
        for (id, state) in &mut self.snapshot.optimization_states {
            self.snapshot.meshes.remove(id);
            if matches!(
                state,
                OptimizationState::Calculated
                    | OptimizationState::Running
                    | OptimizationState::Diagnostic
            ) {
                *state = OptimizationState::Obsolete;
            }
        }
    }
}
