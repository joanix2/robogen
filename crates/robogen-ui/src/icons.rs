use super::*;

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Undo,
    Redo,
    Save,
    Plus,
    Minus,
    Export,
    OrbitLeft,
    OrbitRight,
    Home,
    Ortho,
}

pub(super) fn paint_icon(painter: &egui::Painter, rect: egui::Rect, icon: Icon, stroke: Stroke) {
    let center = rect.center();
    let path = |points: &[[f32; 2]]| {
        painter.add(Shape::line(
            points.iter().map(|[x, y]| center + vec2(*x, *y)).collect(),
            stroke,
        ));
    };
    match icon {
        Icon::Undo | Icon::Redo => {
            let direction = if matches!(icon, Icon::Undo) {
                1.0
            } else {
                -1.0
            };
            let points = [
                [-7.0, -3.0],
                [1.0, -3.0],
                [5.0, -1.0],
                [7.0, 3.0],
                [7.0, 6.0],
            ];
            path(&points.map(|[x, y]| [x * direction, y]));
            path(&[
                [-3.0 * direction, -7.0],
                [-7.0 * direction, -3.0],
                [-3.0 * direction, 1.0],
            ]);
        }
        Icon::Save => {
            path(&[
                [-7.0, -7.0],
                [4.0, -7.0],
                [7.0, -4.0],
                [7.0, 7.0],
                [-7.0, 7.0],
                [-7.0, -7.0],
            ]);
            path(&[[-3.0, -7.0], [-3.0, -2.0], [3.0, -2.0], [3.0, -7.0]]);
            path(&[[-4.0, 7.0], [-4.0, 2.0], [4.0, 2.0], [4.0, 7.0]]);
        }
        Icon::Plus | Icon::Minus => {
            path(&[[-6.0, 0.0], [6.0, 0.0]]);
            if matches!(icon, Icon::Plus) {
                path(&[[0.0, -6.0], [0.0, 6.0]]);
            }
        }
        Icon::Export => {
            path(&[[-7.0, 2.0], [-7.0, 7.0], [7.0, 7.0], [7.0, 2.0]]);
            path(&[[0.0, 3.0], [0.0, -7.0]]);
            path(&[[-4.0, -3.0], [0.0, -7.0], [4.0, -3.0]]);
        }
        Icon::OrbitLeft | Icon::OrbitRight => {
            let direction = if matches!(icon, Icon::OrbitLeft) {
                1.0
            } else {
                -1.0
            };
            let points: Vec<_> = (0..=20)
                .map(|step| {
                    let angle = std::f32::consts::TAU * (0.15 + 0.8 * step as f32 / 20.0);
                    [7.0 * angle.sin() * direction, -7.0 * angle.cos()]
                })
                .collect();
            path(&points);
            path(&[
                [1.0 * direction, -3.0],
                [-2.0 * direction, -7.0],
                [-6.0 * direction, -4.0],
            ]);
        }
        Icon::Home => {
            path(&[[-8.0, -1.0], [0.0, -8.0], [8.0, -1.0]]);
            path(&[
                [-6.0, -2.0],
                [-6.0, 7.0],
                [-2.0, 7.0],
                [-2.0, 2.0],
                [2.0, 2.0],
                [2.0, 7.0],
                [6.0, 7.0],
                [6.0, -2.0],
            ]);
        }
        Icon::Ortho => path(&[
            [-7.0, -7.0],
            [7.0, -7.0],
            [7.0, 7.0],
            [-7.0, 7.0],
            [-7.0, -7.0],
        ]),
    }
}

pub(super) fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    enabled: bool,
    tooltip: &str,
    unavailable: &str,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let response = ui.add_sized(vec2(30.0, 30.0), Button::new(""));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tooltip)
            });
            paint_icon(
                ui.painter(),
                response.rect,
                icon,
                ui.style().interact(&response).fg_stroke,
            );
            response
        })
        .inner;
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("icon_button", tooltip)), response.clone());
    });
    response
        .on_hover_text(tooltip)
        .on_disabled_hover_text(unavailable)
}
