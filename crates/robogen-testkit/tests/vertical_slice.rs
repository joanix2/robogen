use robogen_cad::Mesh;
use robogen_render::{Camera, PreviewMesh, RobotPreviewRenderer, ViewportSize};

#[test]
fn battery_pack_colors_closed_shells_and_stl() -> Result<(), Box<dyn std::error::Error>> {
    let project = robogen_project::Project::from_source(include_str!(
        "../../../examples/li_ion_battery/main.rgn"
    ))
    .map_err(|errors| format!("{errors:?}"))?;
    let mesh = project
        .snapshot()
        .meshes
        .values()
        .next()
        .ok_or("missing battery")?;
    let colors: std::collections::BTreeSet<_> = mesh
        .triangles
        .iter()
        .map(|triangle| triangle.color)
        .collect();
    assert_eq!(
        colors,
        [
            [245, 207, 35],
            [212, 171, 25],
            [203, 38, 40],
            [28, 29, 32],
            [190, 35, 40]
        ]
        .into_iter()
        .map(Some)
        .collect()
    );
    let mut edges = std::collections::BTreeMap::<(u32, u32), (usize, i32)>::new();
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
        "closed oriented shells required"
    );
    let mut stl = Vec::new();
    robogen_export::write_binary_stl(mesh, &mut stl)?;
    assert_eq!(stl.len(), 84 + mesh.triangles.len() * 50);
    let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
    for triangle in stl[84..].chunks_exact(50) {
        let normal: Vec<_> = triangle[..12]
            .chunks_exact(4)
            .map(|bytes| <[u8; 4]>::try_from(bytes).map(f32::from_le_bytes))
            .collect::<Result<_, _>>()?;
        assert!(normal.iter().map(|value| value * value).sum::<f32>() > 0.99);
        for vertex in triangle[12..48].chunks_exact(12) {
            for (axis, bytes) in vertex.chunks_exact(4).enumerate() {
                let coordinate = f32::from_le_bytes(bytes.try_into()?);
                assert!(coordinate.is_finite());
                bounds[0][axis] = bounds[0][axis].min(coordinate);
                bounds[1][axis] = bounds[1][axis].max(coordinate);
            }
        }
    }
    for (axis, (minimum, maximum)) in [(-9.0, 9.0), (-9.0, 9.0), (0.0, 98.0)]
        .into_iter()
        .enumerate()
    {
        assert!((bounds[0][axis] - minimum).abs() < 1e-4);
        assert!((bounds[1][axis] - maximum).abs() < 1e-4);
    }
    Ok(())
}

#[test]
fn jetson_generator_preserves_the_existing_board_meshes() -> Result<(), Box<dyn std::error::Error>>
{
    let source = include_str!("../../../examples/compute_board/main.rgn");
    let original =
        robogen_project::Project::from_source(source).map_err(|errors| format!("{errors:?}"))?;
    let mut document = robogen_project::SourceDocument::new("module jetson;");
    document
        .instantiate_generator(robogen_project::LibraryGenerator::JetsonNanoSuper)
        .map_err(|errors| format!("{errors:?}"))?;
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    let generated = document.project().ok_or("missing generated project")?;
    let mut expected = Mesh::default();
    for mesh in original.snapshot().meshes.values() {
        expected.append(mesh);
    }
    let actual = generated
        .snapshot()
        .meshes
        .values()
        .next()
        .ok_or("missing generated mesh")?;
    assert_eq!(actual.triangles.len(), expected.triangles.len());
    assert_eq!(actual.vertices.len(), expected.vertices.len());
    let positions = |mesh: &Mesh| {
        let mut positions: Vec<_> = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.map(f32::to_bits))
            .collect();
        positions.sort();
        positions
    };
    assert_eq!(positions(actual), positions(&expected));
    Ok(())
}

#[test]
fn compound_preserves_separate_bodies_and_translation() -> Result<(), Box<dyn std::error::Error>> {
    let source = "module compound_test;
material Plastic { density: 1000 kg/m3; young: 1 GPa; poisson: 0.3; yield_strength: 10 MPa; }
component pair() -> Solid { return compound(box([10 mm, 10 mm, 10 mm]), box([10 mm, 10 mm, 10 mm])); }
part Pair { material: Plastic; body = translate([20 mm, 0 mm, 0 mm], color(10, 20, 30, pair())); }";
    let project = robogen_project::Project::from_source(source)
        .map_err(|diagnostics| std::io::Error::other(format!("{diagnostics:?}")))?;
    let mesh = project
        .snapshot()
        .meshes
        .values()
        .next()
        .ok_or("missing pair")?;
    assert_eq!(mesh.triangles.len(), 24);
    assert!(
        mesh.triangles
            .iter()
            .all(|triangle| triangle.color == Some([10, 20, 30]))
    );
    assert!(
        mesh.vertices
            .iter()
            .all(|vertex| vertex.position[0] >= 0.02 - 1e-6)
    );
    Ok(())
}

#[test]
fn camera_reference_holes_bounds_colors_and_stl() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../../../examples/raspberry_pi_camera/main.rgn");
    for diameter in [2.0_f64, 3.0] {
        let source = source.replace(
            "MOUNT_DIAMETER = 2 mm",
            &format!("MOUNT_DIAMETER = {diameter} mm"),
        );
        let project = robogen_project::Project::from_source(source)
            .map_err(|diagnostics| std::io::Error::other(format!("{diagnostics:?}")))?;
        assert_eq!(project.snapshot().model.parts.len(), 1);
        let mesh = project
            .snapshot()
            .meshes
            .values()
            .next()
            .ok_or("missing camera")?;
        let colors: std::collections::BTreeSet<_> = mesh
            .triangles
            .iter()
            .map(|triangle| triangle.color)
            .collect();
        assert_eq!(
            colors,
            [
                Some([35, 125, 72]),
                Some([34, 37, 42]),
                Some([54, 115, 153]),
                Some([184, 112, 48]),
                Some([205, 209, 212]),
                Some([225, 229, 232])
            ]
            .into_iter()
            .collect()
        );
        let mut edges = std::collections::BTreeMap::<(u32, u32), (usize, i32)>::new();
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
            "camera must be closed and oriented"
        );
        for center in [[9.35, 2.0], [21.85, 2.0], [9.35, 23.0], [21.85, 23.0]] {
            for triangle in &mesh.triangles {
                let [first, second, third] = triangle.indices.map(|index| {
                    mesh.vertices[index as usize]
                        .position
                        .map(|value| f64::from(value) * 1000.0)
                });
                let determinant = (second[1] - third[1]) * (first[0] - third[0])
                    + (third[0] - second[0]) * (first[1] - third[1]);
                if determinant.abs() < 1e-8 {
                    continue;
                }
                let first_weight = ((second[1] - third[1]) * (center[0] - third[0])
                    + (third[0] - second[0]) * (center[1] - third[1]))
                    / determinant;
                let second_weight = ((third[1] - first[1]) * (center[0] - third[0])
                    + (first[0] - third[0]) * (center[1] - third[1]))
                    / determinant;
                assert!(
                    [
                        first_weight,
                        second_weight,
                        1.0 - first_weight - second_weight
                    ]
                    .iter()
                    .any(|weight| *weight < -1e-6),
                    "camera hole capped at {center:?}"
                );
            }
            let rim = mesh
                .vertices
                .iter()
                .filter(|vertex| (f64::from(vertex.position[2]) * 1000.0 - 0.95).abs() < 1e-4)
                .map(|vertex| {
                    ((f64::from(vertex.position[0]) * 1000.0 - center[0]).powi(2)
                        + (f64::from(vertex.position[1]) * 1000.0 - center[1]).powi(2))
                    .sqrt()
                })
                .fold(f64::INFINITY, f64::min);
            assert!(
                (rim - diameter / 2.0).abs() < 0.01,
                "hole radius {rim}, diameter {diameter}"
            );
        }
        let mut stl = Vec::new();
        robogen_export::write_binary_stl(mesh, &mut stl)?;
        assert_eq!(stl.len(), 84 + mesh.triangles.len() * 50);
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for triangle in stl[84..].chunks_exact(50) {
            for vertex in triangle[12..48].chunks_exact(12) {
                for (axis, bytes) in vertex.chunks_exact(4).enumerate() {
                    let coordinate = f32::from_le_bytes(bytes.try_into()?);
                    assert!(coordinate.is_finite());
                    minimum[axis] = minimum[axis].min(coordinate);
                    maximum[axis] = maximum[axis].max(coordinate);
                }
            }
        }
        for (axis, (lower, upper)) in [(-5.0, 23.9), (0.0, 25.0), (-2.8, 6.15)]
            .into_iter()
            .enumerate()
        {
            assert!(
                (minimum[axis] - lower).abs() < 1e-4,
                "axis {axis}: {}",
                minimum[axis]
            );
            assert!(
                (maximum[axis] - upper).abs() < 1e-4,
                "axis {axis}: {}",
                maximum[axis]
            );
        }
    }
    Ok(())
}

#[test]
fn compute_board_fan_occludes_pcb_in_raster_preview() -> Result<(), Box<dyn std::error::Error>> {
    let project = robogen_project::Project::from_source(include_str!(
        "../../../examples/compute_board/main.rgn"
    ))
    .map_err(|diagnostics| std::io::Error::other(format!("{diagnostics:?}")))?;
    let mut mesh = Mesh::default();
    for part_mesh in project.snapshot().meshes.values() {
        mesh.append(part_mesh);
    }
    let mut preview = PreviewMesh::new(
        mesh.vertices
            .iter()
            .map(|vertex| {
                [
                    vertex.position[0] * 30.0,
                    vertex.position[2] * 30.0,
                    -vertex.position[1] * 30.0,
                ]
            })
            .collect(),
        mesh.triangles
            .iter()
            .map(|triangle| triangle.indices)
            .collect(),
    );
    preview.colors = mesh
        .triangles
        .iter()
        .map(|triangle| {
            triangle
                .color
                .map(|[red, green, blue]| robogen_render::Rgba8::rgb(red, green, blue))
        })
        .collect();
    for (width, height) in [(600, 400), (320, 240)] {
        let camera = Camera {
            target: robogen_render::Vec3::new(0.063 * 30.0, 0.037 * 30.0, -0.055 * 30.0),
            yaw: 0.7,
            pitch: 1.2,
            distance: 5.0,
            ..Camera::default()
        };
        let size = ViewportSize::new(width as f32, height as f32);
        let image = RobotPreviewRenderer.rasterize_mesh(&camera, size, &preview);
        let pixel = image.pixels[(height as f32 * 0.52) as usize * width + width / 2];
        assert_eq!(pixel.a, 255);
        assert!(
            pixel.r >= 30 && pixel.b > pixel.g && pixel.g > pixel.r,
            "fan hub must cover PCB: {pixel:?}"
        );
        let covered = image.pixels.iter().filter(|pixel| pixel.a == 255).count();
        assert!(covered > width * height / 20);
        let mut reversed = preview.clone();
        reversed.triangles.reverse();
        reversed.colors.reverse();
        let other = RobotPreviewRenderer.rasterize_mesh(&camera, size, &reversed);
        assert_eq!(
            pixel,
            other.pixels[(height as f32 * 0.52) as usize * width + width / 2]
        );
    }
    Ok(())
}

#[test]
fn compute_board_matches_reference_envelope_and_exports() -> Result<(), Box<dyn std::error::Error>>
{
    let source = include_str!("../../../examples/compute_board/main.rgn");
    for height in [37.0_f32, 42.0] {
        let source = source.replace(
            "TOTAL_HEIGHT = 37 mm",
            &format!("TOTAL_HEIGHT = {height} mm"),
        );
        let project = robogen_project::Project::from_source(source)
            .map_err(|diagnostics| std::io::Error::other(format!("{diagnostics:?}")))?;
        assert_eq!(project.snapshot().model.parts.len(), 7);
        let mut mesh = Mesh::default();
        for (part_id, part_mesh) in &project.snapshot().meshes {
            let mut edges = std::collections::BTreeMap::<(u32, u32), (usize, i32)>::new();
            for triangle in &part_mesh.triangles {
                assert!(triangle.color.is_some());
                let [first, second, third] = triangle.indices;
                for (start, end) in [(first, second), (second, third), (third, first)] {
                    let edge = edges.entry((start.min(end), start.max(end))).or_default();
                    edge.0 += 1;
                    edge.1 += if start < end { 1 } else { -1 };
                }
            }
            let part_name = project
                .snapshot()
                .model
                .parts
                .values()
                .find(|part| part.id == *part_id)
                .map(|part| &part.name);
            let open_edges: Vec<_> = edges
                .iter()
                .filter(|(_, edge)| **edge != (2, 0))
                .take(8)
                .map(|((start, end), count)| {
                    (
                        part_mesh.vertices[*start as usize].position,
                        part_mesh.vertices[*end as usize].position,
                        count,
                    )
                })
                .collect();
            assert!(
                open_edges.is_empty(),
                "open body {part_name:?}: {open_edges:?}"
            );
            mesh.append(part_mesh);
        }
        let mut stl = Vec::new();
        robogen_export::write_binary_stl(&mesh, &mut stl)?;
        assert_eq!(stl.len(), 84 + mesh.triangles.len() * 50);
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for triangle in stl[84..].chunks_exact(50) {
            for vertex in triangle[12..48].chunks_exact(12) {
                for (axis, bytes) in vertex.chunks_exact(4).enumerate() {
                    let coordinate = f32::from_le_bytes(bytes.try_into()?);
                    assert!(coordinate.is_finite());
                    minimum[axis] = minimum[axis].min(coordinate);
                    maximum[axis] = maximum[axis].max(coordinate);
                }
            }
        }
        for (axis, expected) in [104.0, 90.0, height].into_iter().enumerate() {
            assert!(minimum[axis].abs() < 1e-4);
            assert!(
                (maximum[axis] - expected).abs() < 1e-4,
                "axis {axis}: {}",
                maximum[axis]
            );
        }
    }
    Ok(())
}

#[test]
fn servo_reference_is_closed_and_reaches_stl() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../../../examples/servo/main.rgn");
    let started = std::time::Instant::now();
    let project = robogen_project::Project::from_source(source)
        .map_err(|diagnostics| std::io::Error::other(format!("{diagnostics:?}")))?;
    let mesh = project
        .snapshot()
        .meshes
        .values()
        .next()
        .ok_or("missing servo")?;
    eprintln!(
        "servo build: {:?}, {} triangles",
        started.elapsed(),
        mesh.triangles.len()
    );
    let colors: std::collections::BTreeSet<_> = mesh
        .triangles
        .iter()
        .map(|triangle| triangle.color)
        .collect();
    assert_eq!(
        colors,
        [
            Some([48, 108, 224]),
            Some([210, 215, 222]),
            Some([25, 28, 32])
        ]
        .into_iter()
        .collect()
    );
    for (axis, (minimum, maximum)) in [(0.0, 11.8), (-4.7, 27.2), (0.0, 29.9)]
        .into_iter()
        .enumerate()
    {
        let actual_min = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[axis] * 1_000.0)
            .fold(f32::INFINITY, f32::min);
        let actual_max = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[axis] * 1_000.0)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            (actual_min - minimum).abs() < 1e-4,
            "axis {axis}: {actual_min}"
        );
        assert!(
            (actual_max - maximum).abs() < 1e-4,
            "axis {axis}: {actual_max}"
        );
    }
    let mut edges = std::collections::BTreeMap::<(u32, u32), (usize, i32)>::new();
    for triangle in &mesh.triangles {
        let [first, second, third] = triangle.indices;
        for (start, end) in [(first, second), (second, third), (third, first)] {
            let edge = edges.entry((start.min(end), start.max(end))).or_default();
            edge.0 += 1;
            edge.1 += if start < end { 1 } else { -1 };
        }
    }
    let bad_edges: Vec<_> = edges.values().filter(|edge| **edge != (2, 0)).collect();
    assert!(
        bad_edges.is_empty(),
        "{} non-manifold edges",
        bad_edges.len()
    );
    for center in [[5.9_f64, -2.4], [5.9, 24.9]] {
        for triangle in &mesh.triangles {
            let [first, second, third] = triangle.indices.map(|index| {
                mesh.vertices[index as usize]
                    .position
                    .map(|value| f64::from(value) * 1_000.0)
            });
            let determinant = (second[1] - third[1]) * (first[0] - third[0])
                + (third[0] - second[0]) * (first[1] - third[1]);
            if determinant.abs() < 1e-8 {
                continue;
            }
            let first_weight = ((second[1] - third[1]) * (center[0] - third[0])
                + (third[0] - second[0]) * (center[1] - third[1]))
                / determinant;
            let second_weight = ((third[1] - first[1]) * (center[0] - third[0])
                + (first[0] - third[0]) * (center[1] - third[1]))
                / determinant;
            let third_weight = 1.0 - first_weight - second_weight;
            let height =
                first_weight * first[2] + second_weight * second[2] + third_weight * third[2];
            if (15.9 - 1e-4..=18.4 + 1e-4).contains(&height) {
                assert!(
                    [first_weight, second_weight, third_weight]
                        .iter()
                        .any(|weight| *weight < -1e-6),
                    "mounting tab hole is capped at {center:?}, z={height}"
                );
            }
        }
    }
    let mut stl = Vec::new();
    robogen_export::write_binary_stl(mesh, &mut stl)?;
    assert_eq!(stl.len(), 84 + mesh.triangles.len() * 50);
    Ok(())
}

#[test]
fn servo_clearance_history_updates_mesh_and_stl() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../../../examples/servo/main.rgn");
    let mut document = robogen_project::SourceDocument::new(source);
    let original = servo_stl(&document, 0.0)?;
    let original_id = document
        .project()
        .ok_or("missing project")?
        .snapshot()
        .model
        .parts["Servo"]
        .id;
    document.replace_source(source.replace("JEU = 0 mm", "JEU = 0.2 mm"));
    let changed = servo_stl(&document, 0.2)?;
    assert_ne!(original, changed);
    assert_eq!(
        document
            .project()
            .ok_or("missing project")?
            .snapshot()
            .model
            .parts["Servo"]
            .id,
        original_id
    );
    document.undo();
    assert_eq!(servo_stl(&document, 0.0)?, original);
    document.redo();
    assert_eq!(servo_stl(&document, 0.2)?, changed);
    document.replace_source(source.replace("JEU = 0 mm", "JEU = 0 deg"));
    assert!(!document.is_valid());
    assert!(!document.diagnostics().is_empty());
    Ok(())
}

fn servo_stl(
    document: &robogen_project::SourceDocument,
    clearance: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    let mesh = document
        .project()
        .ok_or("missing project")?
        .snapshot()
        .meshes
        .values()
        .next()
        .ok_or("missing mesh")?;
    let mut stl = Vec::new();
    robogen_export::write_binary_stl(mesh, &mut stl)?;
    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    for triangle in stl[84..].chunks_exact(50) {
        for vertex in triangle[12..48].chunks_exact(12) {
            let coordinate = f32::from_le_bytes(vertex[..4].try_into()?);
            minimum = minimum.min(coordinate);
            maximum = maximum.max(coordinate);
        }
    }
    assert!((minimum + clearance).abs() < 1e-4);
    assert!((maximum - 11.8 - clearance).abs() < 1e-4);
    Ok(stl)
}

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
