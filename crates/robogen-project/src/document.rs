//! Undoable source history and background build lifecycle.

use crate::{
    compilation::CompilationWorker, topology::part_operations, ExecutionSettings,
    OptimizationState, Project,
};
use robogen_domain::{Diagnostic, PartId, SourceSpan};
use std::collections::{BTreeMap, VecDeque};

/// An editable source with synchronous validation and bounded source-only history.
/// Whole-source replacement is the undoable command; this is not a general command stack.
pub struct SourceDocument {
    source: String,
    project: Option<Project>,
    diagnostics: Vec<Diagnostic>,
    is_valid: bool,
    undo: VecDeque<String>,
    redo: Vec<String>,
    worker: Option<CompilationWorker>,
    building: bool,
    optimizing: Option<PartId>,
    optimization_errors: BTreeMap<PartId, Vec<Diagnostic>>,
}

impl SourceDocument {
    /// Maximum number of source replacements retained for undo/redo, not a byte limit.
    pub const HISTORY_LIMIT: usize = 128;

    pub fn new(source: impl Into<String>) -> Self {
        let mut document = Self {
            source: source.into(),
            project: None,
            diagnostics: Vec::new(),
            is_valid: false,
            undo: VecDeque::new(),
            redo: Vec::new(),
            worker: None,
            building: false,
            optimizing: None,
            optimization_errors: BTreeMap::new(),
        };
        document.refresh();
        document
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn enable_background_compilation(&mut self) -> std::io::Result<()> {
        if self.worker.is_none() {
            self.worker = Some(CompilationWorker::new()?);
        }
        Ok(())
    }

    pub fn build_progress(&self) -> Option<(usize, usize)> {
        self.worker
            .as_ref()
            .filter(|_| self.building)
            .map(CompilationWorker::progress)
    }

    pub fn cancel_build(&mut self) {
        if !self.building {
            return;
        }
        if let Some(worker) = &self.worker {
            worker.cancel();
        }
        self.building = false;
        if let Some(part) = self.optimizing.take() {
            if let Some(project) = &mut self.project {
                project
                    .snapshot
                    .optimization_states
                    .insert(part, OptimizationState::Cancelled);
            }
            return;
        }
        self.diagnostics = vec![Diagnostic::error(
            "E321",
            "CAD build cancelled",
            SourceSpan::default(),
        )];
    }

    pub fn poll_compilation(&mut self) -> bool {
        self.poll_optimization_progress();
        let Some(result) = self.worker.as_ref().and_then(CompilationWorker::poll) else {
            return false;
        };
        if let Some(part) = self.optimizing.take() {
            self.building = false;
            match result {
                Ok(project) => self.project = Some(project),
                Err(diagnostics) => {
                    self.optimization_errors.insert(part, diagnostics);
                    if let Some(project) = &mut self.project {
                        project
                            .snapshot
                            .optimization_states
                            .insert(part, OptimizationState::Failed);
                    }
                }
            }
        } else {
            self.accept_result(result);
        }
        true
    }

    pub fn optimization_diagnostics(&self, part: PartId) -> &[Diagnostic] {
        self.optimization_errors
            .get(&part)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn poll_optimization_progress(&mut self) -> bool {
        let Some((part, diagnostic)) = self
            .worker
            .as_ref()
            .and_then(CompilationWorker::poll_diagnostic)
        else {
            return false;
        };
        if self.optimizing != Some(part) || !self.building {
            return false;
        }
        let Some(project) = &mut self.project else {
            return false;
        };
        project
            .snapshot
            .mass_diagnostics
            .insert(diagnostic.id, diagnostic);
        true
    }

    pub fn start_optimization(
        &mut self,
        part: PartId,
        settings: ExecutionSettings,
    ) -> Result<(), Diagnostic> {
        let error = |message: &str| Diagnostic::error("E330", message, SourceSpan::default());
        if !self.is_valid || self.building {
            return Err(error(
                "Document unavailable or another operation is running",
            ));
        }
        self.enable_background_compilation()
            .map_err(|_| error("Worker unavailable"))?;
        let project = self
            .project
            .as_mut()
            .ok_or_else(|| error("No prepared document"))?;
        if !project.snapshot.preparation.contains_key(&part) {
            return Err(error("Part has no topology declaration"));
        }
        let ids: Vec<_> = project
            .snapshot
            .model
            .parts
            .values()
            .filter(|item| item.id == part)
            .flat_map(part_operations)
            .map(|operation| operation.id)
            .collect();
        for id in ids {
            project.snapshot.topology.remove(&id);
            project.snapshot.mass_diagnostics.remove(&id);
        }
        project.snapshot.meshes.remove(&part);
        project
            .snapshot
            .optimization_states
            .insert(part, OptimizationState::Running);
        self.optimization_errors.remove(&part);
        let worker = self
            .worker
            .as_ref()
            .ok_or_else(|| error("Worker unavailable"))?;
        self.building = worker.optimize(project.clone(), part, settings);
        if !self.building {
            project
                .snapshot
                .optimization_states
                .insert(part, OptimizationState::Failed);
            return Err(error("Worker unavailable"));
        }
        self.optimizing = Some(part);
        Ok(())
    }

    /// Last successfully built project, possibly stale when `is_valid()` is false.
    /// Its `source()` identifies the text that produced the display snapshot.
    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    /// Diagnostics for the current editor source, not the last valid project.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether the project snapshot represents the current editor source.
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    pub fn has_errors(&self) -> bool {
        !self.is_valid
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Replaces source as one command, including invalid edits. Returns whether text changed.
    /// Identical text does not rebuild or alter history; a new edit discards redo.
    pub fn replace_source(&mut self, source: String) -> bool {
        if self.source == source {
            return false;
        }
        if self.undo.len() == Self::HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo
            .push_back(std::mem::replace(&mut self.source, source));
        self.redo.clear();
        self.refresh();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(source) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.source, source));
        self.refresh();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(source) = self.redo.pop() else {
            return false;
        };
        self.undo
            .push_back(std::mem::replace(&mut self.source, source));
        self.refresh();
        true
    }

    fn refresh(&mut self) {
        self.optimizing = None;
        self.optimization_errors.clear();
        if let Some(project) = &mut self.project {
            project.invalidate_topology();
        }
        if let Some(worker) = &self.worker {
            self.is_valid = false;
            self.diagnostics.clear();
            self.building = worker.submit(self.source.clone());
            if !self.building {
                self.diagnostics.push(Diagnostic::error(
                    "E321",
                    "CAD worker unavailable",
                    SourceSpan::default(),
                ));
            }
        } else {
            self.accept_result(Project::from_source(self.source.clone()));
        }
    }

    fn accept_result(&mut self, result: Result<Project, Vec<Diagnostic>>) {
        self.building = false;
        match result {
            Ok(mut project) => {
                if let Some(previous) = &self.project {
                    for (id, state) in &mut project.snapshot.optimization_states {
                        if previous.snapshot.optimization_states.get(id)
                            == Some(&OptimizationState::Obsolete)
                        {
                            *state = OptimizationState::Obsolete;
                        }
                    }
                }
                self.project = Some(project);
                self.diagnostics.clear();
                self.is_valid = true;
            }
            Err(diagnostics) => {
                self.diagnostics = diagnostics;
                self.is_valid = false;
            }
        }
    }
}
