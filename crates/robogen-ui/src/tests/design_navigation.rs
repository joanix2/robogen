use super::*;

fn draw(
    state: &mut RoboGenUi,
    context: &Context,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |context| state.show(context),
    )
}

fn text_position(shapes: &[egui::epaint::ClippedShape], target: &str) -> Option<egui::Pos2> {
    fn find(shape: &Shape, target: &str) -> Option<egui::Pos2> {
        match shape {
            Shape::Text(text) if text.galley.job.text == target => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, target)),
            _ => None,
        }
    }
    shapes.iter().find_map(|shape| find(&shape.shape, target))
}

#[test]
fn design_tabs_are_ordered_and_navigate_at_both_supported_sizes() -> Result<(), String> {
    for size in [vec2(1080.0, 680.0), vec2(1440.0, 831.0)] {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        assert_eq!(state.design_tab, DesignTab::Dsl);
        let source = state.dsl_source().to_owned();
        let _ = draw(&mut state, &context, size, Vec::new());
        let mut output = draw(&mut state, &context, size, Vec::new());
        let labels = ["DSL", "Graphe cinématique 2D", "Vue 3D"];
        let positions: Vec<_> = labels
            .iter()
            .map(|label| {
                text_position(&output.shapes, label).ok_or_else(|| format!("missing tab {label}"))
            })
            .collect::<Result<_, _>>()?;
        assert!(positions.windows(2).all(|pair| pair[0].x < pair[1].x));
        assert!(positions
            .iter()
            .all(|pos| (pos.y - positions[0].y).abs() < 1.0));
        for (label, tab) in [
            (labels[1], DesignTab::Kinematics),
            (labels[2], DesignTab::Viewport),
            (labels[0], DesignTab::Dsl),
        ] {
            let position = text_position(&output.shapes, label).ok_or("missing tab")?;
            let _ = draw(
                &mut state,
                &context,
                size,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            output = draw(
                &mut state,
                &context,
                size,
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(state.design_tab, tab);
        }
        assert_eq!(state.dsl_source(), source);
        assert!(!state.can_undo());
        assert!(state.can_export());
        assert!(state.take_action().is_none());
    }
    Ok(())
}

#[test]
fn graph_shows_only_declared_ports_and_marks_the_last_valid_model() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    state.set_dsl_source(
        r#"module graph;
component object_model() -> Object {
    return object(shape: box([10 mm, 10 mm, 10 mm]), void: [],
        interfaces: [interface(name: "mount", shape: cylinder(2 mm, 1 mm))]);
}
instance ActuatorA = object_model();
instance ActuatorB = translate([30 mm, 0 mm, 0 mm], object_model());
"#,
    );
    assert!(state.document.is_valid());
    state.design_tab = DesignTab::Kinematics;
    let context = Context::default();
    let _ = draw(&mut state, &context, vec2(1440.0, 831.0), Vec::new());
    let output = draw(&mut state, &context, vec2(1440.0, 831.0), Vec::new());
    let mut text = String::new();
    for shape in output.shapes {
        collect_painted_text(&shape.shape, &mut text);
    }
    assert!(text.contains("ActuatorA") && text.contains("ActuatorB"));
    assert_eq!(text.matches("mount").count(), 2);
    assert!(text.contains("non encore prises en charge"));
    let output = draw(&mut state, &context, vec2(1440.0, 831.0), Vec::new());
    let position = text_position(&output.shapes, "ActuatorA").ok_or("missing instance card")?;
    for pressed in [true, false] {
        let _ = draw(
            &mut state,
            &context,
            vec2(1440.0, 831.0),
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(state.design_tab, DesignTab::Dsl);
    assert!(state.source_selection.is_some());
    state.design_tab = DesignTab::Kinematics;
    state.source_selection = None;
    state.set_dsl_source("module broken; part".to_owned());
    let output = draw(&mut state, &context, vec2(1440.0, 831.0), Vec::new());
    let mut text = String::new();
    for shape in output.shapes {
        collect_painted_text(&shape.shape, &mut text);
    }
    assert!(text.contains("Dernier modèle valide"));
    assert!(text.contains("ActuatorA"));
    let output = draw(&mut state, &context, vec2(1440.0, 831.0), Vec::new());
    let position = text_position(&output.shapes, "ActuatorA").ok_or("missing stale instance")?;
    for pressed in [true, false] {
        let _ = draw(
            &mut state,
            &context,
            vec2(1440.0, 831.0),
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(state.design_tab, DesignTab::Kinematics);
    assert!(state.source_selection.is_none());
    assert!(!state.can_export());
    Ok(())
}
