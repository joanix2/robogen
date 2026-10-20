use super::*;

#[test]
fn left_column_shows_generated_instances_above_function_library(
) -> Result<(), Box<dyn std::error::Error>> {
    for size in [vec2(1440.0, 831.0), vec2(1080.0, 680.0)] {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        let draw = |state: &mut RoboGenUi| {
            context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |context| state.show(context),
            )
        };
        draw(&mut state);
        let output = draw(&mut state);
        let taxonomy = text_rect(&output, "Taxonomie").ok_or("missing taxonomy")?;
        let library = text_rect(&output, "Bibliothèque").ok_or("missing library")?;
        let assistant = text_rect(&output, "Assistant IA").ok_or("missing assistant")?;
        assert!(taxonomy.right() < size.x / 2.0);
        assert!(library.top() > taxonomy.bottom());
        assert!(assistant.left() > size.x / 2.0);
        assert!(text_rect(&output, "Bracket").is_some());
        for label in ["Servomoteur", "Camera Raspberry Pi", "Jetson Nano Super"] {
            let rect = text_rect(&output, label).ok_or("missing generator")?;
            assert!(rect.top() > library.bottom() && rect.bottom() < size.y);
        }
        assert!(text_rect(&output, "Patte").is_none());
        assert!(state.instantiate_generator(LibraryGenerator::Camera));
        assert_eq!(state.taxonomy.len(), 2);
        let function = state
            .functions
            .iter()
            .find(|function| function.name == "raspberry_pi_camera")
            .ok_or("missing definition")?
            .clone();
        state.open_source(function.span);
        assert_eq!(state.design_tab, DesignTab::Dsl);
        draw(&mut state);
        assert!(state.source_selection.is_none());
        assert!(state.undo());
        assert_eq!(state.taxonomy.len(), 1);
    }
    Ok(())
}

#[test]
fn starts_in_design_mode() {
    let ui = RoboGenUi::default();
    assert_eq!(ui.mode(), WorkspaceMode::Design);
    assert_eq!(
        ui.selected_component,
        ui.taxonomy.first().map(|instance| EntityId(instance.id.0))
    );
    assert_eq!(ui.taxonomy[0].name, "Bracket");
    assert!(ui.dsl_source.contains("parameter WIDTH = 40 mm"));
    assert!(ui.diagnostics.is_empty());
    assert!(!ui.preview_mesh.is_empty());
}

#[test]
fn mode_can_be_driven_by_the_host_application() {
    let mut ui = RoboGenUi::default();
    ui.set_mode(WorkspaceMode::Optimization);
    assert_eq!(ui.mode(), WorkspaceMode::Optimization);
}

#[test]
fn workflow_views_render_headlessly_without_actions() {
    for size in [vec2(1080.0, 680.0), vec2(1440.0, 831.0)] {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        let original_positions = state.preview_mesh.positions.clone();
        for (mode, design_tab, optimization_tab, expected_text) in [
            (
                WorkspaceMode::Design,
                DesignTab::Viewport,
                0,
                "APERÇU PARAMÉTRIQUE",
            ),
            (WorkspaceMode::Design, DesignTab::Dsl, 0, "Modèle valide"),
            (
                WorkspaceMode::Optimization,
                DesignTab::Viewport,
                0,
                "Solveur indisponible",
            ),
            (
                WorkspaceMode::Optimization,
                DesignTab::Viewport,
                1,
                "Aucune comparaison",
            ),
            (
                WorkspaceMode::Optimization,
                DesignTab::Viewport,
                2,
                "Analyse mécanique",
            ),
            (
                WorkspaceMode::Optimization,
                DesignTab::Viewport,
                3,
                "Aucune exécution",
            ),
            (
                WorkspaceMode::Learning,
                DesignTab::Viewport,
                0,
                "Simulation et entraînement indisponibles",
            ),
        ] {
            state.set_mode(mode);
            state.design_tab = design_tab;
            state.optimization_tab = optimization_tab;
            for _ in 0..2 {
                let output = context.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |context| state.show(context),
                );
                let mut text = String::new();
                for shape in &output.shapes {
                    collect_painted_text(&shape.shape, &mut text);
                }
                assert!(text.contains(expected_text), "{mode:?} at {size:?}: {text}");
                let required_visible = match mode {
                    WorkspaceMode::Design => {
                        vec!["Assistant IA", "Bibliothèque", "Suggestions indisponibles"]
                    }
                    WorkspaceMode::Optimization => {
                        vec!["Lancer l’optimisation", "Paramètres d’optimisation"]
                    }
                    WorkspaceMode::Learning => vec![
                        "Lancer l’entraînement",
                        "Foundation model non configuré",
                        "Chronologie du comportement",
                    ],
                };
                for label in required_visible {
                    assert!(
                        output.shapes.iter().any(|shape| match &shape.shape {
                            Shape::Text(painted) if painted.galley.job.text == label => {
                                let rect =
                                    egui::Rect::from_min_size(painted.pos, painted.galley.size());
                                shape.clip_rect.expand(1.0).contains_rect(rect)
                                    && egui::Rect::from_min_size(egui::Pos2::ZERO, size)
                                        .contains_rect(rect)
                            }
                            _ => false,
                        }),
                        "clipped {label} in {mode:?} at {size:?}"
                    );
                }
                match mode {
                    WorkspaceMode::Design => assert!(text.contains("Indisponible")),
                    WorkspaceMode::Optimization => {
                        assert!(text.contains("Aucun résultat calculé"))
                    }
                    WorkspaceMode::Learning => {
                        assert!(text.contains("Aucune donnée"));
                        assert!(text.contains("Aucune trajectoire"));
                    }
                }
                assert!(!context
                    .tessellate(output.shapes, output.pixels_per_point)
                    .is_empty());
                assert!(state.take_action().is_none());
                assert_eq!(state.mode(), mode);
                assert_eq!(state.dsl_source, DEFAULT_DSL);
                assert_eq!(state.document.source(), DEFAULT_DSL);
                assert!(!state.can_undo());
                assert!(!state.can_redo());
                assert_eq!(state.preview_mesh.positions, original_positions);
            }
        }
    }
}

#[test]
fn disabled_assistant_and_backends_are_honest() {
    let context = Context::default();
    let mut state = RoboGenUi::default();
    assert_eq!(
        state.assistant_message,
        robogen_agent_api::AssistantError::Disabled.to_string()
    );
    for (mode, expected) in [
        (
            WorkspaceMode::Design,
            vec![
                "Conversation de démonstration",
                "assistant IA non configuré",
                "Suggestions indisponibles",
            ],
        ),
        (
            WorkspaceMode::Optimization,
            vec!["Solveur indisponible", "Aucune métrique calculée"],
        ),
        (
            WorkspaceMode::Learning,
            vec![
                "Foundation model non configuré",
                "Entraînement indisponible",
                "Aucune donnée",
            ],
        ),
    ] {
        state.set_mode(mode);
        let output = frame(&mut state, &context, Vec::new());
        let mut text = String::new();
        for shape in &output.shapes {
            collect_painted_text(&shape.shape, &mut text);
        }
        for expected in expected {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        assert!(state.take_action().is_none());
    }
}

#[test]
fn two_top_bars_keep_global_and_workspace_controls_separate(
) -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::default();
    let mut state = RoboGenUi::default();
    frame(&mut state, &context, Vec::new());
    let output = frame(&mut state, &context, Vec::new());
    let brand = text_rect(&output, "RoboGen").ok_or("missing application name")?;
    let profile = text_rect(&output, "Profil").ok_or("missing profile menu")?;
    assert!(brand.bottom() < 38.0 && profile.bottom() < 38.0);
    assert!(brand.left() < 40.0 && profile.left() > 980.0);
    assert!(text_rect(&output, "From idea to real robots").is_none());
    let mut previous_right = 0.0;
    for title in [
        "1    Design\n       Définir la robotique",
        "2    Optimisation\n       Alléger et renforcer",
        "3    Apprentissage\n       Apprendre à agir",
    ] {
        let rect =
            text_rect(&output, title).ok_or_else(|| format!("missing topbar label {title}"))?;
        assert!(
            rect.left() >= previous_right
                && rect.right() <= 1080.0
                && rect.top() >= 38.0
                && rect.bottom() <= 94.0,
            "{title}: {rect:?}"
        );
        previous_right = rect.right();
    }
    let undo_rect = icon_response(&context, "Annuler (Ctrl+Z)")
        .ok_or("missing undo button")?
        .rect;
    assert!(undo_rect.left() > previous_right);
    assert!(undo_rect.top() >= 38.0 && undo_rect.bottom() <= 94.0);
    click(&mut state, &context, profile.center());
    let profile_output = frame(&mut state, &context, Vec::new());
    assert!(text_rect(&profile_output, "Profil non configuré").is_some());
    click(&mut state, &context, profile.center());
    let orbit_rect = icon_response(&context, "Orbite droite")
        .ok_or("missing orbit button")?
        .rect;
    let original_camera = state.camera;
    click(&mut state, &context, orbit_rect.center());
    assert_ne!(state.camera, original_camera);
    let reset_rect = icon_response(&context, "Réinitialiser la caméra")
        .ok_or("missing reset button")?
        .rect;
    click(&mut state, &context, reset_rect.center());
    assert_eq!(state.camera, original_camera);
    state.set_dsl_source(DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 70 mm"));
    frame(&mut state, &context, Vec::new());
    click(&mut state, &context, undo_rect.center());
    assert_eq!(state.dsl_source(), DEFAULT_DSL);
    assert!(state.can_redo());
    Ok(())
}
