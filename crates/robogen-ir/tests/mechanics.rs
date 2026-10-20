use robogen_domain::{mechanics::JointKind, RigidBodyId};
use robogen_dsl::parse;
use robogen_ir::{lower, LowerOutput};

fn compile(source: &str) -> LowerOutput {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed.module.as_ref().map(lower).unwrap_or_default()
}

const SOURCE: &str = r#"module machine;
parameter HEIGHT = 25 mm;
body A { parts: []; grounded: true; }
body B { parts: []; }
joint J { kind: helical; parent: A; child: B;
    origin: [0 mm, HEIGHT, 0 mm]; axis: [2, 0, 0]; pitch: -2 mm; }
"#;

#[test]
fn mechanical_declarations_resolve_names_units_axis_and_provenance() -> Result<(), String> {
    let output = compile(SOURCE);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let model = output.model.mechanics;
    assert_eq!(model.bodies.len(), 2);
    assert_eq!(model.joints.len(), 1);
    let joint = model.joints.values().next().ok_or("joint")?;
    assert_eq!(joint.parent, RigidBodyId::from_name("machine::A"));
    assert_eq!(joint.kind, JointKind::Helical);
    assert_eq!(joint.origin[1].millimetres(), 25.0);
    assert_eq!(joint.axis.ok_or("axis")?.0, [1.0, 0.0, 0.0]);
    assert_eq!(joint.pitch.ok_or("pitch")?.millimetres(), -2.0);
    assert!(SOURCE[joint.span.start..joint.span.end].starts_with("joint J"));
    assert!(model.bodies[&joint.parent].grounded);
    Ok(())
}

#[test]
fn unknown_members_endpoints_fields_and_wrong_units_return_diagnostics() {
    for (source, code) in [
        (SOURCE.replace("parent: A", "parent: Unknown"), "E361"),
        (
            SOURCE.replace("parts: []; grounded", "parts: [Missing]; grounded"),
            "E361",
        ),
        (
            SOURCE.replace("axis: [2, 0, 0]", "axis: [2 mm, 0, 0]"),
            "E360",
        ),
        (SOURCE.replace("HEIGHT, 0 mm", "2, 0 mm"), "E360"),
        (SOURCE.replace("pitch: -2 mm", "pitch: -2"), "E360"),
        (SOURCE.replace("pitch: -2 mm", "guess: 3 mm"), "E360"),
        (
            SOURCE.replace("pitch: -2 mm", "pitch: 2 mm; pitch: 3 mm"),
            "E360",
        ),
        (SOURCE.replace("kind: helical", "kind: fake_joint"), "E362"),
    ] {
        let output = compile(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{source}: {:?}",
            output.diagnostics
        );
        assert!(output
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.span.end > diagnostic.span.start));
    }
}

#[test]
fn component_library_does_not_silently_drop_mechanical_records() {
    let diagnostics = robogen_dsl::component_library(SOURCE, "imported_", "main")
        .err()
        .unwrap_or_default();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E330"));
}
