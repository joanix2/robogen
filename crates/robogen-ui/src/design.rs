use super::*;

impl RoboGenUi {
    pub(super) fn design_workspace(&mut self, context: &Context) {
        self.assistant_panel(context);
        self.taxonomy_panel(context);

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(10)))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    tab_button(ui, &mut self.design_tab, DesignTab::Dsl, "DSL");
                    tab_button(
                        ui,
                        &mut self.design_tab,
                        DesignTab::Kinematics,
                        "Graphe cinématique 2D",
                    );
                    tab_button(ui, &mut self.design_tab, DesignTab::Viewport, "Vue 3D");
                });
                ui.add_space(4.0);

                match self.design_tab {
                    DesignTab::Viewport => self.viewport(ui),
                    DesignTab::Dsl => self.dsl_editor(ui),
                    DesignTab::Kinematics => self.kinematics_view(ui),
                }
            });
    }

    fn taxonomy_panel(&mut self, context: &Context) {
        SidePanel::left("taxonomy_panel")
            .default_width(310.0)
            .min_width(260.0)
            .max_width(420.0)
            .resizable(true)
            .frame(side_frame())
            .show(context, |ui| {
                let height = ui.available_height();
                let upper = (height * self.taxonomy_fraction)
                    .max(140.0)
                    .min((height - 200.0).max(140.0));
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), upper),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(RichText::new("Taxonomie").size(16.0).color(TEXT));
                            ui.label(
                                RichText::new(format!("{} instances", self.taxonomy.len()))
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        });
                        if !self.document.is_valid() {
                            ui.label(
                                RichText::new(if self.document.build_progress().is_some() {
                                    "Calcul en cours · résultat précédent"
                                } else {
                                    "Source invalide · résultat précédent"
                                })
                                .size(11.0)
                                .color(Color32::from_rgb(224, 171, 90)),
                            );
                        }
                        ui.add_sized(
                            [ui.available_width(), 28.0],
                            TextEdit::singleline(&mut self.taxonomy_filter)
                                .id(egui::Id::new("robogen_taxonomy_search"))
                                .hint_text("Rechercher une instance…"),
                        );
                        ScrollArea::vertical()
                            .id_salt("instance_tree")
                            .max_height(ui.available_height())
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let query = self.taxonomy_filter.trim().to_lowercase();
                                let instances: Vec<_> = self
                                    .taxonomy
                                    .iter()
                                    .filter(|instance| {
                                        query.is_empty()
                                            || instance.name.to_lowercase().contains(&query)
                                            || instance.features.iter().any(|feature| {
                                                feature.name.to_lowercase().contains(&query)
                                                    || feature.functions.iter().any(|name| {
                                                        name.to_lowercase().contains(&query)
                                                    })
                                            })
                                    })
                                    .cloned()
                                    .collect();
                                if instances.is_empty() {
                                    ui.label(if self.taxonomy.is_empty() {
                                        "Aucune instance"
                                    } else {
                                        "Aucun résultat"
                                    });
                                }
                                for instance in instances {
                                    let id = EntityId(instance.id.0);
                                    let response = ui.selectable_label(
                                        self.selected_component == Some(id),
                                        &instance.name,
                                    );
                                    if response.clicked() {
                                        self.selected_component = Some(id);
                                    }
                                    if response.double_clicked() {
                                        self.open_source(instance.span);
                                    }
                                    response.on_hover_text("Double-clic : déclaration DSL");
                                    ui.indent(instance.id, |ui| {
                                        for feature in &instance.features {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    RichText::new(&feature.name)
                                                        .size(11.0)
                                                        .color(MUTED),
                                                );
                                                for name in &feature.functions {
                                                    if ui.link(format!("{name}()")).clicked() {
                                                        if let Some(function) = self
                                                            .functions
                                                            .iter()
                                                            .find(|function| function.name == *name)
                                                        {
                                                            self.open_source(function.span);
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                    },
                );
                let (divider, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::drag());
                ui.painter().hline(
                    divider.x_range(),
                    divider.center().y,
                    Stroke::new(1.0, BORDER),
                );
                let response = response.on_hover_cursor(egui::CursorIcon::ResizeVertical);
                if response.dragged() {
                    self.taxonomy_fraction = (self.taxonomy_fraction
                        + ui.input(|input| input.pointer.delta().y) / height.max(1.0))
                    .clamp(0.25, 0.7);
                }
                self.library_panel(ui);
            });
    }
}
