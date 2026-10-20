use super::*;

pub(super) fn collect_painted_text(shape: &Shape, text: &mut String) {
    match shape {
        Shape::Text(label) => {
            text.push_str(&label.galley.job.text);
            text.push('\n');
        }
        Shape::Vec(shapes) => {
            for shape in shapes {
                collect_painted_text(shape, text);
            }
        }
        _ => {}
    }
}

pub(super) fn frame(
    state: &mut RoboGenUi,
    context: &Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(1080.0, 680.0),
            )),
            events,
            ..Default::default()
        },
        |context| state.show(context),
    )
}

pub(super) fn text_rect(output: &egui::FullOutput, text: &str) -> Option<egui::Rect> {
    output.shapes.iter().find_map(|shape| match &shape.shape {
        Shape::Text(label) if label.galley.job.text == text => {
            Some(egui::Rect::from_min_size(label.pos, label.galley.size()))
        }
        _ => None,
    })
}

pub(super) fn icon_response(context: &Context, label: &str) -> Option<egui::Response> {
    context.data(|data| data.get_temp(egui::Id::new(("icon_button", label))))
}

pub(super) fn click(state: &mut RoboGenUi, context: &Context, position: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            state,
            context,
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
