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
        assert_eq!(servo.interfaces.len(), 1, "{}: one interface", servo.name);
        assert!(
            servo.interfaces.contains_key("housing"),
            "{}: housing interface",
            servo.name
        );
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
