use robogen_domain::{Force, Length, Pressure};
use robogen_topopt::{
    fem::{Beam, BeamModel},
    linear::FaerSolver,
    simp::{BeamRequest, SimpOptions, StopReason, optimize_beam},
};

#[test]
fn manual_beam_optimization_reanalyses_final_state_and_scales_load()
-> Result<(), Box<dyn std::error::Error>> {
    let request = BeamRequest {
        beam: Beam {
            dimensions: [80.0, 40.0, 20.0].map(Length::from_millimetres),
            cells: [8, 4, 2],
            young_modulus: Pressure::from_pascals(70e9),
            poisson_ratio: 0.3,
            end_force: [0.0, 0.0, -2.0].map(Force::from_newtons),
        },
        options: SimpOptions {
            volume_fraction: 0.4,
            penalty: 3.0,
            stiffness_floor: 1e-6,
            filter_radius: Length::from_millimetres(15.0),
            move_limit: 0.2,
            change_tolerance: 1e-6,
            max_iterations: 12,
        },
        document_revision: 42,
    };
    let mut progress = Vec::new();
    let result = optimize_beam(&request, &FaerSolver, &|| false, &mut |entry| {
        progress.push(entry.number)
    })?;
    assert_eq!(result.request.document_revision, 42);
    assert_eq!(result.solver_backend, "faer-0.22.6-sparse-llt");
    assert_eq!(result.stop_reason, StopReason::IterationLimit);
    assert_eq!(progress, (0..=12).collect::<Vec<_>>());
    assert!(result.analysis.compliance.joules() < 0.98 * result.uniform_reference.joules());
    assert_eq!(result.densities.len(), 64);
    assert!((result.densities.iter().sum::<f64>() / 64.0 - 0.4).abs() < 1e-6);
    let reconstructed = result.reconstruct_surface(0.5, &|| false)?;
    assert_eq!(reconstructed.request.document_revision, 42);
    assert_eq!(reconstructed.solver_backend, result.solver_backend);
    assert_eq!(reconstructed.surface.backend, "fast-surface-nets-0.2.1");
    assert_eq!(reconstructed.surface.threshold, 0.5);
    assert_eq!(reconstructed.surface.sampling_subdivisions, 2);
    assert!(reconstructed.surface.enclosed_volume.cubic_metres() > 0.0);
    assert!(
        (reconstructed.filtered_material_volume.cubic_metres() / (0.08 * 0.04 * 0.02) - 0.4).abs()
            < 1e-6
    );
    let mesh = robogen_cad::Mesh {
        vertices: reconstructed
            .surface
            .vertices()
            .iter()
            .map(|point| robogen_cad::Vertex {
                position: point.map(|length| length.metres() as f32),
            })
            .collect(),
        triangles: reconstructed
            .surface
            .triangles()
            .iter()
            .map(|indices| robogen_cad::Triangle {
                indices: *indices,
                color: None,
            })
            .collect(),
    };
    let mut stl = Vec::new();
    robogen_export::write_binary_stl(&mesh, &mut stl)?;
    assert_eq!(stl.len(), 84 + 50 * mesh.triangles.len());
    for (record, triangle) in stl[84..].chunks_exact(50).zip(&mesh.triangles) {
        for (corner, index) in triangle.indices.iter().enumerate() {
            for axis in 0..3 {
                let offset = 12 + corner * 12 + axis * 4;
                let value = f32::from_le_bytes(record[offset..offset + 4].try_into()?);
                assert!(value.is_finite());
                assert!(
                    (value - mesh.vertices[*index as usize].position[axis] * 1000.0).abs() < 1e-5
                );
            }
        }
    }
    let mut double_load = request.beam;
    double_load.end_force[2] = Force::from_newtons(-4.0);
    let doubled =
        BeamModel::new(double_load)?.solve(&result.densities, 3.0, 1e-6, &FaerSolver, &|| false)?;
    assert!((doubled.compliance.joules() / result.analysis.compliance.joules() - 4.0).abs() < 1e-8);
    assert!(
        (doubled.mean_tip_displacement[2].metres()
            / result.analysis.mean_tip_displacement[2].metres()
            - 2.0)
            .abs()
            < 1e-8
    );
    Ok(())
}
