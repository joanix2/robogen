//! Project loading and incremental orchestration for the shared model pipeline.

use robogen_cad::{build_part_mesh, Mesh, NativeCadKernel};
use robogen_constraints::{ConstraintSolver, NativeConstraintSolver};
use robogen_domain::{Diagnostic, PartId, Quantity, Severity, SourceSpan};
use robogen_dsl::{Declaration, Expr, ExprKind, Module};
use robogen_ir::{lower, SemanticModel};
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
        };
        document.refresh();
        document
    }

    pub fn source(&self) -> &str {
        &self.source
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
        match Project::from_source(self.source.clone()) {
            Ok(project) => {
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
        let source = source.into();
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
        let meshes = build_all_meshes(&model, &mut diagnostics);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(diagnostics);
        }
        Ok(Self {
            source,
            ast,
            overrides: BTreeMap::new(),
            snapshot: ProjectSnapshot { model, meshes },
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

        let mut meshes = self.snapshot.meshes.clone();
        for node in &dirty {
            if let DirtyNode::Part(name) = node {
                if let Some(part) = model.parts.get(name) {
                    match build_part_mesh(&model, part, &NativeCadKernel) {
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
        self.snapshot = ProjectSnapshot { model, meshes };
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

fn build_all_meshes(
    model: &SemanticModel,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<PartId, Mesh> {
    let mut meshes = BTreeMap::new();
    for part in model.parts.values() {
        match build_part_mesh(model, part, &NativeCadKernel) {
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
    meshes
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
                    || feature.args.iter().any(|expression| references_any(expression, &dependencies))
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
            ExprKind::Vector(values) | ExprKind::Call { args: values, .. } => pending.extend(values),
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
    };
    Expr {
        kind: ExprKind::Number { value, unit },
        span,
    }
}

#[cfg(test)]
mod tests {
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
        let source = SOURCE.replace("parameter DEPTH = 4 mm;", "parameter DEPTH = WIDTH/10;")
            .replace("extrude(Profile, DEPTH)", "extrude(Profile, DEPTH*2)");
        let mut project = Project::from_source(source)?;
        project.set_parameter("WIDTH", Quantity::Length(Length::from_millimetres(80.0)))
            .map_err(|diagnostic| vec![diagnostic])?;
        let report = project.rebuild()?;
        assert!(report.rebuilt.contains(&DirtyNode::Parameter("DEPTH".into())));
        assert!(report.rebuilt.contains(&DirtyNode::Part("Body".into())));
        let robogen_ir::Feature::Extrude { depth, .. } = project.snapshot().model.parts["Body"].features[0] else {
            return Err(vec![Diagnostic::error("test", "expected extrusion", SourceSpan::default())]);
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
