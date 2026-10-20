use super::*;

impl RoboGenUi {
    pub(super) fn learning_workspace(&mut self, context: &Context) {
        SidePanel::left("learning_parameters")
            .exact_width(285.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| learning_parameters(ui, &mut self.selected_terrain));
            });

        SidePanel::right("learning_metrics")
            .exact_width(265.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Métriques").size(16.0).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        status_badge(ui, "Hors ligne", MUTED);
                    });
                });
                ui.add_space(8.0);
                let chart_height = ((ui.available_height() - 16.0) / 3.0).min(170.0);
                empty_chart(ui, "Récompense moyenne", "reward", chart_height);
                ui.add_space(8.0);
                empty_chart(ui, "Vitesse", "m/s", chart_height);
                ui.add_space(8.0);
                empty_chart(ui, "Taux de chute", "%", chart_height);
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(12)))
            .show(context, |ui| {
                if self.document.has_errors() {
                    ui.label(RichText::new("Dernier aperçu valide · source invalide").color(MUTED));
                }
                simulation_preview_card(
                    ui,
                    &self.preview_renderer,
                    &self.camera,
                    &self.preview_mesh,
                    (ui.available_height() - 132.0).max(260.0),
                );
                ui.add_space(8.0);
                behavior_timeline(ui);
            });
    }
}

fn learning_parameters(ui: &mut Ui, selected_terrain: &mut usize) {
    ui.heading(RichText::new("Environnement").size(16.0).color(TEXT));
    ui.add_space(10.0);
    ui.label(
        RichText::new("Entraînement indisponible")
            .size(11.0)
            .color(MUTED),
    );
    ui.label(
        RichText::new("Foundation model non configuré")
            .size(11.0)
            .color(MUTED),
    );
    form_label(ui, "Terrain");
    value_field(ui, "Irrégulier");
    ui.add_enabled_ui(false, |ui| {
        ui.columns(4, |columns| {
            for (index, name) in ["Plat", "Rochers", "Escaliers", "Sable"]
                .into_iter()
                .enumerate()
            {
                if environment_tile(&mut columns[index], name, *selected_terrain == index).clicked()
                {
                    *selected_terrain = index;
                }
            }
        });
    })
    .response
    .on_disabled_hover_text("Configuration de simulation indisponible");
    ui.add_space(14.0);
    ui.heading(
        RichText::new("Paramètres d’entraînement")
            .size(15.0)
            .color(TEXT),
    );
    form_label(ui, "Algorithme");
    value_field(ui, "PPO");
    value_field(ui, "4096 environnements");
    value_field(ui, "10 M étapes");
    ui.add_space(10.0);
    form_label(ui, "Récompense");
    for criterion in [
        "Vitesse avant",
        "Stabilité",
        "Économie d’énergie",
        "Éviter les chutes",
    ] {
        ui.label(
            RichText::new(format!("●  {criterion}"))
                .size(12.0)
                .color(MUTED),
        );
    }
    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.add_enabled(
            false,
            Button::new("Lancer l’entraînement").min_size(vec2(ui.available_width(), 34.0)),
        );
    });
}

fn simulation_preview_card(
    ui: &mut Ui,
    renderer: &RobotPreviewRenderer,
    camera: &Camera,
    mesh: &PreviewMesh,
    height: f32,
) {
    model_preview_card(
        ui,
        renderer,
        camera,
        mesh,
        "Aperçu statique du modèle",
        "Simulation et entraînement indisponibles",
        height,
    );
}

fn behavior_timeline(ui: &mut Ui) {
    unavailable_result_card(
        ui,
        "Chronologie du comportement",
        "Aucune trajectoire · Simulation indisponible",
        100.0,
    );
}

fn environment_tile(ui: &mut Ui, name: &str, selected: bool) -> egui::Response {
    Frame::new()
        .fill(if selected {
            Color32::from_rgb(24, 57, 91)
        } else {
            PANEL_RAISED
        })
        .stroke(Stroke::new(
            1.0,
            if selected { ACCENT_HOVER } else { BORDER },
        ))
        .corner_radius(5)
        .inner_margin(Margin::symmetric(3, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                diamond_mark(
                    ui,
                    17.0,
                    if selected { ACCENT_HOVER } else { MUTED },
                    selected,
                );
                ui.label(RichText::new(name).size(9.0).color(TEXT));
            });
        })
        .response
        .interact(Sense::click())
}
