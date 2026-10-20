use super::*;

fn painted_green_lines(shapes: &[egui::epaint::ClippedShape]) -> usize {
    fn count(shape: &Shape) -> usize {
        match shape {
            Shape::LineSegment { stroke, .. }
                if stroke.color == crate::viewport::INSTANCE_INTERFACE_COLOR =>
            {
                1
            }
            Shape::Vec(children) => children.iter().map(count).sum(),
            _ => 0,
        }
    }
    shapes.iter().map(|shape| count(&shape.shape)).sum()
}

#[test]
fn automatic_mounts_exist_without_a_topology_declaration() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    state.set_dsl_source(
        r#"module mounts;
material M { density: 1240 kg/m3; young: 2 GPa; poisson: 0.35; yield_strength: 25 MPa; }
component servo_object() -> Object {
    return object(shape: box([10 mm, 20 mm, 30 mm]), void: [],
        interfaces: [interface(name: "mount", shape: cylinder(2 mm, 2 mm))]);
}
instance A = servo_object();
instance B = translate([40 mm, 0 mm, 0 mm], servo_object());
part First { material: M; body = instance_shape(A); }
part Second { material: M; body = instance_shape(B); }
"#,
    );
    let project = state.document.project().ok_or("missing project")?;
    assert!(project.snapshot().preparation.is_empty());
    assert_eq!(project.snapshot().instance_interfaces.len(), 2);
    assert!(state.show_instance_interfaces);
    state.show_components = false;
    state.design_tab = DesignTab::Viewport;
    state.camera.target = robogen_render::Vec3::new(0.0, 0.0, 0.0);
    let context = Context::default();
    let _ = frame(&mut state, &context, Vec::new());
    let output = frame(&mut state, &context, Vec::new());
    let mut text = String::new();
    for shape in &output.shapes {
        collect_painted_text(&shape.shape, &mut text);
    }
    assert!(text.contains("Fixations auto · 2 objets"), "{text}");
    assert!(painted_green_lines(&output.shapes) > 0);
    state.show_instance_interfaces = false;
    let output = frame(&mut state, &context, Vec::new());
    assert_eq!(painted_green_lines(&output.shapes), 0);
    Ok(())
}

#[test]
fn all_fourteen_servo_mounts_remain_when_a_different_support_is_selected() -> Result<(), String> {
    let mut state = RoboGenUi::default();
    state.load_biped_example();
    let snapshot = state.document.project().ok_or("missing biped")?.snapshot();
    assert_eq!(snapshot.instance_interfaces.len(), 14);
    let expected_triangles: usize = snapshot
        .instance_interfaces
        .values()
        .map(|mesh| mesh.triangles.len())
        .sum();
    let parts: Vec<_> = snapshot.preparation.keys().copied().collect();
    let before = snapshot.model.topology_operations();
    let specifications = before.into_iter().cloned().collect::<Vec<_>>();
    assert!(state.show_instance_interfaces);
    assert_eq!(
        state.instance_interface_meshes.triangles.len(),
        expected_triangles
    );
    state.set_mode(WorkspaceMode::Optimization);
    state.show_regions = [false; 3];
    state.show_components = false;
    state.show_results = false;
    for part in parts {
        state.selected_optimization = Some(part);
        state.rebuild_preview();
        assert_eq!(
            state.instance_interface_meshes.triangles.len(),
            expected_triangles
        );
    }
    let context = Context::default();
    let _ = frame(&mut state, &context, Vec::new());
    let output = frame(&mut state, &context, Vec::new());
    assert!(painted_green_lines(&output.shapes) > 0);
    let snapshot = state.document.project().ok_or("lost biped")?.snapshot();
    assert_eq!(
        snapshot
            .model
            .topology_operations()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        specifications
    );
    assert!(snapshot.topology.is_empty());
    assert!(snapshot.mass_diagnostics.is_empty());
    assert!(!state.can_export());
    Ok(())
}
