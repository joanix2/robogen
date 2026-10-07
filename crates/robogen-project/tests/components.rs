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

const BATTERY_SOURCE: &str = include_str!("../../../examples/li_ion_battery/main.rgn");

const TOPOLOGY_SOURCE: &str = include_str!("../../../examples/topology_battery_support/main.rgn");

#[test]
fn topology_is_semantically_valid_but_never_publishes_placeholder_geometry(
) -> Result<(), Vec<Diagnostic>> {
    let model = compile_source(TOPOLOGY_SOURCE)?;
    let operations = model.topology_operations();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].constraints.len(), 6);
    let errors = Project::from_source(TOPOLOGY_SOURCE)
        .err()
        .expect("backend must be unavailable");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E330");
    assert_eq!(errors[0].span, operations[0].span);
    assert!(errors[0].message.contains("Solver unavailable"));
    Ok(())
}

#[test]
fn topology_background_history_cancellation_and_revisions_keep_last_valid_mesh(
) -> Result<(), Box<dyn std::error::Error>> {
    fn finish(document: &mut SourceDocument) -> Result<(), &'static str> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !document.poll_compilation() {
            if std::time::Instant::now() > deadline {
                return Err("topology compilation timeout");
            }
            std::thread::yield_now();
        }
        Ok(())
    }
    let mut document = SourceDocument::new(SOURCE);
    let meshes = document
        .project()
        .ok_or("missing initial project")?
        .snapshot()
        .meshes
        .clone();
    document.enable_background_compilation()?;
    document.replace_source(TOPOLOGY_SOURCE.into());
    assert!(!document.is_valid());
    finish(&mut document)?;
    assert!(!document.is_valid());
    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "E330"));
    assert_eq!(
        document.project().ok_or("lost old mesh")?.snapshot().meshes,
        meshes
    );
    assert!(document.undo());
    finish(&mut document)?;
    assert!(document.is_valid());
    assert_eq!(document.source(), SOURCE);
    assert!(document.redo());
    finish(&mut document)?;
    assert!(!document.is_valid());
    document.replace_source(SOURCE.into());
    document.replace_source(TOPOLOGY_SOURCE.into());
    document.replace_source(SOURCE.into());
    finish(&mut document)?;
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    assert_eq!(
        document
            .project()
            .ok_or("missing current project")?
            .source(),
        SOURCE
    );
    document.replace_source(TOPOLOGY_SOURCE.into());
    document.cancel_build();
    assert!(!document.is_valid());
    assert!(document.build_progress().is_none());
    assert!(!document.poll_compilation());
    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "E321"));
    assert!(document.undo());
    finish(&mut document)?;
    assert!(document.is_valid());
    Ok(())
}

fn battery_bounds(source: &str) -> Result<[[f32; 3]; 2], Vec<Diagnostic>> {
    let project = Project::from_source(source)?;
    let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
    for vertex in project
        .snapshot()
        .meshes
        .values()
        .flat_map(|mesh| &mesh.vertices)
    {
        for (axis, coordinate) in vertex.position.iter().enumerate() {
            bounds[0][axis] = bounds[0][axis].min(*coordinate);
            bounds[1][axis] = bounds[1][axis].max(*coordinate);
        }
    }
    Ok(bounds)
}

#[test]
fn battery_pack_dimensions_and_leads_are_parametric() -> Result<(), Vec<Diagnostic>> {
    let pack = BATTERY_SOURCE.replace("body = li_ion_battery();", "body = battery_body();");
    let bounds = battery_bounds(&pack)?;
    for axis in [0, 1] {
        assert!((bounds[0][axis] + 0.009).abs() < 1e-6);
        assert!((bounds[1][axis] - 0.009).abs() < 1e-6);
    }
    assert!(bounds[0][2].abs() < 1e-6);
    assert!((bounds[1][2] - 0.068).abs() < 1e-6);
    assert!((battery_bounds(BATTERY_SOURCE)?[1][2] - 0.098).abs() < 1e-6);
    let changed = BATTERY_SOURCE
        .replace("PACK_DIAMETER = 18 mm", "PACK_DIAMETER = 20 mm")
        .replace("PACK_LENGTH = 68 mm", "PACK_LENGTH = 72 mm")
        .replace("WIRE_LENGTH = 20 mm", "WIRE_LENGTH = 25 mm")
        .replace("CONNECTOR_LENGTH = 10 mm", "CONNECTOR_LENGTH = 12 mm");
    let bounds = battery_bounds(&changed)?;
    assert!((bounds[1][0] - 0.010).abs() < 1e-6);
    assert!((bounds[1][2] - 0.109).abs() < 1e-6);
    assert!(Project::from_source(
        BATTERY_SOURCE.replace("PACK_DIAMETER = 18 mm", "PACK_DIAMETER = -1 mm")
    )
    .is_err());
    Ok(())
}

fn width(project: &Project, part: &str) -> Option<f64> {
    let Feature::Solid { geometry, .. } = &project.snapshot().model.parts[part].features[0] else {
        return None;
    };
    let SolidOperation::Box { size } = geometry.operation else {
        return None;
    };
    Some(size[0].millimetres())
}

fn mesh_width(project: &Project, part: &str) -> f32 {
    let id = project.snapshot().model.parts[part].id;
    project.snapshot().meshes[&id]
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::NEG_INFINITY, f32::max)
}

#[test]
fn background_compilation_discards_stale_revisions_and_preserves_history(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut document = SourceDocument::new(SOURCE);
    document.enable_background_compilation()?;
    let changed = SOURCE.replace("WIDTH = 23 mm", "WIDTH = 31 mm");
    document.replace_source(changed);
    assert!(!document.is_valid());
    assert!(document.build_progress().is_some());
    document.replace_source(SOURCE.replace("WIDTH = 23 mm", "WIDTH = 41 mm"));
    wait_for_compilation(&mut document)?;
    assert_eq!(
        width(document.project().ok_or("missing project")?, "First"),
        Some(41.0)
    );
    document.undo();
    wait_for_compilation(&mut document)?;
    assert_eq!(
        width(document.project().ok_or("missing project")?, "First"),
        Some(31.0)
    );
    document.redo();
    document.cancel_build();
    assert!(!document.is_valid());
    assert!(document.build_progress().is_none());
    assert!(!document.poll_compilation());
    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "E321"));
    Ok(())
}

fn wait_for_compilation(document: &mut SourceDocument) -> Result<(), &'static str> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !document.poll_compilation() {
        if std::time::Instant::now() > deadline {
            return Err("compilation timeout");
        }
        std::thread::yield_now();
    }
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    Ok(())
}

#[test]
fn global_parameter_rebuild_updates_component_mesh_and_preserves_ids() -> Result<(), Vec<Diagnostic>>
{
    let mut project = Project::from_source(SOURCE)?;
    let first_id = project.snapshot().model.parts["First"].id;
    let second_id = project.snapshot().model.parts["Second"].id;
    let second_mesh = project.snapshot().meshes[&second_id].clone();
    project
        .set_parameter("WIDTH", Quantity::Length(Length::from_millimetres(31.0)))
        .map_err(|error| vec![error])?;
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
fn definition_edits_undo_redo_and_invalid_edits_preserve_snapshot_validity(
) -> Result<(), &'static str> {
    let mut document = SourceDocument::new(SOURCE);
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    let original_id = document
        .project()
        .ok_or("missing initial project")?
        .snapshot()
        .model
        .parts["First"]
        .id;
    let changed = SOURCE.replace("[width,12 mm,24 mm]", "[width*2,12 mm,24 mm]");
    assert!(document.replace_source(changed.clone()));
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    assert_eq!(
        width(document.project().ok_or("missing edited project")?, "First"),
        Some(46.0)
    );
    assert!(
        (mesh_width(document.project().ok_or("missing edited mesh")?, "First") - 0.046).abs()
            < 1e-6
    );
    assert_eq!(
        document
            .project()
            .ok_or("missing edited ID")?
            .snapshot()
            .model
            .parts["First"]
            .id,
        original_id
    );
    assert!(document.undo());
    assert_eq!(document.source(), SOURCE);
    assert_eq!(
        width(document.project().ok_or("missing undo project")?, "First"),
        Some(23.0)
    );
    assert!(document.redo());
    assert_eq!(document.source(), changed);
    assert_eq!(
        width(document.project().ok_or("missing redo project")?, "First"),
        Some(46.0)
    );
    assert!(document.replace_source(changed.replace("width*2", "width/0")));
    assert!(!document.is_valid());
    assert_eq!(
        document
            .project()
            .ok_or("missing retained project")?
            .source(),
        changed
    );
    assert!(!document.diagnostics().is_empty());
    assert!(document.undo());
    assert!(document.is_valid());
    assert_eq!(
        width(
            document.project().ok_or("missing restored project")?,
            "First"
        ),
        Some(46.0)
    );
    Ok(())
}

#[test]
fn imports_fail_explicitly_in_compile_and_history_entry_points() -> Result<(), &'static str> {
    let source = format!("{SOURCE}\nimport external.servos;");
    let errors = compile_source(&source).err().ok_or("imports must fail")?;
    assert!(errors.iter().any(|error| error.code == "E109"));
    let document = SourceDocument::new(source);
    assert!(!document.is_valid());
    assert!(document
        .diagnostics()
        .iter()
        .any(|error| error.code == "E109"));
    Ok(())
}
