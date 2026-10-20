use robogen_domain::{mechanics::JointKind, Length, Quantity};
use robogen_project::{compile_source, Project, SourceDocument};

const VISE: &str = include_str!("../../../examples/kinematic_vise/main.rgn");

#[test]
fn vise_cycle_shares_body_part_ids_with_cad_and_updates_via_history() -> Result<(), String> {
    let mut project = Project::from_source(VISE).map_err(|error| format!("{error:?}"))?;
    let model = &project.snapshot().model;
    assert_eq!(model.mechanics.bodies.len(), 4);
    assert_eq!(model.mechanics.joints.len(), 4);
    assert!(model
        .mechanics
        .bodies
        .values()
        .flat_map(|body| &body.parts)
        .all(|id| project.snapshot().meshes.contains_key(id)));
    let joint_id = model
        .mechanics
        .joints
        .values()
        .find(|joint| joint.name == "BC")
        .ok_or("joint")?
        .id;
    let origin = model.mechanics.joints[&joint_id].origin;
    project
        .set_parameter("OPENING", Quantity::Length(Length::from_millimetres(50.0)))
        .map_err(|error| format!("{error:?}"))?;
    project.rebuild().map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        project.snapshot().model.mechanics.joints[&joint_id].origin[0].millimetres(),
        30.0
    );
    assert_ne!(
        origin,
        project.snapshot().model.mechanics.joints[&joint_id].origin
    );
    let mut document = SourceDocument::new(VISE);
    document.replace_source(VISE.replace("axis: [1, 0, 0]", "axis: [0, 0, 0]"));
    assert!(!document.is_valid());
    assert!(document.project().is_some());
    document.undo();
    assert!(document.is_valid());
    Ok(())
}

#[test]
fn locking_a_moving_joint_by_a_fixed_path_fails_all_compile_entrypoints() {
    let source = r#"module locked; body A {} body B {} body C {}
joint AB { kind: fixed; parent: A; child: B; origin: [0 mm,0 mm,0 mm]; }
joint BC { kind: fixed; parent: B; child: C; origin: [0 mm,0 mm,0 mm]; }
joint AC { kind: revolute; parent: A; child: C; origin: [0 mm,0 mm,0 mm]; axis:[1,0,0]; }"#;
    let direct = compile_source(source).err().unwrap_or_default();
    let project = Project::from_source(source).err().unwrap_or_default();
    assert!(direct.iter().any(|error| error.code == "E379"));
    assert!(project.iter().any(|error| error.code == "E379"));
}

#[test]
fn biped_has_fourteen_declared_actuated_pivots_and_fifteen_bodies() -> Result<(), String> {
    let model = compile_source(include_str!("../../../examples/mini_biped/main.rgn"))
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(model.mechanics.bodies.len(), 15);
    assert_eq!(model.mechanics.joints.len(), 14);
    assert!(model
        .mechanics
        .joints
        .values()
        .all(|joint| joint.kind == JointKind::Revolute && joint.actuator.is_some()));
    assert!(model.mechanics.joints.values().all(|joint| model
        .instances
        .values()
        .any(|instance| Some(instance.id) == joint.actuator)));
    Ok(())
}
