use super::*;

#[test]
fn topology_example_reports_unavailability_and_preserves_undo() {
    let mut state = RoboGenUi::default();
    let previous = state.dsl_source().to_owned();
    state.load_topology_example();
    assert_eq!(state.design_tab, DesignTab::Viewport);
    assert!(!state.can_export());
    assert!(!state.request_export());
    assert!(state.take_action().is_none());
    assert!(state.diagnostics.is_empty());
    assert!(state.cad_mesh.triangles.is_empty());
    assert!(state.undo());
    assert_eq!(state.dsl_source(), previous);
    assert!(state.can_export());
    assert!(state.redo());
    assert!(!state.can_export());
}

#[test]
fn mass_grid_defaults_to_physical_resolution_and_exposes_budget_errors() -> Result<(), String> {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/topology_manual/main.rgn"
    ))
    .replace("maximize_stiffness", "minimize_mass")
    .replace("volume_fraction(0.85), max_mass(230 g), ", "");
    let mut state = RoboGenUi::default();
    state.set_dsl_source(source);
    state.mode = WorkspaceMode::Optimization;
    let context = Context::default();
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(1440.0, 900.0),
            )),
            ..Default::default()
        },
        |context| state.show(context),
    );
    let part = state.selected_optimization.ok_or("missing part")?;
    assert_eq!(state.optimization_settings[&part].cells, [48, 16, 8]);
    assert!(!state.grid_controls[&part].manual);
    assert_eq!(state.grid_controls[&part].cell_size_mm, 2.5);
    let project = state.document.project().ok_or("missing project")?;
    let specifications = project.topology_operations(part);
    assert!(robogen_project::resolution_for_cell_size(
        specifications[0],
        robogen_domain::Length::from_millimetres(0.05)
    )
    .is_err());
    assert!(project.snapshot().mass_diagnostics.is_empty());
    assert!(!state.can_export());
    Ok(())
}

#[test]
fn computed_topology_background_publication_history_and_stale_export() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    state
        .enable_background_compilation()
        .map_err(|error| error.to_string())?;
    let wait = |state: &mut RoboGenUi| -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while !state.document.poll_compilation() {
            if std::time::Instant::now() > deadline {
                return Err("topology timeout".into());
            }
            std::thread::yield_now();
        }
        state.rebuild_preview();
        Ok(())
    };
    state.load_manual_topology_example();
    assert!(!state.can_export());
    wait(&mut state)?;
    assert!(!state.can_export());
    let part = state.selected_optimization.ok_or("missing part")?;
    state
        .document
        .start_optimization(part, robogen_project::topology_settings())
        .map_err(|error| error.message)?;
    wait(&mut state)?;
    assert!(state.can_export(), "{:?}", state.diagnostics);
    assert_eq!(state.cad_mesh.triangles.len(), 336);
    let report = state
        .document
        .project()
        .ok_or("missing project")?
        .snapshot()
        .topology
        .values()
        .next()
        .ok_or("missing report")?
        .clone();
    assert!(report.document_revision > 0);
    for size in [
        ViewportSize::new(600.0, 400.0),
        ViewportSize::new(320.0, 240.0),
    ] {
        assert!(!state
            .preview_renderer
            .render_mesh(&state.camera, size, &state.preview_mesh)
            .triangles
            .is_empty());
    }
    assert!(state.request_export());
    assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
    assert!(state.undo());
    assert!(!state.can_export());
    wait(&mut state)?;
    assert!(state
        .document
        .project()
        .ok_or("missing undo project")?
        .snapshot()
        .topology
        .is_empty());
    assert!(state.redo());
    wait(&mut state)?;
    assert!(!state.can_export());
    assert!(state
        .document
        .project()
        .ok_or("missing prepared project")?
        .snapshot()
        .topology
        .is_empty());
    state
        .document
        .start_optimization(part, robogen_project::topology_settings())
        .map_err(|error| error.message)?;
    wait(&mut state)?;
    assert!(state.can_export());
    let current_revision = state
        .document
        .project()
        .ok_or("missing redo project")?
        .snapshot()
        .topology
        .values()
        .next()
        .ok_or("missing redo report")?
        .document_revision;
    assert!(current_revision > report.document_revision);
    let mesh = state.cad_mesh.clone();
    state.set_dsl_source("module invalid; part");
    wait(&mut state)?;
    assert!(!state.can_export());
    assert_eq!(state.cad_mesh, mesh);
    state.load_manual_topology_example();
    state.document.cancel_build();
    assert!(!state.can_export());
    assert!(!state.document.poll_compilation());
    state.load_manual_topology_example();
    state.set_dsl_source(DEFAULT_DSL);
    wait(&mut state)?;
    assert!(state.can_export());
    assert!(state
        .document
        .project()
        .ok_or("missing current project")?
        .snapshot()
        .topology
        .is_empty());
    Ok(())
}

#[test]
fn preparation_layers_are_distinct_and_never_exportable() {
    let mut state = RoboGenUi::default();
    state.load_topology_example();
    assert!(state.document.is_valid(), "{:?}", state.diagnostics);
    assert!(!state.can_export());
    assert!(state.cad_mesh.triangles.is_empty());
    assert!(state
        .preparation_meshes
        .iter()
        .all(|mesh| !mesh.triangles.is_empty()));
    assert!(state
        .document
        .project()
        .is_some_and(|project| project.snapshot().topology.is_empty()));
    let context = Context::default();
    for mode in [WorkspaceMode::Design, WorkspaceMode::Optimization] {
        state.mode = mode;
        state.design_tab = DesignTab::Viewport;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    vec2(1080.0, 680.0),
                )),
                ..Default::default()
            },
            |context| state.show(context),
        );
        assert!(state
            .document
            .project()
            .is_some_and(|project| project.snapshot().topology.is_empty()));
        assert!(!state.can_export());
    }
}

#[test]
fn selected_result_exports_without_exporting_other_preparations() -> Result<(), String> {
    use robogen_project::topology_settings;
    let mut state = RoboGenUi::default();
    state.set_dsl_source(format!("{}\npart Other {{ material: AluminiumFixture; body = translate(offset: [0 mm, 80 mm, 0 mm], shape: manual_support()); }}", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/topology_manual/main.rgn"))));
    let ids: Vec<_> = state
        .document
        .project()
        .ok_or("project")?
        .snapshot()
        .preparation
        .keys()
        .copied()
        .collect();
    assert_eq!(ids.len(), 2);
    state
        .document
        .start_optimization(ids[0], topology_settings())
        .map_err(|error| error.message)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while !state.document.poll_compilation() {
        if std::time::Instant::now() > deadline {
            return Err("optimization timeout".into());
        }
        std::thread::yield_now();
    }
    state.rebuild_preview();
    assert!(!state.can_export());
    state.mode = WorkspaceMode::Optimization;
    state.selected_optimization = Some(ids[1]);
    assert!(!state.can_export());
    state.selected_optimization = Some(ids[0]);
    assert!(state.request_export());
    let Some(UiAction::ExportStl(mesh)) = state.take_action() else {
        return Err("missing export".into());
    };
    assert_eq!(mesh.triangles.len(), 336);
    state.show_regions = [false; 3];
    state.show_results = false;
    state.rebuild_preview();
    assert!(state.preview_mesh.triangles.is_empty());
    assert!(state.can_export());
    state
        .document
        .start_optimization(ids[1], topology_settings())
        .map_err(|error| error.message)?;
    assert!(state.can_export());
    state.document.cancel_build();
    assert!(state.can_export());
    Ok(())
}
