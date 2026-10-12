//! Project loading and incremental orchestration for the shared model pipeline.

mod compilation;
mod library;
pub use library::{FunctionDefinition, LibraryGenerator, TaxonomyFeature, TaxonomyInstance};

use compilation::CompilationWorker;
use robogen_cad::{build_part_mesh, build_part_mesh_cancellable, Mesh, ResolvedTopologyKernel};
use robogen_constraints::{ConstraintSolver, NativeConstraintSolver};
use robogen_domain::{
    Diagnostic, Length, LoadCaseId, Mass, MaterialId, OptimizationId, PartId, Quantity, Severity,
    SourceSpan,
};
use robogen_dsl::{Declaration, Expr, ExprKind, Module};
pub use robogen_ir::topology::Objective as TopologyObjective;
use robogen_ir::{lower, Feature, Part, SemanticModel, SolidGeometry};
pub use robogen_topopt::execution::{
    grid_resolution, resolution_for_cell_size, ComputeBackend, ExecutionSettings, GridResolution,
};
pub use robogen_topopt::mass::{MassDiagnostic, MASS_BACKEND};
use robogen_topopt::{execution::execute_topology, linear::FaerSolver, TopologyRequest};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    #[serde(default = "manifest_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub source: PathBuf,
}

const fn manifest_schema_version() -> u32 {
    1
}

#[derive(Debug, Error)]
pub enum ProjectIoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] toml::de::Error),
    #[error("project source contains errors")]
    Diagnostics(Vec<Diagnostic>),
    #[error("project source path must stay inside the project: {0}")]
    UnsafeSource(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectSnapshot {
    pub model: SemanticModel,
    pub meshes: BTreeMap<PartId, Mesh>,
    pub topology: BTreeMap<OptimizationId, TopologyReport>,
    pub mass_diagnostics: BTreeMap<OptimizationId, MassDiagnostic>,
    pub preparation: BTreeMap<PartId, [Mesh; 3]>,
    pub optimization_states: BTreeMap<PartId, OptimizationState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizationState {
    NotCalculated,
    Running,
    Calculated,
    Diagnostic,
    Obsolete,
    Failed,
    Cancelled,
}

pub fn topology_settings() -> ExecutionSettings {
    ExecutionSettings {
        compute_backend: ComputeBackend::Cpu,
        cells: [12, 4, 2],
        filter_radius: Length::from_millimetres(12.0),
        threshold: 0.65,
        max_iterations: 100,
    }
}

fn part_operations(part: &Part) -> Vec<&robogen_ir::topology::TopologySpec<SolidGeometry>> {
    part.features
        .iter()
        .flat_map(|feature| match feature {
            Feature::Solid { geometry, .. } => geometry.topology_operations(),
            _ => Vec::new(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopologyReport {
    pub name: String,
    pub span: SourceSpan,
    pub material: MaterialId,
    pub document_revision: u64,
    pub settings: ExecutionSettings,
    pub solver_backend: String,
    pub origin: [Length; 3],
    pub domain_cells: usize,
    pub iterations: usize,
    pub volume_fraction: f64,
    pub mass: Mass,
    pub cases: Vec<TopologyCaseReport>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopologyCaseReport {
    pub id: LoadCaseId,
    pub span: SourceSpan,
    pub compliance: robogen_topopt::fem::Compliance,
    pub relative_residual: f64,
    pub max_displacement: Length,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirtyNode {
    Parameter(String),
    Sketch(String),
    Part(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RebuildReport {
    pub rebuilt: Vec<DirtyNode>,
}

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

pub fn load_manifest(path: &Path) -> Result<(ProjectManifest, String), ProjectIoError> {
    let manifest: ProjectManifest = toml::from_str(&fs::read_to_string(path)?)?;
    if manifest.source.is_absolute()
        || manifest
            .source
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ProjectIoError::UnsafeSource(manifest.source));
    }
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let source = fs::read_to_string(root.join(&manifest.source))?;
    Ok((manifest, source))
}

#[derive(Clone)]
pub struct Project {
    source: String,
    ast: Module,
    overrides: BTreeMap<String, Quantity>,
    snapshot: ProjectSnapshot,
}

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

impl Project {
    pub fn from_source(source: impl Into<String>) -> Result<Self, Vec<Diagnostic>> {
        Self::from_source_controlled(source.into(), 0, &|| false, &|_, _| {})
    }

    fn from_source_controlled(
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

    fn optimize_part_with_diagnostics(
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

    fn invalidate_topology(&mut self) {
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

fn solve_constraints(model: &mut SemanticModel, diagnostics: &mut Vec<Diagnostic>) {
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

fn resolve_topology(
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

fn build_all_meshes(
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

fn dependencies_for<'a>(
    ast: &Module,
    parameters: impl Iterator<Item = &'a String>,
) -> BTreeSet<DirtyNode> {
    let mut parameters: BTreeSet<&str> = parameters.map(String::as_str).collect();
    loop {
        let before = parameters.len();
        for declaration in &ast.declarations {
            if let Declaration::Parameter(parameter) = declaration {
                if references_any(&parameter.value, &parameters) {
                    parameters.insert(parameter.name.as_str());
                }
            }
        }
        if parameters.len() == before {
            break;
        }
    }
    let mut dirty = BTreeSet::new();
    for parameter in &parameters {
        dirty.insert(DirtyNode::Parameter((*parameter).to_owned()));
    }
    let mut sketches = BTreeSet::new();
    for declaration in &ast.declarations {
        if let Declaration::Sketch(sketch) = declaration {
            let depends = sketch.rectangles.iter().any(|rect| {
                [&rect.origin_x, &rect.origin_y, &rect.width, &rect.height]
                    .into_iter()
                    .any(|expr| references_any(expr, &parameters))
            }) || sketch
                .constraints
                .iter()
                .flat_map(|constraint| &constraint.args)
                .any(|expr| references_any(expr, &parameters));
            if depends {
                sketches.insert(sketch.name.as_str());
                dirty.insert(DirtyNode::Sketch(sketch.name.clone()));
            }
        }
    }
    for declaration in &ast.declarations {
        if let Declaration::Part(part) = declaration {
            let dependencies: BTreeSet<_> = sketches.union(&parameters).copied().collect();
            let depends = part.features.iter().any(|feature| {
                (!parameters.is_empty() && feature.operation != "extrude")
                    || feature
                        .args
                        .iter()
                        .any(|expression| references_any(expression, &dependencies))
            });
            if depends {
                dirty.insert(DirtyNode::Part(part.name.clone()));
            }
        }
    }
    dirty
}

fn references_any(expression: &Expr, names: &BTreeSet<&str>) -> bool {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Reference(name) if names.contains(name.as_str()) => return true,
            ExprKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Unary { value, .. } | ExprKind::Named { value, .. } => pending.push(value),
            ExprKind::Vector(values) | ExprKind::Call { args: values, .. } => {
                pending.extend(values)
            }
            _ => {}
        }
    }
    false
}

fn quantity_expr(value: Quantity, span: SourceSpan) -> Expr {
    let (value, unit) = match value {
        Quantity::Scalar(value) => (value, None),
        Quantity::Length(value) => (value.metres(), Some("m".into())),
        Quantity::Angle(value) => (value.radians(), Some("rad".into())),
        Quantity::Density(value) => (value.kg_per_m3(), Some("kg/m3".into())),
        Quantity::Pressure(value) => (value.pascals(), Some("Pa".into())),
        Quantity::Force(value) => (value.newtons(), Some("N".into())),
        Quantity::Torque(value) => (value.newton_metres(), Some("Nm".into())),
        Quantity::Mass(value) => (value.kilograms(), Some("kg".into())),
        Quantity::Frequency(value) => (value.hertz(), Some("Hz".into())),
    };
    Expr {
        kind: ExprKind::Number { value, unit },
        span,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn topology_execution_cancels_before_publication() -> Result<(), Vec<Diagnostic>> {
        let calls = std::cell::Cell::new(0);
        let mut project =
            Project::from_source(include_str!("../../../examples/topology_manual/main.rgn"))?;
        let part = *project
            .snapshot()
            .preparation
            .keys()
            .next()
            .ok_or_else(Vec::new)?;
        let result = project.optimize_part(
            part,
            &topology_settings(),
            7,
            &|| calls.get() > 2,
            &|_, _| calls.set(calls.get() + 1),
        );
        let diagnostics = result.err().ok_or_else(Vec::new)?;
        assert!(diagnostics
            .iter()
            .any(|error| error.message.contains("E345")));
        assert!(project.snapshot().meshes.is_empty());
        Ok(())
    }

    use super::*;
    use robogen_domain::Length;
    const SOURCE: &str = r#"module demo;
parameter WIDTH = 40 mm;
parameter HEIGHT = 20 mm;
parameter DEPTH = 4 mm;
material PLA { density: 1240 kg/m3; young: 3.5 GPa; poisson: 0.36; yield_strength: 50 MPa; }
sketch Profile on XY { rectangle outline { origin: (0 mm, 0 mm); width: WIDTH; height: HEIGHT; } }
part Body { material: PLA; base = extrude(Profile, DEPTH); }"#;

    #[test]
    fn source_commands_rebuild_valid_edits_and_restore_source() -> Result<(), &'static str> {
        let mut document = SourceDocument::new(SOURCE);
        assert!(document.is_valid());
        assert!(!document.has_errors());
        assert!(document.diagnostics().is_empty());
        assert!(!document.can_undo());
        assert!(!document.can_redo());
        assert!(!document.undo());
        assert!(!document.redo());

        let edited = SOURCE.replace("40 mm", "70 mm");
        assert!(document.replace_source(edited.clone()));
        let project = document.project().ok_or("missing valid project")?;
        assert_eq!(project.source(), edited);
        assert_eq!(
            project.snapshot().model.parameters["WIDTH"].value,
            Quantity::Length(Length::from_millimetres(70.0))
        );
        assert!(document.undo());
        assert_eq!(document.source(), SOURCE);
        assert_eq!(
            document.project().ok_or("missing undo project")?.source(),
            SOURCE
        );
        assert!(document.redo());
        assert_eq!(document.source(), edited);
        assert_eq!(
            document.project().ok_or("missing redo project")?.source(),
            edited
        );
        assert!(document.is_valid());
        Ok(())
    }

    #[test]
    fn invalid_edits_keep_source_diagnostics_and_last_valid_project() -> Result<(), &'static str> {
        let mut document = SourceDocument::new(SOURCE);
        let invalid = "parameter WIDTH = ;";
        assert!(document.replace_source(invalid.into()));
        assert_eq!(document.source(), invalid);
        assert!(document.has_errors());
        assert!(!document.is_valid());
        assert!(!document.diagnostics().is_empty());
        assert_eq!(
            document.project().ok_or("lost valid project")?.source(),
            SOURCE
        );
        let diagnostics = document.diagnostics().to_vec();
        assert!(document.undo());
        assert_eq!(document.source(), SOURCE);
        assert!(document.is_valid());
        assert!(document.diagnostics().is_empty());
        assert!(document.redo());
        assert_eq!(document.source(), invalid);
        assert_eq!(document.diagnostics(), diagnostics);
        assert!(!document.is_valid());
        assert_eq!(
            document.project().ok_or("lost redo snapshot")?.source(),
            SOURCE
        );
        Ok(())
    }

    #[test]
    fn invalid_initial_source_can_recover_and_be_undone() -> Result<(), &'static str> {
        let invalid = "parameter WIDTH = ;";
        let mut document = SourceDocument::new(invalid);
        assert_eq!(document.source(), invalid);
        assert!(document.project().is_none());
        assert!(document.has_errors());
        assert!(!document.diagnostics().is_empty());
        assert!(!document.can_undo());
        assert!(!document.can_redo());
        assert!(document.replace_source(SOURCE.into()));
        assert!(document.is_valid());
        assert!(document.undo());
        assert_eq!(document.source(), invalid);
        assert!(!document.is_valid());
        assert_eq!(
            document
                .project()
                .ok_or("lost recovered snapshot")?
                .source(),
            SOURCE
        );
        assert!(document.redo());
        assert!(document.is_valid());
        assert!(document.diagnostics().is_empty());
        Ok(())
    }

    #[test]
    fn fine_density_preview_removes_internal_faces_without_changing_bounds() -> Result<(), String> {
        let mut project =
            Project::from_source(include_str!("../../../examples/topology_manual/main.rgn"))
                .map_err(|errors| format!("{errors:?}"))?;
        let part = *project
            .snapshot
            .preparation
            .keys()
            .next()
            .ok_or("missing part")?;
        let operation = project.topology_operations(part)[0].clone();
        let cells = [48, 16, 8];
        project.snapshot.mass_diagnostics.insert(
            operation.id,
            MassDiagnostic {
                compute_backend: ComputeBackend::Cpu,
                id: operation.id,
                name: operation.name,
                span: operation.span,
                document_revision: 0,
                origin: [Length::from_metres(0.0); 3],
                dimensions: [0.12, 0.04, 0.02].map(Length::from_metres),
                cells,
                regions: vec![robogen_topopt::simp::CellRegion::Design; 6144],
                densities: vec![1.0; 6144],
                reference_densities: vec![1.0; 6144],
                mass: Mass::from_kilograms(0.2592),
                reference_mass: Mass::from_kilograms(0.2592),
                cases: Vec::new(),
                history: Vec::new(),
                finished: false,
                feasible: false,
                converged: false,
                kkt_residual: None,
                binary: None,
                reconstruction_error: None,
            },
        );
        let mesh = project
            .diagnostic_mesh(part, 0.35, false)
            .map_err(|error| error.to_string())?;
        assert_eq!(mesh.triangles.len(), 4 * (48 * 16 + 48 * 8 + 16 * 8));
        assert!(mesh.vertices.len() < 3000);
        for (axis, expected) in [0.12_f32, 0.04, 0.02].into_iter().enumerate() {
            let maximum = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position[axis])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!((maximum - expected).abs() < 1e-6);
        }
        assert!(project.snapshot.meshes.is_empty());
        Ok(())
    }

    #[test]
    fn source_document_preserves_pipeline_error_diagnostics() -> Result<(), &'static str> {
        for source in [
            "parameter WIDTH = ;".to_owned(),
            SOURCE.replace("40 mm", "40 kg"),
            SOURCE.replace("40 mm", "0 mm"),
            SOURCE.replace("4 mm", "0 mm"),
        ] {
            let Err(expected) = Project::from_source(source.clone()) else {
                return Err("fixture must fail project compilation");
            };
            assert!(!expected.is_empty());
            let document = SourceDocument::new(source.clone());
            assert_eq!(document.source(), source);
            assert!(document.project().is_none());
            assert!(document.has_errors());
            assert_eq!(document.diagnostics(), expected);
            let mut document = SourceDocument::new(SOURCE);
            assert!(document.replace_source(source));
            assert_eq!(document.diagnostics(), expected);
            assert!(!document.is_valid());
            assert_eq!(
                document.project().ok_or("lost valid project")?.source(),
                SOURCE
            );
        }
        Ok(())
    }

    #[test]
    fn no_op_preserves_redo_and_new_edit_discards_it() {
        let mut document = SourceDocument::new(SOURCE);
        assert!(!document.replace_source(SOURCE.into()));
        assert!(!document.can_undo());
        assert!(document.replace_source(SOURCE.replace("40 mm", "70 mm")));
        assert!(document.undo());
        assert!(document.can_redo());
        assert!(!document.replace_source(SOURCE.into()));
        assert!(document.can_redo());
        assert!(document.replace_source("parameter WIDTH = ;".into()));
        assert!(!document.can_redo());
        assert!(!document.redo());
        let diagnostics = document.diagnostics().to_vec();
        assert!(!document.replace_source(document.source().to_owned()));
        assert_eq!(document.diagnostics(), diagnostics);
        assert!(document.undo());
        assert_eq!(document.source(), SOURCE);
        assert!(!document.can_undo());
    }

    #[test]
    fn source_history_is_bounded_across_undo_redo_and_branching() {
        let mut document = SourceDocument::new("module demo0;");
        let edits = SourceDocument::HISTORY_LIMIT + 2;
        for step in 1..=edits {
            assert!(document.replace_source(format!("module demo{step};")));
            assert!(document.is_valid());
        }
        for _ in 0..SourceDocument::HISTORY_LIMIT {
            assert!(document.undo());
        }
        assert_eq!(document.source(), "module demo2;");
        assert!(!document.can_undo());
        assert!(!document.undo());
        for _ in 0..SourceDocument::HISTORY_LIMIT {
            assert!(document.redo());
        }
        assert_eq!(document.source(), format!("module demo{edits};"));
        assert!(!document.can_redo());
        assert!(!document.redo());
        assert!(document.undo());
        assert!(document.replace_source("module branch;".into()));
        assert!(!document.can_redo());
        for _ in 0..SourceDocument::HISTORY_LIMIT {
            assert!(document.undo());
        }
        assert_eq!(document.source(), "module demo2;");
        assert!(!document.undo());
    }

    #[test]
    fn parameter_rebuild_changes_only_descendants_and_keeps_ids() -> Result<(), Vec<Diagnostic>> {
        let mut project = Project::from_source(SOURCE)?;
        let part_id = project.snapshot().model.parts["Body"].id;
        project
            .set_parameter("WIDTH", Quantity::Length(Length::from_millimetres(80.0)))
            .map_err(|diagnostic| vec![diagnostic])?;
        let report = project.rebuild()?;
        assert!(report
            .rebuilt
            .contains(&DirtyNode::Sketch("Profile".into())));
        assert!(report.rebuilt.contains(&DirtyNode::Part("Body".into())));
        assert_eq!(project.snapshot().model.parts["Body"].id, part_id);
        assert_eq!(
            project.snapshot().meshes[&part_id].vertices[1].position[0],
            0.08
        );
        Ok(())
    }

    #[test]
    fn extrusion_parameter_invalidates_part_directly() -> Result<(), Vec<Diagnostic>> {
        let mut project = Project::from_source(SOURCE)?;
        project
            .set_parameter("DEPTH", Quantity::Length(Length::from_millimetres(9.0)))
            .map_err(|diagnostic| vec![diagnostic])?;
        let report = project.rebuild()?;
        assert!(report.rebuilt.contains(&DirtyNode::Part("Body".into())));
        assert!(!report
            .rebuilt
            .contains(&DirtyNode::Sketch("Profile".into())));
        Ok(())
    }

    #[test]
    fn arithmetic_parameter_dependencies_invalidate_extrusions() -> Result<(), Vec<Diagnostic>> {
        let source = SOURCE
            .replace("parameter DEPTH = 4 mm;", "parameter DEPTH = WIDTH/10;")
            .replace("extrude(Profile, DEPTH)", "extrude(Profile, DEPTH*2)");
        let mut project = Project::from_source(source)?;
        project
            .set_parameter("WIDTH", Quantity::Length(Length::from_millimetres(80.0)))
            .map_err(|diagnostic| vec![diagnostic])?;
        let report = project.rebuild()?;
        assert!(report
            .rebuilt
            .contains(&DirtyNode::Parameter("DEPTH".into())));
        assert!(report.rebuilt.contains(&DirtyNode::Part("Body".into())));
        let robogen_ir::Feature::Extrude { depth, .. } =
            project.snapshot().model.parts["Body"].features[0]
        else {
            return Err(vec![Diagnostic::error(
                "test",
                "expected extrusion",
                SourceSpan::default(),
            )]);
        };
        assert_eq!(depth, Length::from_millimetres(16.0));
        Ok(())
    }

    #[test]
    fn checked_in_example_executes_end_to_end() -> Result<(), Vec<Diagnostic>> {
        let project = Project::from_source(include_str!(
            "../../../examples/constrained_bracket/main.rgn"
        ))?;
        let part_id = project.snapshot().model.parts["Bracket"].id;
        assert_eq!(project.snapshot().meshes[&part_id].triangles.len(), 12);
        Ok(())
    }
}
