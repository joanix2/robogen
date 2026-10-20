use super::*;

#[test]
fn changing_width_rebuilds_the_preview() {
    let mut ui = RoboGenUi::default();
    let before = ui.preview_mesh.positions.clone();
    ui.set_dsl_source(ui.dsl_source.replace("WIDTH = 40 mm", "WIDTH = 80 mm"));
    assert_eq!(ui.width_mm, 80.0);
    assert_ne!(ui.preview_mesh.positions, before);
    assert!((ui.cad_mesh.vertices[1].position[0] - 0.08).abs() < 1e-6);
    assert!(ui.diagnostics.is_empty());
}

#[test]
fn source_history_preserves_preview_and_guards_export() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = RoboGenUi::default();
    assert!(state.can_export());
    let original = state.preview_mesh.positions.clone();
    let wider = DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 70 mm");
    state.set_dsl_source(&wider);
    let wide_positions = state.preview_mesh.positions.clone();
    assert_eq!(state.width_mm, 70.0);
    assert!(state.request_export());
    let UiAction::ExportStl(mesh) = state.take_action().ok_or("missing STL action")?;
    assert_eq!(cad_preview(&mesh).1, 70.0);
    assert!(state.request_export());
    state.set_dsl_source("module broken; part");
    assert!(!state.can_export());
    assert!(!state.request_export());
    assert!(state.take_action().is_none());
    assert!(!state.diagnostics.is_empty());
    assert_eq!(state.preview_mesh.positions, wide_positions);
    assert!(state.undo());
    assert_eq!(state.dsl_source(), wider);
    assert!(state.can_export());
    assert!(state.diagnostics.is_empty());
    assert!(state.undo());
    assert_eq!(state.preview_mesh.positions, original);
    assert!(!state.undo());
    assert!(state.redo());
    assert_eq!(state.preview_mesh.positions, wide_positions);
    assert!(state.redo());
    assert!(!state.can_export());
    assert!(!state.redo());
    assert!(state.undo());
    state.set_dsl_source(&wider);
    assert!(state.can_redo());
    state.set_dsl_source(DEFAULT_DSL);
    assert!(!state.can_redo());
    state.set_dsl_source("module empty;");
    assert!(state.preview_mesh.is_empty());
    assert!(!state.can_export());
    Ok(())
}

#[test]
fn editor_shortcuts_use_only_document_history() {
    let context = Context::default();
    let mut state = RoboGenUi {
        design_tab: DesignTab::Dsl,
        ..Default::default()
    };
    frame(&mut state, &context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(egui::Id::new("robogen_dsl_source")));
    frame(
        &mut state,
        &context,
        vec![egui::Event::Text("invalid ".to_owned())],
    );
    let edited = state.dsl_source().to_owned();
    assert_ne!(edited, DEFAULT_DSL);
    assert!(state.can_undo());
    for (modifiers, expected) in [
        (egui::Modifiers::CTRL, DEFAULT_DSL),
        (
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            edited.as_str(),
        ),
        (egui::Modifiers::CTRL, DEFAULT_DSL),
    ] {
        frame(
            &mut state,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
        );
        assert_eq!(state.dsl_source(), expected);
        assert_eq!(state.dsl_source, expected);
    }
    assert!(!state.can_undo());
    assert!(state.can_redo());
}

#[test]
fn taxonomy_search_shortcuts_use_only_text_history() {
    let command = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    for (redo_modifiers, redo_key) in [
        (command | egui::Modifiers::SHIFT, egui::Key::Z),
        (command, egui::Key::Y),
    ] {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        let wider = DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 70 mm");
        let widest = DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 80 mm");
        state.set_dsl_source(&wider);
        state.set_dsl_source(&widest);
        assert!(state.undo());
        let positions = state.preview_mesh.positions.clone();
        frame(&mut state, &context, Vec::new());
        let search_id = egui::Id::new("robogen_taxonomy_search");
        context.memory_mut(|memory| memory.request_focus(search_id));
        frame(&mut state, &context, Vec::new());
        frame(
            &mut state,
            &context,
            vec![egui::Event::Text("patte".to_owned())],
        );
        assert_eq!(state.taxonomy_filter, "patte");
        for (modifiers, key, expected) in [
            (command, egui::Key::Z, ""),
            (command, egui::Key::Z, ""),
            (redo_modifiers, redo_key, "patte"),
            (redo_modifiers, redo_key, "patte"),
        ] {
            frame(
                &mut state,
                &context,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert!(context.memory(|memory| memory.has_focus(search_id)));
            assert_eq!(state.taxonomy_filter, expected);
            assert_eq!(state.dsl_source(), wider);
            assert_eq!(state.dsl_source, wider);
            assert_eq!(state.width_mm, 70.0);
            assert_eq!(state.preview_mesh.positions, positions);
            assert!(state.can_undo());
            assert!(state.can_redo());
        }
        context.memory_mut(|memory| memory.surrender_focus(search_id));
        for (modifiers, key, expected) in [
            (command, egui::Key::Z, DEFAULT_DSL),
            (redo_modifiers, redo_key, wider.as_str()),
            (redo_modifiers, redo_key, widest.as_str()),
        ] {
            frame(
                &mut state,
                &context,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert_eq!(state.dsl_source(), expected);
            assert_eq!(state.dsl_source, expected);
            assert_eq!(state.taxonomy_filter, "patte");
        }
        assert!(!state.can_redo());
    }
}

#[test]
fn generated_taxonomy_is_searchable_and_selectable_without_source_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::default();
    let mut state = RoboGenUi::default();
    let source = format!("{DEFAULT_DSL}\ncomponent servo() -> Solid {{ return box([10 mm, 10 mm, 10 mm]); }}\npart servo_gauche {{ material: Aluminium6061; body = servo(); }}\npart servo_droit {{ material: Aluminium6061; body = translate([20 mm, 0 mm, 0 mm], servo()); }}");
    state.set_dsl_source(source.clone());
    let entry_id = EntityId(
        state
            .taxonomy
            .iter()
            .find(|instance| instance.name == "servo_droit")
            .ok_or("missing generated instance")?
            .id
            .0,
    );
    state.taxonomy_filter = "DROIT".to_owned();
    frame(&mut state, &context, Vec::new());
    let output = frame(&mut state, &context, Vec::new());
    let rect = text_rect(&output, "servo_droit").ok_or("instance not rendered")?;
    assert!(text_rect(&output, "servo_gauche").is_none());
    click(&mut state, &context, rect.center());
    assert_eq!(state.selected_component(), Some(entry_id));
    assert_eq!(state.dsl_source(), source);
    state.taxonomy_filter = "GAUCHE".to_owned();
    assert!(text_rect(&frame(&mut state, &context, Vec::new()), "servo_gauche").is_some());
    let before = state.taxonomy.clone();
    state.set_dsl_source("module broken; part");
    assert_eq!(state.taxonomy, before);
    assert!(text_rect(
        &frame(&mut state, &context, Vec::new()),
        "Source invalide · résultat précédent"
    )
    .is_some());
    assert!(!state.instantiate_generator(LibraryGenerator::Camera));
    assert!(state.undo());
    assert!(state.undo());
    assert_eq!(state.dsl_source(), DEFAULT_DSL);
    assert_eq!(state.taxonomy.len(), 1);
    Ok(())
}
