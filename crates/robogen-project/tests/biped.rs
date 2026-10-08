use robogen_domain::{Length, Quantity};
use robogen_project::{compile_source, LibraryGenerator, Project};

const SOURCE: &str = include_str!("../../../examples/mini_biped/main.rgn");

fn bounds(project: &Project, name: &str) -> [[f64; 3]; 2] {
    let mesh = &project.snapshot().meshes[&project.snapshot().model.parts[name].id];
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for vertex in &mesh.vertices {
        for (axis, value) in vertex.position.iter().enumerate() {
            let mm = f64::from(*value) * 1000.0;
            bounds[0][axis] = bounds[0][axis].min(mm);
            bounds[1][axis] = bounds[1][axis].max(mm);
        }
    }
    bounds
}

#[test]
fn biped_reuses_library_geometry_places_components_and_rebuilds_shared_dimensions(
) -> Result<(), String> {
    for generator in LibraryGenerator::ALL {
        let definitions = generator
            .definitions()
            .map_err(|errors| format!("{errors:?}"))?;
        assert!(
            SOURCE.contains(&definitions),
            "library drift: {}",
            generator.name()
        );
    }
    let mut project = Project::from_source(SOURCE).map_err(|errors| format!("{errors:?}"))?;
    assert_eq!(project.snapshot().model.parts.len(), 28);
    assert_eq!(project.taxonomy_instances().len(), 28);
    assert_eq!(project.snapshot().meshes.len(), 17);
    assert_eq!(project.snapshot().preparation.len(), 11);
    assert!(project.snapshot().topology.is_empty());
    let names: Vec<_> = project
        .snapshot()
        .model
        .parts
        .values()
        .filter(|part| project.snapshot().meshes.contains_key(&part.id))
        .map(|part| part.name.clone())
        .collect();
    // Conservative AABB separation verifies this initial pose only, not motion.
    for (i, name) in names.iter().enumerate() {
        let a = bounds(&project, name);
        for other in &names[i + 1..] {
            let b = bounds(&project, other);
            assert!(
                (0..3).any(|axis| a[1][axis] < b[0][axis] || b[1][axis] < a[0][axis]),
                "overlapping component envelopes: {name} {other}: {a:?} {b:?}"
            );
        }
    }
    for joint in [
        "HipPitch",
        "KneePitch",
        "AnklePitch",
        "ShoulderPitch",
        "ElbowPitch",
    ] {
        let left = bounds(&project, &format!("Left{joint}"));
        let right = bounds(&project, &format!("Right{joint}"));
        assert!((left[0][0] + right[1][0]).abs() < 1e-4);
        assert!((left[1][0] + right[0][0]).abs() < 1e-4);
        assert!((left[1][0] - left[0][0] - 29.9).abs() < 1e-4);
    }
    let board = bounds(&project, "TorsoBoard");
    for (axis, expected) in [104.0, 37.0, 90.0].into_iter().enumerate() {
        assert!((board[1][axis] - board[0][axis] - expected).abs() < 1e-4);
    }
    let battery = bounds(&project, "PelvisBattery");
    assert!(battery[1][2] < board[0][2]);
    assert!((battery[1][0] - battery[0][0] - 98.0).abs() < 1e-4);
    let camera = bounds(&project, "HeadCamera");
    assert!(camera[0][2] > board[1][2]);
    assert!((camera[0][1] + 11.15).abs() < 1e-4);
    let original = project.snapshot().clone();
    project
        .set_parameter(
            "THIGH_LENGTH",
            Quantity::Length(Length::from_millimetres(75.0)),
        )
        .map_err(|error| format!("{error:?}"))?;
    project.rebuild().map_err(|errors| format!("{errors:?}"))?;
    for name in &names {
        let id = original.model.parts[name].id;
        assert_eq!(project.snapshot().model.parts[name].id, id);
        let actual = &project.snapshot().meshes[&id];
        let previous = &original.meshes[&id];
        let shift_mm = if name.contains("Knee") || name.contains("Ankle") {
            0.0
        } else {
            10.0
        };
        assert_eq!(actual.vertices.len(), previous.vertices.len());
        for (after, before) in actual.vertices.iter().zip(&previous.vertices) {
            for axis in 0..3 {
                let expected = if axis == 2 { shift_mm } else { 0.0 };
                assert!(
                    (f64::from(after.position[axis] - before.position[axis]) * 1000.0 - expected)
                        .abs()
                        < 1e-4
                );
            }
        }
    }
    let invalid = SOURCE.replace("SHIN_LENGTH = 65 mm", "SHIN_LENGTH = 65 deg");
    let Err(diagnostics) = compile_source(&invalid) else {
        return Err("incompatible length must fail".into());
    };
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "E104" && d.span.end > d.span.start));
    Ok(())
}

#[test]
fn biped_supports_prepare_in_place_but_cannot_run_with_missing_physics() -> Result<(), String> {
    let mut project = Project::from_source(SOURCE).map_err(|errors| format!("{errors:?}"))?;
    for part in project
        .snapshot()
        .model
        .parts
        .values()
        .filter(|part| part.name.ends_with("Support"))
    {
        let layers = &project.snapshot().preparation[&part.id];
        assert!(
            layers.iter().all(|mesh| !mesh.triangles.is_empty()),
            "{}",
            part.name
        );
        for axis in 0..3 {
            let lower = layers[0]
                .vertices
                .iter()
                .map(|vertex| vertex.position[axis])
                .fold(f32::INFINITY, f32::min);
            let upper = layers[0]
                .vertices
                .iter()
                .map(|vertex| vertex.position[axis])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                layers[1]
                    .vertices
                    .iter()
                    .all(|vertex| vertex.position[axis] >= lower - 1e-6
                        && vertex.position[axis] <= upper + 1e-6),
                "preserve outside domain bounds: {} axis {axis}",
                part.name
            );
        }
        let specifications = project.topology_operations(part.id);
        assert_eq!(specifications.len(), 1);
        let specification = specifications[0];
        assert_eq!(
            specification.objective,
            robogen_ir::topology::Objective::MinimizeMass
        );
        if part.name == "LeftShinSupport" {
            assert_eq!(specification.name, "left_shin_PLA_estimated");
            assert!(specification.material.is_some());
            assert_eq!(specification.load_cases.len(), 2);
            assert!(specification.preparation_issues().is_empty());
        } else {
            assert_eq!(specification.material, None);
            assert!(specification.load_cases.is_empty());
            assert_eq!(specification.preparation_issues().len(), 3);
        }
        assert!(!project.snapshot().meshes.contains_key(&part.id));
    }
    let camera = project.snapshot().model.parts["CameraSupport"].id;
    let layers = &project.snapshot().preparation[&camera];
    let lowest = layers[0]
        .vertices
        .iter()
        .map(|vertex| vertex.position[2])
        .fold(f32::INFINITY, f32::min);
    assert!((lowest - 0.322).abs() < 1e-5);
    let Err(errors) = project.optimize_part(
        camera,
        &robogen_project::topology_settings(),
        42,
        &|| false,
        &|_, _| {},
    ) else {
        return Err("incomplete support must not optimize".into());
    };
    assert!(
        errors
            .iter()
            .any(|error| error.code == "E347" && error.span.end > error.span.start),
        "{errors:?}"
    );
    assert!(project.snapshot().topology.is_empty());
    assert!(project.snapshot().mass_diagnostics.is_empty());
    assert_eq!(project.snapshot().meshes.len(), 17);
    Ok(())
}
