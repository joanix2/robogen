use super::*;

impl RoboGenUi {
    pub(super) fn optimization_workspace(&mut self, context: &Context) {
        if self
            .document
            .project()
            .is_some_and(|project| !project.snapshot().preparation.is_empty())
        {
            self.topology_workspace(context);
            return;
        }
        SidePanel::left("optimization_parameters")
            .exact_width(280.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ScrollArea::vertical().show(ui, optimization_parameters);
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(12)))
            .show(context, |ui| {
                section_tabs(
                    ui,
                    &mut self.optimization_tab,
                    &["Résultat", "Comparaison", "Analyse", "Historique"],
                );
                ui.add_space(9.0);

                if self.optimization_tab < 2 {
                    ui.columns(2, |columns| {
                        model_preview_card(
                            &mut columns[0],
                            &self.preview_renderer,
                            &self.camera,
                            &self.preview_mesh,
                            "Avant optimisation",
                            if self.document.is_valid() {
                                "Géométrie paramétrique actuelle"
                            } else {
                                "Dernier aperçu valide · source invalide"
                            },
                            210.0,
                        );
                        unavailable_result_card(
                            &mut columns[1],
                            "Après optimisation",
                            "Aucun résultat TopOpt publié",
                            210.0,
                        );
                    });
                }

                ui.add_space(8.0);
                match self.optimization_tab {
                    0 => {
                        ui.label(RichText::new("Aucune métrique calculée").color(MUTED));
                        ui.columns(3, |columns| {
                            for (index, label) in [
                                "Masse",
                                "Réduction",
                                "Compliance",
                                "Déplacement max",
                                "Von Mises max",
                                "Sécurité",
                            ]
                            .iter()
                            .enumerate()
                            {
                                optimization_metric(&mut columns[index % 3], label, "—");
                            }
                        });
                    }
                    1 => unavailable_result_card(
                        ui,
                        "Comparaison",
                        "Aucune comparaison calculée",
                        ui.available_height(),
                    ),
                    2 => analysis_placeholder(ui, ui.available_height()),
                    _ => unavailable_result_card(
                        ui,
                        "Historique",
                        "Aucune exécution publiée",
                        ui.available_height(),
                    ),
                }
            });
    }
}

fn optimization_parameters(ui: &mut Ui) {
    ui.heading(
        RichText::new("Paramètres d’optimisation")
            .size(16.0)
            .color(TEXT),
    );
    ui.add_space(10.0);
    ui.label(
        RichText::new("Solveur indisponible · paramètres indicatifs")
            .size(11.0)
            .color(MUTED),
    );
    form_label(ui, "Objectif");
    value_field(ui, "Minimiser la masse");
    ui.add_space(10.0);
    form_label(ui, "Contraintes mécaniques");
    value_field(ui, "Aluminium 6061");
    value_field(ui, "Charge maximale : 200 N");
    value_field(ui, "Facteur de sécurité : 2.0");
    ui.add_space(10.0);
    form_label(ui, "Contraintes géométriques");
    toggle_row(ui, "Conserver les interfaces", true);
    toggle_row(ui, "Respecter l’enveloppe", true);
    toggle_row(ui, "Zones interdites", false);
    ui.add_space(10.0);
    form_label(ui, "Contraintes de fabrication");
    value_field(ui, "Impression 3D (FDM)");
    value_field(ui, "Épaisseur min. : 2.0 mm");
    value_field(ui, "Surplomb max. : 45°");
    toggle_row(ui, "Éviter les supports", true);

    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.add_enabled(
            false,
            Button::new("Lancer l’optimisation").min_size(vec2(ui.available_width(), 34.0)),
        );
        ui.label(RichText::new("Backend requis").size(11.0).color(MUTED));
    });
}

fn optimization_metric(ui: &mut Ui, title: &str, value: &str) {
    Frame::new()
        .fill(PANEL_RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(6)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 24.0).max(40.0));
            ui.label(RichText::new(title).size(11.0).color(MUTED));
            ui.add_space(14.0);
            ui.label(RichText::new(value).size(20.0).strong().color(TEXT));
        });
}

/// Present a report already accepted by the project publication guards.
pub(super) fn topology_report_view(ui: &mut Ui, report: &robogen_project::TopologyReport) {
    ui.separator();
    ui.label(RichText::new("Résultat reanalysé").strong());
    ui.label(format!(
        "{} itérations · {:.3} g",
        report.iterations,
        report.mass.kilograms() * 1000.0
    ));
    ui.label(format!(
        "Volume conservé : {:.2} %",
        report.volume_fraction * 100.0
    ));
    ui.label(format!(
        "Grille {:?} · filtre {} mm · seuil {}",
        report.settings.cells,
        report.settings.filter_radius.millimetres(),
        report.settings.threshold
    ));
    ui.label(format!(
        "Révision {} · {}",
        report.document_revision, report.solver_backend
    ));
    for case in &report.cases {
        ui.label(format!(
            "Cas {} · {:.4e} J",
            case.id,
            case.compliance.joules()
        ));
        ui.label(format!(
            "Déplacement {:.6} mm · résidu {:.2e}",
            case.max_displacement.millimetres(),
            case.relative_residual
        ));
    }
}

/// Backend selection inherits the caller's capability and busy guards.
pub(super) fn mass_compute_backend_selector(
    ui: &mut Ui,
    selected: &mut robogen_project::ComputeBackend,
) {
    ui.horizontal(|ui| {
        for (backend, title) in [
            (robogen_project::ComputeBackend::Cpu, "CPU"),
            (robogen_project::ComputeBackend::Cuda, "GPU CUDA"),
        ] {
            let response = ui.selectable_value(selected, backend, title);
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new(("topology_control", title)), response.clone());
            });
            #[cfg(not(test))]
            let _ = response;
        }
    });
}
