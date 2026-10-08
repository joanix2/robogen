use robogen_domain::{Force, Length, Pressure};
use robogen_topopt::{
    fem::Beam,
    linear::FaerSolver,
    simp::{BeamRequest, SimpOptions, optimize_beam},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let output = match arguments.next() {
        None => None,
        Some(flag) if flag == "--surface-stl" => {
            let path = arguments.next().ok_or("--surface-stl requires a path")?;
            if arguments.next().is_some() {
                return Err("usage: beam [--surface-stl PATH]".into());
            }
            Some(std::path::PathBuf::from(path))
        }
        Some(_) => return Err("usage: beam [--surface-stl PATH]".into()),
    };
    let request = BeamRequest {
        beam: Beam {
            dimensions: [120.0, 40.0, 20.0].map(Length::from_millimetres),
            cells: [12, 4, 2],
            young_modulus: Pressure::from_pascals(70e9),
            poisson_ratio: 0.3,
            end_force: [0.0, 0.0, -1.0].map(Force::from_newtons),
        },
        options: SimpOptions {
            volume_fraction: 0.35,
            penalty: 3.0,
            stiffness_floor: 1e-6,
            filter_radius: Length::from_millimetres(15.0),
            move_limit: 0.2,
            change_tolerance: 0.005,
            max_iterations: 100,
        },
        document_revision: 0,
    };
    eprintln!("M7 standalone cantilever benchmark; faer 0.22.6 sparse LLT; revision 0");
    eprintln!("120x40x20 mm; 12x4x2 Hex8; E=70 GPa; nu=0.3; x=0 clamped; tip force [0,0,-1] N");
    eprintln!(
        "Linear isotropic elasticity; SIMP p=3, Emin/E=1e-6; density filter 15 mm; volume=0.35"
    );
    eprintln!(
        "No robot loads, stress certification or manufacturing result; reconstructed surface is visualization only."
    );
    println!("iteration,compliance_J,volume_fraction,max_design_change,relative_residual");
    let started = std::time::Instant::now();
    let result = optimize_beam(&request, &FaerSolver, &|| false, &mut |entry| {
        println!(
            "{},{:.12e},{:.9},{:.9},{:.6e}",
            entry.number,
            entry.compliance.joules(),
            entry.volume_fraction,
            entry.max_design_change,
            entry.relative_residual
        );
    })?;
    eprintln!(
        "stop={:?}; elapsed_s={:.3}; uniform_compliance_J={:.12e}; final_compliance_J={:.12e}; ratio={:.6}",
        result.stop_reason,
        started.elapsed().as_secs_f64(),
        result.uniform_reference.joules(),
        result.analysis.compliance.joules(),
        result.analysis.compliance.joules() / result.uniform_reference.joules()
    );
    eprintln!(
        "mean_tip_displacement_mm={:?}",
        result
            .analysis
            .mean_tip_displacement
            .map(Length::millimetres)
    );
    let reconstruction = result.reconstruct_surface(0.5, &|| false)?;
    let surface = &reconstruction.surface;
    eprintln!(
        "surface_backend={}; threshold={}; vertices={}; triangles={}; surface_components={}; enclosed_volume_mm3={:.6}; filtered_material_volume_mm3={:.6}; revision={}",
        surface.backend,
        surface.threshold,
        surface.vertices().len(),
        surface.triangles().len(),
        surface.components,
        surface.enclosed_volume.cubic_metres() * 1e9,
        reconstruction.filtered_material_volume.cubic_metres() * 1e9,
        reconstruction.request.document_revision
    );
    if let Some(path) = output {
        let mesh = robogen_cad::Mesh {
            vertices: surface
                .vertices()
                .iter()
                .map(|point| robogen_cad::Vertex {
                    position: point.map(|length| length.metres() as f32),
                })
                .collect(),
            triangles: surface
                .triangles()
                .iter()
                .map(|indices| robogen_cad::Triangle {
                    indices: *indices,
                    color: None,
                })
                .collect(),
        };
        robogen_export::export_binary_stl(&mesh, &path)?;
        eprintln!(
            "Approximate visualization STL (millimetres), NOT mechanically revalidated: {}",
            path.display()
        );
    }
    Ok(())
}
