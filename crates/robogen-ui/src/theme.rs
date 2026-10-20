use super::*;

pub(super) const BACKGROUND: Color32 = Color32::from_rgb(10, 17, 25);
pub(super) const PANEL: Color32 = Color32::from_rgb(15, 24, 34);
pub(super) const PANEL_RAISED: Color32 = Color32::from_rgb(22, 33, 45);
pub(super) const VIEWPORT: Color32 = Color32::from_rgb(17, 25, 34);
pub(super) const BORDER: Color32 = Color32::from_rgb(43, 58, 74);
pub(super) const TEXT: Color32 = Color32::from_rgb(226, 232, 240);
pub(super) const MUTED: Color32 = Color32::from_rgb(137, 151, 169);
pub(super) const ACCENT: Color32 = Color32::from_rgb(40, 116, 235);
pub(super) const ACCENT_HOVER: Color32 = Color32::from_rgb(56, 132, 250);
pub(super) const SUCCESS: Color32 = Color32::from_rgb(95, 190, 120);

pub(super) fn install_theme(context: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(8, 14, 21);
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.weak_bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(31, 47, 65);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(31, 47, 65);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(65, 91, 120));
    visuals.widgets.active.bg_fill = ACCENT;
    context.set_visuals(visuals);

    context.style_mut(|style| {
        style.spacing.item_spacing = vec2(7.0, 7.0);
        style.spacing.button_padding = vec2(11.0, 6.0);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(13.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(12.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(12.5));
    });
}

pub(super) fn side_frame() -> Frame {
    Frame::new()
        .fill(PANEL)
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0, BORDER))
}
