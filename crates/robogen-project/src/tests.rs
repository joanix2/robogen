//! Existing source, rebuild and density-preview regressions.

use super::*;
use robogen_domain::{Diagnostic, Length, Mass, SourceSpan};

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
