use robogen_domain::{Diagnostic, FeatureId, Length, ObjectInstanceId};
use robogen_ir::{lower, Feature, InstanceRegion, LowerOutput, SolidOperation};

const MATERIAL: &str =
    "material M { density: 1240 kg/m3; young: 3.5 GPa; poisson: 0.36; yield_strength: 50 MPa; }";

const OBJECTS: &str = r#"
component base_shape() -> Solid { return box([10 mm, 20 mm, 30 mm]); }
component base_void() -> Solid { return box([2 mm, 2 mm, 2 mm]); }
component base_mount() -> Solid { return cylinder(radius: 1 mm, height: 2 mm); }
component base_clearance() -> Solid { return cylinder(radius: 4 mm, height: 2 mm); }
component part_object() -> Object {
    return object(shape: base_shape(), void: [base_void()],
        interfaces: [interface(name: "mount", shape: base_mount()),
            interface(name: "clearance", shape: base_clearance())]);
}
"#;

fn compile(body: &str) -> LowerOutput {
    let parsed = robogen_dsl::parse(&format!("module test; {MATERIAL} {OBJECTS} {body}"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(parsed.module.is_some());
    parsed.module.as_ref().map(lower).unwrap_or_default()
}

fn valid(body: &str) -> robogen_ir::SemanticModel {
    let output = compile(body);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    output.model
}

fn error(body: &str, code: &str) -> Vec<Diagnostic> {
    let output = compile(body);
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "{body}: {:?}",
        output.diagnostics
    );
    assert!(output
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.span.end > diagnostic.span.start));
    output.diagnostics
}

#[test]
fn object_instances_are_materialized_with_stable_ids_and_regions() -> Result<(), &'static str> {
    let model =
        valid("instance Left = translate(offset: [0 mm, 0 mm, 100 mm], shape: part_object());");
    let instance = &model.instances["Left"];
    assert_eq!(instance.id, ObjectInstanceId::from_name("test::Left"));
    assert_eq!(instance.name, "Left");
    let SolidOperation::Translate { offset, .. } = &instance.shape.operation else {
        return Err("expected translate");
    };
    assert_eq!(offset[2], Length::from_millimetres(100.0));
    assert_eq!(instance.voids.len(), 1);
    assert_eq!(instance.interfaces.len(), 2);
    assert!(instance.interfaces.contains_key("mount"));
    assert!(instance.interfaces.contains_key("clearance"));
    assert!(instance.span.end > instance.span.start);
    // IDs are derived from the qualified name, so they are stable across rebuilds.
    assert_eq!(
        ObjectInstanceId::from_name("test::Left"),
        ObjectInstanceId::from_name("test::Left")
    );
    Ok(())
}

#[test]
fn repeated_instances_have_distinct_ids_and_poses() -> Result<(), &'static str> {
    let model = valid(
        "instance A = translate(offset: [0 mm, 0 mm, 0 mm], shape: part_object()); \
         instance B = translate(offset: [50 mm, 0 mm, 0 mm], shape: part_object());",
    );
    let a = &model.instances["A"];
    let b = &model.instances["B"];
    assert_ne!(a.id, b.id);
    let SolidOperation::Translate { offset, .. } = &b.shape.operation else {
        return Err("expected translate");
    };
    assert_eq!(offset[0], Length::from_millimetres(50.0));
    Ok(())
}

#[test]
fn pose_transforms_shape_voids_and_interfaces_together() {
    let model =
        valid("instance Left = translate(offset: [10 mm, 0 mm, 0 mm], shape: part_object());");
    let instance = &model.instances["Left"];
    for geometry in std::iter::once(&instance.shape)
        .chain(instance.voids.iter())
        .chain(instance.interfaces.values())
    {
        assert!(
            matches!(geometry.operation, SolidOperation::Translate { .. }),
            "{:?}",
            geometry.operation
        );
    }
}

#[test]
fn instance_shape_records_provenance_and_selects_the_whole_shape() -> Result<(), &'static str> {
    let model = valid(
        "instance Left = translate(offset: [0 mm, 0 mm, 100 mm], shape: part_object()); \
         part Support { material: M; body = instance_shape(Left); }",
    );
    let Feature::Solid { id, geometry, .. } = &model.parts["Support"].features[0] else {
        return Err("expected solid");
    };
    assert!(matches!(
        geometry.operation,
        SolidOperation::Translate { .. }
    ));
    assert_eq!(model.instance_references.len(), 1);
    let reference = &model.instance_references[0];
    assert_eq!(
        reference.instance,
        ObjectInstanceId::from_name("test::Left")
    );
    assert_eq!(reference.region, InstanceRegion::Shape);
    assert_eq!(reference.feature, *id);
    assert_eq!(
        reference.feature,
        FeatureId::from_name("test::Support::body")
    );
    Ok(())
}

#[test]
fn instance_interface_selects_only_the_requested_interface() -> Result<(), &'static str> {
    let model = valid(
        "instance Left = part_object(); \
         part Support { material: M; body = instance_interface(Left, \"mount\"); }",
    );
    let Feature::Solid { geometry, .. } = &model.parts["Support"].features[0] else {
        return Err("expected solid");
    };
    let SolidOperation::Cylinder { radius, .. } = geometry.operation else {
        return Err("expected cylinder");
    };
    assert_eq!(radius, Length::from_millimetres(1.0));
    assert_eq!(model.instance_references.len(), 1);
    assert_eq!(
        model.instance_references[0].region,
        InstanceRegion::Interface("mount".to_owned())
    );
    Ok(())
}

#[test]
fn instance_void_returns_the_exclusion_region() -> Result<(), &'static str> {
    let model = valid(
        "instance Left = part_object(); \
         part Support { material: M; body = instance_void(Left); }",
    );
    let Feature::Solid { geometry, .. } = &model.parts["Support"].features[0] else {
        return Err("expected solid");
    };
    // A single void is returned directly, without an extra compound wrapper.
    let SolidOperation::Box { size } = geometry.operation else {
        return Err("expected box");
    };
    assert_eq!(size[0], Length::from_millimetres(2.0));
    assert_eq!(model.instance_references[0].region, InstanceRegion::Void);
    Ok(())
}

#[test]
fn instance_regions_are_usable_inside_topology_preserve_and_void() {
    let model = valid(
        "instance Left = part_object(); \
         part Support { material: M; body = topology(name: \"support\", \
            domain: box([50 mm, 50 mm, 50 mm]), \
            preserve: [interface(name: \"mount\", shape: instance_interface(Left, \"mount\"))], \
            void: [instance_void(Left)], \
            material: unknown, load_cases: [], objective: minimize_mass, \
            constraints: [], manufacturing: []); }",
    );
    assert_eq!(model.topology_operations().len(), 1);
    let spec = model.topology_operations()[0];
    assert_eq!(spec.preserve.len(), 1);
    assert_eq!(spec.voids.len(), 1);
    assert_eq!(model.instance_references.len(), 2);
    let regions: Vec<_> = model
        .instance_references
        .iter()
        .map(|reference| reference.region.clone())
        .collect();
    assert!(regions.contains(&InstanceRegion::Interface("mount".to_owned())));
    assert!(regions.contains(&InstanceRegion::Void));
}

#[test]
fn existing_solid_components_and_instances_coexist() {
    let model = valid(
        "component brick() -> Solid { return box([1 mm, 2 mm, 3 mm]); } \
         instance Left = part_object(); \
         part Brick { material: M; body = brick(); } \
         part Support { material: M; body = instance_shape(Left); }",
    );
    assert!(model.parts.contains_key("Brick"));
    assert!(model.parts.contains_key("Support"));
    assert!(model.instances.contains_key("Left"));
}

#[test]
fn instances_reject_duplicates_unknown_cycles_and_type_mismatches() {
    error(
        "instance Left = part_object(); instance Left = part_object();",
        "E230",
    );
    error("instance A = A;", "E201");
    error(
        "part P { material: M; body = instance_shape(Missing); }",
        "E202",
    );
    error("instance Bad = base_shape();", "E234");
    error(
        "part P { material: M; body = instance_shape(base_shape()); }",
        "E234",
    );
    error(
        "instance Left = part_object(); part P { material: M; body = instance_interface(Left, \"missing\"); }",
        "E253",
    );
    error(
        "component bad_object() -> Object { return object(shape: base_shape(), void: [], interfaces: [interface(name: \"mount\", shape: base_mount()), interface(name: \"mount\", shape: base_mount())]); } instance Bad = bad_object();",
        "E254",
    );
    error(
        "component bad_return() -> Solid { return object(shape: base_shape(), void: [], interfaces: []); } part P { material: M; body = bad_return(); }",
        "E234",
    );
}

#[test]
fn object_library_rejects_named_instances() {
    let source = "module lib; parameter P = 1 mm; instance Left = translate(offset: [0 mm,0 mm,0 mm], shape: box([1 mm,1 mm,1 mm])); component c() -> Solid { return box([1 mm,1 mm,1 mm]); }";
    let result = robogen_dsl::component_library(source, "lib_", "c");
    assert!(result.is_err());
}
