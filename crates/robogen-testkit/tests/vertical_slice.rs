use robogen_cad::Mesh;
use robogen_render::{Camera, PreviewMesh, RobotPreviewRenderer, ViewportSize};

#[test]
fn constrained_bracket_reaches_render_and_stl() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../../../examples/constrained_bracket/main.rgn");
    let project = robogen_project::Project::from_source(source)
        .map_err(|diagnostics| std::io::Error::other(format!("{diagnostics:?}")))?;
    let mut mesh = Mesh::default();
    for part_mesh in project.snapshot().meshes.values() {
        mesh.append(part_mesh);
    }
    assert_eq!(mesh.triangles.len(), 12);

    let preview = PreviewMesh::new(
        mesh.vertices.iter().map(|vertex| vertex.position).collect(),
        mesh.triangles
            .iter()
            .map(|triangle| triangle.indices)
            .collect(),
    );
    let frame = RobotPreviewRenderer.render_mesh(
        &Camera::default(),
        ViewportSize::new(800.0, 500.0),
        &preview,
    );
    assert_eq!(frame.triangles.len(), 12);

    let mut stl = Vec::new();
    robogen_export::write_binary_stl(&mesh, &mut stl)?;
    assert_eq!(stl.len(), 84 + 12 * 50);
    Ok(())
}

#[test]
fn source_edit_undo_redo_reaches_mesh_dimensions_and_stl() -> Result<(), Box<dyn std::error::Error>>
{
    let source = include_str!("../../../examples/constrained_bracket/main.rgn");
    let mut document = robogen_project::SourceDocument::new(source);
    assert_document_dimensions_and_stl(&document, 40.0)?;

    let edited = source.replace("WIDTH = 40 mm", "WIDTH = 70 mm");
    assert!(document.replace_source(edited.clone()));
    assert_eq!(document.source(), edited);
    assert_document_dimensions_and_stl(&document, 70.0)?;

    assert!(document.undo());
    assert_eq!(document.source(), source);
    assert_document_dimensions_and_stl(&document, 40.0)?;

    assert!(document.redo());
    assert_eq!(document.source(), edited);
    assert_document_dimensions_and_stl(&document, 70.0)?;
    Ok(())
}

fn assert_document_dimensions_and_stl(
    document: &robogen_project::SourceDocument,
    width_mm: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(document.is_valid());
    assert!(!document.has_errors());
    assert!(document.diagnostics().is_empty());
    let project = document
        .project()
        .ok_or_else(|| std::io::Error::other("missing valid project"))?;
    assert_eq!(project.source(), document.source());
    let mut mesh = Mesh::default();
    for part_mesh in project.snapshot().meshes.values() {
        mesh.append(part_mesh);
    }
    assert_eq!(mesh.triangles.len(), 12);
    let dimensions_mm = [width_mm, 50.0, 8.0];
    for (axis, expected_mm) in dimensions_mm.into_iter().enumerate() {
        let minimum = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[axis])
            .fold(f32::INFINITY, f32::min);
        let maximum = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[axis])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(minimum.abs() < 1e-6);
        assert!((maximum - minimum - expected_mm / 1_000.0).abs() < 1e-6);
    }

    let mut stl = Vec::new();
    robogen_export::write_binary_stl(&mesh, &mut stl)?;
    assert_eq!(stl.len(), 84 + 12 * 50);
    assert_eq!(u32::from_le_bytes(stl[80..84].try_into()?), 12);
    let mut minimum_mm = [f32::INFINITY; 3];
    let mut maximum_mm = [f32::NEG_INFINITY; 3];
    for triangle in stl[84..].chunks_exact(50) {
        for vertex in triangle[12..48].chunks_exact(12) {
            for (axis, bytes) in vertex.chunks_exact(4).enumerate() {
                let coordinate = f32::from_le_bytes(bytes.try_into()?);
                assert!(coordinate.is_finite());
                minimum_mm[axis] = minimum_mm[axis].min(coordinate);
                maximum_mm[axis] = maximum_mm[axis].max(coordinate);
            }
        }
    }
    for (axis, expected_mm) in dimensions_mm.into_iter().enumerate() {
        assert!(minimum_mm[axis].abs() < 1e-4);
        assert!((maximum_mm[axis] - minimum_mm[axis] - expected_mm).abs() < 1e-4);
    }
    Ok(())
}
