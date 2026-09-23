use robogen_domain::{Diagnostic, Length, Quantity};
use robogen_ir::{Feature, SolidOperation};
use robogen_project::{compile_source, DirtyNode, Project, SourceDocument};

const SOURCE: &str = "module components;
parameter WIDTH = 23 mm;
material M { density: 1240 kg/m3; young: 3.5 GPa; poisson: 0.36; yield_strength: 50 MPa; }
component servo(width: Length = WIDTH) -> Solid { return box(size:[width,12 mm,24 mm]); }
component wrapper() -> Solid { return servo(); }
part First { material:M; body=wrapper(); }
part Second { material:M; body=servo(width:25 mm); }";

fn width(project: &Project, part: &str) -> Option<f64> {
    let Feature::Solid { geometry, .. } = &project.snapshot().model.parts[part].features[0] else { return None; };
    let SolidOperation::Box { size } = geometry.operation else { return None; };
    Some(size[0].millimetres())
}

fn mesh_width(project: &Project, part: &str) -> f32 {
    let id = project.snapshot().model.parts[part].id;
    project.snapshot().meshes[&id].vertices.iter().map(|vertex| vertex.position[0]).fold(f32::NEG_INFINITY, f32::max)
}

#[test]
fn global_parameter_rebuild_updates_component_mesh_and_preserves_ids() -> Result<(), Vec<Diagnostic>> {
    let mut project = Project::from_source(SOURCE)?;
    let first_id = project.snapshot().model.parts["First"].id;
    let second_id = project.snapshot().model.parts["Second"].id;
    let second_mesh = project.snapshot().meshes[&second_id].clone();
    project.set_parameter("WIDTH", Quantity::Length(Length::from_millimetres(31.0))).map_err(|error| vec![error])?;
    let report = project.rebuild()?;
    assert!(report.rebuilt.contains(&DirtyNode::Part("First".into())));
    assert_eq!(width(&project, "First"), Some(31.0));
    assert_eq!(width(&project, "Second"), Some(25.0));
    assert_eq!(project.snapshot().model.parts["First"].id, first_id);
    assert_eq!(project.snapshot().meshes[&second_id], second_mesh);
    assert!((mesh_width(&project, "First") - 0.031).abs() < 1e-6);
    Ok(())
}

#[test]
fn definition_edits_undo_redo_and_invalid_edits_preserve_snapshot_validity() -> Result<(), &'static str> {
    let mut document = SourceDocument::new(SOURCE);
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    let original_id = document.project().ok_or("missing initial project")?.snapshot().model.parts["First"].id;
    let changed = SOURCE.replace("[width,12 mm,24 mm]", "[width*2,12 mm,24 mm]");
    assert!(document.replace_source(changed.clone()));
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    assert_eq!(width(document.project().ok_or("missing edited project")?, "First"), Some(46.0));
    assert!((mesh_width(document.project().ok_or("missing edited mesh")?, "First") - 0.046).abs() < 1e-6);
    assert_eq!(document.project().ok_or("missing edited ID")?.snapshot().model.parts["First"].id, original_id);
    assert!(document.undo());
    assert_eq!(document.source(), SOURCE);
    assert_eq!(width(document.project().ok_or("missing undo project")?, "First"), Some(23.0));
    assert!(document.redo());
    assert_eq!(document.source(), changed);
    assert_eq!(width(document.project().ok_or("missing redo project")?, "First"), Some(46.0));
    assert!(document.replace_source(changed.replace("width*2", "width/0")));
    assert!(!document.is_valid());
    assert_eq!(document.project().ok_or("missing retained project")?.source(), changed);
    assert!(!document.diagnostics().is_empty());
    assert!(document.undo());
    assert!(document.is_valid());
    assert_eq!(width(document.project().ok_or("missing restored project")?, "First"), Some(46.0));
    Ok(())
}

#[test]
fn imports_fail_explicitly_in_compile_and_history_entry_points() -> Result<(), &'static str> {
    let source = format!("{SOURCE}\nimport external.servos;");
    let errors = compile_source(&source).err().ok_or("imports must fail")?;
    assert!(errors.iter().any(|error| error.code == "E109"));
    let document = SourceDocument::new(source);
    assert!(!document.is_valid());
    assert!(document.diagnostics().iter().any(|error| error.code == "E109"));
    Ok(())
}