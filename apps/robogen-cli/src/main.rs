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
            "usage: robogen-cli <check|inspect-topology|export-stl> <source.rgn> [output.stl]"
                .into(),
        );
    };
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read {source_path}: {error}"))?;
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
            "{} topology operation(s); semantic validation only; solver unavailable; no geometry generated",
            operations.len()
        )];
        for specification in operations {
            lines.push(format!("{}: schema={}, preserved={}, voids={}, load_cases={}, constraints={}, manufacturing={}, source={}..{}",
                specification.name, specification.schema_version, specification.preserve.len(),
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
            "valid RoboGen document: {} vertices, {} triangles",
            mesh.vertices.len(),
            mesh.triangles.len()
        )),
        "export-stl" => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_inspection_does_not_authorize_check_or_export() -> Result<(), String> {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/topology_battery_support/main.rgn")
            .to_string_lossy()
            .into_owned();
        let report = run(vec!["inspect-topology".into(), source.clone()])?;
        assert!(report.contains("1 topology operation(s)"));
        assert!(report.contains("solver unavailable"));
        assert!(report.contains("preserved=2"));
        for command in ["check", "export-stl"] {
            let mut args = vec![command.to_owned(), source.clone()];
            if command == "export-stl" {
                args.push("unavailable-topology.stl".into());
            }
            assert!(run(args).is_err_and(|message| message.contains("E330")));
        }
        Ok(())
    }
}
