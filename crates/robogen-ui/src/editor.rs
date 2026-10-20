use super::*;

impl RoboGenUi {
    pub(super) fn dsl_editor(&mut self, ui: &mut Ui) {
        let mut source_changed = false;
        Frame::new()
            .fill(VIEWPORT)
            .corner_radius(7)
            .stroke(Stroke::new(1.0, BORDER))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Document DSL").strong().color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        status_badge(ui, "Source de vérité", SUCCESS);
                    });
                });
                ui.separator();
                let mut scroll =
                    ScrollArea::both().max_height((ui.available_height() - 130.0).max(120.0));
                if let Some(span) = self.source_selection {
                    let (line, _) = robogen_dsl::line_column(&self.dsl_source, span.start);
                    scroll = scroll.vertical_scroll_offset(
                        line.saturating_sub(2) as f32 * ui.text_style_height(&TextStyle::Monospace),
                    );
                }
                scroll.show(ui, |ui| {
                    let mut output = TextEdit::multiline(&mut self.dsl_source)
                        .id(egui::Id::new("robogen_dsl_source"))
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(34)
                        .show(ui);
                    source_changed = output.response.changed();
                    if let Some(span) = self.source_selection.take() {
                        let start = self
                            .dsl_source
                            .get(..span.start)
                            .unwrap_or_default()
                            .chars()
                            .count();
                        let end = self
                            .dsl_source
                            .get(..span.end)
                            .unwrap_or_default()
                            .chars()
                            .count();
                        output
                            .state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(start),
                                egui::text::CCursor::new(end),
                            )));
                        output.response.request_focus();
                        ui.ctx().request_repaint();
                    }
                    output.state.clear_undoer();
                    output.state.store(ui.ctx(), output.response.id);
                });
                ui.separator();
                if self.document.build_progress().is_some() {
                    ui.label("Construction CAD en cours");
                } else if self.document.is_valid() && self.diagnostics.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "● Modèle valide · largeur = {:.1} mm",
                            self.width_mm
                        ))
                        .size(11.0)
                        .color(SUCCESS),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("{} diagnostic(s)", self.diagnostics.len()))
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(238, 107, 107)),
                    );
                    for diagnostic in self.diagnostics.iter().take(4) {
                        ui.label(
                            RichText::new(format!(
                                "{} · {}:{} — {}",
                                diagnostic.code,
                                diagnostic.line,
                                diagnostic.column,
                                diagnostic.message
                            ))
                            .monospace()
                            .size(11.0)
                            .color(Color32::from_rgb(230, 139, 139)),
                        );
                    }
                }
            });
        if source_changed {
            self.set_dsl_source(self.dsl_source.clone());
            ui.ctx().request_repaint();
        }
    }
}
