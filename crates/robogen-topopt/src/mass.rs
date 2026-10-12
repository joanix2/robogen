use crate::{
    TopologyRequest,
    execution::{
        ComputeBackend, ExecutionError, ExecutionSettings, PreparedTopology, failure,
        prepare_topology,
    },
    simp::CellRegion,
};
use robogen_domain::{Length, LoadCaseId, Mass, OptimizationId, SourceSpan};
use robogen_ir::topology::{ConstraintKind, Objective};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub const MASS_BACKEND: &str = "TopOpt.jl 0.14.0 + robogen-compat2 (experimental)";

#[derive(Clone, Debug, PartialEq)]
pub struct MassIteration {
    pub mass: Mass,
    pub infeasibility: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryMassDiagnostic {
    pub threshold: f64,
    pub mass: Mass,
    pub displacements: Vec<(LoadCaseId, Length)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassDiagnostic {
    pub compute_backend: ComputeBackend,
    pub id: OptimizationId,
    pub name: String,
    pub span: SourceSpan,
    pub document_revision: u64,
    pub origin: [Length; 3],
    pub dimensions: [Length; 3],
    pub cells: [usize; 3],
    pub regions: Vec<CellRegion>,
    pub densities: Vec<f64>,
    pub reference_densities: Vec<f64>,
    pub mass: Mass,
    pub reference_mass: Mass,
    pub cases: Vec<(LoadCaseId, Length, Length)>,
    pub history: Vec<MassIteration>,
    pub finished: bool,
    pub feasible: bool,
    pub converged: bool,
    pub kkt_residual: Option<f64>,
    pub binary: Option<BinaryMassDiagnostic>,
    pub reconstruction_error: Option<String>,
}

/// Face-connected components of a thresholded diagnostic, not mechanical approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityConnectivity {
    pub components: usize,
    pub preserved_components: usize,
    pub selected_cells: usize,
    pub preserved_cells: usize,
}

impl MassDiagnostic {
    pub fn connectivity(
        &self,
        threshold: f64,
        reference: bool,
    ) -> Result<DensityConnectivity, String> {
        let densities = if reference {
            &self.reference_densities
        } else {
            &self.densities
        };
        let count = self
            .cells
            .iter()
            .try_fold(1usize, |n, axis| n.checked_mul(*axis));
        if self.cells.contains(&0)
            || count != Some(densities.len())
            || densities.len() != self.regions.len()
            || densities.len() > 32768
            || !threshold.is_finite()
            || !(0.0..=1.0).contains(&threshold)
            || densities.iter().zip(&self.regions).any(|(value, role)| {
                !value.is_finite()
                    || !(0.0..=1.0).contains(value)
                    || (*role == CellRegion::Preserve && *value != 1.0)
                    || (*role == CellRegion::Void && *value != 0.0)
            })
        {
            return Err("Invalid density connectivity input".into());
        }
        let active = |index: usize| densities[index] > 0.0 && densities[index] >= threshold;
        let mut seen = vec![false; densities.len()];
        let mut report = DensityConnectivity {
            components: 0,
            preserved_components: 0,
            selected_cells: 0,
            preserved_cells: 0,
        };
        let strides = [self.cells[1] * self.cells[2], self.cells[2], 1];
        let mut queue = Vec::new();
        for start in 0..densities.len() {
            if seen[start] || !active(start) {
                continue;
            }
            report.components += 1;
            let mut has_preserve = false;
            seen[start] = true;
            queue.push(start);
            while let Some(index) = queue.pop() {
                report.selected_cells += 1;
                if self.regions[index] == CellRegion::Preserve {
                    has_preserve = true;
                    report.preserved_cells += 1;
                }
                for (axis, stride) in strides.iter().enumerate() {
                    let coordinate = index / stride % self.cells[axis];
                    let neighbors = [
                        (coordinate > 0).then(|| index - stride),
                        (coordinate + 1 < self.cells[axis]).then(|| index + stride),
                    ];
                    for neighbor in neighbors.into_iter().flatten() {
                        if !seen[neighbor] && active(neighbor) {
                            seen[neighbor] = true;
                            queue.push(neighbor);
                        }
                    }
                }
            }
            report.preserved_components += usize::from(has_preserve);
        }
        Ok(report)
    }

    /// Highest existing density threshold connecting every preserved cell by faces.
    /// Merely proposes a view setting: no density is changed, no bridge is added,
    /// and detached non-preserved islands are neither hidden nor removed.
    pub fn connection_threshold(&self, reference: bool) -> Result<Option<f64>, String> {
        let full = self.connectivity(0.0, reference)?;
        if full.preserved_cells == 0 || full.preserved_components != 1 {
            return Ok(None);
        }
        let mut levels: Vec<_> = if reference {
            self.reference_densities.clone()
        } else {
            self.densities.clone()
        };
        levels.retain(|density| *density > 0.0);
        levels.sort_by(f64::total_cmp);
        levels.dedup();
        let mut lower = 0;
        let mut upper = levels.len();
        while lower + 1 < upper {
            let middle = lower + (upper - lower) / 2;
            if self
                .connectivity(levels[middle], reference)?
                .preserved_components
                == 1
            {
                lower = middle;
            } else {
                upper = middle;
            }
        }
        Ok(Some(levels[lower]))
    }
}

#[derive(Deserialize)]
struct Envelope {
    schema: u32,
    operation: OptimizationId,
    revision: u64,
    compute_backend: ComputeBackend,
    #[serde(flatten)]
    event: Event,
}

#[derive(Clone, Deserialize)]
struct BinaryResult {
    threshold: f64,
    mass_kg: f64,
    displacements_m: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Event {
    Reference {
        densities: Vec<f64>,
        mass_kg: f64,
        reference_mass_kg: f64,
        displacements_m: Vec<f64>,
    },
    Progress {
        mass_kg: f64,
        infeasibility: f64,
    },
    Iteration {
        densities: Vec<f64>,
        mass_kg: f64,
        reference_mass_kg: f64,
        displacements_m: Vec<f64>,
    },
    Result {
        densities: Vec<f64>,
        mass_kg: f64,
        reference_mass_kg: f64,
        displacements_m: Vec<f64>,
        violations: Vec<f64>,
        feasible: bool,
        converged: bool,
        kkt_residual: f64,
        publishable: bool,
        #[serde(default)]
        binary: Option<BinaryResult>,
        #[serde(default)]
        reconstruction_error: Option<String>,
    },
    Error {
        message: String,
    },
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wire_request(prepared: &PreparedTopology, settings: &ExecutionSettings) -> serde_json::Value {
    let request = &prepared.request;
    let limit = request
        .specification
        .constraints
        .iter()
        .filter_map(|constraint| match constraint.kind {
            ConstraintKind::MaxDisplacement(limit) => Some(limit.metres()),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min);
    json!({
        "schema": 1, "operation": request.specification.id,
        "revision": request.document_revision, "objective": "minimize_mass",
        "compute_backend": settings.compute_backend,
        "cells": settings.cells, "dimensions_m": prepared.grid.grid.dimensions.map(Length::metres),
        "young_pa": request.material.young_modulus.pascals(),
        "poisson": request.material.poisson_ratio,
        "density_kg_m3": request.material.density.kg_per_m3(),
        "regions": prepared.grid.regions.iter().map(|region| match region {
            CellRegion::Design => "design", CellRegion::Preserve => "preserve", CellRegion::Void => "void",
        }).collect::<Vec<_>>(),
        "filter_radius_m": settings.filter_radius.metres(),
        "max_iterations": settings.max_iterations,
        "reconstruction_threshold": settings.threshold,
        "limits_m": vec![limit; prepared.grid.load_cases.len()],
        "cases": prepared.grid.load_cases.iter().map(|case| json!({
            "id": case.id,
            "constraints": case.boundary.constraints.iter().map(|constraint| json!({
                "node": constraint.node.0, "fixed": constraint.fixed,
            })).collect::<Vec<_>>(),
            "forces": case.boundary.forces.iter().map(|force| json!({
                "node": force.node.0, "force_n": force.force.map(|value| value.newtons()),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn update_diagnostic(
    diagnostic: &mut MassDiagnostic,
    event: Envelope,
    density: f64,
) -> Result<(), String> {
    if event.schema != 1
        || event.operation != diagnostic.id
        || event.revision != diagnostic.document_revision
        || event.compute_backend != diagnostic.compute_backend
        || diagnostic.finished
    {
        return Err("Stale, incompatible or out-of-order mass event".into());
    }
    let mut binary_result = None;
    let (mut densities, mass, reference_mass, displacements) = match event.event {
        Event::Error { message } => return Err(message.chars().take(2048).collect()),
        Event::Progress {
            mass_kg,
            infeasibility,
        } => {
            if diagnostic.reference_densities.is_empty()
                || !mass_kg.is_finite()
                || mass_kg <= 0.0
                || !infeasibility.is_finite()
                || infeasibility < 0.0
                || diagnostic.history.len() >= 10000
            {
                return Err("Invalid mass progress".into());
            }
            diagnostic.history.push(MassIteration {
                mass: Mass::from_kilograms(mass_kg),
                infeasibility,
            });
            return Ok(());
        }
        Event::Iteration {
            densities,
            mass_kg,
            reference_mass_kg,
            displacements_m,
        } => {
            if diagnostic.reference_densities.is_empty()
                || reference_mass_kg != diagnostic.reference_mass.kilograms()
            {
                return Err("Invalid iteration sequence".into());
            }
            (densities, mass_kg, reference_mass_kg, displacements_m)
        }
        Event::Reference {
            densities,
            mass_kg,
            reference_mass_kg,
            displacements_m,
        } => {
            if !diagnostic.reference_densities.is_empty() || mass_kg != reference_mass_kg {
                return Err("Invalid reference sequence".into());
            }
            (densities, mass_kg, reference_mass_kg, displacements_m)
        }
        Event::Result {
            densities,
            mass_kg,
            reference_mass_kg,
            displacements_m,
            violations,
            feasible,
            converged,
            kkt_residual,
            publishable,
            binary,
            reconstruction_error,
        } => {
            if diagnostic.reference_densities.is_empty()
                || publishable
                || reference_mass_kg != diagnostic.reference_mass.kilograms()
                || !kkt_residual.is_finite()
                || kkt_residual < 0.0
                || (converged && (!feasible || kkt_residual > 1e-4))
                || violations.len() != diagnostic.cases.len()
                || displacements_m.len() != diagnostic.cases.len()
                || violations
                    .iter()
                    .zip(&displacements_m)
                    .zip(&diagnostic.cases)
                    .any(|((violation, displacement), (_, _, limit))| {
                        !violation.is_finite()
                            || (violation - ((displacement / limit.metres()).powi(2) - 1.0)).abs()
                                > 1e-8
                    })
                || feasible != violations.iter().all(|violation| *violation <= 1e-5)
            {
                return Err("Invalid final mass diagnostic".into());
            }
            if reconstruction_error
                .as_ref()
                .is_some_and(|message| message.len() > 2048)
                || (binary.is_some() && reconstruction_error.is_some())
            {
                return Err("Invalid reconstruction status".into());
            }
            diagnostic.reconstruction_error = reconstruction_error;
            binary_result = binary;
            diagnostic.finished = true;
            diagnostic.feasible = feasible;
            diagnostic.converged = converged;
            diagnostic.kkt_residual = Some(kkt_residual);
            (densities, mass_kg, reference_mass_kg, displacements_m)
        }
    };
    let volume = diagnostic
        .dimensions
        .iter()
        .map(|length| length.metres())
        .product::<f64>()
        / diagnostic.regions.len() as f64;
    let integrated = densities.iter().sum::<f64>() * volume * density;
    if densities.len() != diagnostic.regions.len()
        || densities
            .iter()
            .zip(&diagnostic.regions)
            .any(|(value, region)| {
                !value.is_finite()
                    || !(-32.0 * f64::EPSILON..=1.0 + 32.0 * f64::EPSILON).contains(value)
                    || (*region == CellRegion::Preserve && *value != 1.0)
                    || (*region == CellRegion::Void && *value != 0.0)
            })
        || !mass.is_finite()
        || mass <= 0.0
        || (mass - integrated).abs() > 1e-8 * integrated.max(1e-12)
        || !reference_mass.is_finite()
        || reference_mass <= 0.0
        || displacements.len() != diagnostic.cases.len()
        || displacements
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("Invalid density field, mass or displacement data".into());
    }
    for value in &mut densities {
        *value = value.clamp(0.0, 1.0);
    }
    if diagnostic.reference_densities.is_empty() {
        diagnostic.reference_densities = densities.clone();
    }
    diagnostic.densities = densities;
    diagnostic.mass = Mass::from_kilograms(mass);
    diagnostic.reference_mass = Mass::from_kilograms(reference_mass);
    for (case, displacement) in diagnostic.cases.iter_mut().zip(displacements) {
        case.1 = Length::from_metres(displacement);
    }
    if let Some(binary) = binary_result {
        let connection = diagnostic.connectivity(binary.threshold, false)?;
        let binary_mass = connection.selected_cells as f64 * volume * density;
        if !diagnostic.finished
            || !diagnostic.converged
            || !diagnostic.feasible
            || binary.threshold <= 0.0
            || connection.components != 1
            || connection.preserved_components != 1
            || !binary.mass_kg.is_finite()
            || binary.mass_kg <= 0.0
            || (binary.mass_kg - binary_mass).abs() > 1e-8 * binary_mass
            || binary.displacements_m.len() != diagnostic.cases.len()
            || binary
                .displacements_m
                .iter()
                .zip(&diagnostic.cases)
                .any(|(value, (_, _, limit))| {
                    !value.is_finite() || *value < 0.0 || *value > limit.metres()
                })
        {
            return Err("Invalid reconstructed-solid diagnostic".into());
        }
        diagnostic.binary = Some(BinaryMassDiagnostic {
            threshold: binary.threshold,
            mass: Mass::from_kilograms(binary.mass_kg),
            displacements: diagnostic
                .cases
                .iter()
                .zip(binary.displacements_m)
                .map(|((id, _, _), value)| (*id, Length::from_metres(value)))
                .collect(),
        });
    }
    Ok(())
}

pub fn diagnose_mass(
    request: &TopologyRequest,
    settings: &ExecutionSettings,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(&MassDiagnostic),
) -> Result<MassDiagnostic, ExecutionError> {
    let prepared = prepare_topology(request, settings, cancelled, &mut |_| {})?;
    let span = request.specification.span;
    let error = |message| failure("E348", span, message);
    if request.specification.objective != Objective::MinimizeMass {
        return Err(error("Mass backend requires minimize_mass".to_owned()));
    }
    let wire = wire_request(&prepared, settings);
    let limit = wire["limits_m"][0]
        .as_f64()
        .ok_or_else(|| error("Missing displacement limit".into()))?;
    let mut diagnostic = MassDiagnostic {
        compute_backend: settings.compute_backend,
        id: request.specification.id,
        name: request.specification.name.clone(),
        span,
        document_revision: request.document_revision,
        origin: prepared.origin,
        dimensions: prepared.grid.grid.dimensions,
        cells: settings.cells,
        regions: prepared.grid.regions,
        densities: Vec::new(),
        reference_densities: Vec::new(),
        mass: Mass::from_kilograms(0.0),
        reference_mass: Mass::from_kilograms(0.0),
        cases: prepared
            .grid
            .load_cases
            .iter()
            .map(|case| {
                (
                    case.id,
                    Length::from_metres(0.0),
                    Length::from_metres(limit),
                )
            })
            .collect(),
        history: Vec::new(),
        finished: false,
        feasible: false,
        converged: false,
        kkt_residual: None,
        binary: None,
        reconstruction_error: None,
    };
    let backend = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("backends/topopt-jl");
    let tools = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/topopt-tools");
    let julia = std::env::var_os("ROBOGEN_JULIA")
        .map(PathBuf::from)
        .unwrap_or_else(|| tools.join("julia-1.10.10/bin/julia"));
    let mut input = tempfile::NamedTempFile::new().map_err(|failure| error(failure.to_string()))?;
    serde_json::to_writer(input.as_file_mut(), &wire)
        .map_err(|failure| error(failure.to_string()))?;
    if cancelled() {
        return Err(failure("E345", span, "Mass calculation cancelled"));
    }
    let child = Command::new(&julia)
        .arg("--startup-file=no")
        .arg(format!("--project={}", backend.display()))
        .arg(backend.join("native.jl"))
        .arg(input.path())
        .env("JULIA_DEPOT_PATH", tools.join("depot"))
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("JULIA_NUM_THREADS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|failure| {
            error(format!(
                "Julia unavailable at {}: {failure}",
                julia.display()
            ))
        })?;
    let mut process = Process(child);
    let output = process
        .0
        .stdout
        .take()
        .ok_or_else(|| error("Missing process output".into()))?;
    let (sender, receiver) = mpsc::sync_channel(4);
    std::thread::Builder::new()
        .name("robogen-mass-protocol".into())
        .spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut bytes = Vec::new();
                let result = std::io::Read::by_ref(&mut reader)
                    .take(1_048_577)
                    .read_until(b'\n', &mut bytes);
                let message = match result {
                    Ok(0) => break,
                    Ok(_) if bytes.len() > 1_048_576 => {
                        Err("Mass event exceeds byte budget".to_owned())
                    }
                    Ok(_) => serde_json::from_slice::<Envelope>(&bytes)
                        .map_err(|failure| failure.to_string()),
                    Err(failure) => Err(failure.to_string()),
                };
                let failed = message.is_err();
                if sender.send(message).is_err() || failed {
                    break;
                }
            }
        })
        .map_err(|failure| error(failure.to_string()))?;
    let started = Instant::now();
    loop {
        if cancelled() {
            return Err(failure("E345", span, "Mass calculation cancelled"));
        }
        if started.elapsed() > Duration::from_secs(1800) {
            return Err(error("Mass process exceeded 30 minute budget".into()));
        }
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => {
                let mut next = diagnostic.clone();
                update_diagnostic(
                    &mut next,
                    event.map_err(error)?,
                    request.material.density.kg_per_m3(),
                )
                .map_err(error)?;
                diagnostic = next;
                progress(&diagnostic);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(status) = process
                    .0
                    .try_wait()
                    .map_err(|failure| error(failure.to_string()))?
                {
                    if !status.success() || !diagnostic.finished {
                        return Err(error(format!(
                            "Mass process ended without a complete diagnostic: {status}"
                        )));
                    }
                    return Ok(diagnostic);
                }
                std::thread::park_timeout(Duration::from_millis(20));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "exports the current biped numerical request for the Julia CPU/CUDA benchmark"]
    fn export_biped_gpu_qualification_request() -> Result<(), Box<dyn std::error::Error>> {
        let parsed = robogen_dsl::parse(include_str!("../../../examples/mini_biped/main.rgn"));
        assert!(parsed.diagnostics.is_empty());
        let lowered = robogen_ir::lower(&parsed.module.ok_or("missing module")?);
        assert!(lowered.diagnostics.is_empty());
        let specification = lowered
            .model
            .topology_operations()
            .into_iter()
            .find(|specification| specification.name == "left_shin_PLA_estimated")
            .ok_or("missing tibia")?
            .clone();
        let material = lowered
            .model
            .materials
            .values()
            .find(|material| Some(material.id) == specification.material)
            .ok_or("missing material")?
            .clone();
        let request = TopologyRequest {
            specification,
            material,
            document_revision: 16,
        };
        let resolution = crate::execution::resolution_for_cell_size(
            &request.specification,
            Length::from_millimetres(2.5),
        )?;
        let settings = ExecutionSettings {
            compute_backend: ComputeBackend::Cuda,
            cells: resolution.cells,
            filter_radius: Length::from_millimetres(12.0),
            threshold: 0.65,
            max_iterations: 2,
        };
        let prepared = prepare_topology(&request, &settings, &|| false, &mut |_| {})?;
        let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
        std::fs::create_dir_all(&target)?;
        serde_json::to_writer(
            std::fs::File::create(target.join("left-shin-gpu-request.json"))?,
            &wire_request(&prepared, &settings),
        )?;
        Ok(())
    }

    fn diagnostic() -> Result<MassDiagnostic, serde_json::Error> {
        Ok(MassDiagnostic {
            compute_backend: ComputeBackend::Cpu,
            id: serde_json::from_value(json!("00000000-0000-0000-0000-000000000001"))?,
            name: "protocol-test".into(),
            span: SourceSpan::default(),
            document_revision: 7,
            origin: [Length::from_metres(0.0); 3],
            dimensions: [0.02, 0.01, 0.01].map(Length::from_metres),
            cells: [2, 1, 1],
            regions: vec![CellRegion::Preserve, CellRegion::Design],
            densities: Vec::new(),
            reference_densities: Vec::new(),
            mass: Mass::from_kilograms(0.0),
            reference_mass: Mass::from_kilograms(0.0),
            cases: vec![(
                serde_json::from_value(json!("00000000-0000-0000-0000-000000000002"))?,
                Length::from_metres(0.0),
                Length::from_metres(1e-4),
            )],
            history: Vec::new(),
            finished: false,
            feasible: false,
            converged: false,
            kkt_residual: None,
            binary: None,
            reconstruction_error: None,
        })
    }

    #[test]
    fn threshold_connectivity_reports_broken_attachments_without_repairing_density()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut field = diagnostic()?;
        field.cells = [5, 1, 1];
        field.regions = vec![
            CellRegion::Preserve,
            CellRegion::Design,
            CellRegion::Design,
            CellRegion::Design,
            CellRegion::Preserve,
        ];
        field.densities = vec![1.0, 0.8, 0.2, 0.6, 1.0];
        field.reference_densities = vec![1.0; 5];
        let original = field.densities.clone();
        assert_eq!(field.connectivity(0.35, false)?.preserved_components, 2);
        assert_eq!(field.connection_threshold(false)?, Some(0.2));
        assert_eq!(field.connectivity(0.2, false)?.components, 1);
        assert_eq!(field.connection_threshold(true)?, Some(1.0));
        assert_eq!(field.densities, original);
        field.regions[2] = CellRegion::Void;
        field.densities[2] = 0.0;
        assert_eq!(field.connection_threshold(false)?, None);
        assert_eq!(field.connectivity(0.0, false)?.preserved_components, 2);
        field.cells = [2, 2, 1];
        field.regions = vec![
            CellRegion::Preserve,
            CellRegion::Void,
            CellRegion::Void,
            CellRegion::Preserve,
        ];
        field.densities = vec![1.0, 0.0, 0.0, 1.0];
        assert_eq!(field.connection_threshold(false)?, None); // edge contact is not a path
        assert!(field.connectivity(f64::NAN, false).is_err());
        field.cells = [usize::MAX, 2, 1];
        assert!(field.connectivity(0.5, false).is_err());
        Ok(())
    }

    fn reference() -> serde_json::Value {
        json!({"schema": 1, "operation": "00000000-0000-0000-0000-000000000001", "revision": 7,
            "compute_backend": "cpu",
            "kind": "reference", "densities": [1.0, 1.0], "mass_kg": 0.0054, "reference_mass_kg": 0.0054,
            "displacements_m": [1e-6]})
    }

    #[test]
    fn binary_protocol_checks_connectivity_mass_and_each_physical_limit()
    -> Result<(), Box<dyn std::error::Error>> {
        let finish = json!({"schema":1,"operation":"00000000-0000-0000-0000-000000000001",
            "revision":7,"compute_backend":"cpu","kind":"result",
            "densities":[1.0,0.5],"mass_kg":0.00405,"reference_mass_kg":0.0054,
            "displacements_m":[1e-6],"violations":[-0.9999],"feasible":true,
            "converged":true,"kkt_residual":1e-5,"publishable":false,
            "binary":{"threshold":0.5,"mass_kg":0.0054,"displacements_m":[2e-6]}});
        let start = || -> Result<MassDiagnostic, Box<dyn std::error::Error>> {
            let mut d = diagnostic()?;
            update_diagnostic(&mut d, serde_json::from_value(reference())?, 2700.)?;
            Ok(d)
        };
        let mut valid = start()?;
        update_diagnostic(&mut valid, serde_json::from_value(finish.clone())?, 2700.)?;
        let binary = valid.binary.ok_or("missing binary")?;
        assert_eq!(binary.mass.kilograms(), 0.0054);
        assert_eq!(binary.displacements[0].1.metres(), 2e-6);
        for (key, value) in [
            ("threshold", json!(0.0)),
            ("threshold", json!(1.1)),
            ("mass_kg", json!(0.00405)),
            ("displacements_m", json!([0.0002])),
            ("displacements_m", json!([])),
        ] {
            let mut event = finish.clone();
            event["binary"][key] = value;
            assert!(
                update_diagnostic(&mut start()?, serde_json::from_value(event)?, 2700.).is_err()
            );
        }
        let mut premature = finish.clone();
        premature["converged"] = json!(false);
        assert!(
            update_diagnostic(&mut start()?, serde_json::from_value(premature)?, 2700.).is_err()
        );
        let mut broken = start()?;
        broken.cells = [2, 2, 1];
        broken.regions = vec![
            CellRegion::Preserve,
            CellRegion::Void,
            CellRegion::Void,
            CellRegion::Preserve,
        ];
        let mut disconnected = finish;
        disconnected["densities"] = json!([1.0, 0.0, 0.0, 1.0]);
        disconnected["mass_kg"] = json!(0.0027);
        disconnected["binary"]["mass_kg"] = json!(0.0027);
        assert!(
            update_diagnostic(&mut broken, serde_json::from_value(disconnected)?, 2700.).is_err()
        );
        Ok(())
    }

    #[test]
    fn mass_protocol_rejects_stale_inconsistent_and_invalid_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        for (key, value) in [
            ("schema", json!(2)),
            ("revision", json!(8)),
            ("compute_backend", json!("cuda")),
            ("operation", json!("00000000-0000-0000-0000-000000000003")),
            ("densities", json!([1.0])),
            ("densities", json!([0.5, 1.0])),
            ("densities", json!([1.0, 1.00001])),
            ("densities", json!([1.0, -0.01])),
            ("mass_kg", json!(0.003)),
            ("reference_mass_kg", json!(0.003)),
            ("displacements_m", json!([])),
            ("displacements_m", json!([-1.0])),
        ] {
            let mut event = reference();
            event[key] = value;
            assert!(
                update_diagnostic(&mut diagnostic()?, serde_json::from_value(event)?, 2700.0)
                    .is_err(),
                "accepted {key}"
            );
        }
        let mut accepted = diagnostic()?;
        let mut event = reference();
        event["densities"] = json!([1.0, 1.0 + 3.0 * f64::EPSILON]);
        update_diagnostic(&mut accepted, serde_json::from_value(event)?, 2700.0)?;
        assert_eq!(accepted.densities, vec![1.0, 1.0]);
        assert!(
            update_diagnostic(&mut accepted, serde_json::from_value(reference())?, 2700.0).is_err()
        );
        Ok(())
    }

    #[test]
    fn mass_protocol_never_accepts_publication_or_false_feasibility()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut accepted = diagnostic()?;
        update_diagnostic(&mut accepted, serde_json::from_value(reference())?, 2700.0)?;
        let mut result = reference();
        result["kind"] = json!("result");
        result["displacements_m"] = json!([2e-4]);
        result["violations"] = json!([3.0]);
        result["feasible"] = json!(false);
        result["converged"] = json!(false);
        result["kkt_residual"] = json!(0.1);
        result["publishable"] = json!(false);
        for (key, value) in [
            ("publishable", json!(true)),
            ("feasible", json!(true)),
            ("converged", json!(true)),
            ("violations", json!([0.0])),
        ] {
            let mut invalid = result.clone();
            invalid[key] = value;
            assert!(
                update_diagnostic(
                    &mut accepted.clone(),
                    serde_json::from_value(invalid)?,
                    2700.0
                )
                .is_err()
            );
        }
        update_diagnostic(&mut accepted, serde_json::from_value(result)?, 2700.0)?;
        assert!(accepted.finished);
        assert!(!accepted.feasible);
        assert!(
            update_diagnostic(&mut accepted, serde_json::from_value(reference())?, 2700.0).is_err()
        );
        Ok(())
    }
}
