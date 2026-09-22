//! RoboGen's desktop UI shell.
//!
//! The shell owns navigation and presentation state only. Domain commands and
//! solver connections will be injected through dedicated service interfaces in
//! later milestones.

use egui::{
    vec2, Align, Button, Color32, Context, FontId, Frame, Layout, Margin, RichText, ScrollArea,
    Sense, Shape, SidePanel, Stroke, TextEdit, TextStyle, TopBottomPanel, Ui, Vec2,
};
use robogen_render::{Camera, PreviewMesh, Rgba8, RobotPreviewRenderer, ViewportSize};

const BACKGROUND: Color32 = Color32::from_rgb(10, 17, 25);
const PANEL: Color32 = Color32::from_rgb(15, 24, 34);
const PANEL_RAISED: Color32 = Color32::from_rgb(22, 33, 45);
const VIEWPORT: Color32 = Color32::from_rgb(17, 25, 34);
const BORDER: Color32 = Color32::from_rgb(43, 58, 74);
const TEXT: Color32 = Color32::from_rgb(226, 232, 240);
const MUTED: Color32 = Color32::from_rgb(137, 151, 169);
const ACCENT: Color32 = Color32::from_rgb(40, 116, 235);
const ACCENT_HOVER: Color32 = Color32::from_rgb(56, 132, 250);
const SUCCESS: Color32 = Color32::from_rgb(95, 190, 120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceMode {
    Design,
    Optimization,
    Learning,
}

impl Default for WorkspaceMode {
    fn default() -> Self {
        Self::Design
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesignTab {
    Viewport,
    Dsl,
}

#[derive(Debug, Clone)]
pub enum UiAction {
    ExportStl(robogen_cad::Mesh),
}

/// Local presentation state for the Milestone 1 shell.
pub struct RoboGenUi {
    mode: WorkspaceMode,
    design_tab: DesignTab,
    optimization_tab: usize,
    selected_terrain: usize,
    taxonomy_filter: String,
    selected_component: &'static str,
    dsl_source: String,
    camera: Camera,
    preview_renderer: RobotPreviewRenderer,
    cad_mesh: robogen_cad::Mesh,
    preview_mesh: PreviewMesh,
    diagnostics: Vec<EditorDiagnostic>,
    width_mm: f32,
    pending_action: Option<UiAction>,
    status_message: Option<String>,
    theme_installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditorDiagnostic {
    line: usize,
    column: usize,
    code: String,
    message: String,
}

impl Default for RoboGenUi {
    fn default() -> Self {
        let mut state = Self {
            mode: WorkspaceMode::Design,
            design_tab: DesignTab::Viewport,
            optimization_tab: 0,
            selected_terrain: 1,
            taxonomy_filter: String::new(),
            selected_component: "Patte",
            dsl_source: DEFAULT_DSL.to_owned(),
            camera: Camera::default(),
            preview_renderer: RobotPreviewRenderer,
            cad_mesh: robogen_cad::Mesh::default(),
            preview_mesh: PreviewMesh::default(),
            diagnostics: Vec::new(),
            width_mm: 40.0,
            pending_action: None,
            status_message: None,
            theme_installed: false,
        };
        state.rebuild_preview();
        state
    }
}

impl RoboGenUi {
    pub fn mode(&self) -> WorkspaceMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WorkspaceMode) {
        self.mode = mode;
    }

    /// Replace the editor contents and rebuild the preview immediately.
    pub fn set_dsl_source(&mut self, source: impl Into<String>) {
        self.dsl_source = source.into();
        self.rebuild_preview();
    }

    pub fn take_action(&mut self) -> Option<UiAction> {
        self.pending_action.take()
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Draw one complete UI frame.
    pub fn show(&mut self, context: &Context) {
        if !self.theme_installed {
            install_theme(context);
            self.theme_installed = true;
        }

        self.top_bar(context);
        match self.mode {
            WorkspaceMode::Design => self.design_workspace(context),
            WorkspaceMode::Optimization => self.optimization_workspace(context),
            WorkspaceMode::Learning => self.learning_workspace(context),
        }
    }

    fn top_bar(&mut self, context: &Context) {
        TopBottomPanel::top("robogen_top_bar")
            .exact_height(66.0)
            .frame(
                Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::symmetric(16, 8))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.set_height(48.0);
                    ui.allocate_ui_with_layout(
                        vec2(250.0, 48.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new("◇")
                                    .size(29.0)
                                    .strong()
                                    .color(Color32::from_rgb(135, 174, 255)),
                            );
                            ui.vertical(|ui| {
                                ui.label(RichText::new("RoboGen").size(21.0).strong().color(TEXT));
                                ui.label(
                                    RichText::new("From idea to real robots")
                                        .size(11.0)
                                        .color(MUTED),
                                );
                            });
                        },
                    );

                    stage_button(
                        ui,
                        &mut self.mode,
                        WorkspaceMode::Design,
                        "1",
                        "Design",
                        "Définir la robotique",
                    );
                    stage_button(
                        ui,
                        &mut self.mode,
                        WorkspaceMode::Optimization,
                        "2",
                        "Optimisation",
                        "Alléger et renforcer",
                    );
                    stage_button(
                        ui,
                        &mut self.mode,
                        WorkspaceMode::Learning,
                        "3",
                        "Apprentissage",
                        "Apprendre à agir",
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Frame::new()
                            .fill(PANEL_RAISED)
                            .corner_radius(18)
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.label(RichText::new("JD").size(11.0).strong().color(TEXT));
                            });
                        if small_top_button(ui, "Exporter").clicked()
                            && !self.cad_mesh.vertices.is_empty()
                        {
                            self.pending_action = Some(UiAction::ExportStl(self.cad_mesh.clone()));
                        }
                        let _ = small_top_button(ui, "Enregistrer");
                        let _ = small_top_button(ui, "Nouveau projet");
                        if let Some(message) = &self.status_message {
                            ui.label(RichText::new(message).size(10.0).color(SUCCESS));
                        }
                    });
                });
            });
    }

    fn design_workspace(&mut self, context: &Context) {
        self.assistant_panel(context);
        self.taxonomy_panel(context);

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(10)))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    tab_button(ui, &mut self.design_tab, DesignTab::Viewport, "Vue 3D");
                    tab_button(ui, &mut self.design_tab, DesignTab::Dsl, "Code DSL");
                });
                ui.add_space(4.0);

                match self.design_tab {
                    DesignTab::Viewport => self.viewport(ui),
                    DesignTab::Dsl => self.dsl_editor(ui),
                }
            });
    }

    fn taxonomy_panel(&mut self, context: &Context) {
        SidePanel::left("taxonomy_panel")
            .exact_width(255.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Bibliothèque").size(16.0).color(TEXT));
                    ui.label(RichText::new("/").color(MUTED));
                    ui.heading(RichText::new("Taxonomie").size(16.0).color(TEXT));
                });
                ui.add_space(8.0);
                ui.add_sized(
                    [ui.available_width(), 30.0],
                    TextEdit::singleline(&mut self.taxonomy_filter)
                        .hint_text("Rechercher un composant…")
                        .margin(Margin::symmetric(9, 6)),
                );
                ui.add_space(8.0);

                ScrollArea::vertical().show(ui, |ui| {
                    taxonomy_group(
                        ui,
                        "Robot",
                        &[
                            ("Corps", &["Châssis", "Coque", "Support"]),
                            ("Membre", &["Patte", "Bras", "Roue", "Aile", "Queue"]),
                            ("Articulation", &["Rotule", "Pivot", "Prismatique"]),
                            ("Actionneur", &["Servo", "Moteur BLDC", "Vérin"]),
                            ("Capteur", &["IMU", "Caméra", "Capteur de force"]),
                            ("Matériau", &["PLA", "Aluminium", "Titane", "Carbone"]),
                        ],
                        &self.taxonomy_filter,
                        &mut self.selected_component,
                    );
                });
            });
    }

    fn assistant_panel(&mut self, context: &Context) {
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
                    "Assistant IA non configuré. Le modèle déclaratif, les commandes et l’aperçu restent utilisables manuellement.",
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
                            RichText::new("○  Interface provider prête")
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
                ui.label(RichText::new("Suggestions").size(12.0).color(MUTED));
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

    fn viewport(&mut self, ui: &mut Ui) {
        let viewport_height = (ui.available_height() - 38.0).max(220.0);
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

        let frame = self.preview_renderer.render_mesh(
            &self.camera,
            ViewportSize::new(rect.width(), rect.height()),
            &self.preview_mesh,
        );
        for triangle in frame.triangles {
            let points = triangle
                .points
                .into_iter()
                .map(|point| rect.min + vec2(point.x, point.y))
                .collect();
            painter.add(Shape::convex_polygon(
                points,
                color(triangle.fill),
                Stroke::new(1.0, color(triangle.outline)),
            ));
        }
        for line in frame.lines {
            painter.line_segment(
                [
                    rect.min + vec2(line.from.x, line.from.y),
                    rect.min + vec2(line.to.x, line.to.y),
                ],
                Stroke::new(line.width, color(line.color)),
            );
        }
        for joint in frame.joints {
            let center = rect.min + vec2(joint.center.x, joint.center.y);
            painter.circle_filled(center, joint.radius, color(joint.fill));
            painter.circle_stroke(center, joint.radius, Stroke::new(1.3, color(joint.outline)));
            painter.circle_filled(center, (joint.radius * 0.35).max(1.5), VIEWPORT);
        }

        paint_viewport_chrome(&painter, rect);

        painter.text(
            rect.left_top() + vec2(14.0, 13.0),
            egui::Align2::LEFT_TOP,
            "APERÇU PARAMÉTRIQUE · BRACKET",
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
        painter.text(
            rect.center_bottom() - vec2(0.0, 12.0),
            egui::Align2::CENTER_BOTTOM,
            "Glisser pour orbiter  •  Molette pour zoomer",
            FontId::proportional(11.0),
            MUTED,
        );

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            status_badge(ui, "Vue éclatée", MUTED);
            status_badge(ui, "Centre de masse", MUTED);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                status_badge(ui, "Perspective", TEXT);
                ui.label(
                    RichText::new(format!("Aperçu live — WIDTH = {:.0} mm", self.width_mm))
                        .size(11.0)
                        .color(MUTED),
                );
            });
        });
    }

    fn dsl_editor(&mut self, ui: &mut Ui) {
        let mut source_changed = false;
        Frame::new()
            .fill(VIEWPORT)
            .corner_radius(7)
            .stroke(Stroke::new(1.0, BORDER))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("spider_robot.rgn").strong().color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        status_badge(ui, "Source de vérité", SUCCESS);
                    });
                });
                ui.separator();
                ScrollArea::both().show(ui, |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.dsl_source)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(34),
                    );
                    source_changed = response.changed();
                });
                ui.separator();
                if self.diagnostics.is_empty() {
                    ui.label(
                        RichText::new(format!("● Modèle valide · WIDTH = {:.0} mm", self.width_mm))
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
            self.rebuild_preview();
            ui.ctx().request_repaint();
        }
    }

    fn rebuild_preview(&mut self) {
        let project = match robogen_project::Project::from_source(self.dsl_source.clone()) {
            Ok(project) => project,
            Err(diagnostics) => {
                self.diagnostics = diagnostics
                    .iter()
                    .map(|diagnostic| {
                        let (line, column) =
                            robogen_dsl::line_column(&self.dsl_source, diagnostic.span.start);
                        EditorDiagnostic {
                            line,
                            column,
                            code: diagnostic.code.clone(),
                            message: diagnostic.message.clone(),
                        }
                    })
                    .collect();
                return;
            }
        };

        let mut mesh = robogen_cad::Mesh::default();
        for part_mesh in project.snapshot().meshes.values() {
            mesh.append(part_mesh);
        }
        if mesh.vertices.is_empty() {
            self.diagnostics = vec![EditorDiagnostic {
                line: 1,
                column: 1,
                code: "E320".to_owned(),
                message: "le modèle ne produit aucune géométrie".to_owned(),
            }];
            return;
        }
        let (preview, width_mm) = cad_preview(&mesh);
        self.cad_mesh = mesh;
        self.preview_mesh = preview;
        self.width_mm = width_mm;
        self.diagnostics.clear();
    }

    fn optimization_workspace(&mut self, context: &Context) {
        SidePanel::left("optimization_parameters")
            .exact_width(280.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, optimization_parameters);

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(12)))
            .show(context, |ui| {
                section_tabs(
                    ui,
                    &mut self.optimization_tab,
                    &["Résultat", "Comparaison", "Analyse", "Historique"],
                );
                ui.add_space(9.0);

                ui.columns(2, |columns| {
                    model_preview_card(
                        &mut columns[0],
                        &self.preview_renderer,
                        &self.camera,
                        &self.preview_mesh,
                        "Avant optimisation",
                        "Géométrie paramétrique actuelle",
                        210.0,
                    );
                    unavailable_result_card(
                        &mut columns[1],
                        "Après optimisation",
                        "Solver unavailable",
                        210.0,
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    for (label, value) in [
                        ("Masse", "—"),
                        ("Réduction", "— %"),
                        ("Compliance", "—"),
                        ("Déplacement max", "— mm"),
                        ("Von Mises max", "— MPa"),
                        ("Sécurité", "—"),
                    ] {
                        optimization_metric(ui, label, value);
                    }
                });

                ui.add_space(8.0);
                analysis_placeholder(ui, ui.available_height().max(150.0));
            });
    }

    fn learning_workspace(&mut self, context: &Context) {
        SidePanel::left("learning_parameters")
            .exact_width(285.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                learning_parameters(ui, &mut self.selected_terrain)
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
                empty_chart(ui, "Récompense moyenne", "reward", 142.0);
                ui.add_space(8.0);
                empty_chart(ui, "Vitesse", "m/s", 142.0);
                ui.add_space(8.0);
                empty_chart(ui, "Taux de chute", "%", 142.0);
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(12)))
            .show(context, |ui| {
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

fn install_theme(context: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(8, 14, 21);
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.weak_bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(31, 47, 65);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(31, 47, 65);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(65, 91, 120));
    visuals.widgets.active.bg_fill = ACCENT;
    context.set_visuals(visuals);

    context.style_mut(|style| {
        style.spacing.item_spacing = vec2(7.0, 7.0);
        style.spacing.button_padding = vec2(11.0, 6.0);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(13.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(12.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(12.5));
    });
}

fn side_frame() -> Frame {
    Frame::new()
        .fill(PANEL)
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0, BORDER))
}

fn stage_button(
    ui: &mut Ui,
    current: &mut WorkspaceMode,
    target: WorkspaceMode,
    number: &str,
    title: &str,
    subtitle: &str,
) {
    let selected = *current == target;
    let text = RichText::new(format!("{number}    {title}\n       {subtitle}"))
        .size(12.0)
        .color(if selected { Color32::WHITE } else { TEXT });
    let button = Button::new(text)
        .fill(if selected {
            ACCENT
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0,
            if selected { ACCENT_HOVER } else { BORDER },
        ))
        .corner_radius(5);
    if ui.add_sized([185.0, 47.0], button).clicked() {
        *current = target;
    }
}

fn small_top_button(ui: &mut Ui, title: &str) -> egui::Response {
    ui.add_sized(
        [88.0, 30.0],
        Button::new(RichText::new(title).size(11.0).color(TEXT))
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER)),
    )
}

fn tab_button(ui: &mut Ui, current: &mut DesignTab, target: DesignTab, title: &str) {
    let selected = *current == target;
    let button =
        Button::new(RichText::new(title).color(if selected { Color32::WHITE } else { MUTED }))
            .fill(if selected { ACCENT } else { PANEL })
            .stroke(Stroke::new(
                1.0,
                if selected { ACCENT_HOVER } else { BORDER },
            ));
    if ui.add_sized([120.0, 31.0], button).clicked() {
        *current = target;
    }
}

fn taxonomy_group(
    ui: &mut Ui,
    root: &str,
    groups: &[(&'static str, &'static [&'static str])],
    filter: &str,
    selected: &mut &'static str,
) {
    egui::CollapsingHeader::new(RichText::new(root).strong().color(TEXT))
        .default_open(true)
        .show(ui, |ui| {
            let query = filter.trim().to_lowercase();
            for &(group, items) in groups {
                let visible_items: Vec<_> = items
                    .iter()
                    .copied()
                    .filter(|item| query.is_empty() || item.to_lowercase().contains(&query))
                    .collect();
                if visible_items.is_empty() && !query.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(RichText::new(group).color(TEXT))
                    .default_open(true)
                    .show(ui, |ui| {
                        for item in visible_items {
                            if ui.selectable_label(*selected == item, item).clicked() {
                                *selected = item;
                            }
                        }
                    });
            }
        });
}

fn status_badge(ui: &mut Ui, text: &str, text_color: Color32) {
    Frame::new()
        .fill(PANEL_RAISED)
        .corner_radius(5)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(text_color));
        });
}

fn section_tabs(ui: &mut Ui, selected: &mut usize, titles: &[&str]) {
    ui.horizontal(|ui| {
        for (index, title) in titles.iter().enumerate() {
            let response = ui.add_sized(
                [105.0, 29.0],
                Button::new(
                    RichText::new(*title)
                        .size(11.0)
                        .color(if index == *selected {
                            Color32::WHITE
                        } else {
                            MUTED
                        }),
                )
                .fill(if index == *selected { ACCENT } else { PANEL })
                .stroke(Stroke::new(
                    1.0,
                    if index == *selected {
                        ACCENT_HOVER
                    } else {
                        BORDER
                    },
                )),
            );
            if response.clicked() {
                *selected = index;
            }
        }
    });
}

fn optimization_parameters(ui: &mut Ui) {
    ui.heading(
        RichText::new("Paramètres d’optimisation")
            .size(16.0)
            .color(TEXT),
    );
    ui.add_space(10.0);
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

fn learning_parameters(ui: &mut Ui, selected_terrain: &mut usize) {
    ui.heading(RichText::new("Environnement").size(16.0).color(TEXT));
    ui.add_space(10.0);
    form_label(ui, "Terrain");
    value_field(ui, "Irrégulier");
    ui.columns(4, |columns| {
        for (index, name) in ["Plat", "Rochers", "Escaliers", "Sable"]
            .into_iter()
            .enumerate()
        {
            if environment_tile(&mut columns[index], name, *selected_terrain == index).clicked() {
                *selected_terrain = index;
            }
        }
    });
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
                .color(SUCCESS),
        );
    }
    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.add_enabled(
            false,
            Button::new("Lancer l’entraînement").min_size(vec2(ui.available_width(), 34.0)),
        );
    });
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

fn model_preview_card(
    ui: &mut Ui,
    renderer: &RobotPreviewRenderer,
    camera: &Camera,
    mesh: &PreviewMesh,
    title: &str,
    detail: &str,
    height: f32,
) {
    Frame::new()
        .fill(VIEWPORT)
        .corner_radius(7)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(13.0).strong().color(TEXT));
            ui.add(egui::Label::new(RichText::new(detail).size(11.0).color(MUTED)).wrap());
            let (rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), (height - 64.0).max(1.0)),
                Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            let frame =
                renderer.render_mesh(camera, ViewportSize::new(rect.width(), rect.height()), mesh);
            for triangle in frame.triangles {
                let points = triangle
                    .points
                    .into_iter()
                    .map(|point| rect.min + vec2(point.x, point.y))
                    .collect();
                painter.add(Shape::convex_polygon(
                    points,
                    color(triangle.fill),
                    Stroke::new(1.0, color(triangle.outline)),
                ));
            }
            for line in frame.lines {
                painter.line_segment(
                    [
                        rect.min + vec2(line.from.x, line.from.y),
                        rect.min + vec2(line.to.x, line.to.y),
                    ],
                    Stroke::new(line.width, color(line.color)),
                );
            }
        });
}

fn unavailable_result_card(ui: &mut Ui, title: &str, detail: &str, height: f32) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::top_down(Align::Min),
        |ui| unavailable_canvas(ui, title, detail, "Aucun résultat calculé"),
    );
}

fn analysis_placeholder(ui: &mut Ui, height: f32) {
    unavailable_result_card(ui, "Analyse mécanique", "Solveur indisponible", height);
}

fn empty_chart(ui: &mut Ui, title: &str, unit: &str, height: f32) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::top_down(Align::Min),
        |ui| {
            unavailable_canvas(
                ui,
                title,
                "Entraînement indisponible",
                &format!("Aucune donnée · {unit}"),
            );
        },
    );
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

fn unavailable_canvas(ui: &mut Ui, title: &str, detail: &str, note: &str) {
    let size = ui.available_size();
    Frame::new()
        .fill(VIEWPORT)
        .corner_radius(7)
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.set_min_size(size);
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(RichText::new(title).size(13.0).color(TEXT));
                    ui.add_space(6.0);
                    ui.add(egui::Label::new(RichText::new(detail).color(MUTED)).wrap());
                    ui.add_space(3.0);
                    ui.add(egui::Label::new(RichText::new(note).size(11.0).color(MUTED)).wrap());
                });
            });
        });
}

fn form_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(MUTED));
}

fn value_field(ui: &mut Ui, value: &str) {
    Frame::new()
        .fill(PANEL_RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(4)
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(value).size(12.0).color(TEXT));
        });
}

fn toggle_row(ui: &mut Ui, label: &str, enabled: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(if enabled { "●" } else { "○" }).color(if enabled {
                    Color32::from_rgb(82, 154, 255)
                } else {
                    MUTED
                }),
            );
        });
    });
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
                ui.label(
                    RichText::new(if selected { "◆" } else { "◇" })
                        .size(17.0)
                        .color(if selected { ACCENT_HOVER } else { MUTED }),
                );
                ui.label(RichText::new(name).size(9.0).color(TEXT));
            });
        })
        .response
        .interact(Sense::click())
}

fn optimization_metric(ui: &mut Ui, title: &str, value: &str) {
    Frame::new()
        .fill(PANEL_RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(6)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(96.0);
            ui.label(RichText::new(title).size(11.0).color(MUTED));
            ui.add_space(14.0);
            ui.label(RichText::new(value).size(20.0).strong().color(TEXT));
        });
}

fn color(color: Rgba8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

fn cad_preview(mesh: &robogen_cad::Mesh) -> (PreviewMesh, f32) {
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(vertex.position[axis]);
            maximum[axis] = maximum[axis].max(vertex.position[axis]);
        }
    }
    let center = [
        (minimum[0] + maximum[0]) * 0.5,
        minimum[1],
        (minimum[2] + maximum[2]) * 0.5,
    ];
    const METRES_TO_PREVIEW: f32 = 40.0;
    let positions = mesh
        .vertices
        .iter()
        .map(|vertex| {
            [
                (vertex.position[0] - center[0]) * METRES_TO_PREVIEW,
                (vertex.position[1] - center[1]) * METRES_TO_PREVIEW,
                (vertex.position[2] - center[2]) * METRES_TO_PREVIEW,
            ]
        })
        .collect();
    let triangles = mesh
        .triangles
        .iter()
        .map(|triangle| triangle.indices)
        .collect();
    let width_mm = (maximum[0] - minimum[0]) * 1_000.0;
    (PreviewMesh::new(positions, triangles), width_mm)
}

const DEFAULT_DSL: &str = r#"module bracket_demo;

parameter WIDTH = 40 mm;
parameter HEIGHT = 28 mm;
parameter THICKNESS = 4 mm;

material Aluminium6061 {
    density: 2700 kg/m3;
    young: 69 GPa;
    poisson: 0.33;
    yield_strength: 276 MPa;
}

sketch BracketProfile on XY {
    rectangle outline {
        origin: (0 mm, 0 mm);
        width: WIDTH;
        height: HEIGHT;
    }
    constrain horizontal(outline.bottom);
}

part Bracket {
    material: Aluminium6061;
    base = extrude(BracketProfile, THICKNESS);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_in_design_mode() {
        let ui = RoboGenUi::default();
        assert_eq!(ui.mode(), WorkspaceMode::Design);
        assert_eq!(ui.selected_component, "Patte");
        assert!(ui.dsl_source.contains("parameter WIDTH = 40 mm"));
        assert!(ui.diagnostics.is_empty());
        assert!(!ui.preview_mesh.is_empty());
    }

    #[test]
    fn mode_can_be_driven_by_the_host_application() {
        let mut ui = RoboGenUi::default();
        ui.set_mode(WorkspaceMode::Optimization);
        assert_eq!(ui.mode(), WorkspaceMode::Optimization);
    }

    fn collect_painted_text(shape: &Shape, text: &mut String) {
        match shape {
            Shape::Text(label) => {
                text.push_str(&label.galley.job.text);
                text.push('\n');
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_painted_text(shape, text);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn workflow_views_render_headlessly_without_actions() {
        for size in [vec2(1280.0, 800.0), vec2(1920.0, 1080.0)] {
            let context = Context::default();
            let mut state = RoboGenUi::default();
            let original_positions = state.preview_mesh.positions.clone();
            for (mode, design_tab, optimization_tab, expected_text) in [
                (
                    WorkspaceMode::Design,
                    DesignTab::Viewport,
                    0,
                    "APERÇU PARAMÉTRIQUE",
                ),
                (WorkspaceMode::Design, DesignTab::Dsl, 0, "Modèle valide"),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    0,
                    "Solver unavailable",
                ),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    1,
                    "Solver unavailable",
                ),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    2,
                    "Solver unavailable",
                ),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    3,
                    "Solver unavailable",
                ),
                (
                    WorkspaceMode::Learning,
                    DesignTab::Viewport,
                    0,
                    "Simulation et entraînement indisponibles",
                ),
            ] {
                state.set_mode(mode);
                state.design_tab = design_tab;
                state.optimization_tab = optimization_tab;
                for _ in 0..2 {
                    let output = context.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |context| state.show(context),
                    );
                    let mut text = String::new();
                    for shape in &output.shapes {
                        collect_painted_text(&shape.shape, &mut text);
                    }
                    assert!(text.contains(expected_text), "{mode:?} at {size:?}: {text}");
                    match mode {
                        WorkspaceMode::Design => assert!(text.contains("Indisponible")),
                        WorkspaceMode::Optimization => {
                            assert!(text.contains("Aucun résultat calculé"))
                        }
                        WorkspaceMode::Learning => {
                            assert!(text.contains("Aucune donnée"));
                            assert!(text.contains("Aucune trajectoire"));
                        }
                    }
                    assert!(!context
                        .tessellate(output.shapes, output.pixels_per_point)
                        .is_empty());
                    assert!(state.take_action().is_none());
                    assert_eq!(state.mode(), mode);
                    assert_eq!(state.dsl_source, DEFAULT_DSL);
                    assert_eq!(state.preview_mesh.positions, original_positions);
                }
            }
        }
    }

    #[test]
    fn changing_width_rebuilds_the_preview() {
        let mut ui = RoboGenUi::default();
        let before = ui.preview_mesh.positions[1][0];
        ui.set_dsl_source(ui.dsl_source.replace("WIDTH = 40 mm", "WIDTH = 80 mm"));
        assert_eq!(ui.width_mm, 80.0);
        assert!(ui.preview_mesh.positions[1][0] > before);
        assert!(ui.diagnostics.is_empty());
    }
}
