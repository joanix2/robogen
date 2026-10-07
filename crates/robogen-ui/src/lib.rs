//! RoboGen's desktop UI shell.
//!
//! The shell owns navigation and presentation state only. Domain commands and
//! solver connections will be injected through dedicated service interfaces in
//! later milestones.

use egui::{
    vec2, Align, Button, Color32, Context, FontId, Frame, Layout, Margin, RichText, ScrollArea,
    Sense, Shape, SidePanel, Stroke, TextEdit, TextStyle, TopBottomPanel, Ui, Vec2,
};
use robogen_agent_api::{AiAssistantProvider, AssistantRequest, DisabledAiAssistant};
use robogen_domain::{EntityId, SourceSpan};
use robogen_project::{FunctionDefinition, LibraryGenerator, SourceDocument, TaxonomyInstance};
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
    library_filter: String,
    taxonomy: Vec<TaxonomyInstance>,
    functions: Vec<FunctionDefinition>,
    taxonomy_fraction: f32,
    source_selection: Option<SourceSpan>,
    selected_generator: Option<LibraryGenerator>,
    selected_component: Option<EntityId>,
    document: SourceDocument,
    dsl_source: String,
    assistant_message: String,
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
            library_filter: String::new(),
            taxonomy: Vec::new(),
            functions: Vec::new(),
            taxonomy_fraction: 0.52,
            source_selection: None,
            selected_generator: None,
            selected_component: None,
            document: SourceDocument::new(DEFAULT_DSL),
            dsl_source: DEFAULT_DSL.to_owned(),
            assistant_message: match DisabledAiAssistant.respond(&AssistantRequest {
                user_message: "Démonstration de disponibilité".to_owned(),
                project_source: DEFAULT_DSL.to_owned(),
            }) {
                Err(error) => error.to_string(),
                Ok(response) => response.message,
            },
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
    pub fn enable_background_compilation(&mut self) -> std::io::Result<()> {
        self.document.enable_background_compilation()
    }

    pub fn load_servo_example(&mut self) {
        self.set_dsl_source(include_str!("../../../examples/servo/main.rgn"));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 1.3, 0.0),
            distance: 6.5,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_compute_board_example(&mut self) {
        self.set_dsl_source(include_str!("../../../examples/compute_board/main.rgn"));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
            yaw: 0.7,
            pitch: 0.75,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_camera_example(&mut self) {
        self.set_dsl_source(include_str!(
            "../../../examples/raspberry_pi_camera/main.rgn"
        ));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
            yaw: 0.5,
            pitch: 0.85,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_battery_example(&mut self) {
        self.set_dsl_source(include_str!("../../../examples/li_ion_battery/main.rgn"));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
            yaw: 0.5,
            pitch: 0.55,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_topology_example(&mut self) {
        self.set_dsl_source(include_str!(
            "../../../examples/topology_battery_support/main.rgn"
        ));
        self.design_tab = DesignTab::Dsl;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_biped_example(&mut self) {
        self.set_dsl_source(include_str!("../../../examples/mini_biped/main.rgn"));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 1.5, 0.0),
            yaw: 0.25,
            pitch: 0.15,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn mode(&self) -> WorkspaceMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WorkspaceMode) {
        self.mode = mode;
    }

    /// Replace the editor contents and rebuild the preview immediately.
    pub fn set_dsl_source(&mut self, source: impl Into<String>) {
        if self.document.replace_source(source.into()) {
            self.refresh_document();
        }
    }

    pub fn dsl_source(&self) -> &str {
        self.document.source()
    }

    pub fn can_undo(&self) -> bool {
        self.document.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.document.can_redo()
    }

    pub fn undo(&mut self) -> bool {
        let changed = self.document.undo();
        if changed {
            self.refresh_document();
        }
        changed
    }

    pub fn redo(&mut self) -> bool {
        let changed = self.document.redo();
        if changed {
            self.refresh_document();
        }
        changed
    }

    pub fn can_export(&self) -> bool {
        self.document.is_valid() && !self.cad_mesh.triangles.is_empty()
    }

    pub fn request_export(&mut self) -> bool {
        if !self.can_export() {
            return false;
        }
        self.pending_action = Some(UiAction::ExportStl(self.cad_mesh.clone()));
        true
    }

    pub fn instantiate_generator(&mut self, generator: LibraryGenerator) -> bool {
        match self.document.instantiate_generator(generator) {
            Ok(_) => {
                self.selected_generator = Some(generator);
                self.refresh_document();
                self.mode = WorkspaceMode::Design;
                self.design_tab = DesignTab::Viewport;
                self.camera = Camera {
                    target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
                    yaw: 0.5,
                    pitch: 0.8,
                    distance: 4.8,
                    ..Camera::default()
                };
                true
            }
            Err(diagnostics) => {
                self.status_message = diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.clone());
                false
            }
        }
    }

    fn open_source(&mut self, span: SourceSpan) {
        if self.document.is_valid() {
            self.source_selection = Some(span);
            self.design_tab = DesignTab::Dsl;
        }
    }

    pub fn selected_component(&self) -> Option<EntityId> {
        self.selected_component
    }

    fn refresh_document(&mut self) {
        self.source_selection = None;
        self.dsl_source = self.document.source().to_owned();
        self.pending_action = None;
        self.status_message = None;
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
        if self.document.poll_compilation() {
            self.rebuild_preview();
        }
        if self.document.build_progress().is_some() {
            context.request_repaint_after(std::time::Duration::from_millis(30));
            egui::TopBottomPanel::bottom("compilation_progress").show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    let (done, total) = self.document.build_progress().unwrap_or_default();
                    ui.label(format!("Construction CAD : {done}/{total} pièces"));
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
        self.workspace_bar(context);
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
                            if ui.button("Mini bipède — disposition").clicked() {
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

    fn library_panel(&mut self, ui: &mut Ui) {
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

    fn viewport(&mut self, ui: &mut Ui) {
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

    fn dsl_editor(&mut self, ui: &mut Ui) {
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

    fn rebuild_preview(&mut self) {
        self.diagnostics = self
            .document
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                let (line, column) =
                    robogen_dsl::line_column(self.document.source(), diagnostic.span.start);
                EditorDiagnostic {
                    line,
                    column,
                    code: diagnostic.code.clone(),
                    message: diagnostic.message.clone(),
                }
            })
            .collect();
        if self.document.has_errors() {
            return;
        }
        let Some(project) = self.document.project() else {
            return;
        };

        self.taxonomy = project.taxonomy_instances();
        self.functions = project.function_definitions();
        if !self
            .taxonomy
            .iter()
            .any(|instance| self.selected_component == Some(EntityId(instance.id.0)))
        {
            self.selected_component = self
                .taxonomy
                .first()
                .map(|instance| EntityId(instance.id.0));
        }

        let mut mesh = robogen_cad::Mesh::default();
        for part_mesh in project.snapshot().meshes.values() {
            mesh.append(part_mesh);
        }
        if mesh.vertices.is_empty() {
            self.cad_mesh = mesh;
            self.preview_mesh = PreviewMesh::default();
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
                            "Solveur indisponible",
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
                        "Aucune comparaison · Solveur indisponible",
                        ui.available_height(),
                    ),
                    2 => analysis_placeholder(ui, ui.available_height()),
                    _ => unavailable_result_card(
                        ui,
                        "Historique",
                        "Aucune exécution · Solveur indisponible",
                        ui.available_height(),
                    ),
                }
            });
    }

    fn learning_workspace(&mut self, context: &Context) {
        SidePanel::left("learning_parameters")
            .exact_width(285.0)
            .resizable(false)
            .frame(side_frame())
            .show(context, |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| learning_parameters(ui, &mut self.selected_terrain));
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
                let chart_height = ((ui.available_height() - 16.0) / 3.0).min(170.0);
                empty_chart(ui, "Récompense moyenne", "reward", chart_height);
                ui.add_space(8.0);
                empty_chart(ui, "Vitesse", "m/s", chart_height);
                ui.add_space(8.0);
                empty_chart(ui, "Taux de chute", "%", chart_height);
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(12)))
            .show(context, |ui| {
                if self.document.has_errors() {
                    ui.label(RichText::new("Dernier aperçu valide · source invalide").color(MUTED));
                }
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
    compact: bool,
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
    if ui
        .add_sized([if compact { 148.0 } else { 185.0 }, 47.0], button)
        .clicked()
    {
        *current = target;
    }
}

#[derive(Clone, Copy)]
enum Icon {
    Undo,
    Redo,
    Save,
    Plus,
    Minus,
    Export,
    OrbitLeft,
    OrbitRight,
    Home,
    Ortho,
}

fn paint_icon(painter: &egui::Painter, rect: egui::Rect, icon: Icon, stroke: Stroke) {
    let center = rect.center();
    let path = |points: &[[f32; 2]]| {
        painter.add(Shape::line(
            points.iter().map(|[x, y]| center + vec2(*x, *y)).collect(),
            stroke,
        ));
    };
    match icon {
        Icon::Undo | Icon::Redo => {
            let direction = if matches!(icon, Icon::Undo) {
                1.0
            } else {
                -1.0
            };
            let points = [
                [-7.0, -3.0],
                [1.0, -3.0],
                [5.0, -1.0],
                [7.0, 3.0],
                [7.0, 6.0],
            ];
            path(&points.map(|[x, y]| [x * direction, y]));
            path(&[
                [-3.0 * direction, -7.0],
                [-7.0 * direction, -3.0],
                [-3.0 * direction, 1.0],
            ]);
        }
        Icon::Save => {
            path(&[
                [-7.0, -7.0],
                [4.0, -7.0],
                [7.0, -4.0],
                [7.0, 7.0],
                [-7.0, 7.0],
                [-7.0, -7.0],
            ]);
            path(&[[-3.0, -7.0], [-3.0, -2.0], [3.0, -2.0], [3.0, -7.0]]);
            path(&[[-4.0, 7.0], [-4.0, 2.0], [4.0, 2.0], [4.0, 7.0]]);
        }
        Icon::Plus | Icon::Minus => {
            path(&[[-6.0, 0.0], [6.0, 0.0]]);
            if matches!(icon, Icon::Plus) {
                path(&[[0.0, -6.0], [0.0, 6.0]]);
            }
        }
        Icon::Export => {
            path(&[[-7.0, 2.0], [-7.0, 7.0], [7.0, 7.0], [7.0, 2.0]]);
            path(&[[0.0, 3.0], [0.0, -7.0]]);
            path(&[[-4.0, -3.0], [0.0, -7.0], [4.0, -3.0]]);
        }
        Icon::OrbitLeft | Icon::OrbitRight => {
            let direction = if matches!(icon, Icon::OrbitLeft) {
                1.0
            } else {
                -1.0
            };
            let points: Vec<_> = (0..=20)
                .map(|step| {
                    let angle = std::f32::consts::TAU * (0.15 + 0.8 * step as f32 / 20.0);
                    [7.0 * angle.sin() * direction, -7.0 * angle.cos()]
                })
                .collect();
            path(&points);
            path(&[
                [1.0 * direction, -3.0],
                [-2.0 * direction, -7.0],
                [-6.0 * direction, -4.0],
            ]);
        }
        Icon::Home => {
            path(&[[-8.0, -1.0], [0.0, -8.0], [8.0, -1.0]]);
            path(&[
                [-6.0, -2.0],
                [-6.0, 7.0],
                [-2.0, 7.0],
                [-2.0, 2.0],
                [2.0, 2.0],
                [2.0, 7.0],
                [6.0, 7.0],
                [6.0, -2.0],
            ]);
        }
        Icon::Ortho => path(&[
            [-7.0, -7.0],
            [7.0, -7.0],
            [7.0, 7.0],
            [-7.0, 7.0],
            [-7.0, -7.0],
        ]),
    }
}

fn diamond_mark(ui: &mut Ui, size: f32, color: Color32, filled: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let center = rect.center();
    let radius = size * 0.4;
    ui.painter().add(Shape::convex_polygon(
        vec![
            center + vec2(0.0, -radius),
            center + vec2(radius, 0.0),
            center + vec2(0.0, radius),
            center + vec2(-radius, 0.0),
        ],
        if filled { color } else { Color32::TRANSPARENT },
        Stroke::new(1.5, color),
    ));
}

fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    enabled: bool,
    tooltip: &str,
    unavailable: &str,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let response = ui.add_sized(vec2(30.0, 30.0), Button::new(""));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tooltip)
            });
            paint_icon(
                ui.painter(),
                response.rect,
                icon,
                ui.style().interact(&response).fg_stroke,
            );
            response
        })
        .inner;
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("icon_button", tooltip)), response.clone());
    });
    response
        .on_hover_text(tooltip)
        .on_disabled_hover_text(unavailable)
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

fn learning_parameters(ui: &mut Ui, selected_terrain: &mut usize) {
    ui.heading(RichText::new("Environnement").size(16.0).color(TEXT));
    ui.add_space(10.0);
    ui.label(
        RichText::new("Entraînement indisponible")
            .size(11.0)
            .color(MUTED),
    );
    ui.label(
        RichText::new("Foundation model non configuré")
            .size(11.0)
            .color(MUTED),
    );
    form_label(ui, "Terrain");
    value_field(ui, "Irrégulier");
    ui.add_enabled_ui(false, |ui| {
        ui.columns(4, |columns| {
            for (index, name) in ["Plat", "Rochers", "Escaliers", "Sable"]
                .into_iter()
                .enumerate()
            {
                if environment_tile(&mut columns[index], name, *selected_terrain == index).clicked()
                {
                    *selected_terrain = index;
                }
            }
        });
    })
    .response
    .on_disabled_hover_text("Configuration de simulation indisponible");
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
                .color(MUTED),
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
            paint_preview(
                &painter,
                rect,
                egui::Id::new(("model_preview", title)),
                renderer,
                camera,
                mesh,
            );
        });
}

fn unavailable_result_card(ui: &mut Ui, title: &str, detail: &str, height: f32) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::top_down(Align::Min),
        |ui| unavailable_canvas(ui, title, detail, "Aucun résultat calculé"),
    );
}

struct CachedPreview {
    camera: Camera,
    size: ViewportSize,
    mesh: PreviewMesh,
    texture: egui::TextureHandle,
}

fn paint_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    id: egui::Id,
    renderer: &RobotPreviewRenderer,
    camera: &Camera,
    mesh: &PreviewMesh,
) {
    let size = ViewportSize::new(rect.width(), rect.height());
    for line in renderer.grid(camera, size) {
        painter.line_segment(
            [
                rect.min + vec2(line.from.x, line.from.y),
                rect.min + vec2(line.to.x, line.to.y),
            ],
            Stroke::new(line.width, color(line.color)),
        );
    }
    let existing = painter
        .ctx()
        .data(|data| data.get_temp::<std::sync::Arc<CachedPreview>>(id));
    let cache = existing
        .filter(|cache| cache.camera == *camera && cache.size == size && cache.mesh == *mesh)
        .unwrap_or_else(|| {
            let image = renderer.rasterize_mesh(camera, size, mesh);
            let pixels = image
                .pixels
                .into_iter()
                .map(|pixel| Color32::from_rgba_unmultiplied(pixel.r, pixel.g, pixel.b, pixel.a))
                .collect();
            let texture = painter.ctx().load_texture(
                "cad_depth_preview",
                egui::ColorImage {
                    size: [image.width, image.height],
                    pixels,
                },
                egui::TextureOptions::NEAREST,
            );
            let cache = std::sync::Arc::new(CachedPreview {
                camera: *camera,
                size,
                mesh: mesh.clone(),
                texture,
            });
            painter
                .ctx()
                .data_mut(|data| data.insert_temp(id, cache.clone()));
            cache
        });
    painter.image(
        cache.texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
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
    ui.add_enabled(
        false,
        Button::new(value).min_size(vec2(ui.available_width(), 27.0)),
    )
    .on_disabled_hover_text("Paramètre indicatif · backend indisponible");
}

fn toggle_row(ui: &mut Ui, label: &str, enabled: bool) {
    let mut checked = enabled;
    ui.add_enabled(false, egui::Checkbox::new(&mut checked, label))
        .on_disabled_hover_text("Contrainte indicative · backend indisponible");
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
                diamond_mark(
                    ui,
                    17.0,
                    if selected { ACCENT_HOVER } else { MUTED },
                    selected,
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
            ui.set_width((ui.available_width() - 24.0).max(40.0));
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
        (minimum[1] + maximum[1]) * 0.5,
        minimum[2],
    ];
    let scale = 3.0
        / (0..3)
            .map(|axis| maximum[axis] - minimum[axis])
            .fold(f32::EPSILON, f32::max);
    let positions = mesh
        .vertices
        .iter()
        .map(|vertex| {
            [
                (vertex.position[0] - center[0]) * scale,
                (vertex.position[2] - center[2]) * scale,
                -(vertex.position[1] - center[1]) * scale,
            ]
        })
        .collect();
    let triangles = mesh
        .triangles
        .iter()
        .map(|triangle| triangle.indices)
        .collect();
    let width_mm = (maximum[0] - minimum[0]) * 1_000.0;
    let mut preview = PreviewMesh::new(positions, triangles);
    preview.colors = mesh
        .triangles
        .iter()
        .map(|triangle| {
            triangle
                .color
                .map(|[red, green, blue]| Rgba8::rgb(red, green, blue))
        })
        .collect();
    (preview, width_mm)
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
    fn left_column_shows_generated_instances_above_function_library(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for size in [vec2(1440.0, 831.0), vec2(1080.0, 680.0)] {
            let context = Context::default();
            let mut state = RoboGenUi::default();
            let draw = |state: &mut RoboGenUi| {
                context.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |context| state.show(context),
                )
            };
            draw(&mut state);
            let output = draw(&mut state);
            let taxonomy = text_rect(&output, "Taxonomie").ok_or("missing taxonomy")?;
            let library = text_rect(&output, "Bibliothèque").ok_or("missing library")?;
            let assistant = text_rect(&output, "Assistant IA").ok_or("missing assistant")?;
            assert!(taxonomy.right() < size.x / 2.0);
            assert!(library.top() > taxonomy.bottom());
            assert!(assistant.left() > size.x / 2.0);
            assert!(text_rect(&output, "Bracket").is_some());
            for label in ["Servomoteur", "Camera Raspberry Pi", "Jetson Nano Super"] {
                let rect = text_rect(&output, label).ok_or("missing generator")?;
                assert!(rect.top() > library.bottom() && rect.bottom() < size.y);
            }
            assert!(text_rect(&output, "Patte").is_none());
            assert!(state.instantiate_generator(LibraryGenerator::Camera));
            assert_eq!(state.taxonomy.len(), 2);
            let function = state
                .functions
                .iter()
                .find(|function| function.name == "raspberry_pi_camera")
                .ok_or("missing definition")?
                .clone();
            state.open_source(function.span);
            assert_eq!(state.design_tab, DesignTab::Dsl);
            draw(&mut state);
            assert!(state.source_selection.is_none());
            assert!(state.undo());
            assert_eq!(state.taxonomy.len(), 1);
        }
        Ok(())
    }

    #[test]
    fn starts_in_design_mode() {
        let ui = RoboGenUi::default();
        assert_eq!(ui.mode(), WorkspaceMode::Design);
        assert_eq!(
            ui.selected_component,
            ui.taxonomy.first().map(|instance| EntityId(instance.id.0))
        );
        assert_eq!(ui.taxonomy[0].name, "Bracket");
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
        for size in [vec2(1080.0, 680.0), vec2(1440.0, 831.0)] {
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
                    "Solveur indisponible",
                ),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    1,
                    "Aucune comparaison",
                ),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    2,
                    "Analyse mécanique",
                ),
                (
                    WorkspaceMode::Optimization,
                    DesignTab::Viewport,
                    3,
                    "Aucune exécution",
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
                    let required_visible = match mode {
                        WorkspaceMode::Design => {
                            vec!["Assistant IA", "Bibliothèque", "Suggestions indisponibles"]
                        }
                        WorkspaceMode::Optimization => {
                            vec!["Lancer l’optimisation", "Paramètres d’optimisation"]
                        }
                        WorkspaceMode::Learning => vec![
                            "Lancer l’entraînement",
                            "Foundation model non configuré",
                            "Chronologie du comportement",
                        ],
                    };
                    for label in required_visible {
                        assert!(
                            output.shapes.iter().any(|shape| match &shape.shape {
                                Shape::Text(painted) if painted.galley.job.text == label => {
                                    let rect = egui::Rect::from_min_size(
                                        painted.pos,
                                        painted.galley.size(),
                                    );
                                    shape.clip_rect.expand(1.0).contains_rect(rect)
                                        && egui::Rect::from_min_size(egui::Pos2::ZERO, size)
                                            .contains_rect(rect)
                                }
                                _ => false,
                            }),
                            "clipped {label} in {mode:?} at {size:?}"
                        );
                    }
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
                    assert_eq!(state.document.source(), DEFAULT_DSL);
                    assert!(!state.can_undo());
                    assert!(!state.can_redo());
                    assert_eq!(state.preview_mesh.positions, original_positions);
                }
            }
        }
    }

    #[test]
    fn servo_example_reaches_colored_preview_and_export() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state = RoboGenUi::default();
        state.enable_background_compilation()?;
        state.load_servo_example();
        assert!(!state.can_export());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !state.document.poll_compilation() {
            if std::time::Instant::now() > deadline {
                return Err("servo compilation timeout".into());
            }
            std::thread::yield_now();
        }
        state.rebuild_preview();
        assert!(state.can_export(), "{:?}", state.diagnostics);
        assert!((state.width_mm - 11.8).abs() < 1e-4);
        assert_eq!(
            state.preview_mesh.colors.len(),
            state.cad_mesh.triangles.len()
        );
        assert!(state.preview_mesh.colors.iter().all(Option::is_some));
        assert!(state.request_export());
        assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
        state.set_dsl_source("module broken; part");
        assert!(!state.can_export());
        assert!(state.take_action().is_none());
        Ok(())
    }

    #[test]
    fn camera_example_loads_in_background_exports_and_undoes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = RoboGenUi::default();
        let original = state.dsl_source().to_owned();
        state.enable_background_compilation()?;
        state.load_camera_example();
        assert!(!state.can_export());
        assert_eq!(state.mode(), WorkspaceMode::Design);
        assert_eq!(state.design_tab, DesignTab::Viewport);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !state.document.poll_compilation() {
            if std::time::Instant::now() > deadline {
                return Err("camera compilation timeout".into());
            }
            std::thread::yield_now();
        }
        state.rebuild_preview();
        assert!(state.can_export(), "{:?}", state.diagnostics);
        assert!((state.width_mm - 28.9).abs() < 1e-4);
        assert!(state.preview_mesh.colors.iter().all(Option::is_some));
        for size in [
            ViewportSize::new(600.0, 400.0),
            ViewportSize::new(320.0, 240.0),
        ] {
            let image =
                state
                    .preview_renderer
                    .rasterize_mesh(&state.camera, size, &state.preview_mesh);
            assert!(
                image.pixels.iter().filter(|pixel| pixel.a == 255).count()
                    > image.pixels.len() / 20
            );
            assert!(
                image.pixels.iter().any(|pixel| pixel.a == 255
                    && pixel.b > pixel.g
                    && pixel.g > pixel.r
                    && pixel.r >= 30),
                "lens must be visible"
            );
        }
        assert!(state.request_export());
        assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
        assert!(state.undo());
        assert_eq!(state.dsl_source(), original);
        assert!(!state.can_export());
        Ok(())
    }

    #[test]
    fn biped_loads_asynchronously_is_visible_and_undoes() -> Result<(), String> {
        let mut state = RoboGenUi::default();
        let previous = state.dsl_source().to_owned();
        state
            .enable_background_compilation()
            .map_err(|error| error.to_string())?;
        state.load_biped_example();
        assert!(!state.can_export());
        let wait = |state: &mut RoboGenUi| -> Result<(), String> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
            while !state.document.poll_compilation() {
                if std::time::Instant::now() > deadline {
                    return Err("biped compilation timeout".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            state.rebuild_preview();
            Ok(())
        };
        wait(&mut state)?;
        assert!(state.can_export(), "{:?}", state.diagnostics);
        assert_eq!(state.taxonomy.len(), 17);
        for size in [
            ViewportSize::new(600.0, 400.0),
            ViewportSize::new(320.0, 240.0),
        ] {
            let frame =
                state
                    .preview_renderer
                    .render_mesh(&state.camera, size, &state.preview_mesh);
            assert!(!frame.triangles.is_empty());
            assert!(
                frame
                    .triangles
                    .iter()
                    .flat_map(|triangle| triangle.points)
                    .all(|point| point.x > 8.0
                        && point.x < size.width - 8.0
                        && point.y > 8.0
                        && point.y < size.height - 8.0),
                "entire biped including head and ankles must fit at {size:?}"
            );
            let image =
                state
                    .preview_renderer
                    .rasterize_mesh(&state.camera, size, &state.preview_mesh);
            assert!(
                image
                    .pixels
                    .iter()
                    .filter(|pixel| pixel.a == 255 && pixel.b > 100 && pixel.b > pixel.r)
                    .count()
                    > 100,
                "blue servos visible at {size:?}"
            );
        }
        assert!(state.request_export());
        assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
        assert!(state.undo());
        assert_eq!(state.dsl_source(), previous);
        assert!(!state.can_export());
        wait(&mut state)?;
        assert!(state.redo());
        assert!(!state.can_export());
        wait(&mut state)?;
        assert_eq!(state.taxonomy.len(), 17);
        assert!(state.can_export());
        state.set_dsl_source("module invalid; part");
        assert!(!state.can_export());
        Ok(())
    }

    #[test]
    fn topology_example_reports_unavailability_and_preserves_undo() {
        let mut state = RoboGenUi::default();
        let previous = state.dsl_source().to_owned();
        let mesh = state.cad_mesh.clone();
        state.load_topology_example();
        assert_eq!(state.design_tab, DesignTab::Dsl);
        assert!(!state.can_export());
        assert!(!state.request_export());
        assert!(state.take_action().is_none());
        assert!(state
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E330"));
        assert_eq!(state.cad_mesh, mesh);
        assert!(state.undo());
        assert_eq!(state.dsl_source(), previous);
        assert!(state.can_export());
        assert!(state.redo());
        assert!(!state.can_export());
    }

    #[test]
    fn battery_example_is_visible_exportable_and_undoable() {
        let mut state = RoboGenUi::default();
        let previous = state.dsl_source().to_owned();
        state.load_battery_example();
        assert!(state.can_export(), "{:?}", state.diagnostics);
        assert!((state.width_mm - 18.0).abs() < 1e-4);
        for size in [
            ViewportSize::new(600.0, 400.0),
            ViewportSize::new(320.0, 240.0),
        ] {
            let image =
                state
                    .preview_renderer
                    .rasterize_mesh(&state.camera, size, &state.preview_mesh);
            assert!(
                image
                    .pixels
                    .iter()
                    .filter(|pixel| pixel.a == 255 && pixel.r > 100 && pixel.g > 80 && pixel.b < 40)
                    .count()
                    > 100,
                "yellow pack must be visible"
            );
            assert!(
                image
                    .pixels
                    .iter()
                    .filter(|pixel| pixel.a == 255 && pixel.r > 100 && pixel.g < 60 && pixel.b < 60)
                    .count()
                    > 10,
                "red connector must be visible"
            );
        }
        assert!(state.request_export());
        assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
        assert!(state.undo());
        assert_eq!(state.dsl_source(), previous);
        assert!(state.instantiate_generator(LibraryGenerator::Battery));
        assert!(state
            .taxonomy
            .iter()
            .any(|instance| instance.name == "li_ion_battery_1"));
        assert!(state.undo());
        assert_eq!(state.dsl_source(), previous);
    }

    #[test]
    fn compute_board_example_is_colored_exportable_and_undoable() {
        let mut state = RoboGenUi::default();
        let previous = state.dsl_source().to_owned();
        state.load_compute_board_example();
        assert!(state.can_export(), "{:?}", state.diagnostics);
        assert!((state.width_mm - 104.0).abs() < 1e-4);
        assert!(state.preview_mesh.colors.iter().all(Option::is_some));
        assert!(state.request_export());
        assert!(matches!(state.take_action(), Some(UiAction::ExportStl(_))));
        assert!(state.undo());
        assert_eq!(state.dsl_source(), previous);
    }

    #[test]
    fn raster_cache_reuses_texture_and_invalidates_camera_mesh_and_size() -> Result<(), &'static str>
    {
        let context = Context::default();
        let id = egui::Id::new("preview_cache_test");
        let mut camera = Camera::default();
        let mut mesh = PreviewMesh::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
        );
        let draw = |camera: &Camera, mesh: &PreviewMesh, width: f32| {
            let _output = context.run(egui::RawInput::default(), |context| {
                let painter = context.layer_painter(egui::LayerId::background());
                paint_preview(
                    &painter,
                    egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 100.0)),
                    id,
                    &RobotPreviewRenderer,
                    camera,
                    mesh,
                );
            });
            context
                .data(|data| data.get_temp::<std::sync::Arc<CachedPreview>>(id))
                .ok_or("missing raster cache")
        };
        let first = draw(&camera, &mesh, 160.0)?;
        let unchanged = draw(&camera, &mesh, 160.0)?;
        assert!(std::sync::Arc::ptr_eq(&first, &unchanged));
        camera.orbit(20.0, 10.0);
        let moved = draw(&camera, &mesh, 160.0)?;
        assert!(!std::sync::Arc::ptr_eq(&first, &moved));
        mesh.colors = vec![Some(Rgba8::rgb(255, 0, 0))];
        let recolored = draw(&camera, &mesh, 160.0)?;
        assert!(!std::sync::Arc::ptr_eq(&moved, &recolored));
        let resized = draw(&camera, &mesh, 180.0)?;
        assert!(!std::sync::Arc::ptr_eq(&recolored, &resized));
        Ok(())
    }

    #[test]
    fn changing_width_rebuilds_the_preview() {
        let mut ui = RoboGenUi::default();
        let before = ui.preview_mesh.positions.clone();
        ui.set_dsl_source(ui.dsl_source.replace("WIDTH = 40 mm", "WIDTH = 80 mm"));
        assert_eq!(ui.width_mm, 80.0);
        assert_ne!(ui.preview_mesh.positions, before);
        assert!((ui.cad_mesh.vertices[1].position[0] - 0.08).abs() < 1e-6);
        assert!(ui.diagnostics.is_empty());
    }

    fn frame(
        state: &mut RoboGenUi,
        context: &Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(1080.0, 680.0),
                )),
                events,
                ..Default::default()
            },
            |context| state.show(context),
        )
    }

    fn text_rect(output: &egui::FullOutput, text: &str) -> Option<egui::Rect> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            Shape::Text(label) if label.galley.job.text == text => {
                Some(egui::Rect::from_min_size(label.pos, label.galley.size()))
            }
            _ => None,
        })
    }

    fn icon_response(context: &Context, label: &str) -> Option<egui::Response> {
        context.data(|data| data.get_temp(egui::Id::new(("icon_button", label))))
    }

    #[test]
    fn painter_icons_are_font_free_and_fit_the_button() {
        let context = Context::default();
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), vec2(30.0, 30.0));
        for icon in [
            Icon::Undo,
            Icon::Redo,
            Icon::Save,
            Icon::Plus,
            Icon::Minus,
            Icon::Export,
            Icon::OrbitLeft,
            Icon::OrbitRight,
            Icon::Home,
            Icon::Ortho,
        ] {
            let output = context.run(egui::RawInput::default(), |context| {
                let painter = context.layer_painter(egui::LayerId::background());
                paint_icon(&painter, rect, icon, Stroke::new(1.5, TEXT));
            });
            assert!(!output.shapes.is_empty());
            for shape in output.shapes {
                assert!(matches!(shape.shape, Shape::Path(_)));
                assert!(rect
                    .shrink(4.0)
                    .contains_rect(shape.shape.visual_bounding_rect()));
            }
        }
    }

    #[test]
    fn icon_buttons_preserve_keyboard_labels_and_disabled_states(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (label, enabled) in [
            ("Exporter le STL du modèle courant", true),
            ("Enregistrer", false),
        ] {
            let context = Context::default();
            let mut state = RoboGenUi::default();
            frame(&mut state, &context, Vec::new());
            frame(&mut state, &context, Vec::new());
            let response = icon_response(&context, label).ok_or("missing icon button")?;
            assert_eq!(response.enabled(), enabled);
            assert_eq!(response.rect.size(), vec2(30.0, 30.0));
            response.request_focus();
            let output = frame(
                &mut state,
                &context,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            let named_click = output.platform_output.events.iter().any(|event| {
                matches!(event, egui::output::OutputEvent::Clicked(info)
                    if info.typ == egui::WidgetType::Button
                        && info.label.as_deref() == Some(label))
            });
            assert_eq!(named_click, enabled);
            assert_eq!(state.take_action().is_some(), enabled);
            click(&mut state, &context, response.rect.center());
            assert_eq!(state.take_action().is_some(), enabled);
        }
        Ok(())
    }

    fn click(state: &mut RoboGenUi, context: &Context, position: egui::Pos2) {
        for pressed in [true, false] {
            frame(
                state,
                context,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn source_history_preserves_preview_and_guards_export() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state = RoboGenUi::default();
        assert!(state.can_export());
        let original = state.preview_mesh.positions.clone();
        let wider = DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 70 mm");
        state.set_dsl_source(&wider);
        let wide_positions = state.preview_mesh.positions.clone();
        assert_eq!(state.width_mm, 70.0);
        assert!(state.request_export());
        let UiAction::ExportStl(mesh) = state.take_action().ok_or("missing STL action")?;
        assert_eq!(cad_preview(&mesh).1, 70.0);
        assert!(state.request_export());
        state.set_dsl_source("module broken; part");
        assert!(!state.can_export());
        assert!(!state.request_export());
        assert!(state.take_action().is_none());
        assert!(!state.diagnostics.is_empty());
        assert_eq!(state.preview_mesh.positions, wide_positions);
        assert!(state.undo());
        assert_eq!(state.dsl_source(), wider);
        assert!(state.can_export());
        assert!(state.diagnostics.is_empty());
        assert!(state.undo());
        assert_eq!(state.preview_mesh.positions, original);
        assert!(!state.undo());
        assert!(state.redo());
        assert_eq!(state.preview_mesh.positions, wide_positions);
        assert!(state.redo());
        assert!(!state.can_export());
        assert!(!state.redo());
        assert!(state.undo());
        state.set_dsl_source(&wider);
        assert!(state.can_redo());
        state.set_dsl_source(DEFAULT_DSL);
        assert!(!state.can_redo());
        state.set_dsl_source("module empty;");
        assert!(state.preview_mesh.is_empty());
        assert!(!state.can_export());
        Ok(())
    }

    #[test]
    fn editor_shortcuts_use_only_document_history() {
        let context = Context::default();
        let mut state = RoboGenUi {
            design_tab: DesignTab::Dsl,
            ..Default::default()
        };
        frame(&mut state, &context, Vec::new());
        context.memory_mut(|memory| memory.request_focus(egui::Id::new("robogen_dsl_source")));
        frame(
            &mut state,
            &context,
            vec![egui::Event::Text("invalid ".to_owned())],
        );
        let edited = state.dsl_source().to_owned();
        assert_ne!(edited, DEFAULT_DSL);
        assert!(state.can_undo());
        for (modifiers, expected) in [
            (egui::Modifiers::CTRL, DEFAULT_DSL),
            (
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                edited.as_str(),
            ),
            (egui::Modifiers::CTRL, DEFAULT_DSL),
        ] {
            frame(
                &mut state,
                &context,
                vec![egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert_eq!(state.dsl_source(), expected);
            assert_eq!(state.dsl_source, expected);
        }
        assert!(!state.can_undo());
        assert!(state.can_redo());
    }

    #[test]
    fn taxonomy_search_shortcuts_use_only_text_history() {
        let command = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        for (redo_modifiers, redo_key) in [
            (command | egui::Modifiers::SHIFT, egui::Key::Z),
            (command, egui::Key::Y),
        ] {
            let context = Context::default();
            let mut state = RoboGenUi::default();
            let wider = DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 70 mm");
            let widest = DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 80 mm");
            state.set_dsl_source(&wider);
            state.set_dsl_source(&widest);
            assert!(state.undo());
            let positions = state.preview_mesh.positions.clone();
            frame(&mut state, &context, Vec::new());
            let search_id = egui::Id::new("robogen_taxonomy_search");
            context.memory_mut(|memory| memory.request_focus(search_id));
            frame(&mut state, &context, Vec::new());
            frame(
                &mut state,
                &context,
                vec![egui::Event::Text("patte".to_owned())],
            );
            assert_eq!(state.taxonomy_filter, "patte");
            for (modifiers, key, expected) in [
                (command, egui::Key::Z, ""),
                (command, egui::Key::Z, ""),
                (redo_modifiers, redo_key, "patte"),
                (redo_modifiers, redo_key, "patte"),
            ] {
                frame(
                    &mut state,
                    &context,
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    }],
                );
                assert!(context.memory(|memory| memory.has_focus(search_id)));
                assert_eq!(state.taxonomy_filter, expected);
                assert_eq!(state.dsl_source(), wider);
                assert_eq!(state.dsl_source, wider);
                assert_eq!(state.width_mm, 70.0);
                assert_eq!(state.preview_mesh.positions, positions);
                assert!(state.can_undo());
                assert!(state.can_redo());
            }
            context.memory_mut(|memory| memory.surrender_focus(search_id));
            for (modifiers, key, expected) in [
                (command, egui::Key::Z, DEFAULT_DSL),
                (redo_modifiers, redo_key, wider.as_str()),
                (redo_modifiers, redo_key, widest.as_str()),
            ] {
                frame(
                    &mut state,
                    &context,
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    }],
                );
                assert_eq!(state.dsl_source(), expected);
                assert_eq!(state.dsl_source, expected);
                assert_eq!(state.taxonomy_filter, "patte");
            }
            assert!(!state.can_redo());
        }
    }

    #[test]
    fn generated_taxonomy_is_searchable_and_selectable_without_source_mutation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        let source = format!("{DEFAULT_DSL}\ncomponent servo() -> Solid {{ return box([10 mm, 10 mm, 10 mm]); }}\npart servo_gauche {{ material: Aluminium6061; body = servo(); }}\npart servo_droit {{ material: Aluminium6061; body = translate([20 mm, 0 mm, 0 mm], servo()); }}");
        state.set_dsl_source(source.clone());
        let entry_id = EntityId(
            state
                .taxonomy
                .iter()
                .find(|instance| instance.name == "servo_droit")
                .ok_or("missing generated instance")?
                .id
                .0,
        );
        state.taxonomy_filter = "DROIT".to_owned();
        frame(&mut state, &context, Vec::new());
        let output = frame(&mut state, &context, Vec::new());
        let rect = text_rect(&output, "servo_droit").ok_or("instance not rendered")?;
        assert!(text_rect(&output, "servo_gauche").is_none());
        click(&mut state, &context, rect.center());
        assert_eq!(state.selected_component(), Some(entry_id));
        assert_eq!(state.dsl_source(), source);
        state.taxonomy_filter = "GAUCHE".to_owned();
        assert!(text_rect(&frame(&mut state, &context, Vec::new()), "servo_gauche").is_some());
        let before = state.taxonomy.clone();
        state.set_dsl_source("module broken; part");
        assert_eq!(state.taxonomy, before);
        assert!(text_rect(
            &frame(&mut state, &context, Vec::new()),
            "Source invalide · résultat précédent"
        )
        .is_some());
        assert!(!state.instantiate_generator(LibraryGenerator::Camera));
        assert!(state.undo());
        assert!(state.undo());
        assert_eq!(state.dsl_source(), DEFAULT_DSL);
        assert_eq!(state.taxonomy.len(), 1);
        Ok(())
    }

    #[test]
    fn disabled_assistant_and_backends_are_honest() {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        assert_eq!(
            state.assistant_message,
            robogen_agent_api::AssistantError::Disabled.to_string()
        );
        for (mode, expected) in [
            (
                WorkspaceMode::Design,
                vec![
                    "Conversation de démonstration",
                    "assistant IA non configuré",
                    "Suggestions indisponibles",
                ],
            ),
            (
                WorkspaceMode::Optimization,
                vec!["Solveur indisponible", "Aucune métrique calculée"],
            ),
            (
                WorkspaceMode::Learning,
                vec![
                    "Foundation model non configuré",
                    "Entraînement indisponible",
                    "Aucune donnée",
                ],
            ),
        ] {
            state.set_mode(mode);
            let output = frame(&mut state, &context, Vec::new());
            let mut text = String::new();
            for shape in &output.shapes {
                collect_painted_text(&shape.shape, &mut text);
            }
            for expected in expected {
                assert!(text.contains(expected), "missing {expected}: {text}");
            }
            assert!(state.take_action().is_none());
        }
    }

    #[test]
    fn two_top_bars_keep_global_and_workspace_controls_separate(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = Context::default();
        let mut state = RoboGenUi::default();
        frame(&mut state, &context, Vec::new());
        let output = frame(&mut state, &context, Vec::new());
        let brand = text_rect(&output, "RoboGen").ok_or("missing application name")?;
        let profile = text_rect(&output, "Profil").ok_or("missing profile menu")?;
        assert!(brand.bottom() < 38.0 && profile.bottom() < 38.0);
        assert!(brand.left() < 40.0 && profile.left() > 980.0);
        assert!(text_rect(&output, "From idea to real robots").is_none());
        let mut previous_right = 0.0;
        for title in [
            "1    Design\n       Définir la robotique",
            "2    Optimisation\n       Alléger et renforcer",
            "3    Apprentissage\n       Apprendre à agir",
        ] {
            let rect =
                text_rect(&output, title).ok_or_else(|| format!("missing topbar label {title}"))?;
            assert!(
                rect.left() >= previous_right
                    && rect.right() <= 1080.0
                    && rect.top() >= 38.0
                    && rect.bottom() <= 94.0,
                "{title}: {rect:?}"
            );
            previous_right = rect.right();
        }
        let undo_rect = icon_response(&context, "Annuler (Ctrl+Z)")
            .ok_or("missing undo button")?
            .rect;
        assert!(undo_rect.left() > previous_right);
        assert!(undo_rect.top() >= 38.0 && undo_rect.bottom() <= 94.0);
        click(&mut state, &context, profile.center());
        let profile_output = frame(&mut state, &context, Vec::new());
        assert!(text_rect(&profile_output, "Profil non configuré").is_some());
        click(&mut state, &context, profile.center());
        let orbit_rect = icon_response(&context, "Orbite droite")
            .ok_or("missing orbit button")?
            .rect;
        let original_camera = state.camera;
        click(&mut state, &context, orbit_rect.center());
        assert_ne!(state.camera, original_camera);
        let reset_rect = icon_response(&context, "Réinitialiser la caméra")
            .ok_or("missing reset button")?
            .rect;
        click(&mut state, &context, reset_rect.center());
        assert_eq!(state.camera, original_camera);
        state.set_dsl_source(DEFAULT_DSL.replace("WIDTH = 40 mm", "WIDTH = 70 mm"));
        frame(&mut state, &context, Vec::new());
        click(&mut state, &context, undo_rect.center());
        assert_eq!(state.dsl_source(), DEFAULT_DSL);
        assert!(state.can_redo());
        Ok(())
    }
}
