use robogen_domain::{Force, Length, LoadCaseId, Pressure, SourceSpan};
use robogen_topopt::{
    fem::{Beam, BeamModel, FemError, GridModel},
    linear::FaerSolver,
    simp::{CellRegion, GridLoadCase, GridRequest, SimpError, SimpOptions, optimize_grid},
    surface::GridSolidError,
};

#[test]
fn multiple_cases_optimize_reanalyse_and_export_without_combining_forces()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = BeamModel::new(Beam {
        dimensions: [120.0, 40.0, 20.0].map(Length::from_millimetres),
        cells: [12, 4, 2],
        young_modulus: Pressure::from_pascals(70e9),
        poisson_ratio: 0.3,
        end_force: [0.0, 0.0, -1.0].map(Force::from_newtons),
    })?;
    let mut opposite = adapter.boundary_conditions().clone();
    for load in &mut opposite.forces {
        load.force = load.force.map(|f| Force::from_newtons(-f.newtons()));
    }
    let request = GridRequest {
        grid: adapter.grid_definition().clone(),
        load_cases: vec![
            GridLoadCase {
                id: LoadCaseId::from_name("down"),
                span: SourceSpan::new(0, 10),
                boundary: adapter.boundary_conditions().clone(),
            },
            GridLoadCase {
                id: LoadCaseId::from_name("up"),
                span: SourceSpan::new(11, 20),
                boundary: opposite,
            },
        ],
        options: SimpOptions {
            volume_fraction: 0.85,
            penalty: 3.0,
            stiffness_floor: 1e-6,
            filter_radius: Length::from_millimetres(15.0),
            move_limit: 0.2,
            change_tolerance: 0.005,
            max_iterations: 100,
        },
        regions: (0..96)
            .map(|i| {
                if !(8..88).contains(&i) {
                    CellRegion::Preserve
                } else {
                    CellRegion::Design
                }
            })
            .collect(),
        document_revision: 93,
    };
    let mut reported = Vec::new();
    let result = optimize_grid(&request, &FaerSolver, &|| false, &mut |entry| {
        reported.push(entry.aggregate.number)
    })?;
    assert_eq!(reported, (0..result.history.len()).collect::<Vec<_>>());
    assert_eq!(result.request.document_revision, 93);
    assert_eq!(result.analyses.len(), 2);
    let mut reanalysed = Vec::new();
    let solid = result.reconstruct_solid(0.65, &FaerSolver, &|| false, &mut |case| {
        reanalysed.push(case)
    })?;
    assert_eq!(
        reanalysed,
        request.load_cases.iter().map(|c| c.id).collect::<Vec<_>>()
    );
    assert_eq!(solid.request.document_revision, 93);
    assert!(solid.within_volume_budget);
    assert_eq!(solid.surface.components, 1);
    assert!((solid.surface.enclosed_volume.cubic_metres() - 80_000.0e-9).abs() < 1e-12);
    let model = GridModel::new(request.grid.clone())?;
    let mask: Vec<_> = result.densities.iter().map(|rho| *rho >= 0.65).collect();
    for (case, published) in request.load_cases.iter().zip(&solid.analyses) {
        assert_eq!(published.id, case.id);
        assert_eq!(published.span, case.span);
        let direct = model.solve_solid_mask(&mask, &case.boundary, &FaerSolver, &|| false)?;
        assert!(
            (published.analysis.compliance.joules() / direct.compliance.joules() - 1.0).abs()
                < 1e-12
        );
        assert!(published.analysis.compliance.joules() > 0.0);
    }
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
    // Cancelling after one successful case must not publish a partial solid.
    let cancel = std::cell::Cell::new(false);
    assert!(matches!(
        result.reconstruct_solid(0.65, &FaerSolver, &|| cancel.get(), &mut |_| cancel
            .set(true)),
        Err(GridSolidError::Solve(SimpError::Fem(FemError::Cancelled)))
    ));
    eprintln!(
        "multi-case benchmark: {} updates, volume={}, {} triangles; compliances={:?}",
        result.history.len() - 1,
        solid.volume_fraction,
        solid.surface.triangles().len(),
        solid
            .analyses
            .iter()
            .map(|c| c.analysis.compliance.joules())
            .collect::<Vec<_>>()
    );
    Ok(())
}
