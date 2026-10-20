use super::*;
use robogen_domain::{
    JointId, Length, ObjectInstanceId, PartId, SourceSpan,
    mechanics::{AxisDirection, RigidBody},
};

fn body(name: &str) -> RigidBody {
    RigidBody {
        id: RigidBodyId::from_name(name),
        name: name.into(),
        parts: vec![],
        grounded: false,
        span: SourceSpan::new(1, 10),
    }
}

fn joint(name: &str, parent: &RigidBody, child: &RigidBody, kind: JointKind) -> KinematicJoint {
    KinematicJoint {
        id: JointId::from_name(name),
        name: name.into(),
        parent: parent.id,
        child: child.id,
        kind,
        origin: [Length::ZERO; 3],
        axis: (kind != JointKind::Fixed).then_some(AxisDirection([0.0, 0.0, 1.0])),
        pitch: (kind == JointKind::Helical).then_some(Length::from_millimetres(-2.0)),
        actuator: None,
        span: SourceSpan::new(12, 50),
    }
}

fn model(bodies: &[RigidBody], joints: &[KinematicJoint]) -> MechanicalModel {
    MechanicalModel {
        bodies: bodies.iter().map(|body| (body.id, body.clone())).collect(),
        joints: joints
            .iter()
            .map(|joint| (joint.id, joint.clone()))
            .collect(),
    }
}

#[test]
fn moving_triangle_and_isolated_body_are_valid() {
    let [a, b, c, d] = [body("a"), body("b"), body("c"), body("isolated")];
    let joints = [
        joint("slide", &a, &b, JointKind::Prismatic),
        joint("screw", &b, &c, JointKind::Helical),
        joint("pivot", &c, &a, JointKind::Revolute),
    ];
    assert!(validate_mechanism(&model(&[a, b, c, d], &joints)).is_empty());
}

#[test]
fn moving_joint_is_rejected_when_fixed_path_locks_it_indirectly() {
    let [a, b, c] = [body("a"), body("b"), body("c")];
    let joints = [
        joint("ab", &a, &b, JointKind::Fixed),
        joint("bc", &b, &c, JointKind::Fixed),
        joint("ca", &c, &a, JointKind::Revolute),
    ];
    let diagnostics = validate_mechanism(&model(&[a, b, c], &joints));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E379");
    assert_eq!(diagnostics[0].span, joints[2].span);
}

#[test]
fn ground_locks_separate_fixed_branches_but_not_a_free_branch() {
    let [mut a, mut b, c, d] = [body("a"), body("b"), body("c"), body("d")];
    a.grounded = true;
    b.grounded = true;
    let mut joints = [
        joint("ac", &a, &c, JointKind::Fixed),
        joint("bd", &b, &d, JointKind::Fixed),
        joint("cd", &c, &d, JointKind::Cylindrical),
    ];
    let bodies = [a, b, c, d];
    assert_eq!(validate_mechanism(&model(&bodies, &joints))[0].code, "E379");
    joints[1].kind = JointKind::Revolute;
    joints[1].axis = Some(AxisDirection([1.0, 0.0, 0.0]));
    assert!(validate_mechanism(&model(&bodies, &joints)).is_empty());
}

#[test]
fn axes_must_be_finite_nonzero_and_unit() {
    let [a, b] = [body("a"), body("b")];
    for axis in [
        None,
        Some(AxisDirection([0.0; 3])),
        Some(AxisDirection([2.0, 0.0, 0.0])),
        Some(AxisDirection([f64::NAN, 0.0, 1.0])),
        Some(AxisDirection([f64::INFINITY, 0.0, 1.0])),
    ] {
        let mut j = joint("j", &a, &b, JointKind::Revolute);
        j.axis = axis;
        let diagnostics = validate_mechanism(&model(&[a.clone(), b.clone()], &[j]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E375");
    }
    let mut j = joint("j", &a, &b, JointKind::Prismatic);
    let n = 1.0 / f64::sqrt(2.0);
    j.axis = Some(AxisDirection([n, n, 0.0]));
    assert!(validate_mechanism(&model(&[a, b], &[j])).is_empty());
}

#[test]
fn helical_pitch_is_signed_finite_nonzero_and_exclusive() {
    let [a, b] = [body("a"), body("b")];
    for pitch in [
        None,
        Some(Length::ZERO),
        Some(Length::from_metres(f64::NAN)),
        Some(Length::from_metres(f64::INFINITY)),
    ] {
        let mut j = joint("j", &a, &b, JointKind::Helical);
        j.pitch = pitch;
        assert_eq!(
            validate_mechanism(&model(&[a.clone(), b.clone()], &[j]))[0].code,
            "E376"
        );
    }
    for pitch in [-0.001, 0.001] {
        let mut j = joint("j", &a, &b, JointKind::Helical);
        j.pitch = Some(Length::from_metres(pitch));
        assert!(validate_mechanism(&model(&[a.clone(), b.clone()], &[j])).is_empty());
    }
    let mut j = joint("j", &a, &b, JointKind::Fixed);
    j.pitch = Some(Length::from_metres(0.001));
    assert_eq!(validate_mechanism(&model(&[a, b], &[j]))[0].code, "E376");
}

#[test]
fn identities_endpoints_memberships_and_origins_report_source_diagnostics() {
    let [mut a, mut b] = [body("a"), body("b")];
    let part = PartId::from_name("shared");
    a.parts.push(part);
    b.parts.push(part);
    let mut j = joint("j", &a, &b, JointKind::Revolute);
    j.parent = RigidBodyId::from_name("missing");
    j.origin[2] = Length::from_metres(f64::NAN);
    let mut m = model(&[a.clone(), b.clone()], &[j.clone()]);
    m.bodies.insert(
        a.id,
        RigidBody {
            id: RigidBodyId::from_name("mismatch"),
            ..a
        },
    );
    m.joints.insert(
        j.id,
        KinematicJoint {
            id: JointId::from_name("mismatch"),
            ..j.clone()
        },
    );
    let diagnostics = validate_mechanism(&m);
    for code in ["E370", "E371", "E372", "E373", "E374"] {
        assert!(diagnostics.iter().any(|d| d.code == code), "missing {code}");
    }
    assert!(
        diagnostics
            .iter()
            .all(|d| d.span == b.span || d.span == j.span)
    );
    j.parent = b.id;
    j.child = b.id;
    assert!(
        validate_mechanism(&model(&[b], &[j]))
            .iter()
            .any(|d| d.code == "E373")
    );
}

#[test]
fn actuator_is_unique_and_revolute_only() {
    let [a, b, c] = [body("a"), body("b"), body("c")];
    let mut j = joint("ab", &a, &b, JointKind::Revolute);
    j.actuator = Some(ObjectInstanceId::from_name("motor"));
    let mut k = joint("bc", &b, &c, JointKind::Prismatic);
    k.actuator = j.actuator;
    let diagnostics = validate_mechanism(&model(&[a, b, c], &[j, k]));
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|d| d.code == "E377"));
    assert!(diagnostics.iter().any(|d| d.code == "E378"));
}
