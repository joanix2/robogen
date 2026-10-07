use robogen_domain::{Force, Length, Pressure};
use robogen_topopt::{
    fem::Beam,
    linear::FaerSolver,
    simp::{BeamRequest, SimpOptions, optimize_beam},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        "No robot loads, stress certification, reconstructed surface or manufacturing result."
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
    Ok(())
}
