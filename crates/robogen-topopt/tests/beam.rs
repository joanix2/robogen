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
        regions: Vec::new(),
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

#[test]
fn preserved_interfaces_produce_a_reanalysed_binary_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    use robogen_topopt::simp::CellRegion;
    use robogen_topopt::surface::{BeamSolidError, SurfaceError};
    let request = BeamRequest {
        beam: Beam {
            dimensions: [120.0, 40.0, 20.0].map(Length::from_millimetres),
            cells: [12, 4, 2],
            young_modulus: Pressure::from_pascals(70e9),
            poisson_ratio: 0.3,
            end_force: [0.0, 0.0, -1.0].map(Force::from_newtons),
        },
        options: SimpOptions {
            volume_fraction: 0.85,
            penalty: 3.0,
            stiffness_floor: 1e-6,
            filter_radius: Length::from_millimetres(15.0),
            move_limit: 0.2,
            change_tolerance: 0.005,
            max_iterations: 100,
        },
        document_revision: 43,
        regions: (0..96)
            .map(|i| {
                if !(8..88).contains(&i) {
                    CellRegion::Preserve
                } else {
                    CellRegion::Design
                }
            })
            .collect(),
    };
    let mut result = optimize_beam(&request, &FaerSolver, &|| false, &mut |_| {})?;
    let over_budget = result.reconstruct_solid(0.5, &FaerSolver, &|| false)?;
    assert!(!over_budget.within_volume_budget);
    assert!(matches!(
        result.reconstruct_solid(0.75, &FaerSolver, &|| false),
        Err(BeamSolidError::Fem(
            robogen_topopt::fem::FemError::Disconnected
        ))
    ));
    let solid = result.reconstruct_solid(0.65, &FaerSolver, &|| false)?;
    assert!(solid.within_volume_budget);
    assert_eq!(solid.request.document_revision, 43);
    assert_eq!(solid.surface.backend, "robogen-voxel-boundary-v1");
    assert_eq!(solid.solver_backend, result.solver_backend);
    assert!(solid.analysis.relative_residual < 1e-7);
    assert!(
        (solid.surface.enclosed_volume.cubic_metres() / (0.12 * 0.04 * 0.02)
            - solid.volume_fraction)
            .abs()
            < 1e-10
    );
    assert_eq!(
        solid
            .surface
            .vertices()
            .iter()
            .map(|v| v[0].metres())
            .fold(f64::INFINITY, f64::min),
        0.0
    );
    assert!(
        (solid
            .surface
            .vertices()
            .iter()
            .map(|v| v[0].metres())
            .fold(0.0, f64::max)
            - 0.12)
            .abs()
            < 1e-12
    );
    // Reanalysis must use binary material, never report the density compliance.
    let mask: Vec<_> = result.densities.iter().map(|rho| *rho >= 0.65).collect();
    let independent =
        BeamModel::new(request.beam.clone())?.solve_solid_mask(&mask, &FaerSolver, &|| false)?;
    assert!(
        (independent.compliance.joules() / solid.analysis.compliance.joules() - 1.0).abs() < 1e-12
    );
    assert!(
        (solid.analysis.compliance.joules() / result.analysis.compliance.joules() - 1.0).abs()
            > 0.01
    );
    let mesh = robogen_cad::Mesh {
        vertices: solid
            .surface
            .vertices()
            .iter()
            .map(|p| robogen_cad::Vertex {
                position: p.map(|x| x.metres() as f32),
            })
            .collect(),
        triangles: solid
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
    assert_eq!(stl.len(), 84 + mesh.triangles.len() * 50);
    assert!(matches!(
        result.reconstruct_solid(0.65, &FaerSolver, &|| true),
        Err(BeamSolidError::Surface(SurfaceError::Cancelled))
    ));
    result.densities[0] = 0.9;
    assert!(matches!(
        result.reconstruct_solid(0.65, &FaerSolver, &|| false),
        Err(BeamSolidError::Regions)
    ));
    Ok(())
}
