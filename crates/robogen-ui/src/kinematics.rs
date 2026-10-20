//! Read-only 2D view of declared instances and ports.
//!
//! Mechanical joints are not inferred from geometry or spatial proximity.
use super::*;

impl RoboGenUi {
    pub(super) fn kinematics_view(&mut self, ui: &mut Ui) {
        ui.heading("Graphe cinématique 2D");
        if !self.document.is_valid() {
            ui.colored_label(
                Color32::from_rgb(224, 171, 90),
                "Dernier modèle valide · graphe non actualisé",
            );
        }
        ui.label("Instances et interfaces déclarées dans le DSL");
        ui.colored_label(
            MUTED,
            "Articulations cinématiques non encore prises en charge.",
        );
        ui.small("Les interfaces affichées ne définissent pas encore les corps rigides et les articulations.");
        let source_current = self.document.is_valid();
        let Some(project) = self.document.project() else {
            ui.label("Aucun modèle valide disponible.");
            return;
        };
        let instances = &project.snapshot().model.instances;
        if instances.is_empty() {
            ui.add_space(16.0);
            ui.label("Aucun objet instancié dans ce document.");
            return;
        }
        ui.separator();
        ScrollArea::both()
            .id_salt("kinematics_graph_canvas")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = (ui.available_width() / 235.0).floor().max(1.0) as usize;
                let mut row_height = 0.0_f32;
                let mut y = 12.0;
                let origin = ui.cursor().min;
                let mut bottom = 0.0_f32;
                for (index, instance) in instances.values().enumerate() {
                    let column = index % columns;
                    if column == 0 && index > 0 {
                        y += row_height + 24.0;
                        row_height = 0.0;
                    }
                    let height = 68.0 + instance.interfaces.len().max(1) as f32 * 26.0;
                    row_height = row_height.max(height);
                    let rect = egui::Rect::from_min_size(
                        origin + vec2(column as f32 * 235.0, y),
                        vec2(220.0, height),
                    );
                    let response = ui.allocate_rect(rect, Sense::click());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 7.0, PANEL);
                    painter.rect_stroke(
                        rect,
                        7.0,
                        Stroke::new(1.0, BORDER),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        rect.left_top() + vec2(12.0, 18.0),
                        egui::Align2::LEFT_CENTER,
                        &instance.name,
                        FontId::proportional(12.0),
                        TEXT,
                    );
                    painter.text(
                        rect.left_top() + vec2(12.0, 43.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{} interface(s)", instance.interfaces.len()),
                        FontId::proportional(11.0),
                        MUTED,
                    );
                    for (port, name) in instance.interfaces.keys().enumerate() {
                        let point = rect.left_top() + vec2(18.0, 70.0 + port as f32 * 26.0);
                        painter.circle_filled(point, 4.0, SUCCESS);
                        painter.text(
                            point + vec2(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            name,
                            FontId::monospace(11.0),
                            TEXT,
                        );
                    }
                    let response = response
                        .on_hover_text("Ouvrir la déclaration de cette instance dans le DSL");
                    if response.clicked() && source_current {
                        self.source_selection = Some(instance.span);
                        self.design_tab = DesignTab::Dsl;
                    }
                    bottom = bottom.max(y + height + 12.0);
                }
                ui.set_min_size(vec2(columns as f32 * 235.0 - 15.0, bottom));
            });
    }
}
