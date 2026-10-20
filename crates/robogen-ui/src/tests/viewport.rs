use super::*;

#[test]
fn raster_cache_reuses_texture_and_invalidates_camera_mesh_and_size() -> Result<(), &'static str> {
    let context = Context::default();
    let id = egui::Id::new("preview_cache_test");
    let mut camera = Camera::default();
    let mut mesh = PreviewMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
    );
    let draw = |camera: &Camera, mesh: &PreviewMesh, width: f32| {
        let _output = context.run(egui::RawInput::default(), |context| {
            let painter = context.layer_painter(egui::LayerId::background());
            paint_preview(
                &painter,
                egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 100.0)),
                id,
                &RobotPreviewRenderer,
                camera,
                mesh,
            );
        });
        context
            .data(|data| data.get_temp::<std::sync::Arc<CachedPreview>>(id))
            .ok_or("missing raster cache")
    };
    let first = draw(&camera, &mesh, 160.0)?;
    let unchanged = draw(&camera, &mesh, 160.0)?;
    assert!(std::sync::Arc::ptr_eq(&first, &unchanged));
    camera.orbit(20.0, 10.0);
    let moved = draw(&camera, &mesh, 160.0)?;
    assert!(!std::sync::Arc::ptr_eq(&first, &moved));
    mesh.colors = vec![Some(Rgba8::rgb(255, 0, 0))];
    let recolored = draw(&camera, &mesh, 160.0)?;
    assert!(!std::sync::Arc::ptr_eq(&moved, &recolored));
    let resized = draw(&camera, &mesh, 180.0)?;
    assert!(!std::sync::Arc::ptr_eq(&recolored, &resized));
    Ok(())
}

#[test]
fn painter_icons_are_font_free_and_fit_the_button() {
    let context = Context::default();
    let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), vec2(30.0, 30.0));
    for icon in [
        Icon::Undo,
        Icon::Redo,
        Icon::Save,
        Icon::Plus,
        Icon::Minus,
        Icon::Export,
        Icon::OrbitLeft,
        Icon::OrbitRight,
        Icon::Home,
        Icon::Ortho,
    ] {
        let output = context.run(egui::RawInput::default(), |context| {
            let painter = context.layer_painter(egui::LayerId::background());
            paint_icon(&painter, rect, icon, Stroke::new(1.5, TEXT));
        });
        assert!(!output.shapes.is_empty());
        for shape in output.shapes {
            assert!(matches!(shape.shape, Shape::Path(_)));
            assert!(rect
                .shrink(4.0)
                .contains_rect(shape.shape.visual_bounding_rect()));
        }
    }
}

#[test]
fn icon_buttons_preserve_keyboard_labels_and_disabled_states(
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, enabled) in [
        ("Exporter le STL du modèle courant", true),
        ("Enregistrer", false),
    ] {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        frame(&mut state, &context, Vec::new());
        frame(&mut state, &context, Vec::new());
        let response = icon_response(&context, label).ok_or("missing icon button")?;
        assert_eq!(response.enabled(), enabled);
        assert_eq!(response.rect.size(), vec2(30.0, 30.0));
        response.request_focus();
        let output = frame(
            &mut state,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let named_click = output.platform_output.events.iter().any(|event| {
            matches!(event, egui::output::OutputEvent::Clicked(info)
                if info.typ == egui::WidgetType::Button
                    && info.label.as_deref() == Some(label))
        });
        assert_eq!(named_click, enabled);
        assert_eq!(state.take_action().is_some(), enabled);
        click(&mut state, &context, response.rect.center());
        assert_eq!(state.take_action().is_some(), enabled);
    }
    Ok(())
}
