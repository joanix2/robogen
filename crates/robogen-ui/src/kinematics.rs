//! Two presentations of the same declared mechanical model.
use super::*;
pub(super) mod canvas;
mod drawing;
mod layout;
mod symbols;
use canvas::{DiagramMode, Projection};

impl RoboGenUi {
    pub(super) fn kinematics_view(&mut self, ui: &mut Ui) {
        ui.heading("Graphe cinématique 2D");
        if !self.document.is_valid() {
            ui.colored_label(
                Color32::from_rgb(224, 171, 90),
                "Dernier modèle valide · graphe non actualisé",
            );
        }
        ui.horizontal_wrapped(|ui| {
            for (mode, label) in [
                (DiagramMode::Graph, "Graphe des liaisons"),
                (DiagramMode::Schematic, "Schéma cinématique"),
            ] {
                if ui
                    .selectable_label(self.graph_canvas.mode == mode, label)
                    .clicked()
                {
                    self.graph_canvas.set_mode(mode);
                }
            }
            if self.graph_canvas.mode == DiagramMode::Schematic {
                for (projection, label) in [
                    (Projection::Xy, "XY"),
                    (Projection::Xz, "XZ"),
                    (Projection::Yz, "YZ"),
                ] {
                    if ui
                        .selectable_label(self.graph_canvas.projection == projection, label)
                        .clicked()
                    {
                        self.graph_canvas.set_projection(projection);
                    }
                }
            }
        });
        ui.small("Corps rigides et liaisons déclarés · configuration au repos, sans résolution de mouvement.");
        let Some(project) = self.document.project() else {
            ui.label("Aucun modèle valide disponible.");
            return;
        };
        let model = &project.snapshot().model.mechanics;
        if model.bodies.is_empty() {
            ui.label("Aucun corps rigide déclaré dans ce document.");
            ui.small("Déclarez les corps (body) et les liaisons (joint) dans le DSL.");
            return;
        }
        let mut open_source = false;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Cadrer").clicked() {
                self.graph_canvas.fit_view();
            }
            if self.graph_canvas.mode == DiagramMode::Graph && ui.button("Réorganiser").clicked() {
                self.graph_canvas.reset_layout();
            }
            ui.label(format!(
                "{} corps · {} liaisons",
                model.bodies.len(),
                model.joints.len()
            ));
            if let Some((name, _)) = self.graph_canvas.selection_source(model) {
                open_source = ui
                    .add_enabled(
                        self.document.is_valid(),
                        egui::Button::new("Ouvrir dans le DSL"),
                    )
                    .clicked();
                ui.label(name);
            }
        });
        ui.small(if self.graph_canvas.mode == DiagramMode::Graph {
            "Molette : zoom · fond : déplacer · corps : sélectionner / déplacer · liaison : sélectionner"
        } else {
            "Molette : zoom · fond : déplacer · symbole : sélectionner · ⊙ : axe perpendiculaire au plan"
        });
        let selected_source = self.graph_canvas.show(ui, model);
        if self.document.is_valid() {
            if let Some(span) = selected_source.or_else(|| {
                open_source
                    .then(|| {
                        self.graph_canvas
                            .selection_source(model)
                            .map(|(_, span)| span)
                    })
                    .flatten()
            }) {
                self.source_selection = Some(span);
                self.design_tab = DesignTab::Dsl;
            }
        }
    }
}
