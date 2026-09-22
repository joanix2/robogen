use eframe::egui;
use robogen_ui::{RoboGenUi, UiAction};
use std::path::Path;

#[derive(Default)]
struct DesktopApp {
    ui: RoboGenUi,
}

impl eframe::App for DesktopApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(context);
        if let Some(UiAction::ExportStl(mesh)) = self.ui.take_action() {
            let output = Path::new("robogen-export.stl");
            let status = match robogen_export::export_binary_stl(&mesh, output) {
                Ok(()) => format!("STL exporté : {}", output.display()),
                Err(error) => format!("Échec export STL : {error}"),
            };
            self.ui.set_status(status);
        }
    }
}

fn main() -> eframe::Result<()> {
    if let Err(error) = robogen_telemetry::init() {
        eprintln!("RoboGen telemetry was already initialized: {error}");
    }
    tracing::info!("starting RoboGen desktop with the eframe wgpu renderer");

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("RoboGen")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1080.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RoboGen",
        options,
        Box::new(|_creation_context| Ok(Box::<DesktopApp>::default())),
    )
}
