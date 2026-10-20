use super::*;

impl RoboGenUi {
    pub(super) fn viewport(&mut self, ui: &mut Ui) {
        if self
            .document
            .project()
            .is_some_and(|project| !project.snapshot().preparation.is_empty())
        {
            let mut changed = false;
            ui.horizontal_wrapped(|ui| {
                changed |= ui
                    .checkbox(&mut self.show_components, "Composants")
                    .changed();
                changed |= ui.checkbox(&mut self.show_results, "Résultats").changed();
                if self.mode == WorkspaceMode::Optimization {
                    changed |= ui.checkbox(&mut self.show_densities, "Densités").changed();
                }
                for (index, (label, color)) in [
                    ("Domaine", Color32::from_rgb(100, 160, 255)),
                    ("Conserver", Color32::from_rgb(75, 215, 120)),
                    ("Exclure", Color32::from_rgb(255, 100, 85)),
                ]
                .into_iter()
                .enumerate()
                {
                    changed |= ui
                        .checkbox(
                            &mut self.show_regions[index],
                            RichText::new(label).color(color),
                        )
                        .changed();
                }
                changed |= ui
                    .checkbox(
                        &mut self.show_instance_interfaces,
                        RichText::new("Interface").color(Color32::from_rgb(0, 210, 210)),
                    )
                    .changed();
            });
            if changed {
                self.rebuild_preview();
            }
        }
        ui.horizontal(|ui| {
            if icon_button(ui, Icon::OrbitLeft, true, "Orbite gauche", "").clicked() {
                self.camera.orbit(-30.0, 0.0);
            }
            if icon_button(ui, Icon::OrbitRight, true, "Orbite droite", "").clicked() {
                self.camera.orbit(30.0, 0.0);
            }
            if icon_button(ui, Icon::Plus, true, "Zoom avant", "").clicked() {
                self.camera.zoom(120.0);
            }
            if icon_button(ui, Icon::Minus, true, "Zoom arrière", "").clicked() {
                self.camera.zoom(-120.0);
            }
            if icon_button(ui, Icon::Home, true, "Réinitialiser la caméra", "").clicked() {
                self.camera = Camera::default();
            }
            icon_button(
                ui,
                Icon::Ortho,
                false,
                "Projection orthographique",
                "Projection orthographique indisponible",
            );
            ui.label(RichText::new("Perspective").size(11.0).color(MUTED));
        });
        let viewport_height = (ui.available_height() - 72.0).max(180.0);
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), viewport_height),
            Sense::click_and_drag(),
        );

        if response.dragged() {
            let delta = response.drag_delta();
            self.camera.orbit(delta.x, delta.y);
            ui.ctx().request_repaint();
        }
        if response.hovered() {
            let scroll = ui.ctx().input(|input| input.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.camera.zoom(scroll);
                ui.ctx().request_repaint();
            }
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 7.0, VIEWPORT);
        painter.rect_stroke(
            rect,
            7.0,
            Stroke::new(1.0, BORDER),
            egui::StrokeKind::Inside,
        );

        paint_preview(
            &painter,
            rect,
            egui::Id::new("design_cad_preview"),
            &self.preview_renderer,
            &self.camera,
            &self.preview_mesh,
        );

        paint_viewport_chrome(&painter, rect);

        for (index, mesh) in self.preparation_meshes.iter().enumerate() {
            if !self.show_regions[index] {
                continue;
            }
            let color = [
                Color32::from_rgba_unmultiplied(100, 160, 255, 150),
                Color32::from_rgb(75, 215, 120),
                Color32::from_rgb(255, 100, 85),
            ][index];
            let frame = self.preview_renderer.render_mesh(
                &self.camera,
                ViewportSize::new(rect.width(), rect.height()),
                mesh,
            );
            for triangle in frame.triangles {
                let points = triangle
                    .points
                    .map(|point| egui::pos2(rect.left() + point.x, rect.top() + point.y));
                for edge in [[0, 1], [1, 2], [2, 0]] {
                    painter
                        .line_segment([points[edge[0]], points[edge[1]]], Stroke::new(1.0, color));
                }
            }
        }

        if self.show_instance_interfaces {
            let color = Color32::from_rgb(0, 210, 210);
            let frame = self.preview_renderer.render_mesh(
                &self.camera,
                ViewportSize::new(rect.width(), rect.height()),
                &self.instance_interface_meshes,
            );
            for triangle in frame.triangles {
                let points = triangle
                    .points
                    .map(|point| egui::pos2(rect.left() + point.x, rect.top() + point.y));
                for edge in [[0, 1], [1, 2], [2, 0]] {
                    painter
                        .line_segment([points[edge[0]], points[edge[1]]], Stroke::new(1.0, color));
                }
            }
        }

        painter.text(
            rect.left_top() + vec2(14.0, 13.0),
            egui::Align2::LEFT_TOP,
            if self.document.is_valid() {
                "APERÇU PARAMÉTRIQUE"
            } else {
                "DERNIER APERÇU VALIDE"
            },
            FontId::proportional(10.0),
            MUTED,
        );
        painter.text(
            rect.right_top() + vec2(-14.0, 13.0),
            egui::Align2::RIGHT_TOP,
            "Z  Y  X",
            FontId::monospace(11.0),
            Color32::from_rgb(116, 165, 255),
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled(false, Button::new("Vue éclatée"))
                .on_disabled_hover_text("Assemblage indisponible");
            ui.add_enabled(false, Button::new("Centre de masse"))
                .on_disabled_hover_text("Propriétés inertielles indisponibles");
        });
        ui.label(
            RichText::new(if self.document.build_progress().is_some() {
                "Construction en cours · aperçu précédent · export désactivé".to_owned()
            } else if self.document.is_valid() {
                format!("Largeur de l’aperçu : {:.1} mm", self.width_mm)
            } else {
                "Source invalide · aperçu précédent · export désactivé".to_owned()
            })
            .size(11.0)
            .color(MUTED),
        );
    }
}

fn paint_viewport_chrome(painter: &egui::Painter, rect: egui::Rect) {
    for corner in [rect.left_top(), rect.right_top()] {
        let direction = if corner.x == rect.left() { 1.0 } else { -1.0 };
        let start = corner + vec2(direction * 8.0, 8.0);
        painter.line_segment(
            [start, start + vec2(direction * 18.0, 0.0)],
            Stroke::new(1.0, ACCENT),
        );
    }
}
