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

const MANUAL_TOPOLOGY: &str = include_str!("../../../examples/topology_manual/main.rgn");

fn calculate_first(project: &mut Project) -> Result<(), Vec<Diagnostic>> {
    let part = project
        .snapshot()
        .preparation
        .keys()
        .next()
        .copied()
        .ok_or_else(Vec::new)?;
    project.optimize_part(
        part,
        &robogen_project::topology_settings(),
        0,
        &|| false,
        &|_, _| {},
    )
}

#[test]
fn preparation_never_publishes_a_computed_mesh() -> Result<(), Vec<Diagnostic>> {
    for source in [MANUAL_TOPOLOGY, TOPOLOGY_SOURCE] {
        let project = Project::from_source(source)?;
        assert!(project.snapshot().meshes.is_empty());
        assert!(project.snapshot().topology.is_empty());
        assert_eq!(project.snapshot().preparation.len(), 1);
        for layers in project.snapshot().preparation.values() {
            assert!(!layers[0].triangles.is_empty());
            assert!(!layers[1].triangles.is_empty());
        }
    }
    Ok(())
}

#[test]
fn explicit_jobs_preserve_other_parts_and_invalidate_without_recomputing() -> Result<(), String> {
    use robogen_project::{topology_settings, OptimizationState};
    let source = format!("{MANUAL_TOPOLOGY}\npart Other {{ material: AluminiumFixture; body = translate(offset: [0 mm, 80 mm, 0 mm], shape: manual_support()); }}");
    let mut document = SourceDocument::new(source.clone());
    let ids: Vec<_> = document
        .project()
        .ok_or("no project")?
        .snapshot()
        .preparation
        .keys()
        .copied()
        .collect();
    assert_eq!(ids.len(), 2);
    let finish = |document: &mut SourceDocument| -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while !document.poll_compilation() {
            if std::time::Instant::now() > deadline {
                return Err("job timeout".into());
            }
            std::thread::yield_now();
        }
        Ok(())
    };
    document
        .start_optimization(ids[0], topology_settings())
        .map_err(|error| error.message)?;
    finish(&mut document)?;
    let snapshot = document.project().ok_or("no project")?.snapshot();
    assert_eq!(
        snapshot.optimization_states[&ids[0]],
        OptimizationState::Calculated
    );
    assert_eq!(
        snapshot.optimization_states[&ids[1]],
        OptimizationState::NotCalculated
    );
    let first = snapshot.meshes[&ids[0]].clone();
    let reports = snapshot.topology.clone();
    document
        .start_optimization(ids[1], topology_settings())
        .map_err(|error| error.message)?;
    finish(&mut document)?;
    let snapshot = document.project().ok_or("no project")?.snapshot();
    assert_eq!(snapshot.meshes[&ids[0]], first);
    for (id, report) in reports {
        assert_eq!(snapshot.topology[&id], report);
    }
    assert_eq!(snapshot.meshes.len(), 2);
    document
        .start_optimization(ids[1], topology_settings())
        .map_err(|error| error.message)?;
    document.cancel_build();
    assert_eq!(
        document
            .project()
            .ok_or("no project")?
            .snapshot()
            .optimization_states[&ids[1]],
        OptimizationState::Cancelled
    );
    assert!(document.is_valid());
    document.replace_source(format!("{source}\n// edited"));
    finish(&mut document)?;
    let snapshot = document.project().ok_or("no project")?.snapshot();
    assert!(snapshot.meshes.is_empty());
    assert!(snapshot.topology.is_empty());
    assert_eq!(
        snapshot.optimization_states[&ids[0]],
        OptimizationState::Obsolete
    );
    assert!(document.undo());
    finish(&mut document)?;
    assert!(document
        .project()
        .ok_or("no project")?
        .snapshot()
        .topology
        .is_empty());
    document
        .start_optimization(ids[0], topology_settings())
        .map_err(|error| error.message)?;
    document.replace_source("module empty;".into());
    finish(&mut document)?;
    assert!(document
        .project()
        .ok_or("no project")?
        .snapshot()
        .topology
        .is_empty());
    Ok(())
}

#[test]
fn topology_parameter_rebuild_reanalyses_loads_and_keeps_last_valid_snapshot(
) -> Result<(), Vec<Diagnostic>> {
    let source = MANUAL_TOPOLOGY
        .replace(
            "module topology_manual;",
            "module topology_manual; parameter LOAD_FACTOR = 1;",
        )
        .replace("0 N, 0 N, -1 N", "0 N, 0 N, -1 N * LOAD_FACTOR")
        .replace("0 N, 0 N, 1 N", "0 N, 0 N, 1 N * LOAD_FACTOR");
    let mut project = Project::from_source(source)?;
    calculate_first(&mut project)?;
    let initial = project
        .snapshot()
        .topology
        .values()
        .next()
        .ok_or_else(Vec::new)?
        .cases[0]
        .compliance
        .joules();
    project
        .set_parameter("LOAD_FACTOR", Quantity::Scalar(2.0))
        .map_err(|error| vec![error])?;
    project.rebuild()?;
    assert!(project.snapshot().topology.is_empty());
    calculate_first(&mut project)?;
    let doubled = project
        .snapshot()
        .topology
        .values()
        .next()
        .ok_or_else(Vec::new)?
        .cases[0]
        .compliance
        .joules();
    assert!((doubled / initial - 4.0).abs() < 1e-5);
    project
        .set_parameter("LOAD_FACTOR", Quantity::Scalar(100.0))
        .map_err(|error| vec![error])?;
    project.rebuild()?;
    let previous = project.snapshot().clone();
    assert!(calculate_first(&mut project)
        .err()
        .ok_or_else(Vec::new)?
        .iter()
        .any(|error| error.message.contains("E344")));
    assert_eq!(project.snapshot(), &previous);
    Ok(())
}

#[test]
fn topology_publishes_reanalysed_mesh_and_report_in_local_frame() -> Result<(), Vec<Diagnostic>> {
    let source = MANUAL_TOPOLOGY.replace("body = manual_support();", "body = color(red: 40, green: 170, blue: 100, shape: translate(offset: [1000 mm, 2000 mm, 3000 mm], shape: manual_support()));");
    let mut project = Project::from_source(source)?;
    calculate_first(&mut project)?;
    let snapshot = project.snapshot();
    assert_eq!(snapshot.topology.len(), 1);
    let report = snapshot.topology.values().next().ok_or_else(Vec::new)?;
    assert_eq!(report.document_revision, 0);
    assert_eq!(report.settings.cells, [12, 4, 2]);
    assert_eq!(report.iterations, 22);
    assert_eq!(report.cases.len(), 2);
    assert!((report.mass.kilograms() - 0.216).abs() < 1e-8);
    assert!(report
        .cases
        .iter()
        .all(|case| case.relative_residual < 1e-8 && case.max_displacement.millimetres() < 0.01));
    let mesh = snapshot.meshes.values().next().ok_or_else(Vec::new)?;
    assert_eq!(mesh.triangles.len(), 336);
    assert!(mesh
        .triangles
        .iter()
        .all(|triangle| triangle.color == Some([40, 170, 100])));
    for (axis, expected) in [1.0_f32, 2.0, 3.0].into_iter().enumerate() {
        assert!(
            (mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position[axis])
                .fold(f32::INFINITY, f32::min)
                - expected)
                .abs()
                < 1e-6
        );
    }
    Ok(())
}

#[test]
fn topology_is_semantically_valid_but_never_publishes_placeholder_geometry(
) -> Result<(), Vec<Diagnostic>> {
    let model = compile_source(TOPOLOGY_SOURCE)?;
    let operations = model.topology_operations();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].constraints.len(), 6);
    let mut project = Project::from_source(TOPOLOGY_SOURCE)?;
    assert!(project.snapshot().meshes.is_empty());
    let errors = calculate_first(&mut project).err().ok_or_else(Vec::new)?;
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E330");
    assert_eq!(errors[0].span, operations[0].span);
    assert!(errors[0].message.contains("E348"));
    assert!(errors[0].message.contains("unavailable"));
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
    assert!(document.is_valid());
    let part = *document
        .project()
        .ok_or("missing project")?
        .snapshot()
        .preparation
        .keys()
        .next()
        .ok_or("missing part")?;
    document
        .start_optimization(part, robogen_project::topology_settings())
        .map_err(|error| error.message)?;
    finish(&mut document)?;
    assert!(document.is_valid());
    assert!(document
        .optimization_diagnostics(part)
        .iter()
        .any(|diagnostic| diagnostic.code == "E330"));
    assert!(document
        .project()
        .ok_or("lost preparation")?
        .snapshot()
        .meshes
        .is_empty());
    assert!(document.undo());
    finish(&mut document)?;
    assert!(document.is_valid());
    assert_eq!(document.source(), SOURCE);
    assert_eq!(
        document
            .project()
            .ok_or("missing restored project")?
            .snapshot()
            .meshes,
        meshes
    );
    assert!(document.redo());
    finish(&mut document)?;
    assert!(document.is_valid());
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

#[test]
fn unavailable_mass_worker_publishes_failure_without_mesh_or_metrics(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../../../examples/topology_manual/main.rgn")
        .replace("maximize_stiffness", "minimize_mass")
        .replace("volume_fraction(0.85), max_mass(230 g), ", "");
    let mut document = SourceDocument::new(source);
    document.enable_background_compilation()?;
    let part = *document
        .project()
        .ok_or("missing project")?
        .snapshot()
        .preparation
        .keys()
        .next()
        .ok_or("missing preparation")?;
    document
        .start_optimization(part, robogen_project::topology_settings())
        .map_err(|error| error.message)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while document.build_progress().is_some() {
        assert!(!document.poll_optimization_progress());
        document.poll_compilation();
        if std::time::Instant::now() > deadline {
            return Err("worker timeout".into());
        }
        std::thread::park_timeout(std::time::Duration::from_millis(1));
    }
    let snapshot = document.project().ok_or("lost project")?.snapshot();
    assert_eq!(
        snapshot.optimization_states[&part],
        robogen_project::OptimizationState::Failed
    );
    assert!(snapshot.mass_diagnostics.is_empty());
    assert!(snapshot.topology.is_empty());
    assert!(snapshot.meshes.is_empty());
    let errors = document.optimization_diagnostics(part);
    let span = document
        .project()
        .ok_or("lost project")?
        .topology_operations(part)[0]
        .span;
    assert!(errors.iter().any(|error| error.code == "E330"
        && error.message.contains("E348")
        && error.message.contains("unavailable")
        && error.span == span));
    Ok(())
}
