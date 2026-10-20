use super::*;

impl RoboGenUi {
    pub(super) fn assistant_panel(&mut self, context: &Context) {
        SidePanel::right("assistant_panel")
            .exact_width(310.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Assistant IA").size(16.0).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        status_badge(ui, "Indisponible", MUTED);
                    });
                });
                ui.add_space(14.0);
                ui.label(RichText::new("Conversation de démonstration").size(11.0).color(MUTED));

                chat_bubble(
                    ui,
                    "Vous",
                    "Crée-moi un robot araignée à 8 pattes capable d’évoluer sur un terrain irrégulier.",
                    true,
                );
                ui.add_space(9.0);
                chat_bubble(
                    ui,
                    "Assistant IA",
                    &self.assistant_message,
                    false,
                );

                ui.add_space(10.0);
                Frame::new()
                    .fill(Color32::from_rgb(17, 31, 44))
                    .corner_radius(7)
                    .inner_margin(Margin::same(10))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(38, 71, 98)))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("○  Provider désactivé")
                                .size(11.0)
                                .color(Color32::from_rgb(115, 180, 229)),
                        );
                        ui.label(
                            RichText::new("Aucun appel réseau ni modèle chargé")
                                .size(11.0)
                                .color(MUTED),
                        );
                    });

                ui.add_space(12.0);
                ui.label(RichText::new("Suggestions indisponibles").size(12.0).color(MUTED));
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled(false, Button::new("Ajuster la taille"));
                    ui.add_enabled(false, Button::new("Ajouter des capteurs"));
                    ui.add_enabled(false, Button::new("Faire plus léger"));
                });

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.add_enabled(
                        false,
                        TextEdit::singleline(&mut String::new())
                            .hint_text("Posez une question ou demandez une modification…")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("L’assistant est désactivé").size(11.0).color(MUTED));
                });
            });
    }
}

fn chat_bubble(ui: &mut Ui, author: &str, message: &str, from_user: bool) {
    Frame::new()
        .fill(if from_user { PANEL_RAISED } else { PANEL })
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(author)
                    .size(11.0)
                    .strong()
                    .color(if from_user { TEXT } else { ACCENT_HOVER }),
            );
            ui.add(egui::Label::new(RichText::new(message).color(TEXT)).wrap());
        });
}
