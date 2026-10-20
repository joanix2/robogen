//! Presentation of experimental mass diagnostics; never grants export eligibility.

use super::*;
use robogen_project::OptimizationState;

pub(super) fn mass_diagnostic_view<'a>(
    ui: &mut Ui,
    diagnostic: &robogen_project::MassDiagnostic,
    state: OptimizationState,
    case_name: impl Fn(robogen_domain::LoadCaseId) -> &'a str,
    density_reference: &mut bool,
    density_threshold: &mut f64,
) -> bool {
    let mut diagnostic_view_changed = false;
    ui.separator();
    ui.colored_label(
        Color32::from_rgb(255, 190, 100),
        "Champ de densité · non validé pour fabrication",
    );
    if let Some(binary) = &diagnostic.binary {
        ui.label(format!(
            "Solide connecté réanalysé : {:.3} g · seuil {:.4}",
            binary.mass.kilograms() * 1000.0,
            binary.threshold
        ));
        for (id, displacement) in &binary.displacements {
            let name = case_name(*id);
            ui.label(format!(
                "{name} · solide : {:.6} mm",
                displacement.millimetres()
            ));
        }
        if ui.button("Afficher le solide réanalysé").clicked() {
            *density_reference = false;
            *density_threshold = binary.threshold;
            diagnostic_view_changed = true;
        }
    }
    if let Some(reason) = &diagnostic.reconstruction_error {
        ui.label(format!("Reconstruction refusée : {reason}"));
    }
    ui.horizontal(|ui| {
        diagnostic_view_changed |= ui
            .selectable_value(density_reference, false, "Calcul")
            .changed();
        diagnostic_view_changed |= ui
            .selectable_value(density_reference, true, "Référence")
            .changed();
    });
    diagnostic_view_changed |= ui
        .add(egui::Slider::new(density_threshold, 0.0..=1.0).text("Seuil visuel"))
        .changed();
    if let Ok(connection) = diagnostic.connectivity(*density_threshold, *density_reference) {
        let connected = connection.preserved_components == 1;
        ui.colored_label(
            if connected {
                MUTED
            } else {
                Color32::from_rgb(255, 150, 100)
            },
            format!(
                "{} composante(s) · zones conservées {}",
                connection.components,
                if connected {
                    "reliées"
                } else {
                    "non reliées"
                }
            ),
        );
        if !connected {
            if let Ok(Some(threshold)) = diagnostic.connection_threshold(*density_reference) {
                if ui
                    .button(format!("Afficher au seuil de connexion ({threshold:.4})"))
                    .clicked()
                {
                    *density_threshold = threshold;
                    diagnostic_view_changed = true;
                }
            } else {
                ui.label("Aucun seuil ne relie les zones conservées.");
            }
        }
        ui.small("La connexion visuelle ne valide pas la résistance du solide.");
    }
    ui.label(format!(
        "Masse du champ calculé : {:.3} g",
        diagnostic.mass.kilograms() * 1000.0
    ));
    ui.label(format!(
        "Référence : {:.3} g",
        diagnostic.reference_mass.kilograms() * 1000.0
    ));
    ui.label(format!(
        "Variation : {:.2} %",
        100.0 * (diagnostic.mass.kilograms() / diagnostic.reference_mass.kilograms() - 1.0)
    ));
    ui.label(if diagnostic.finished {
        if diagnostic.converged && diagnostic.feasible {
            if diagnostic.binary.is_some() {
                "Convergence et réanalyse du solide · fabrication non validée"
            } else {
                "Convergence numérique atteinte · solide non validé"
            }
        } else {
            "Arrêt sans validation des critères numériques"
        }
    } else {
        match state {
            OptimizationState::Running => "Calcul en cours · dernier champ reçu",
            OptimizationState::Cancelled => "Dernier champ avant annulation",
            OptimizationState::Failed => "Dernier champ avant échec",
            _ => "Dernier champ reçu · non validé",
        }
    });
    if let Some(residual) = diagnostic.kkt_residual {
        ui.label(format!("Résidu KKT : {residual:.3e} · limite 1e-4"));
    }
    for (case_id, displacement, limit) in &diagnostic.cases {
        let name = case_name(*case_id);
        ui.label(format!(
            "{name} : {:.6} / {:.6} mm",
            displacement.millimetres(),
            limit.millimetres()
        ));
    }
    if diagnostic.history.len() > 1 {
        ui.label("Masse du champ (g)");
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 85.0), Sense::hover());
        let maximum = diagnostic
            .history
            .iter()
            .map(|entry| entry.mass.kilograms())
            .fold(diagnostic.reference_mass.kilograms(), f64::max);
        let points: Vec<_> = diagnostic
            .history
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                egui::pos2(
                    rect.left()
                        + rect.width() * index as f32 / (diagnostic.history.len() - 1) as f32,
                    rect.bottom() - rect.height() * (entry.mass.kilograms() / maximum) as f32,
                )
            })
            .collect();
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, MUTED),
        );
        ui.painter().add(egui::Shape::line(
            points,
            Stroke::new(2.0, Color32::from_rgb(75, 215, 160)),
        ));
        ui.label(format!(
            "{} évaluations · référence {:.3} g",
            diagnostic.history.len(),
            maximum * 1000.0
        ));
    }
    ui.label(format!(
        "Grille {:?} · révision {}",
        diagnostic.cells, diagnostic.document_revision
    ));
    ui.label(robogen_project::MASS_BACKEND);
    ui.label(match diagnostic.compute_backend {
        robogen_project::ComputeBackend::Cpu => "Résolution CPU directe",
        robogen_project::ComputeBackend::Cuda => "Résolution GPU CUDA · CG Float64",
    });
    diagnostic_view_changed
}
