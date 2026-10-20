use robogen_cad::{CadKernel, NativeCadKernel};
use robogen_domain::mechanics::JointKind;
use robogen_ir::{SolidGeometry, SolidOperation};
use robogen_project::Project;

const SOURCE: &str = include_str!("../../../examples/mini_biped/main.rgn");

/// `placed_servo` wraps every region in translate(placement) -> rotate(angles)
/// -> translate(anchor), so the intrinsic region geometry is three layers deep.
fn strip_pose(mut geometry: &SolidGeometry) -> &SolidGeometry {
    for _ in 0..3 {
        geometry = match &geometry.operation {
            SolidOperation::Translate { shape, .. } | SolidOperation::Rotate { shape, .. } => shape,
            _ => break,
        };
    }
    geometry
}

#[test]
fn every_servo_instance_shares_identical_region_geometry() -> Result<(), String> {
    let project = Project::from_source(SOURCE).map_err(|errors| format!("{errors:?}"))?;
    let model = &project.snapshot().model;
    let servos: Vec<_> = model
        .instances
        .values()
        .filter(|instance| instance.name.ends_with("Servo"))
        .collect();
    assert_eq!(servos.len(), 14, "expected 14 servo instances");
    let reference = servos[0];
    for servo in &servos {
        assert_eq!(servo.voids.len(), 1, "{}: one exclusion", servo.name);
        assert_eq!(servo.interfaces.len(), 2, "{}: two interfaces", servo.name);
        for name in ["housing", "shaft"] {
            assert!(
                servo.interfaces.contains_key(name),
                "{}: {name} interface",
                servo.name
            );
        }
    }
    for servo in &servos[1..] {
        assert_eq!(
            strip_pose(&servo.voids[0]),
            strip_pose(&reference.voids[0]),
            "exclusion geometry differs: {}",
            servo.name
        );
        assert_eq!(
            strip_pose(&servo.interfaces["housing"]),
            strip_pose(&reference.interfaces["housing"]),
            "housing contact geometry differs: {}",
            servo.name
        );
        assert_eq!(
            strip_pose(&servo.interfaces["shaft"]),
            strip_pose(&reference.interfaces["shaft"]),
            "shaft contact geometry differs: {}",
            servo.name
        );
    }
    let SolidOperation::Compound { shapes } = &strip_pose(&reference.voids[0]).operation else {
        return Err("expected a compound exclusion".into());
    };
    assert_eq!(shapes.len(), 8, "exclusion is eight bodies");
    let SolidOperation::Compound { shapes } =
        &strip_pose(&reference.interfaces["housing"]).operation
    else {
        return Err("expected a compound housing".into());
    };
    assert_eq!(shapes.len(), 2, "housing is two contact rings");
    Ok(())
}

#[test]
fn instance_interfaces_project_all_servo_housings_without_touching_topology() -> Result<(), String>
{
    let project = Project::from_source(SOURCE).map_err(|errors| format!("{errors:?}"))?;
    let snapshot = project.snapshot();
    assert_eq!(
        snapshot.instance_interfaces.len(),
        14,
        "expected 14 servo interface overlays"
    );
    for mesh in snapshot.instance_interfaces.values() {
        assert!(!mesh.triangles.is_empty(), "interface overlay is empty");
    }
    assert_eq!(
        snapshot.model.topology_operations().len(),
        11,
        "topology preparations must be unchanged"
    );
    Ok(())
}

/// Axis-aligned bounds, in millimetres, of an instance region in the document frame.
fn bounds_mm(geometry: &SolidGeometry) -> Result<[[f64; 3]; 2], String> {
    let mesh = NativeCadKernel
        .solid(geometry)
        .map_err(|error| error.to_string())?;
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for vertex in &mesh.vertices {
        for (axis, value) in vertex.position.iter().enumerate() {
            let mm = f64::from(*value) * 1000.0;
            bounds[0][axis] = bounds[0][axis].min(mm);
            bounds[1][axis] = bounds[1][axis].max(mm);
        }
    }
    Ok(bounds)
}

#[test]
fn shaft_pad_sits_on_each_declared_joint_axis_facing_outwards() -> Result<(), String> {
    let project = Project::from_source(SOURCE).map_err(|errors| format!("{errors:?}"))?;
    let model = &project.snapshot().model;
    let mut checked = 0;
    for joint in model.mechanics.joints.values() {
        let (Some(actuator), Some(axis)) = (joint.actuator, joint.axis) else {
            continue;
        };
        assert_eq!(joint.kind, JointKind::Revolute);
        let servo = model
            .instances
            .values()
            .find(|instance| instance.id == actuator)
            .ok_or("missing actuator instance")?;
        let bounds = bounds_mm(&servo.interfaces["shaft"])?;
        let origin = joint.origin.map(|value| value.millimetres());
        let along = |point: [f64; 3]| -> f64 {
            (0..3)
                .map(|index| (point[index] - origin[index]) * axis.0[index])
                .sum()
        };
        // The pad is a disc of the shaft radius whose centre lies on the joint axis.
        let centre: [f64; 3] = std::array::from_fn(|i| (bounds[0][i] + bounds[1][i]) / 2.0);
        let offset = along(centre);
        let radial: f64 = (0..3)
            .map(|i| (centre[i] - origin[i] - offset * axis.0[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            radial < 0.05,
            "{}: pad off the joint axis by {radial} mm",
            joint.name
        );
        // It starts on the output face and extends outwards by its thickness.
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for x in [bounds[0][0], bounds[1][0]] {
            for y in [bounds[0][1], bounds[1][1]] {
                for z in [bounds[0][2], bounds[1][2]] {
                    let value = along([x, y, z]);
                    low = low.min(value);
                    high = high.max(value);
                }
            }
        }
        assert!(
            low.abs() < 0.05,
            "{}: pad starts {low} mm from the output face",
            joint.name
        );
        assert!(
            (high - 2.0).abs() < 0.05,
            "{}: pad ends {high} mm from the output face",
            joint.name
        );
        checked += 1;
    }
    assert_eq!(
        checked, 14,
        "every servo drives one declared revolute joint"
    );
    Ok(())
}

#[test]
fn child_parts_preserve_the_shaft_and_parent_parts_the_housing() -> Result<(), String> {
    let project = Project::from_source(SOURCE).map_err(|errors| format!("{errors:?}"))?;
    let model = &project.snapshot().model;
    let preserved = |part: &str| -> Result<Vec<String>, String> {
        let id = model.parts.get(part).ok_or("missing part")?.id;
        Ok(project
            .topology_operations(id)
            .iter()
            .flat_map(|spec| spec.preserve.iter())
            .filter_map(|region| region.interface.as_ref().map(|i| i.name.clone()))
            .collect())
    };
    for part in [
        "LeftThighSupport",
        "RightThighSupport",
        "RightShinSupport",
        "LeftArmSupport",
        "RightArmSupport",
    ] {
        assert_eq!(
            preserved(part)?,
            ["distal_housing", "proximal_shaft"],
            "{part}"
        );
    }
    assert_eq!(
        preserved("LeftShinSupport")?,
        [
            "estimated_distal_rigid_transfer",
            "estimated_proximal_rigid_transfer"
        ]
    );
    for part in ["LeftFootSupport", "RightFootSupport"] {
        assert_eq!(preserved(part)?, ["ankle_shaft"], "{part}");
    }
    assert_eq!(
        preserved("TorsoSupport")?,
        [
            "board_mount",
            "left_shoulder_housing",
            "right_shoulder_housing"
        ]
    );
    assert_eq!(
        preserved("PelvisSupport")?,
        ["left_hip_housing", "right_hip_housing"]
    );
    Ok(())
}
