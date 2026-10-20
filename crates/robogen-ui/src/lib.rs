//! RoboGen's desktop UI shell.
//!
//! The facade owns navigation and presentation state. Feature views dispatch
//! undoable document commands through `SourceDocument`; project services retain
//! revision, computation and publication guards.

use egui::{
    vec2, Align, Button, Color32, Context, FontId, Frame, Layout, Margin, RichText, ScrollArea,
    Sense, Shape, SidePanel, Stroke, TextEdit, TextStyle, TopBottomPanel, Ui, Vec2,
};
use robogen_agent_api::{AiAssistantProvider, AssistantRequest, DisabledAiAssistant};
use robogen_domain::{EntityId, SourceSpan};
use robogen_project::{FunctionDefinition, LibraryGenerator, SourceDocument, TaxonomyInstance};
use robogen_render::{Camera, PreviewMesh, Rgba8, RobotPreviewRenderer, ViewportSize};

mod assistant;
mod design;
mod editor;
mod examples;
mod icons;
mod learning;
mod library;
mod mass_diagnostics;
mod optimization;
mod preview;
mod shell;
mod theme;
mod topology;
mod viewport;
mod widgets;

use icons::*;
use preview::*;
use theme::*;
use widgets::*;

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
    preparation_meshes: [PreviewMesh; 3],
    show_regions: [bool; 3],
    instance_interface_meshes: PreviewMesh,
    show_instance_interfaces: bool,
    show_components: bool,
    show_results: bool,
    show_densities: bool,
    density_reference: bool,
    density_threshold: f64,
    selected_optimization: Option<robogen_domain::PartId>,
    optimization_settings:
        std::collections::BTreeMap<robogen_domain::PartId, robogen_project::ExecutionSettings>,
    grid_controls: std::collections::BTreeMap<robogen_domain::PartId, GridControls>,
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

struct GridControls {
    manual: bool,
    cell_size_mm: f64,
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
            preparation_meshes: std::array::from_fn(|_| PreviewMesh::default()),
            show_regions: [true; 3],
            instance_interface_meshes: PreviewMesh::default(),
            show_instance_interfaces: true,
            show_components: true,
            show_results: true,
            show_densities: true,
            density_reference: false,
            density_threshold: 0.35,
            selected_optimization: None,
            optimization_settings: std::collections::BTreeMap::new(),
            grid_controls: std::collections::BTreeMap::new(),
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
        if !self.document.is_valid() {
            return false;
        }
        let Some(project) = self.document.project() else {
            return false;
        };
        if self.mode == WorkspaceMode::Optimization {
            if let Some(part) = self.selected_optimization {
                return project.snapshot().optimization_states.get(&part)
                    == Some(&robogen_project::OptimizationState::Calculated);
            }
        }
        project
            .snapshot()
            .optimization_states
            .values()
            .all(|state| *state == robogen_project::OptimizationState::Calculated)
            && !self.cad_mesh.triangles.is_empty()
    }

    pub fn request_export(&mut self) -> bool {
        if !self.can_export() {
            return false;
        }
        let mesh = if self.mode == WorkspaceMode::Optimization {
            self.selected_optimization
                .and_then(|part| self.document.project()?.snapshot().meshes.get(&part))
                .unwrap_or(&self.cad_mesh)
        } else {
            &self.cad_mesh
        };
        self.pending_action = Some(UiAction::ExportStl(mesh.clone()));
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
mod tests;
