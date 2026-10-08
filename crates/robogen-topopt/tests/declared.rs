use robogen_domain::{Force, Length, Mass, SourceSpan, Torque};
use robogen_ir::{
    SolidGeometry, SolidOperation,
    topology::{Constraint, ConstraintKind, LoadKind, Objective, PreservedRegion},
};
use robogen_topopt::{
    TopologyRequest,
    execution::{ExecutionProgress, ExecutionSettings, execute_topology, prepare_topology},
    linear::FaerSolver,
    simp::{CellRegion, StopReason},
};
const SOURCE: &str = include_str!("../../../examples/topology_manual/main.rgn");
fn request() -> Result<TopologyRequest, Box<dyn std::error::Error>> {
    let parsed = robogen_dsl::parse(SOURCE);
    assert!(parsed.diagnostics.is_empty());
    let module = parsed.module.ok_or("no module")?;
    let lowered = robogen_ir::lower(&module);
    assert!(lowered.diagnostics.is_empty());
    let specification = lowered
        .model
        .topology_operations()
        .first()
        .ok_or("no topology")?
        .to_owned()
        .clone();
    let material = lowered
        .model
        .materials
        .values()
        .find(|m| m.id == specification.material)
        .ok_or("no material")?
        .clone();
    Ok(TopologyRequest {
        specification,
        material,
        document_revision: 107,
    })
}
fn settings() -> ExecutionSettings {
    ExecutionSettings {
        cells: [12, 4, 2],
        filter_radius: Length::from_millimetres(12.0),
        threshold: 0.65,
        max_iterations: 100,
    }
}
fn geometry(operation: SolidOperation) -> SolidGeometry {
    SolidGeometry {
        operation,
        span: SourceSpan::new(101, 102),
    }
}

#[test]
fn interfaces_keep_total_forces_centroids_and_moments() -> Result<(), Box<dyn std::error::Error>> {
    let mut req = request()?;
    let projected = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    assert_eq!(
        projected
            .grid
            .regions
            .iter()
            .filter(|r| **r == CellRegion::Preserve)
            .count(),
        16
    );
    assert_eq!(projected.domain_cells, 96);
    let loaded = projected
        .interfaces
        .iter()
        .find(|i| i.name == "loaded")
        .ok_or("no interface")?;
    assert!((loaded.centroid[0].millimetres() - 115.0).abs() < 1e-10);
    for (case, expected) in projected.grid.load_cases.iter().zip([-1.0, 1.0]) {
        let total: f64 = case
            .boundary
            .forces
            .iter()
            .map(|f| f.force[2].newtons())
            .sum();
        assert!((total - expected).abs() < 1e-12);
        assert_eq!(case.boundary.constraints.len(), 30);
    }
    req.specification.load_cases[0].loads[0].kind =
        LoadKind::Moment([0.0, 0.0, 1.0].map(Torque::from_newton_metres));
    let moment = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    let mut force = [0.0; 3];
    let mut torque = [0.0; 3];
    for load in &moment.grid.load_cases[0].boundary.forces {
        let node = load.node.0;
        let p = [node / 15, node / 3 % 5, node % 3].map(|v| v as f64 * 0.01);
        let f = load.force.map(Force::newtons);
        for a in 0..3 {
            force[a] += f[a];
            torque[a] += p[(a + 1) % 3] * f[(a + 2) % 3] - p[(a + 2) % 3] * f[(a + 1) % 3];
        }
    }
    assert!(force.iter().all(|f| f.abs() < 1e-10));
    assert!(torque[..2].iter().all(|m| m.abs() < 1e-10));
    assert!((torque[2] - 1.0).abs() < 1e-10);
    Ok(())
}

#[test]
fn common_domain_translation_preserves_grid_and_interface_weights()
-> Result<(), Box<dyn std::error::Error>> {
    let mut req = request()?;
    let original = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    let translate = |shape: SolidGeometry| {
        geometry(SolidOperation::Translate {
            offset: [1.0, 2.0, 3.0].map(Length::from_metres),
            shape: Box::new(shape),
        })
    };
    req.specification.domain = translate(req.specification.domain.clone());
    for region in &mut req.specification.preserve {
        region.geometry = translate(region.geometry.clone());
    }
    let shifted = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    assert_eq!(shifted.origin, [1.0, 2.0, 3.0].map(Length::from_metres));
    assert_eq!(shifted.grid.regions, original.grid.regions);
    for (before, after) in original.interfaces.iter().zip(&shifted.interfaces) {
        assert_eq!(before.cell_count, after.cell_count);
        for a in 0..3 {
            assert!(
                (after.centroid[a].metres() - before.centroid[a].metres() - [1.0, 2.0, 3.0][a])
                    .abs()
                    < 1e-12
            );
        }
    }
    Ok(())
}

#[test]
fn geometry_masks_detect_small_keepouts_conflicts_and_domain_discretisation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut req = request()?;
    let hole = geometry(SolidOperation::Translate {
        offset: [60.0, 20.0, 0.0].map(Length::from_millimetres),
        shape: Box::new(geometry(SolidOperation::Cylinder {
            radius: Length::from_millimetres(1.0),
            height: Length::from_millimetres(20.0),
        })),
    });
    req.specification.voids.push(hole.clone());
    let projected = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    // A hole narrower than a cell is still excluded (eight intersecting cells).
    assert_eq!(
        projected
            .grid
            .regions
            .iter()
            .filter(|r| **r == CellRegion::Void)
            .count(),
        8
    );
    assert_eq!(projected.domain_cells, 96);
    req.specification.voids.clear();
    req.specification.domain = geometry(SolidOperation::Difference {
        base: Box::new(req.specification.domain.clone()),
        tools: vec![hole],
    });
    let cut = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    assert_eq!(cut.domain_cells, 88);
    assert!((cut.grid.options.volume_fraction - 0.85 * 88.0 / 96.0).abs() < 1e-12);
    req.specification
        .voids
        .push(req.specification.preserve[0].geometry.clone());
    let error = prepare_topology(&req, &settings(), &|| false, &mut |_| {})
        .err()
        .ok_or("missing conflict")?;
    assert_eq!(error.diagnostic.code, "E341");
    assert_eq!(error.diagnostic.span, req.specification.preserve[0].span);
    Ok(())
}

#[test]
fn unsupported_capabilities_and_cancellation_never_compute_fake_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let mut req = request()?;
    req.specification.objective = Objective::MinimizeMass;
    let e = prepare_topology(&req, &settings(), &|| false, &mut |_| {})
        .err()
        .ok_or("expected unsupported")?;
    assert_eq!(e.diagnostic.code, "E340");
    req = request()?;
    req.specification.constraints.push(Constraint {
        span: SourceSpan::new(555, 560),
        kind: ConstraintKind::SafetyFactor(2.0),
    });
    let e = prepare_topology(&req, &settings(), &|| false, &mut |_| {})
        .err()
        .ok_or("expected unsupported")?;
    assert_eq!(e.diagnostic.span, SourceSpan::new(555, 560));
    req = request()?;
    for kind in [
        robogen_ir::topology::SupportKind::Pin {
            axis: [0.0, 0.0, 1.0],
        },
        robogen_ir::topology::SupportKind::Frictionless {
            normal: [0.6, 0.8, 0.0],
        },
    ] {
        req.specification.load_cases[0].supports[0].kind = kind;
        let e = prepare_topology(&req, &settings(), &|| false, &mut |_| {})
            .err()
            .ok_or("expected unsupported support")?;
        assert_eq!(e.diagnostic.code, "E340");
    }
    req.specification.load_cases[0].supports[0].kind =
        robogen_ir::topology::SupportKind::Frictionless {
            normal: [0.0, 0.0, 1.0],
        };
    let directional = prepare_topology(&req, &settings(), &|| false, &mut |_| {})?;
    assert!(
        directional.grid.load_cases[0]
            .boundary
            .constraints
            .iter()
            .all(|c| c.fixed == [false, false, true])
    );
    req = request()?;
    let cancel = std::cell::Cell::new(false);
    let e = prepare_topology(&req, &settings(), &|| cancel.get(), &mut |_| {
        cancel.set(true)
    })
    .err()
    .ok_or("expected cancellation")?;
    assert_eq!(e.diagnostic.code, "E345");
    let mut options = settings();
    options.max_iterations = 1;
    let e = execute_topology(&req, &options, &FaerSolver, &|| false, &mut |_| {})
        .err()
        .ok_or("expected iteration limit")?;
    assert_eq!(e.diagnostic.code, "E346");
    Ok(())
}

#[test]
fn fully_prescribed_geometry_is_analysed_and_displacement_limits_are_enforced()
-> Result<(), Box<dyn std::error::Error>> {
    let mut req = request()?;
    req.specification.preserve.push(PreservedRegion {
        interface: None,
        geometry: req.specification.domain.clone(),
        span: SourceSpan::default(),
    });
    req.specification.constraints = vec![
        Constraint {
            span: SourceSpan::new(77, 88),
            kind: ConstraintKind::VolumeFraction(1.0),
        },
        Constraint {
            span: SourceSpan::new(90, 99),
            kind: ConstraintKind::MaxMass(Mass::from_kilograms(1.0)),
        },
    ];
    let mut options = settings();
    options.cells = [6, 2, 1];
    let full = execute_topology(&req, &options, &FaerSolver, &|| false, &mut |_| {})?;
    assert_eq!(full.optimization.stop_reason, StopReason::FixedRegions);
    assert_eq!(full.optimization.history.len(), 1);
    assert!((full.mass.kilograms() - 0.2592).abs() < 1e-10);
    req.specification.constraints.push(Constraint {
        span: SourceSpan::new(333, 334),
        kind: ConstraintKind::MaxDisplacement(Length::from_metres(1e-12)),
    });
    let e = execute_topology(&req, &options, &FaerSolver, &|| false, &mut |_| {})
        .err()
        .ok_or("expected displacement refusal")?;
    assert_eq!(e.diagnostic.code, "E344");
    assert_eq!(e.diagnostic.span, SourceSpan::new(333, 334));
    Ok(())
}

#[test]
fn dsl_topology_executes_reanalyses_and_exports_actual_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let req = request()?;
    let mut iterations = Vec::new();
    let mut reanalysed = Vec::new();
    let result =
        execute_topology(
            &req,
            &settings(),
            &FaerSolver,
            &|| false,
            &mut |event| match event {
                ExecutionProgress::Optimization(i) => iterations.push(i.aggregate.number),
                ExecutionProgress::Reanalysis(id) => reanalysed.push(id),
                _ => {}
            },
        )?;
    assert_eq!(result.request.document_revision, 107);
    assert_eq!(result.solid.request.document_revision, 107);
    assert_eq!(
        reanalysed,
        req.specification
            .load_cases
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        iterations,
        (0..result.optimization.history.len()).collect::<Vec<_>>()
    );
    assert!(result.solid.within_volume_budget);
    assert!(result.mass.kilograms() <= 0.230);
    assert!(
        result
            .max_displacements
            .iter()
            .all(|(_, d)| d.millimetres() <= 0.01)
    );
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
    let mut stl = Vec::new();
    robogen_export::write_binary_stl(&mesh, &mut stl)?;
    assert_eq!(stl.len(), 84 + mesh.triangles.len() * 50);
    eprintln!(
        "declared support: {} updates; mass={} kg; volume={} mm3; triangles={}; displacements_mm={:?}",
        iterations.len() - 1,
        result.mass.kilograms(),
        result.solid.surface.enclosed_volume.cubic_metres() * 1e9,
        mesh.triangles.len(),
        result
            .max_displacements
            .iter()
            .map(|(_, d)| d.millimetres())
            .collect::<Vec<_>>()
    );
    Ok(())
}
