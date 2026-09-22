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
        return Err("usage: robogen-cli <check|export-stl> <source.rgn> [output.stl]".into());
    };
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read {source_path}: {error}"))?;
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
