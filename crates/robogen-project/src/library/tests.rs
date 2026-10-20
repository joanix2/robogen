use super::*;

#[test]
fn generators_insert_independent_instances_and_undo() -> Result<(), Vec<Diagnostic>> {
    let mut document = SourceDocument::new("module library_test;");
    for generator in LibraryGenerator::ALL {
        let before = document.source().to_owned();
        let name = document.instantiate_generator(generator)?;
        assert!(document.is_valid(), "{:?}", document.diagnostics());
        assert!(document.project().is_some_and(|project| project
            .snapshot()
            .model
            .parts
            .contains_key(&name)));
        assert!(document
            .source()
            .contains(&format!("component {}(", generator.name())));
        assert!(document.undo());
        assert_eq!(document.source(), before);
        assert!(document.redo());
        assert!(document.is_valid());
    }
    let second = document.instantiate_generator(LibraryGenerator::Servo)?;
    assert_eq!(second, "servo_2");
    assert!(document.is_valid(), "{:?}", document.diagnostics());
    assert_eq!(document.source().matches("component servo(").count(), 1);
    assert_eq!(
        document
            .project()
            .map(|project| project.snapshot().model.parts.len()),
        Some(LibraryGenerator::ALL.len() + 1)
    );
    let taxonomy = document
        .project()
        .map(Project::taxonomy_instances)
        .unwrap_or_default();
    let servo = taxonomy
        .iter()
        .find(|instance| instance.name == "servo_2")
        .ok_or_else(|| {
            vec![Diagnostic::error(
                "TEST",
                "missing servo instance",
                SourceSpan::default(),
            )]
        })?;
    assert_eq!(servo.features[0].functions, ["servo"]);
    assert!(document
        .project()
        .map(Project::function_definitions)
        .unwrap_or_default()
        .iter()
        .any(|function| function.name == "jetson_nano_super"));
    let source = document.source().to_owned();
    let standalone = crate::Project::from_source(source)?;
    assert_eq!(
        standalone.snapshot().model.parts.len(),
        LibraryGenerator::ALL.len() + 1
    );
    assert_eq!(
        standalone.snapshot().model.parts["servo_1"].material,
        standalone.snapshot().model.parts["servo_2"].material
    );
    document.replace_source("module broken; part".to_owned());
    assert!(document
        .instantiate_generator(LibraryGenerator::Camera)
        .is_err());
    Ok(())
}

#[test]
fn library_conflicts_do_not_mutate_source_and_materials_are_reused() -> Result<(), Vec<Diagnostic>>
{
    let mut conflict =
        SourceDocument::new("module conflict; parameter lib_camera_PCB_THICKNESS = 1 mm;");
    let before = conflict.source().to_owned();
    assert!(conflict
        .instantiate_generator(LibraryGenerator::Camera)
        .is_err());
    assert_eq!(conflict.source(), before);
    assert!(!conflict.can_undo());
    let source = "module materials;
material First { density: 1000 kg/m3; young: 1 GPa; poisson: 0.3; yield_strength: 10 MPa; }
material Second { density: 2000 kg/m3; young: 2 GPa; poisson: 0.3; yield_strength: 20 MPa; }
component servo() -> Solid { return box([1 mm, 1 mm, 1 mm]); }
part Original { material: Second; body = servo(); }";
    let mut document = SourceDocument::new(source);
    document.instantiate_generator(LibraryGenerator::Servo)?;
    assert!(document
        .source()
        .contains("part servo_1 {\n    material: Second;"));
    Ok(())
}
