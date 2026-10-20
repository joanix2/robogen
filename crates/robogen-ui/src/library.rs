use super::*;

impl RoboGenUi {
    pub(super) fn library_panel(&mut self, ui: &mut Ui) {
        ui.heading(RichText::new("Bibliothèque").size(16.0).color(TEXT));
        ui.add_sized(
            [ui.available_width(), 28.0],
            TextEdit::singleline(&mut self.library_filter)
                .id(egui::Id::new("robogen_library_search"))
                .hint_text("Rechercher une fonction…"),
        );
        ScrollArea::vertical()
            .id_salt("function_library")
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let query = self.library_filter.trim().to_lowercase();
                for generator in LibraryGenerator::ALL {
                    if !query.is_empty()
                        && !generator.label().to_lowercase().contains(&query)
                        && !generator.name().contains(&query)
                    {
                        continue;
                    }
                    ui.push_id(generator.name(), |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(
                                    self.selected_generator == Some(generator),
                                    generator.label(),
                                )
                                .clicked()
                            {
                                self.selected_generator = Some(generator);
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icon_button(
                                    ui,
                                    Icon::Plus,
                                    self.document.is_valid(),
                                    &format!("Instancier {}", generator.label()),
                                    "Document invalide ou calcul en cours",
                                )
                                .clicked()
                                {
                                    self.instantiate_generator(generator);
                                }
                            });
                        });
                        ui.label(
                            RichText::new(format!("{}()", generator.name()))
                                .monospace()
                                .size(11.0)
                                .color(MUTED),
                        );
                        if self.selected_generator == Some(generator) {
                            if generator == LibraryGenerator::JetsonNanoSuper {
                                ui.label(
                                    RichText::new("Modèle approximatif · 104 × 90 × 37 mm")
                                        .size(11.0)
                                        .color(MUTED),
                                );
                            }
                            if let Some(function) = self
                                .functions
                                .iter()
                                .find(|function| function.name == generator.name())
                                .cloned()
                            {
                                ui.label(RichText::new(&function.signature).monospace().size(11.0));
                                if ui
                                    .add_enabled(
                                        self.document.is_valid(),
                                        Button::new("Définition DSL"),
                                    )
                                    .clicked()
                                {
                                    self.open_source(function.span);
                                }
                            } else {
                                ui.label(
                                    RichText::new("Non instanciée dans ce document")
                                        .size(11.0)
                                        .color(MUTED),
                                );
                            }
                        }
                        ui.separator();
                    });
                }
                let other: Vec<_> = self
                    .functions
                    .iter()
                    .filter(|function| {
                        !LibraryGenerator::ALL
                            .iter()
                            .any(|generator| generator.name() == function.name)
                            && (query.is_empty() || function.name.to_lowercase().contains(&query))
                    })
                    .cloned()
                    .collect();
                if !other.is_empty() {
                    egui::CollapsingHeader::new("Fonctions du document")
                        .open((!query.is_empty()).then_some(true))
                        .show(ui, |ui| {
                            for function in other {
                                if ui
                                    .add_enabled(
                                        self.document.is_valid(),
                                        egui::Button::new(&function.name).frame(false),
                                    )
                                    .on_hover_text(&function.signature)
                                    .clicked()
                                {
                                    self.open_source(function.span);
                                }
                            }
                        });
                }
            });
    }
}
