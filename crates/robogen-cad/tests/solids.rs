use robogen_cad::{CadError, CadKernel, Mesh, NativeCadKernel};
use robogen_domain::{Length, SourceSpan};
use robogen_ir::{SolidGeometry, SolidOperation};
use std::collections::BTreeMap;

fn geometry(operation: SolidOperation) -> SolidGeometry {
    SolidGeometry {
        operation,
        span: SourceSpan::default(),
    }
}

fn cuboid(size: [f64; 3]) -> SolidGeometry {
    geometry(SolidOperation::Box {
        size: size.map(Length::from_millimetres),
    })
}

#[test]
fn unresolved_topology_is_rejected_under_all_solid_wrappers() {
    use robogen_domain::{Angle, FeatureId, MaterialId, OptimizationId};
    use robogen_ir::topology::{Objective, TopologySpec, TOPOLOGY_SCHEMA_VERSION};
    let topology = geometry(SolidOperation::Topology {
        specification: Box::new(TopologySpec {
            schema_version: TOPOLOGY_SCHEMA_VERSION,
            id: OptimizationId::from_name("test"),
            name: "test".into(),
            feature: FeatureId::from_name("test"),
            span: SourceSpan::default(),
            domain: cuboid([10.0; 3]),
            preserve: vec![],
            voids: vec![],
            material: MaterialId::from_name("test"),
            load_cases: vec![],
            objective: Objective::MinimizeMass,
            constraints: vec![],
            manufacturing: vec![],
        }),
    });
    for shape in [
        topology.clone(),
        translated(topology.clone(), [1.0; 3]),
        geometry(SolidOperation::Rotate {
            angles: [Angle::from_degrees(45.0); 3],
            shape: Box::new(topology.clone()),
        }),
        geometry(SolidOperation::Color {
            rgb: [10, 20, 30],
            shape: Box::new(topology.clone()),
        }),
        geometry(SolidOperation::Compound {
            shapes: vec![cuboid([1.0; 3]), topology.clone()],
        }),
        geometry(SolidOperation::Union {
            shapes: vec![topology.clone(), cuboid([1.0; 3])],
        }),
        geometry(SolidOperation::Difference {
            base: Box::new(cuboid([1.0; 3])),
            tools: vec![topology],
        }),
    ] {
        assert!(
            matches!(NativeCadKernel.solid(&shape), Err(CadError::InvalidSolid { reason, .. }) if reason.contains("unresolved topology"))
        );
        assert_eq!(
            NativeCadKernel.solid_cancellable(&shape, &|| true),
            Err(CadError::Cancelled)
        );
    }
}

fn translated(shape: SolidGeometry, offset: [f64; 3]) -> SolidGeometry {
    geometry(SolidOperation::Translate {
        offset: offset.map(Length::from_millimetres),
        shape: Box::new(shape),
    })
}

fn volume_mm3(mesh: &Mesh) -> f64 {
    mesh.triangles
        .iter()
        .map(|triangle| {
            let [first, second, third] = triangle.indices.map(|index| {
                mesh.vertices[index as usize]
                    .position
                    .map(|value| value as f64 * 1_000.0)
            });
            (first[0] * (second[1] * third[2] - second[2] * third[1])
                + first[1] * (second[2] * third[0] - second[0] * third[2])
                + first[2] * (second[0] * third[1] - second[1] * third[0]))
                / 6.0
        })
        .sum()
}

fn assert_closed(mesh: &Mesh) {
    let mut edges = BTreeMap::<(u32, u32), (usize, i32)>::new();
    for triangle in &mesh.triangles {
        let [first, second, third] = triangle.indices;
        for (start, end) in [(first, second), (second, third), (third, first)] {
            let edge = edges.entry((start.min(end), start.max(end))).or_default();
            edge.0 += 1;
            edge.1 += if start < end { 1 } else { -1 };
        }
    }
    assert!(
        edges.values().all(|edge| *edge == (2, 0)),
        "non-manifold edges: {:?}",
        edges
            .iter()
            .filter(|(_, edge)| **edge != (2, 0))
            .collect::<Vec<_>>()
    );
}

#[test]
fn compounds_preserve_colors_and_reject_invalid_compositions() -> Result<(), CadError> {
    let blue = geometry(SolidOperation::Color {
        rgb: [0, 0, 255],
        shape: Box::new(cuboid([10.0; 3])),
    });
    let compound = geometry(SolidOperation::Compound {
        shapes: vec![blue, cuboid([10.0; 3])],
    });
    let colored = geometry(SolidOperation::Color {
        rgb: [255, 0, 0],
        shape: Box::new(compound.clone()),
    });
    let mesh = NativeCadKernel.solid(&translated(colored, [5.0, 0.0, 0.0]))?;
    assert_eq!(
        mesh.triangles
            .iter()
            .filter(|triangle| triangle.color == Some([0, 0, 255]))
            .count(),
        12
    );
    assert_eq!(
        mesh.triangles
            .iter()
            .filter(|triangle| triangle.color == Some([255, 0, 0]))
            .count(),
        12
    );
    assert!((volume_mm3(&mesh) - 2000.0).abs() < 0.01);
    assert_closed(&mesh);
    assert_eq!(
        NativeCadKernel.solid_cancellable(&compound, &|| true),
        Err(CadError::Cancelled)
    );
    for invalid in [
        geometry(SolidOperation::Compound { shapes: vec![] }),
        geometry(SolidOperation::Compound {
            shapes: vec![cuboid([1.0; 3]); 129],
        }),
        geometry(SolidOperation::Union {
            shapes: vec![compound.clone(), cuboid([1.0; 3])],
        }),
        translated(compound, [f64::NAN, 0.0, 0.0]),
    ] {
        assert!(NativeCadKernel.solid(&invalid).is_err());
    }
    Ok(())
}

#[test]
fn solid_build_can_be_cancelled_during_tessellation() {
    let calls = std::cell::Cell::new(0);
    let result = NativeCadKernel.solid_cancellable(&cuboid([10.0; 3]), &|| {
        calls.set(calls.get() + 1);
        calls.get() > 10
    });
    assert_eq!(result, Err(CadError::Cancelled));
    assert!(calls.get() > 10);
}

#[test]
fn small_round_holes_on_a_board_preserve_volume_and_closure() -> Result<(), CadError> {
    let holes = [[4.0, 4.0], [100.0, 4.0], [4.0, 86.0], [100.0, 86.0]]
        .into_iter()
        .map(|[x_mm, y_mm]| {
            translated(
                geometry(SolidOperation::Cylinder {
                    radius: Length::from_millimetres(1.5),
                    height: Length::from_millimetres(3.6),
                }),
                [x_mm, y_mm, -1.0],
            )
        })
        .collect();
    let mesh = NativeCadKernel.solid(&geometry(SolidOperation::Difference {
        base: Box::new(cuboid([104.0, 90.0, 1.6])),
        tools: holes,
    }))?;
    let hole_area = 16.0 * 1.5_f64.powi(2) * (std::f64::consts::TAU / 32.0).sin();
    let expected_volume = 104.0 * 90.0 * 1.6 - 4.0 * hole_area * 1.6;
    assert!((volume_mm3(&mesh) - expected_volume).abs() < 0.03);
    assert_closed(&mesh);
    Ok(())
}

#[test]
fn corner_box_has_outward_winding() -> Result<(), CadError> {
    let mesh = NativeCadKernel.solid(&cuboid([10.0; 3]))?;
    assert!(
        (volume_mm3(&mesh) - 1_000.0).abs() < 0.001,
        "{}",
        volume_mm3(&mesh)
    );
    assert_closed(&mesh);
    Ok(())
}

#[test]
fn through_hole_is_real_subtraction_and_watertight() -> Result<(), CadError> {
    let shape = geometry(SolidOperation::Difference {
        base: Box::new(cuboid([10.0; 3])),
        tools: vec![translated(cuboid([2.0, 4.0, 12.0]), [4.0, 3.0, -1.0])],
    });
    let mesh = NativeCadKernel.solid(&shape)?;
    assert!(
        (volume_mm3(&mesh) - 920.0).abs() < 0.001,
        "{}",
        volume_mm3(&mesh)
    );
    assert_closed(&mesh);
    Ok(())
}

#[test]
fn overlapping_union_has_no_duplicate_internal_volume() -> Result<(), CadError> {
    let mesh = NativeCadKernel.solid(&geometry(SolidOperation::Union {
        shapes: vec![
            cuboid([10.0; 3]),
            translated(cuboid([10.0; 3]), [5.0, 0.0, 0.0]),
        ],
    }))?;
    assert!(
        (volume_mm3(&mesh) - 1_500.0).abs() < 0.001,
        "{}",
        volume_mm3(&mesh)
    );
    assert_closed(&mesh);
    Ok(())
}

fn rotated(shape: SolidGeometry, degrees: [f64; 3]) -> SolidGeometry {
    geometry(SolidOperation::Rotate {
        angles: degrees.map(robogen_domain::Angle::from_degrees),
        shape: Box::new(shape),
    })
}

fn bounds_mm(mesh: &Mesh) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for vertex in &mesh.vertices {
        for (axis, value) in vertex.position.iter().enumerate() {
            let value = f64::from(*value) * 1000.0;
            bounds[0][axis] = bounds[0][axis].min(value);
            bounds[1][axis] = bounds[1][axis].max(value);
        }
    }
    bounds
}

#[test]
fn rotations_preserve_boolean_planes_winding_volume_and_units() -> Result<(), CadError> {
    // X then Y then Z maps (x,y,z) to (z,y,-x) for three quarter turns.
    let mesh = NativeCadKernel.solid(&rotated(cuboid([2.0, 3.0, 4.0]), [90.0; 3]))?;
    for (actual, expected) in bounds_mm(&mesh)
        .into_iter()
        .flatten()
        .zip([0.0, 0.0, -2.0, 4.0, 3.0, 0.0])
    {
        assert!((actual - expected).abs() < 1e-5);
    }
    assert_closed(&mesh);
    assert!((volume_mm3(&mesh) - 24.0).abs() < 0.001);
    let colored = geometry(SolidOperation::Color {
        rgb: [12, 34, 56],
        shape: Box::new(cuboid([10.0; 3])),
    });
    let cut = geometry(SolidOperation::Difference {
        base: Box::new(rotated(colored, [0.0, 0.0, 90.0])),
        tools: vec![translated(cuboid([2.0, 4.0, 12.0]), [-6.0, 3.0, -1.0])],
    });
    let mesh = NativeCadKernel.solid(&cut)?;
    assert_closed(&mesh);
    assert!((volume_mm3(&mesh) - 920.0).abs() < 0.001);
    assert!(mesh
        .triangles
        .iter()
        .any(|triangle| triangle.color == Some([12, 34, 56])));
    assert!(NativeCadKernel
        .solid(&rotated(cuboid([1.0; 3]), [f64::NAN, 0.0, 0.0]))
        .is_err());
    Ok(())
}

#[test]
fn compound_transforms_match_individual_shells_in_nesting_order() -> Result<(), CadError> {
    let blue = geometry(SolidOperation::Color {
        rgb: [0, 0, 255],
        shape: Box::new(cuboid([2.0, 3.0, 4.0])),
    });
    let second = translated(cuboid([1.0; 3]), [5.0, 0.0, 0.0]);
    let wrap = |shape| {
        translated(
            rotated(translated(shape, [7.0, -2.0, 3.0]), [32.0, -45.0, 90.0]),
            [-1.0, 8.0, 2.0],
        )
    };
    let compound = geometry(SolidOperation::Compound {
        shapes: vec![blue.clone(), second.clone()],
    });
    let actual = NativeCadKernel.solid(&wrap(compound.clone()))?;
    let mut expected = NativeCadKernel.solid(&wrap(blue))?;
    expected.append(&NativeCadKernel.solid(&wrap(second))?);
    // Triangulation can change under rotation; compare the closed volumes and bounds.
    assert_closed(&actual);
    assert_closed(&expected);
    assert!((volume_mm3(&actual) - 25.0).abs() < 0.001);
    assert!((volume_mm3(&actual) - volume_mm3(&expected)).abs() < 0.001);
    for (a, b) in bounds_mm(&actual)
        .into_iter()
        .flatten()
        .zip(bounds_mm(&expected).into_iter().flatten())
    {
        assert!((a - b).abs() < 1e-5);
    }
    assert_eq!(
        actual
            .triangles
            .iter()
            .filter(|t| t.color == Some([0, 0, 255]))
            .count(),
        12
    );
    assert!(NativeCadKernel
        .solid(&rotated(compound, [0.0, f64::INFINITY, 0.0]))
        .is_err());
    Ok(())
}
