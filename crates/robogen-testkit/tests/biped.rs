use robogen_cad::Mesh;
use std::collections::BTreeMap;

#[test]
fn biped_source_to_closed_colored_shells_and_millimetre_stl() -> Result<(), String> {
    let project = robogen_project::Project::from_source(include_str!(
        "../../../examples/mini_biped/main.rgn"
    ))
    .map_err(|errors| format!("{errors:?}"))?;
    let mut assembly = Mesh::default();
    for mesh in project.snapshot().meshes.values() {
        let mut edges = BTreeMap::<(u32, u32), (usize, i32)>::new();
        for triangle in &mesh.triangles {
            assert!(triangle.color.is_some());
            let [a, b, c] = triangle.indices;
            for (from, to) in [(a, b), (b, c), (c, a)] {
                let edge = edges.entry((from.min(to), from.max(to))).or_default();
                edge.0 += 1;
                edge.1 += if from < to { 1 } else { -1 };
            }
        }
        assert!(edges.values().all(|edge| *edge == (2, 0)));
        assembly.append(mesh);
    }
    let mut stl = Vec::new();
    robogen_export::write_binary_stl(&assembly, &mut stl).map_err(|error| error.to_string())?;
    assert_eq!(stl.len(), 84 + assembly.triangles.len() * 50);
    let mut height_mm = f32::NEG_INFINITY;
    for triangle in stl[84..].chunks_exact(50) {
        for vertex in triangle[12..48].chunks_exact(12) {
            let z = f32::from_le_bytes(vertex[8..12].try_into().map_err(|_| "STL vertex")?);
            assert!(z.is_finite());
            height_mm = height_mm.max(z);
        }
    }
    assert!((height_mm - 367.5).abs() < 1e-3);
    Ok(())
}
