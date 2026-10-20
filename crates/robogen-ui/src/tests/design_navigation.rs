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

fn click(state: &mut RoboGenUi, context: &Context, position: egui::Pos2) {
    for pressed in [true, false] {
        let _ = draw(
            state,
            context,
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
}
fn painted(output: &egui::FullOutput) -> String {
    let mut text = String::new();
    for shape in &output.shapes {
        collect_painted_text(&shape.shape, &mut text);
    }
    text
}
const VISE: &str = include_str!("../../../../examples/kinematic_vise/main.rgn");

#[test]
fn mechanical_graph_and_schematic_share_declared_joints_and_source_provenance() -> Result<(), String>
{
    let mut state = RoboGenUi::default();
    state.load_kinematic_vise_example();
    assert!(state.document.is_valid());
    state.design_tab = DesignTab::Kinematics;
    let context = Context::default();
    let size = vec2(1440., 831.);
    let _ = draw(&mut state, &context, size, vec![]);
    let output = draw(&mut state, &context, size, vec![]);
    let text = painted(&output);
    for label in [
        "4 corps · 4 liaisons",
        "Glissière · axe X",
        "Pivot · axe X",
        "Hélicoïdale · axe X · pas 2.0 mm/tr",
        "Pivot glissant · axe Y",
    ] {
        assert!(text.contains(label), "missing {label}: {text}");
    }
    let node = text_position(&output.shapes, "A").ok_or("missing rigid body")?;
    click(&mut state, &context, node);
    assert!(state.graph_canvas.selected.is_some());
    let output = draw(&mut state, &context, size, vec![]);
    let button = text_position(&output.shapes, "Schéma cinématique").ok_or("missing schematic")?;
    click(&mut state, &context, button);
    assert_eq!(
        state.graph_canvas.mode,
        kinematics::canvas::DiagramMode::Schematic
    );
    for plane in ["XY", "YZ", "XZ"] {
        let output = draw(&mut state, &context, size, vec![]);
        let button = text_position(&output.shapes, plane).ok_or("missing projection")?;
        click(&mut state, &context, button);
        let output = draw(&mut state, &context, size, vec![]);
        assert!(painted(&output).contains("Pivot glissant · axe Y"));
    }
    let output = draw(&mut state, &context, size, vec![]);
    let joint = text_position(&output.shapes, "BC\nPivot · axe X").ok_or("missing joint label")?;
    click(&mut state, &context, joint);
    assert!(matches!(
        state.graph_canvas.selected,
        Some(kinematics::canvas::Selection::Joint(_))
    ));
    assert_eq!(state.dsl_source(), VISE);
    let output = draw(&mut state, &context, size, vec![]);
    let source = text_position(&output.shapes, "Ouvrir dans le DSL").ok_or("missing source")?;
    click(&mut state, &context, source);
    assert_eq!(state.design_tab, DesignTab::Dsl);
    assert!(state.source_selection.is_some());
    state.design_tab = DesignTab::Kinematics;
    state.source_selection = None;
    state.set_dsl_source("module broken; part");
    let output = draw(&mut state, &context, size, vec![]);
    let text = painted(&output);
    assert!(text.contains("Dernier modèle valide"));
    assert!(text.contains("Pivot glissant · axe Y"));
    let source =
        text_position(&output.shapes, "Ouvrir dans le DSL").ok_or("missing disabled source")?;
    click(&mut state, &context, source);
    assert_eq!(state.design_tab, DesignTab::Kinematics);
    assert!(state.source_selection.is_none());
    assert!(!state.can_export());
    Ok(())
}

fn canvas_text_rects(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
    fn collect(shape: &Shape, found: &mut Vec<(String, egui::Rect)>) {
        match shape {
            Shape::Text(text) => {
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                let in_canvas = rect.min.x > 320.0 && rect.max.x < 1130.0 && rect.min.y > 300.0;
                if in_canvas {
                    found.push((text.galley.job.text.clone(), rect));
                }
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, &mut found);
    }
    found
}

fn assert_canvas_text_is_legible(output: &egui::FullOutput, context: &str, joints: usize) {
    let texts = canvas_text_rects(output);
    let labels = texts.iter().filter(|(text, _)| text.contains(" · axe"));
    let labels: Vec<_> = labels.collect();
    assert_eq!(
        labels.len(),
        joints,
        "{context}: one label per joint {labels:?}"
    );
    for (index, (a_text, a)) in texts.iter().enumerate() {
        for (b_text, b) in &texts[index + 1..] {
            assert!(
                !a.intersects(*b),
                "{context}: {a_text:?} {a:?} overlaps {b_text:?} {b:?}"
            );
        }
    }
}

fn open_kinematics(state: &mut RoboGenUi, context: &Context, size: Vec2) {
    state.design_tab = DesignTab::Kinematics;
    let _ = draw(state, context, size, vec![]);
}

#[test]
fn schematic_labels_never_overlap_in_any_projection() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    state.load_kinematic_vise_example();
    let context = Context::default();
    let size = vec2(1440., 831.);
    open_kinematics(&mut state, &context, size);
    let output = draw(&mut state, &context, size, vec![]);
    let button = text_position(&output.shapes, "Schéma cinématique").ok_or("missing schematic")?;
    click(&mut state, &context, button);
    for plane in ["XY", "XZ", "YZ"] {
        let output = draw(&mut state, &context, size, vec![]);
        let button = text_position(&output.shapes, plane).ok_or("missing projection")?;
        click(&mut state, &context, button);
        let output = draw(&mut state, &context, size, vec![]);
        assert_canvas_text_is_legible(&output, plane, 4);
    }
    Ok(())
}

#[test]
fn graph_labels_and_body_names_never_overlap_for_vise_and_biped() -> Result<(), String> {
    let context = Context::default();
    let size = vec2(1440., 831.);
    let mut vise = RoboGenUi::default();
    vise.load_kinematic_vise_example();
    open_kinematics(&mut vise, &context, size);
    let output = draw(&mut vise, &context, size, vec![]);
    assert_canvas_text_is_legible(&output, "vise", 4);

    let mut biped = RoboGenUi::default();
    biped.load_biped_example();
    assert!(biped.document.is_valid());
    open_kinematics(&mut biped, &context, size);
    let output = draw(&mut biped, &context, size, vec![]);
    assert_canvas_text_is_legible(&output, "biped", 14);
    Ok(())
}

#[test]
fn geometry_alone_does_not_invent_a_mechanical_graph() {
    let mut state = RoboGenUi {
        design_tab: DesignTab::Kinematics,
        ..RoboGenUi::default()
    };
    let output = draw(&mut state, &Context::default(), vec2(1440., 831.), vec![]);
    assert!(painted(&output).contains("Aucun corps rigide déclaré"));
}
