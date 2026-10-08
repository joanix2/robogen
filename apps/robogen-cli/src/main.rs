use std::{env, fs, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    tracing_subscriber::fmt().with_target(false).init();
    match run(env::args().skip(1).collect()) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let [command, source_path, rest @ ..] = args.as_slice() else {
        return Err(
            "usage: robogen-cli <check|inspect-topology|optimize-topology|export-stl> <source.rgn> [output.stl]"
                .into(),
        );
    };
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read {source_path}: {error}"))?;
    if command == "optimize-topology" {
        return optimize_declared(&source, rest);
    }
    if command == "inspect-topology" {
        if !rest.is_empty() {
            return Err("inspect-topology only accepts a source path".into());
        }
        let model = robogen_project::compile_source(&source).map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        let operations = model.topology_operations();
        let mut lines = vec![format!(
            "{} topology operation(s); semantic validation only; preparation is not a computed result; no geometry generated",
            operations.len()
        )];
        for specification in operations {
            lines.push(format!("{}: id={}, schema={}, preserved={}, voids={}, load_cases={}, constraints={}, manufacturing={}, source={}..{}",
                specification.name, specification.id, specification.schema_version, specification.preserve.len(),
                specification.voids.len(), specification.load_cases.len(), specification.constraints.len(),
                specification.manufacturing.len(), specification.span.start, specification.span.end));
        }
        return Ok(lines.join("\n"));
    }
    let project = robogen_project::Project::from_source(source).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut mesh = robogen_cad::Mesh::default();
    for part_mesh in project.snapshot().meshes.values() {
        mesh.append(part_mesh);
    }

    match command.as_str() {
        "check" if rest.is_empty() => Ok(format!(
            "valid RoboGen document: {} vertices, {} triangles; {} topology part(s) prepared, no implicit optimization",
            mesh.vertices.len(),
            mesh.triangles.len(),
            project.snapshot().preparation.len()
        )),
        "export-stl" => {
            if !project.snapshot().preparation.is_empty() {
                return Err("E330: topology parts require explicit optimization before export; use optimize-topology".into());
            }
            let [output] = rest else {
                return Err("export-stl requires an output path".into());
            };
            let output = PathBuf::from(output);
            robogen_export::export_binary_stl(&mesh, &output).map_err(|error| error.to_string())?;
            Ok(format!("exported {}", output.display()))
        }
        _ => Err(format!("unknown command or arguments: {command}")),
    }
}

fn optimize_declared(source: &str, args: &[String]) -> Result<String, String> {
    use robogen_topopt::{
        TopologyRequest,
        execution::{ExecutionProgress, ExecutionSettings, execute_topology},
        linear::FaerSolver,
    };
    let [name, output, nx, ny, nz, filter_mm, threshold] = args else {
        return Err("usage: optimize-topology SOURCE NAME_OR_ID OUTPUT.stl NX NY NZ FILTER_MM THRESHOLD (exports this node in its local frame)".into());
    };
    let cells = [nx, ny, nz].map(|s| {
        s.parse::<usize>()
            .map_err(|_| "Invalid cell count".to_owned())
    });
    let [nx, ny, nz] = cells;
    let settings = ExecutionSettings {
        compute_backend: robogen_topopt::execution::ComputeBackend::Cpu,
        cells: [nx?, ny?, nz?],
        filter_radius: robogen_domain::Length::from_millimetres(
            filter_mm
                .parse::<f64>()
                .map_err(|_| "Invalid filter radius")?,
        ),
        threshold: threshold.parse::<f64>().map_err(|_| "Invalid threshold")?,
        max_iterations: 100,
    };
    let model = robogen_project::compile_source(source).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|d| {
                format!(
                    "{}: {} at {}..{}",
                    d.code, d.message, d.span.start, d.span.end
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let operations = model.topology_operations();
    let matches: Vec<_> = operations
        .into_iter()
        .filter(|s| s.name == *name || s.id.to_string() == *name)
        .collect();
    let [spec] = matches.as_slice() else {
        return Err("Select one topology operation by unique name or ID".into());
    };
    let issues = spec.preparation_issues();
    if !issues.is_empty() {
        return Err(format!(
            "E347: Incomplete topology preparation: {} at {}..{}",
            issues.join("; "),
            spec.span.start,
            spec.span.end
        ));
    }
    let material = model
        .materials
        .values()
        .find(|m| Some(m.id) == spec.material)
        .ok_or("Missing topology material")?;
    let result = execute_topology(
        &TopologyRequest {
            specification: (*spec).clone(),
            material: material.clone(),
            document_revision: 0,
        },
        &settings,
        &FaerSolver,
        &|| false,
        &mut |event| match event {
            ExecutionProgress::Projection { completed, total } if completed == total => eprintln!(
                "projected {total} cells; interface loads distributed over preserved voxel volumes"
            ),
            ExecutionProgress::Optimization(entry) => eprintln!(
                "iteration={} compliance_J={:.12e} volume={:.9} cases={}",
                entry.aggregate.number,
                entry.aggregate.compliance.joules(),
                entry.aggregate.volume_fraction,
                entry.cases.len()
            ),
            ExecutionProgress::Reanalysis(id) => eprintln!("reanalysed load case {id}"),
            _ => {}
        },
    )
    .map_err(|e| {
        format!(
            "{}: {} at {}..{}",
            e.diagnostic.code, e.diagnostic.message, e.diagnostic.span.start, e.diagnostic.span.end
        )
    })?;
    let mesh = robogen_cad::Mesh {
        vertices: result
            .solid
            .surface
            .vertices()
            .iter()
            .map(|p| robogen_cad::Vertex {
                position: std::array::from_fn(|a| {
                    (p[a].metres() + result.origin[a].metres()) as f32
                }),
            })
            .collect(),
        triangles: result
            .solid
            .surface
            .triangles()
            .iter()
            .map(|indices| robogen_cad::Triangle {
                indices: *indices,
                color: None,
            })
            .collect(),
    };
    robogen_export::export_binary_stl(&mesh, &PathBuf::from(output)).map_err(|e| e.to_string())?;
    Ok(format!(
        "exported topology {} in its local frame to {}: {} triangles, mass={:.9} kg, {} independently reanalysed cases; downstream document operations not applied",
        name,
        output,
        mesh.triangles.len(),
        result.mass.kilograms(),
        result.solid.analyses.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_execution_exports_only_after_validation() -> Result<(), String> {
        let source = include_str!("../../../examples/topology_manual/main.rgn")
            .replace(
                "preserve: [",
                "preserve: [box(size: [120 mm, 40 mm, 20 mm]),",
            )
            .replace("volume_fraction(0.85)", "volume_fraction(1)")
            .replace("max_mass(230 g)", "max_mass(1 kg)");
        let path = env::temp_dir().join(format!("robogen-cli-declared-{}.stl", std::process::id()));
        let args = vec![
            "manual_support".into(),
            path.to_string_lossy().into_owned(),
            "6".into(),
            "2".into(),
            "1".into(),
            "12".into(),
            "0.65".into(),
        ];
        let report = optimize_declared(&source, &args)?;
        assert!(report.contains("2 independently reanalysed cases"));
        let original = fs::read(&path).map_err(|e| e.to_string())?;
        assert!(original.len() > 84);
        let failed = optimize_declared(
            &source.replace(
                "max_displacement(0.01 mm)",
                "max_displacement(0.000000001 mm)",
            ),
            &args,
        );
        assert!(failed.is_err_and(|e| e.contains("E344")));
        assert_eq!(fs::read(&path).map_err(|e| e.to_string())?, original);
        fs::remove_file(path).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn topology_preparation_checks_but_cannot_export() -> Result<(), String> {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/topology_battery_support/main.rgn")
            .to_string_lossy()
            .into_owned();
        let report = run(vec!["inspect-topology".into(), source.clone()])?;
        assert!(report.contains("1 topology operation(s)"));
        assert!(report.contains("preparation is not a computed result"));
        assert!(report.contains("preserved=2"));
        assert!(run(vec!["check".into(), source.clone()])?.contains("1 topology part(s) prepared"));
        assert!(
            run(vec![
                "export-stl".into(),
                source,
                "unavailable-topology.stl".into()
            ])
            .is_err_and(|message| message.contains("E330"))
        );
        Ok(())
    }
}
