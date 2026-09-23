use robogen_cad::{CadError, CadKernel, Mesh, NativeCadKernel};
use robogen_domain::{Length, SourceSpan};
use robogen_ir::{SolidGeometry, SolidOperation};
use std::collections::BTreeMap;

fn geometry(operation: SolidOperation) -> SolidGeometry {
    SolidGeometry { operation, span: SourceSpan::default() }
}

fn cuboid(size: [f64; 3]) -> SolidGeometry {
    geometry(SolidOperation::Box { size: size.map(Length::from_millimetres) })
}

fn translated(shape: SolidGeometry, offset: [f64; 3]) -> SolidGeometry {
    geometry(SolidOperation::Translate {
        offset: offset.map(Length::from_millimetres), shape: Box::new(shape),
    })
}

fn volume_mm3(mesh: &Mesh) -> f64 {
    mesh.triangles.iter().map(|triangle| {
        let [first, second, third] = triangle.indices.map(|index|
            mesh.vertices[index as usize].position.map(|value| value as f64 * 1_000.0));
        (first[0] * (second[1] * third[2] - second[2] * third[1])
            + first[1] * (second[2] * third[0] - second[0] * third[2])
            + first[2] * (second[0] * third[1] - second[1] * third[0])) / 6.0
    }).sum()
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
    assert!(edges.values().all(|edge| *edge == (2, 0)), "non-manifold edges: {:?}",
        edges.iter().filter(|(_, edge)| **edge != (2, 0)).collect::<Vec<_>>());
}

#[test]
fn corner_box_has_outward_winding() -> Result<(), CadError> {
    let mesh = NativeCadKernel.solid(&cuboid([10.0; 3]))?;
    assert!((volume_mm3(&mesh) - 1_000.0).abs() < 0.001, "{}", volume_mm3(&mesh));
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
    assert!((volume_mm3(&mesh) - 920.0).abs() < 0.001, "{}", volume_mm3(&mesh));
    assert_closed(&mesh);
    Ok(())
}

#[test]
fn overlapping_union_has_no_duplicate_internal_volume() -> Result<(), CadError> {
    let mesh = NativeCadKernel.solid(&geometry(SolidOperation::Union {
        shapes: vec![cuboid([10.0; 3]), translated(cuboid([10.0; 3]), [5.0, 0.0, 0.0])],
    }))?;
    assert!((volume_mm3(&mesh) - 1_500.0).abs() < 0.001, "{}", volume_mm3(&mesh));
    assert_closed(&mesh);
    Ok(())
}