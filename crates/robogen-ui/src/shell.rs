use super::*;

impl RoboGenUi {
    /// Draw one complete UI frame.
    pub fn show(&mut self, context: &Context) {
        let diagnostic_changed = self.document.poll_optimization_progress();
        let compilation_changed = self.document.poll_compilation();
        if diagnostic_changed || compilation_changed {
            self.rebuild_preview();
        }
        if self.document.build_progress().is_some() {
            context.request_repaint_after(std::time::Duration::from_millis(30));
            egui::TopBottomPanel::bottom("compilation_progress").show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    let (done, total) = self.document.build_progress().unwrap_or_default();
                    if total == 0 {
                        ui.label("Préparation du calcul...");
                    } else {
                        ui.label(format!("Construction CAD / TopOpt : {done}/{total} étapes"));
                    }
                    if ui.small_button("Annuler le calcul").clicked() {
                        self.document.cancel_build();
                        self.rebuild_preview();
                    }
                });
            });
        }
        if !self.theme_installed {
            install_theme(context);
            self.theme_installed = true;
        }

        let search_focused = context.memory(|memory| {
            memory.has_focus(egui::Id::new("robogen_taxonomy_search"))
                || memory.has_focus(egui::Id::new("robogen_library_search"))
        });
        if !search_focused {
            let (undo, redo) = context.input_mut(|input| {
                let redo = input
                    .consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
                    | input.consume_key(egui::Modifiers::CTRL, egui::Key::Y);
                let undo = input.consume_key(egui::Modifiers::CTRL, egui::Key::Z);
                (undo, redo)
            });
            if redo {
                self.redo();
            } else if undo {
                self.undo();
            }
        }

        self.application_bar(context);
        let previous_mode = self.mode;
        self.workspace_bar(context);
        if previous_mode != self.mode {
            self.rebuild_preview();
        }
        match self.mode {
            WorkspaceMode::Design => self.design_workspace(context),
            WorkspaceMode::Optimization => self.optimization_workspace(context),
            WorkspaceMode::Learning => self.learning_workspace(context),
        }
    }

    fn application_bar(&self, context: &Context) {
        TopBottomPanel::top("robogen_application_bar")
            .exact_height(38.0)
            .frame(
                Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::symmetric(16, 4))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.set_height(28.0);
                    ui.label(RichText::new("RoboGen").size(18.0).strong().color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.menu_button("Profil", |ui| {
                            ui.label("Profil non configuré");
                            ui.add_enabled(false, Button::new("Paramètres du compte"));
                        });
                    });
                });
            });
    }

    fn workspace_bar(&mut self, context: &Context) {
        TopBottomPanel::top("robogen_workspace_bar")
            .exact_height(56.0)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(12, 4))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(context, |ui| {
                let compact = ui.available_width() < 1280.0;
                ui.horizontal(|ui| {
                    ui.set_height(46.0);

                    stage_button(
                        ui,
                        &mut self.mode,
                        WorkspaceMode::Design,
                        "1",
                        "Design",
                        "Définir la robotique",
                        compact,
                    );
                    stage_button(
                        ui,
                        &mut self.mode,
                        WorkspaceMode::Optimization,
                        "2",
                        "Optimisation",
                        "Alléger et renforcer",
                        compact,
                    );
                    stage_button(
                        ui,
                        &mut self.mode,
                        WorkspaceMode::Learning,
                        "3",
                        "Apprentissage",
                        "Apprendre à agir",
                        compact,
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(
                            ui,
                            Icon::Export,
                            self.can_export(),
                            "Exporter le STL du modèle courant",
                            "Export indisponible : source invalide ou géométrie absente",
                        )
                        .clicked()
                        {
                            self.request_export();
                        }
                        icon_button(
                            ui,
                            Icon::Save,
                            false,
                            "Enregistrer",
                            "Enregistrement indisponible (M3)",
                        );
                        icon_button(
                            ui,
                            Icon::Plus,
                            false,
                            "Nouveau projet",
                            "Création de projet indisponible (M3)",
                        );
                        ui.menu_button("Exemples", |ui| {
                            if ui.button("Servomoteur").clicked() {
                                self.load_servo_example();
                                ui.close_menu();
                            }
                            if ui.button("Carte embarquée (104 × 90 mm)").clicked() {
                                self.load_compute_board_example();
                                ui.close_menu();
                            }
                            if ui.button("Caméra Raspberry Pi (2013)").clicked() {
                                self.load_camera_example();
                                ui.close_menu();
                            }
                            if ui.button("Batterie Li-ion (18 × 68 mm)").clicked() {
                                self.load_battery_example();
                                ui.close_menu();
                            }
                            if ui.button("Support batterie (contraintes TopOpt)").clicked() {
                                self.load_topology_example();
                                ui.close_menu();
                            }
                            if ui.button("TopOpt calculable (charges manuelles)").clicked() {
                                self.load_manual_topology_example();
                                ui.close_menu();
                            }
                            if ui
                                .button("Mini bipède — préparation des supports")
                                .clicked()
                            {
                                self.load_biped_example();
                                ui.close_menu();
                            }
                            if ui.button("Support contraint").clicked() {
                                self.set_dsl_source(DEFAULT_DSL);
                                ui.close_menu();
                            }
                        });
                        if icon_button(
                            ui,
                            Icon::Redo,
                            self.can_redo(),
                            "Rétablir (Ctrl+Shift+Z)",
                            "Rien à rétablir",
                        )
                        .clicked()
                        {
                            self.redo();
                        }
                        if icon_button(
                            ui,
                            Icon::Undo,
                            self.can_undo(),
                            "Annuler (Ctrl+Z)",
                            "Rien à annuler",
                        )
                        .clicked()
                        {
                            self.undo();
                        }
                        if let Some(message) = &self.status_message {
                            ui.label(RichText::new("ⓘ").color(MUTED))
                                .on_hover_text(message);
                        }
                    });
                });
            });
    }
}
