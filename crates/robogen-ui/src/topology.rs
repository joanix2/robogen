use super::mass_diagnostics::mass_diagnostic_view;
use super::*;

impl RoboGenUi {
    pub(super) fn topology_workspace(&mut self, context: &Context) {
        use robogen_project::OptimizationState;
        let Some(project) = self.document.project() else {
            return;
        };
        let parts: Vec<_> = project
            .snapshot()
            .model
            .parts
            .values()
            .filter_map(|part| {
                project
                    .snapshot()
                    .optimization_states
                    .get(&part.id)
                    .map(|state| (part.id, part.name.clone(), *state))
            })
            .collect();
        let previous = self.selected_optimization;
        let mut launch = false;
        let mut edit_source = false;
        let mut diagnostic_view_changed = false;
        SidePanel::right("topology_controls")
            .exact_width(320.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Optimisation topologique");
                    egui::ComboBox::from_id_salt("optimization_part")
                        .width(280.0)
                        .selected_text(
                            parts
                                .iter()
                                .find(|part| Some(part.0) == self.selected_optimization)
                                .map(|part| part.1.as_str())
                                .unwrap_or("Pièce"),
                        )
                        .show_ui(ui, |ui| {
                            for (id, name, _) in &parts {
                                ui.selectable_value(
                                    &mut self.selected_optimization,
                                    Some(*id),
                                    name,
                                );
                            }
                        });
                    let Some(id) = self.selected_optimization else {
                        return;
                    };
                    let state = parts
                        .iter()
                        .find(|part| part.0 == id)
                        .map(|part| part.2)
                        .unwrap_or(OptimizationState::NotCalculated);
                    ui.label(match state {
                        OptimizationState::NotCalculated => "Non calculée",
                        OptimizationState::Running => "Calcul en cours",
                        OptimizationState::Calculated => "Calculée",
                        OptimizationState::Diagnostic => "Diagnostic · non validé",
                        OptimizationState::Obsolete => "Obsolète · relancement requis",
                        OptimizationState::Failed => "Échec du calcul",
                        OptimizationState::Cancelled => "Calcul annulé",
                    });
                    if !self.document.is_valid() {
                        ui.colored_label(MUTED, "Document en cours ou invalide");
                    }
                    let specifications = project.topology_operations(id);
                    let mass_objective = specifications.iter().any(|specification| {
                        specification.objective == robogen_project::TopologyObjective::MinimizeMass
                    });
                    let backend_available =
                        !mass_objective || robogen_project::MASS_BACKEND_AVAILABLE;
                    if mass_objective && !backend_available {
                        ui.colored_label(MUTED, "Minimisation de masse indisponible");
                    }
                    let controls = self.grid_controls.entry(id).or_insert(GridControls {
                        manual: !mass_objective,
                        cell_size_mm: 2.5,
                    });
                    let settings = self
                        .optimization_settings
                        .entry(id)
                        .or_insert_with(robogen_project::topology_settings);
                    ui.separator();
                    ui.label("Réglages du prochain calcul");
                    ui.add_enabled_ui(
                        backend_available && self.document.build_progress().is_none(),
                        |ui| {
                            if mass_objective {
                                super::optimization::mass_compute_backend_selector(
                                    ui,
                                    &mut settings.compute_backend,
                                );
                            }
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut controls.manual, false, "Maille (mm)");
                                ui.selectable_value(&mut controls.manual, true, "Cellules XYZ");
                            });
                            if controls.manual {
                                ui.horizontal(|ui| {
                                    for count in &mut settings.cells {
                                        ui.add(
                                            egui::DragValue::new(count)
                                                .range(1..=if mass_objective { 256 } else { 64 }),
                                        );
                                    }
                                });
                            } else {
                                ui.add(
                                    egui::DragValue::new(&mut controls.cell_size_mm)
                                        .range(0.05..=1000.0)
                                        .speed(0.1)
                                        .prefix("Maille cible ")
                                        .suffix(" mm"),
                                );
                            }
                            let mut filter = settings.filter_radius.millimetres();
                            if ui
                                .add(
                                    egui::DragValue::new(&mut filter)
                                        .range(0.01..=1000.0)
                                        .speed(0.1)
                                        .prefix("Filtre ")
                                        .suffix(" mm"),
                                )
                                .changed()
                            {
                                settings.filter_radius =
                                    robogen_domain::Length::from_millimetres(filter);
                            }
                            ui.add(
                                egui::DragValue::new(&mut settings.threshold)
                                    .range(0.5..=0.99)
                                    .speed(0.01)
                                    .prefix("Seuil "),
                            );
                            ui.add(
                                egui::DragValue::new(&mut settings.max_iterations)
                                    .range(1..=500)
                                    .prefix("Itérations max "),
                            );
                        },
                    );
                    let resolution = specifications.first().map(|specification| {
                        if controls.manual {
                            robogen_project::grid_resolution(specification, settings.cells)
                        } else {
                            robogen_project::resolution_for_cell_size(
                                specification,
                                robogen_domain::Length::from_millimetres(controls.cell_size_mm),
                            )
                        }
                    });
                    let grid_valid = match resolution {
                        Some(Ok(resolution)) => {
                            settings.cells = resolution.cells;
                            ui.label(format!(
                                "{} × {} × {} = {} cellules",
                                resolution.cells[0],
                                resolution.cells[1],
                                resolution.cells[2],
                                resolution.total_cells
                            ));
                            ui.label(format!(
                                "Maille réelle : {:.3} × {:.3} × {:.3} mm",
                                resolution.cell_size[0].millimetres(),
                                resolution.cell_size[1].millimetres(),
                                resolution.cell_size[2].millimetres()
                            ));
                            ui.label(format!(
                                "{} degrés de liberté",
                                resolution.degrees_of_freedom
                            ));
                            if mass_objective && backend_available {
                                ui.label(match settings.compute_backend {
                                    robogen_project::ComputeBackend::Cpu => {
                                        "CPU · jusqu'à 32 768 cellules"
                                    }
                                    robogen_project::ComputeBackend::Cuda => {
                                        "CUDA Float64 · jusqu'à 32 768 cellules"
                                    }
                                });
                                ui.label("Les maillages fins peuvent consommer plusieurs Gio.");
                            }
                            true
                        }
                        Some(Err(error)) => {
                            ui.colored_label(
                                Color32::from_rgb(255, 130, 110),
                                error.diagnostic.message,
                            );
                            false
                        }
                        None => false,
                    };
                    ui.separator();
                    let incomplete = specifications
                        .iter()
                        .any(|specification| !specification.preparation_issues().is_empty());
                    if incomplete {
                        ui.colored_label(
                            Color32::from_rgb(255, 190, 100),
                            "Préparation incomplète",
                        );
                        for specification in &specifications {
                            for issue in specification.preparation_issues() {
                                ui.label(match issue {
                                    "Material not specified" => "Matériau mécanique à renseigner",
                                    "Preserved interfaces not specified" => "Interfaces à définir",
                                    "Load cases and supports not specified" => {
                                        "Charges et appuis à renseigner"
                                    }
                                    "Mechanical limits not specified" => {
                                        "Limites mécaniques à renseigner"
                                    }
                                    _ => issue,
                                });
                            }
                        }
                    }
                    egui::CollapsingHeader::new("Interfaces, charges et contraintes").show(
                        ui,
                        |ui| {
                            for specification in &specifications {
                                ui.label(&specification.name);
                                ui.label(format!(
                                    "{} zones conservées · {} exclusions",
                                    specification.preserve.len(),
                                    specification.voids.len()
                                ));
                                if let Some((name, _)) =
                                    project.snapshot().model.materials.iter().find(
                                        |(_, material)| Some(material.id) == specification.material,
                                    )
                                {
                                    ui.label(format!("Matériau : {name}"));
                                }
                                ui.label(match specification.objective {
                                    robogen_project::TopologyObjective::MinimizeMass => {
                                        "Objectif : minimiser la masse"
                                    }
                                    robogen_project::TopologyObjective::MaximizeStiffness => {
                                        "Objectif : maximiser la rigidité"
                                    }
                                });
                                for case in &specification.load_cases {
                                    ui.label(RichText::new(&case.name).strong());
                                    for load in &case.loads {
                                        ui.label(format!("{:?}", load.kind));
                                    }
                                    for support in &case.supports {
                                        ui.label(format!("Appui : {:?}", support.kind));
                                    }
                                }
                                for constraint in &specification.constraints {
                                    ui.label(format!("{:?}", constraint.kind));
                                }
                            }
                            edit_source = ui.button("Modifier le DSL").clicked();
                        },
                    );
                    let launch_response = ui.add_enabled(
                        backend_available
                            && !incomplete
                            && grid_valid
                            && self.document.is_valid()
                            && self.document.build_progress().is_none(),
                        Button::new("Lancer cette pièce").min_size(vec2(280.0, 34.0)),
                    );
                    launch = launch_response.clicked();
                    #[cfg(test)]
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("topology_control", "launch")),
                            launch_response,
                        );
                    });
                    for diagnostic in self.document.optimization_diagnostics(id) {
                        ui.colored_label(
                            Color32::from_rgb(255, 130, 110),
                            format!("{} : {}", diagnostic.code, diagnostic.message),
                        );
                    }
                    for specification in specifications {
                        if let Some(diagnostic) =
                            project.snapshot().mass_diagnostics.get(&specification.id)
                        {
                            diagnostic_view_changed |= mass_diagnostic_view(
                                ui,
                                diagnostic,
                                state,
                                |id| {
                                    specification
                                        .load_cases
                                        .iter()
                                        .find(|case| case.id == id)
                                        .map(|case| case.name.as_str())
                                        .unwrap_or("Cas")
                                },
                                &mut self.density_reference,
                                &mut self.density_threshold,
                            );
                        }
                        if let Some(report) = project.snapshot().topology.get(&specification.id) {
                            super::optimization::topology_report_view(ui, report);
                        }
                    }
                });
            });
        if launch {
            if let Some(id) = self.selected_optimization {
                let settings = self
                    .optimization_settings
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(robogen_project::topology_settings);
                if let Err(error) = self.document.start_optimization(id, settings) {
                    self.status_message = Some(error.message);
                }
                self.pending_action = None;
            }
        }
        if launch || previous != self.selected_optimization || diagnostic_view_changed {
            self.rebuild_preview();
        }
        if edit_source {
            self.mode = WorkspaceMode::Design;
            self.design_tab = DesignTab::Dsl;
            self.rebuild_preview();
        }
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(12)))
            .show(context, |ui| self.viewport(ui));
    }
}
