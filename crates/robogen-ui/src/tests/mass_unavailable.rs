use super::*;

fn minimum_mass_source() -> String {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/topology_manual/main.rgn"
    ))
    .replace("maximize_stiffness", "minimize_mass")
    .replace("volume_fraction(0.85), max_mass(230 g), ", "")
}

fn draw(state: &mut RoboGenUi, context: &Context) -> egui::FullOutput {
    context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(1440.0, 1000.0),
            )),
            ..Default::default()
        },
        |context| state.show(context),
    )
}

fn control(context: &Context, name: &str) -> Result<egui::Response, String> {
    context
        .data(|data| data.get_temp(egui::Id::new(("topology_control", name))))
        .ok_or_else(|| format!("missing topology control: {name}"))
}

#[test]
fn minimum_mass_view_blocks_both_backends_launch_and_export() -> Result<(), String> {
    for backend in [
        robogen_project::ComputeBackend::Cpu,
        robogen_project::ComputeBackend::Cuda,
    ] {
        let mut state = RoboGenUi::default();
        state.set_dsl_source(minimum_mass_source());
        state.set_mode(WorkspaceMode::Optimization);
        let part = state.selected_optimization.ok_or("missing part")?;
        let project = state.document.project().ok_or("missing project")?;
        assert!(project
            .topology_operations(part)
            .iter()
            .all(|specification| specification.preparation_issues().is_empty()));
        let mut settings = robogen_project::topology_settings();
        settings.compute_backend = backend;
        state.optimization_settings.insert(part, settings);
        let context = Context::default();
        draw(&mut state, &context);
        let output = draw(&mut state, &context);
        let mut text = String::new();
        for shape in output.shapes {
            collect_painted_text(&shape.shape, &mut text);
        }
        assert!(
            text.contains("Minimisation de masse indisponible"),
            "{text}"
        );
        assert!(!text.contains("moteur expérimental"));
        for name in ["CPU", "GPU CUDA", "launch"] {
            assert!(!control(&context, name)?.enabled(), "enabled: {name}");
        }
        assert_eq!(state.optimization_settings[&part].compute_backend, backend);
        assert!(state.document.build_progress().is_none());
        let snapshot = state
            .document
            .project()
            .ok_or("missing project")?
            .snapshot();
        assert!(snapshot.mass_diagnostics.is_empty());
        assert!(snapshot.topology.is_empty());
        assert!(snapshot.meshes.is_empty());
        assert!(!state.can_export());
        assert!(!state.request_export());
        assert!(state.take_action().is_none());
    }
    Ok(())
}

#[test]
fn unavailable_mass_keeps_source_history_and_stiffness_launch_available() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    let source = minimum_mass_source();
    state.set_dsl_source(&source);
    state.set_mode(WorkspaceMode::Optimization);
    state.set_dsl_source(format!("{source}\n// preparation remains editable\n"));
    assert!(state.document.is_valid());
    assert!(state.undo());
    assert_eq!(state.dsl_source(), source);
    assert!(state.redo());
    assert!(state.document.is_valid());
    assert!(!state.can_export());
    state.load_manual_topology_example();
    state.set_mode(WorkspaceMode::Optimization);
    let context = Context::default();
    draw(&mut state, &context);
    draw(&mut state, &context);
    assert!(control(&context, "launch")?.enabled());
    assert!(state.document.build_progress().is_none());
    Ok(())
}
