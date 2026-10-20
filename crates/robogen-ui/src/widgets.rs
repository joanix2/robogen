use super::*;

pub(super) fn stage_button(
    ui: &mut Ui,
    current: &mut WorkspaceMode,
    target: WorkspaceMode,
    number: &str,
    title: &str,
    subtitle: &str,
    compact: bool,
) {
    let selected = *current == target;
    let text = RichText::new(format!("{number}    {title}\n       {subtitle}"))
        .size(12.0)
        .color(if selected { Color32::WHITE } else { TEXT });
    let button = Button::new(text)
        .fill(if selected {
            ACCENT
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0,
            if selected { ACCENT_HOVER } else { BORDER },
        ))
        .corner_radius(5);
    if ui
        .add_sized([if compact { 148.0 } else { 185.0 }, 47.0], button)
        .clicked()
    {
        *current = target;
    }
}

pub(super) fn diamond_mark(ui: &mut Ui, size: f32, color: Color32, filled: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let center = rect.center();
    let radius = size * 0.4;
    ui.painter().add(Shape::convex_polygon(
        vec![
            center + vec2(0.0, -radius),
            center + vec2(radius, 0.0),
            center + vec2(0.0, radius),
            center + vec2(-radius, 0.0),
        ],
        if filled { color } else { Color32::TRANSPARENT },
        Stroke::new(1.5, color),
    ));
}

pub(super) fn tab_button(ui: &mut Ui, current: &mut DesignTab, target: DesignTab, title: &str) {
    let selected = *current == target;
    let button =
        Button::new(RichText::new(title).color(if selected { Color32::WHITE } else { MUTED }))
            .fill(if selected { ACCENT } else { PANEL })
            .stroke(Stroke::new(
                1.0,
                if selected { ACCENT_HOVER } else { BORDER },
            ));
    let width = if target == DesignTab::Kinematics {
        190.0
    } else {
        120.0
    };
    if ui.add_sized([width, 31.0], button).clicked() {
        *current = target;
    }
}

pub(super) fn status_badge(ui: &mut Ui, text: &str, text_color: Color32) {
    Frame::new()
        .fill(PANEL_RAISED)
        .corner_radius(5)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(text_color));
        });
}

pub(super) fn section_tabs(ui: &mut Ui, selected: &mut usize, titles: &[&str]) {
    ui.horizontal(|ui| {
        for (index, title) in titles.iter().enumerate() {
            let response = ui.add_sized(
                [105.0, 29.0],
                Button::new(
                    RichText::new(*title)
                        .size(11.0)
                        .color(if index == *selected {
                            Color32::WHITE
                        } else {
                            MUTED
                        }),
                )
                .fill(if index == *selected { ACCENT } else { PANEL })
                .stroke(Stroke::new(
                    1.0,
                    if index == *selected {
                        ACCENT_HOVER
                    } else {
                        BORDER
                    },
                )),
            );
            if response.clicked() {
                *selected = index;
            }
        }
    });
}

pub(super) fn unavailable_result_card(ui: &mut Ui, title: &str, detail: &str, height: f32) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::top_down(Align::Min),
        |ui| unavailable_canvas(ui, title, detail, "Aucun résultat calculé"),
    );
}

pub(super) fn analysis_placeholder(ui: &mut Ui, height: f32) {
    unavailable_result_card(ui, "Analyse mécanique", "Solveur indisponible", height);
}

pub(super) fn empty_chart(ui: &mut Ui, title: &str, unit: &str, height: f32) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::top_down(Align::Min),
        |ui| {
            unavailable_canvas(
                ui,
                title,
                "Entraînement indisponible",
                &format!("Aucune donnée · {unit}"),
            );
        },
    );
}

pub(super) fn unavailable_canvas(ui: &mut Ui, title: &str, detail: &str, note: &str) {
    let size = ui.available_size();
    Frame::new()
        .fill(VIEWPORT)
        .corner_radius(7)
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.set_min_size(size);
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(RichText::new(title).size(13.0).color(TEXT));
                    ui.add_space(6.0);
                    ui.add(egui::Label::new(RichText::new(detail).color(MUTED)).wrap());
                    ui.add_space(3.0);
                    ui.add(egui::Label::new(RichText::new(note).size(11.0).color(MUTED)).wrap());
                });
            });
        });
}

pub(super) fn form_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(MUTED));
}

pub(super) fn value_field(ui: &mut Ui, value: &str) {
    ui.add_enabled(
        false,
        Button::new(value).min_size(vec2(ui.available_width(), 27.0)),
    )
    .on_disabled_hover_text("Paramètre indicatif · backend indisponible");
}

pub(super) fn toggle_row(ui: &mut Ui, label: &str, enabled: bool) {
    let mut checked = enabled;
    ui.add_enabled(false, egui::Checkbox::new(&mut checked, label))
        .on_disabled_hover_text("Contrainte indicative · backend indisponible");
}
