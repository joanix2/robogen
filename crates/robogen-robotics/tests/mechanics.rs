use robogen_domain::{
    JointId, Length, RigidBodyId, SourceSpan,
    mechanics::{AxisDirection, JointKind, KinematicJoint, MechanicalModel, RigidBody},
};
use robogen_robotics::validate_mechanism;

#[test]
fn public_validation_accepts_empty_model_and_reports_grounded_motion_at_source() {
    assert!(validate_mechanism(&MechanicalModel::default()).is_empty());
    let mut model = MechanicalModel::default();
    let a = RigidBodyId::from_name("a");
    let b = RigidBodyId::from_name("b");
    for (id, name) in [(a, "a"), (b, "b")] {
        model.bodies.insert(
            id,
            RigidBody {
                id,
                name: name.into(),
                parts: vec![],
                grounded: true,
                span: SourceSpan::new(0, 10),
            },
        );
    }
    let id = JointId::from_name("joint");
    let span = SourceSpan::new(12, 60);
    model.joints.insert(
        id,
        KinematicJoint {
            id,
            name: "joint".into(),
            parent: a,
            child: b,
            kind: JointKind::Revolute,
            origin: [Length::ZERO; 3],
            axis: Some(AxisDirection([0.0, 0.0, 1.0])),
            pitch: None,
            actuator: None,
            span,
        },
    );
    let diagnostics = validate_mechanism(&model);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E379");
    assert_eq!(diagnostics[0].span, span);
}
