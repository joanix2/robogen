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
        })
    }

    fn reference() -> serde_json::Value {
        json!({"schema": 1, "operation": "00000000-0000-0000-0000-000000000001", "revision": 7,
            "compute_backend": "cpu",
            "kind": "reference", "densities": [1.0, 1.0], "mass_kg": 0.0054, "reference_mass_kg": 0.0054,
            "displacements_m": [1e-6]})
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
