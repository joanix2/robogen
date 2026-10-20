use super::*;

#[test]
fn servo_example_reaches_colored_preview_and_export() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = RoboGenUi::default();
    state.enable_background_compilation()?;
    state.load_servo_example();
    assert!(!state.can_export());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !state.document.poll_compilation() {
        if std::time::Instant::now() > deadline {
            return Err("servo compilation timeout".into());
        }
        std::thread::yield_now();
    }
    state.rebuild_preview();
    assert!(state.can_export(), "{:?}", state.diagnostics);
    assert!((state.width_mm - 11.8).abs() < 1e-4);
    assert_eq!(
        state.preview_mesh.colors.len(),
        state.cad_mesh.triangles.len()
    );
    assert!(state.preview_mesh.colors.iter().all(Option::is_some));
    assert!(state.request_export());
    assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
    state.set_dsl_source("module broken; part");
    assert!(!state.can_export());
    assert!(state.take_action().is_none());
    Ok(())
}

#[test]
fn camera_example_loads_in_background_exports_and_undoes() -> Result<(), Box<dyn std::error::Error>>
{
    let mut state = RoboGenUi::default();
    let original = state.dsl_source().to_owned();
    state.enable_background_compilation()?;
    state.load_camera_example();
    assert!(!state.can_export());
    assert_eq!(state.mode(), WorkspaceMode::Design);
    assert_eq!(state.design_tab, DesignTab::Viewport);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !state.document.poll_compilation() {
        if std::time::Instant::now() > deadline {
            return Err("camera compilation timeout".into());
        }
        std::thread::yield_now();
    }
    state.rebuild_preview();
    assert!(state.can_export(), "{:?}", state.diagnostics);
    assert!((state.width_mm - 28.9).abs() < 1e-4);
    assert!(state.preview_mesh.colors.iter().all(Option::is_some));
    for size in [
        ViewportSize::new(600.0, 400.0),
        ViewportSize::new(320.0, 240.0),
    ] {
        let image = state
            .preview_renderer
            .rasterize_mesh(&state.camera, size, &state.preview_mesh);
        assert!(
            image.pixels.iter().filter(|pixel| pixel.a == 255).count() > image.pixels.len() / 20
        );
        assert!(
            image.pixels.iter().any(|pixel| pixel.a == 255
                && pixel.b > pixel.g
                && pixel.g > pixel.r
                && pixel.r >= 30),
            "lens must be visible"
        );
    }
    assert!(state.request_export());
    assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
    assert!(state.undo());
    assert_eq!(state.dsl_source(), original);
    assert!(!state.can_export());
    Ok(())
}

#[test]
fn biped_loads_asynchronously_is_visible_and_undoes() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    let previous = state.dsl_source().to_owned();
    state
        .enable_background_compilation()
        .map_err(|error| error.to_string())?;
    state.load_biped_example();
    assert!(!state.can_export());
    let wait = |state: &mut RoboGenUi| -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
        while !state.document.poll_compilation() {
            if std::time::Instant::now() > deadline {
                return Err("biped compilation timeout".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        state.rebuild_preview();
        Ok(())
    };
    wait(&mut state)?;
    assert!(state.document.is_valid(), "{:?}", state.diagnostics);
    assert!(!state.can_export());
    assert_eq!(state.taxonomy.len(), 28);
    assert!(state
        .preparation_meshes
        .iter()
        .all(|layer| !layer.triangles.is_empty()));
    let project = state.document.project().ok_or("missing biped project")?;
    let camera_support = project.snapshot().model.parts["CameraSupport"].id;
    assert_eq!(state.selected_optimization, Some(camera_support));
    assert_eq!(project.snapshot().preparation.len(), 11);
    assert!(project.snapshot().topology.is_empty());
    for size in [
        ViewportSize::new(600.0, 400.0),
        ViewportSize::new(320.0, 240.0),
    ] {
        let frame = state
            .preview_renderer
            .render_mesh(&state.camera, size, &state.preview_mesh);
        assert!(!frame.triangles.is_empty());
        assert!(
            frame
                .triangles
                .iter()
                .flat_map(|triangle| triangle.points)
                .all(|point| point.x > 8.0
                    && point.x < size.width - 8.0
                    && point.y > 8.0
                    && point.y < size.height - 8.0),
            "entire biped including head and ankles must fit at {size:?}"
        );
        let image = state
            .preview_renderer
            .rasterize_mesh(&state.camera, size, &state.preview_mesh);
        assert!(
            image
                .pixels
                .iter()
                .filter(|pixel| pixel.a == 255 && pixel.b > 100 && pixel.b > pixel.r)
                .count()
                > 100,
            "blue servos visible at {size:?}"
        );
    }
    assert!(!state.request_export());
    assert!(state.take_action().is_none());
    state.mode = WorkspaceMode::Optimization;
    state.rebuild_preview();
    assert!(!state.can_export());
    let context = egui::Context::default();
    let _ = context.run(egui::RawInput::default(), |context| state.show(context));
    assert_eq!(state.selected_optimization, Some(camera_support));
    assert!(state.document.build_progress().is_none());
    assert!(state.undo());
    assert_eq!(state.dsl_source(), previous);
    assert!(!state.can_export());
    wait(&mut state)?;
    assert!(state.redo());
    assert!(!state.can_export());
    wait(&mut state)?;
    assert_eq!(state.taxonomy.len(), 28);
    assert!(!state.can_export());
    state.set_dsl_source("module invalid; part");
    assert!(!state.can_export());
    Ok(())
}

#[test]
fn battery_example_is_visible_exportable_and_undoable() {
    let mut state = RoboGenUi::default();
    let previous = state.dsl_source().to_owned();
    state.load_battery_example();
    assert!(state.can_export(), "{:?}", state.diagnostics);
    assert!((state.width_mm - 18.0).abs() < 1e-4);
    for size in [
        ViewportSize::new(600.0, 400.0),
        ViewportSize::new(320.0, 240.0),
    ] {
        let image = state
            .preview_renderer
            .rasterize_mesh(&state.camera, size, &state.preview_mesh);
        assert!(
            image
                .pixels
                .iter()
                .filter(|pixel| pixel.a == 255 && pixel.r > 100 && pixel.g > 80 && pixel.b < 40)
                .count()
                > 100,
            "yellow pack must be visible"
        );
        assert!(
            image
                .pixels
                .iter()
                .filter(|pixel| pixel.a == 255 && pixel.r > 100 && pixel.g < 60 && pixel.b < 60)
                .count()
                > 10,
            "red connector must be visible"
        );
    }
    assert!(state.request_export());
    assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
    assert!(state.undo());
    assert_eq!(state.dsl_source(), previous);
    assert!(state.instantiate_generator(LibraryGenerator::Battery));
    assert!(state
        .taxonomy
        .iter()
        .any(|instance| instance.name == "li_ion_battery_1"));
    assert!(state.undo());
    assert_eq!(state.dsl_source(), previous);
}

#[test]
fn compute_board_example_is_colored_exportable_and_undoable() {
    let mut state = RoboGenUi::default();
    let previous = state.dsl_source().to_owned();
    state.load_compute_board_example();
    assert!(state.can_export(), "{:?}", state.diagnostics);
    assert!((state.width_mm - 104.0).abs() < 1e-4);
    assert!(state.preview_mesh.colors.iter().all(Option::is_some));
    assert!(state.request_export());
    assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
    assert!(state.undo());
    assert_eq!(state.dsl_source(), previous);
}
