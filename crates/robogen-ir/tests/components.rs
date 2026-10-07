use robogen_domain::{Diagnostic, Length};
use robogen_ir::{lower, Feature, LowerOutput, SolidOperation};

const MATERIAL: &str =
    "material M { density: 1240 kg/m3; young: 3.5 GPa; poisson: 0.36; yield_strength: 50 MPa; }";
const SERVO: &str = "component servo(width: Length = 23 mm, depth: Length = 12 mm, height: Length = 24 mm, shaft_radius: Length = 2 mm) -> Solid { body = box(size:[width,depth,height]); shaft = translate(offset:[width/2,depth/2,height],shape:cylinder(radius:shaft_radius,height:5 mm)); return union(body,shaft); }";

fn compile(body: &str) -> LowerOutput {
    let parsed = robogen_dsl::parse(&format!("module test; {MATERIAL} {body}"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(parsed.module.is_some());
    parsed.module.as_ref().map(lower).unwrap_or_default()
}

const TOPOLOGY_SOURCE: &str = include_str!("../../../examples/topology_battery_support/main.rgn");

fn topology_output(source: &str) -> LowerOutput {
    let parsed = robogen_dsl::parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(parsed.module.is_some());
    parsed.module.as_ref().map(lower).unwrap_or_default()
}

#[test]
fn topology_contract_preserves_types_ids_and_composition() -> Result<(), &'static str> {
    use robogen_ir::topology::{ConstraintKind, LoadKind, ManufacturingProcess};
    let original = "translate(offset: [0 mm, 0 mm, 5 mm], shape: battery_support())";
    for expression in [
        "battery_support()",
        original,
        "rotate(0 deg, 45 deg, 0 deg, battery_support())",
        "color(10, 20, 30, battery_support())",
        "compound(box([1 mm, 1 mm, 1 mm]), battery_support())",
        "union(box([1 mm, 1 mm, 1 mm]), battery_support())",
        "difference(box([1 mm, 1 mm, 1 mm]), battery_support())",
        "difference(battery_support(), box([1 mm, 1 mm, 1 mm]))",
    ] {
        let source = TOPOLOGY_SOURCE.replace(original, expression);
        let output = topology_output(&source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let operations = output.model.topology_operations();
        assert_eq!(operations.len(), 1);
        let spec = operations[0];
        assert_eq!(spec.schema_version, 1);
        let Feature::Solid { id, .. } = &output.model.parts["BatterySupport"].features[0] else {
            return Err("expected solid");
        };
        assert_eq!(spec.feature, *id);
        assert!(source[spec.span.start..spec.span.end].starts_with("topology("));
        assert_eq!(spec.material, output.model.materials["IllustrativePLA"].id);
        let seat = spec.preserve[1]
            .interface
            .as_ref()
            .ok_or("missing seat interface")?;
        assert_eq!(spec.load_cases[0].loads[0].target, seat.id);
        let LoadKind::Force(force) = &spec.load_cases[0].loads[0].kind else {
            return Err("expected force");
        };
        assert_eq!(force.map(|value| value.newtons()), [0.0, 0.0, -3.0]);
        assert!(
            matches!(spec.constraints[3].kind, ConstraintKind::MaxMass(value) if (value.kilograms() - 0.04).abs() < 1e-10)
        );
        assert!(matches!(
            spec.manufacturing[0].process,
            ManufacturingProcess::Fdm { .. }
        ));
        let repeated = topology_output(&source);
        assert_eq!(spec, repeated.model.topology_operations()[0]);
    }
    let source = format!(
        "{TOPOLOGY_SOURCE}\npart Other {{ material: IllustrativePLA; body = battery_support(); }}"
    );
    let output = topology_output(&source);
    assert!(output.diagnostics.is_empty());
    let operations = output.model.topology_operations();
    assert_eq!(operations.len(), 2);
    assert_ne!(operations[0].id, operations[1].id);
    assert_ne!(
        operations[0].load_cases[0].id,
        operations[1].load_cases[0].id
    );
    assert_ne!(
        operations[0].preserve[0].interface,
        operations[1].preserve[0].interface
    );
    Ok(())
}

#[test]
fn topology_rejects_invalid_or_unsupported_declarations() {
    for (before, after, code) in [
        ("DESIGN_FORCE = 3 N", "DESIGN_FORCE = 3 mm", "E104"),
        ("DESIGN_FORCE = 3 N", "DESIGN_FORCE = 0 N", "E252"),
        ("on: \"seat\"", "on: \"missing\"", "E253"),
        ("name: \"seat\"", "name: \"mount\"", "E254"),
        ("name: \"vertical\"", "name: \"bad name\"", "E251"),
        ("max_mass(40 g)", "max_mass(40 N)", "E104"),
        ("max_mass(40 g)", "max_mass(-1 g)", "E252"),
        ("max_stress(20 MPa)", "max_stress(1 / 0)", "E235"),
        ("volume_fraction(0.35)", "volume_fraction(1.1)", "E252"),
        ("safety_factor(2)", "safety_factor(0.9)", "E252"),
        ("min_frequency(30 Hz)", "unknown_limit(30 Hz)", "E258"),
        ("min_frequency(30 Hz)", "max_mass(30 g)", "E254"),
        (
            "objective: minimize_mass",
            "objective: unknown_objective",
            "E256",
        ),
        ("objective: minimize_mass,", "", "E236"),
        ("constraints: [", "unknown_field: [", "E236"),
        (
            "build_direction: [0, 0, 1]",
            "build_direction: [0, 0, 2]",
            "E252",
        ),
        (
            "build_direction: [0, 0, 1]",
            "build_direction: [0 mm, 0 mm, 1 mm]",
            "E104",
        ),
        ("min_thickness: 1.2 mm", "min_thickness: -1 mm", "E238"),
        ("max_overhang: 45 deg", "max_overhang: 91 deg", "E252"),
        (
            "manufacturing: [fdm(",
            "manufacturing: [unknown_process(",
            "E258",
        ),
        ("force(on:", "gravity(on:", "E257"),
        ("fixed(on:", "unknown_support(on:", "E257"),
        ("supports: [fixed(on: \"mount\")]", "supports: []", "E250"),
        (
            "supports: [fixed(on: \"mount\")]",
            "supports: [fixed(on: \"mount\"), fixed(on: \"mount\")]",
            "E254",
        ),
        ("young: 3 GPa", "young: 0 GPa", "E255"),
        ("poisson: 0.35", "poisson: 0.5", "E255"),
        ("poisson: 0.35", "poisson: -1", "E255"),
    ] {
        let output = topology_output(&TOPOLOGY_SOURCE.replace(before, after));
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{before} -> {after}: {:?}",
            output.diagnostics
        );
        assert!(output
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.span.end > diagnostic.span.start));
    }
}

#[test]
fn topology_supports_torque_cnc_and_explicit_empty_manufacturing() {
    use robogen_ir::topology::{LoadKind, ManufacturingProcess, Objective, SupportKind};
    let source = TOPOLOGY_SOURCE
        .replace("force(on: \"seat\", vector: [0 N, 0 N, -DESIGN_FORCE])", "moment(on: \"seat\", vector: [0 Nm, 200 Nmm, 0 Nm])")
        .replace("fixed(on: \"mount\")", "pin(on: \"mount\", axis: [1, 0, 0])")
        .replace("objective: minimize_mass", "objective: maximize_stiffness")
        .replace("fdm(build_direction: [0, 0, 1],\n            min_thickness: 1.2 mm, min_hole: 2 mm, max_overhang: 45 deg)", "cnc_3axis(tool_direction: [0, 0, -1], tool_diameter: 3 mm)");
    let output = topology_output(&source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let operations = output.model.topology_operations();
    assert_eq!(operations[0].objective, Objective::MaximizeStiffness);
    assert!(
        matches!(operations[0].load_cases[0].loads[0].kind, LoadKind::Moment(vector) if (vector[1].newton_metres() - 0.2).abs() < 1e-10)
    );
    assert!(matches!(
        operations[0].load_cases[0].supports[0].kind,
        SupportKind::Pin { .. }
    ));
    assert!(matches!(
        operations[0].manufacturing[0].process,
        ManufacturingProcess::Cnc3Axis { .. }
    ));
    let output = topology_output(
        &source
            .replace(
                "cnc_3axis(tool_direction: [0, 0, -1], tool_diameter: 3 mm)",
                "",
            )
            .replace(
                "pin(on: \"mount\", axis: [1, 0, 0])",
                "frictionless(on: \"mount\", normal: [0, 0, 1])",
            ),
    );
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert!(output.model.topology_operations()[0]
        .manufacturing
        .is_empty());
}

#[test]
fn topology_load_cases_are_distinct_and_lists_are_bounded() {
    let case = "load_case(name: \"vertical\", loads: [\n            force(on: \"seat\", vector: [0 N, 0 N, -DESIGN_FORCE])],\n            supports: [fixed(on: \"mount\")])";
    let second = case
        .replace("\"vertical\"", "\"lateral\"")
        .replace("[0 N, 0 N, -DESIGN_FORCE]", "[DESIGN_FORCE, 0 N, 0 N]");
    let output = topology_output(&TOPOLOGY_SOURCE.replace(case, &format!("{case}, {second}")));
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let operations = output.model.topology_operations();
    assert_eq!(operations[0].load_cases.len(), 2);
    assert_ne!(
        operations[0].load_cases[0].id,
        operations[0].load_cases[1].id
    );
    for (replacement, code) in [
        (format!("{case}, {case}"), "E254"),
        (
            std::iter::repeat_n(case, 17).collect::<Vec<_>>().join(", "),
            "E250",
        ),
    ] {
        let output = topology_output(&TOPOLOGY_SOURCE.replace(case, &replacement));
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{:?}",
            output.diagnostics
        );
    }
    let output = topology_output(&TOPOLOGY_SOURCE.replace(
        "shape: battery_support()",
        "shape: compound(battery_support(), battery_support())",
    ));
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E254"));
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
    output.diagnostics
}

#[test]
fn color_channels_are_typed_bounded_and_preserved() -> Result<(), &'static str> {
    let model = valid(
        "part P { material:M; body=color(red:32,green:110,blue:220,shape:box([1 mm,1 mm,1 mm])); }",
    );
    let Feature::Solid { geometry, .. } = &model.parts["P"].features[0] else {
        return Err("expected solid");
    };
    let SolidOperation::Color { rgb, .. } = geometry.operation else {
        return Err("expected color");
    };
    assert_eq!(rgb, [32, 110, 220]);
    for channel in ["-1", "256", "2.5"] {
        error(
            &format!("part P {{ material:M; body=color({channel},0,0,box([1 mm,1 mm,1 mm])); }}"),
            "E241",
        );
    }
    error(
        "part P { material:M; body=color(1 mm,0,0,box([1 mm,1 mm,1 mm])); }",
        "E104",
    );
    Ok(())
}

#[test]
fn defaults_named_positional_independent_instances_and_provenance() -> Result<(), &'static str> {
    let body = format!("part P {{ material: M; first = servo(); second = servo(width:25 mm); third = servo(26 mm); }} {SERVO}");
    let model = valid(&body);
    let features = &model.parts["P"].features;
    let mut ids = Vec::new();
    for (feature, width) in features.iter().zip([23.0, 25.0, 26.0]) {
        let Feature::Solid {
            id, geometry, span, ..
        } = feature
        else {
            return Err("expected solid");
        };
        ids.push(*id);
        assert_ne!(*span, geometry.span);
        let SolidOperation::Union { shapes } = &geometry.operation else {
            return Err("expected union");
        };
        let SolidOperation::Box { size } = shapes[0].operation else {
            return Err("expected box");
        };
        assert_eq!(size[0], Length::from_millimetres(width));
        let SolidOperation::Translate { offset, .. } = shapes[1].operation else {
            return Err("expected translation");
        };
        assert_eq!(offset[0], Length::from_millimetres(width / 2.0));
    }
    assert_ne!(ids[0], ids[1]);
    let changed = valid(&body.replace("25 mm", "29 mm"));
    assert_eq!(features[0], changed.parts["P"].features[0]);
    let Feature::Solid { id, .. } = changed.parts["P"].features[1] else {
        return Err("expected solid");
    };
    assert_eq!(id, ids[1]);
    Ok(())
}

#[test]
fn nested_forward_components_defaults_and_typed_arithmetic() {
    valid(&format!("parameter GLOBAL = 10 mm; component mount(width: Length = GLOBAL, height: Length = width*2) -> Solid {{ first = servo(width:width+2 mm); second = servo(width:height-1 mm); return difference(base:first,tool:translate([0 mm,0 mm,-(width/2)],second)); }} {SERVO} part P {{ material: M; body=mount(); }}"));
    valid("parameter HALF = FULL/2; parameter FULL = 20 mm; component block(width: Length = HALF, scale: Scalar = 2) -> Solid { return box([scale*width,(width+2 mm)/2,width*(width/width)]); } part P { material: M; body=block(); }");
}

#[test]
fn lexical_scopes_do_not_leak_caller_parameters_or_bindings() -> Result<(), &'static str> {
    error("component inner() -> Solid { return box([secret,1 mm,1 mm]); } component outer(secret: Length = 1 mm) -> Solid { return inner(); } part P { material:M; body=outer(); }", "E202");
    error("component inner(width: Length = secret) -> Solid { return box([width,width,width]); } component outer() -> Solid { secret=1 mm; return inner(); } part P { material:M; body=outer(); }", "E202");
    let model = valid("parameter width = 7 mm; component inner() -> Solid { return box([width,width,width]); } component outer(width: Length = 99 mm) -> Solid { return inner(); } part P { material:M; body=outer(); }");
    let Feature::Solid { geometry, .. } = &model.parts["P"].features[0] else {
        return Err("expected solid");
    };
    let SolidOperation::Box { size } = geometry.operation else {
        return Err("expected box");
    };
    assert_eq!(size[0], Length::from_millimetres(7.0));
    Ok(())
}

#[test]
fn reports_unknown_missing_duplicate_arguments_symbols_and_types() {
    error("part P { material:M; body=unknown(); }", "E221");
    for (call, code) in [
        ("servo(unknown:1 mm)", "E236"),
        ("servo(width:1 mm,width:2 mm)", "E236"),
        ("servo(1 mm,width:2 mm)", "E236"),
        ("servo(width:1 mm,2 mm)", "E236"),
        ("servo(width:3 deg)", "E104"),
        ("servo(width:1 mm+2 deg)", "E104"),
        ("servo(width:1 mm*2 mm)", "E104"),
    ] {
        error(
            &format!("{SERVO} part P {{ material:M; body={call}; }}"),
            code,
        );
    }
    error("component f(width: Length) -> Solid { return box([width,width,width]); } part P { material:M; body=f(); }", "E236");
    error(
        "component f(width: Length, width: Length) -> Solid { return box([width,width,width]); }",
        "E230",
    );
    error(
        "component f(width: Nope) -> Solid { return box([1 mm,1 mm,1 mm]); }",
        "E231",
    );
    error("parameter width = 1 mm; parameter width = 2 mm;", "E230");
    error(
        "component f() -> Solid { a=1 mm; a=2 mm; return box([a,a,a]); }",
        "E230",
    );
    error(
        "component box() -> Solid { return cylinder(1 mm,1 mm); }",
        "E230",
    );
    error("part P { body=box([1 mm,1 mm,1 mm]); }", "E220");
    error(
        "part P { material:M; body=box([1 mm,1 mm,1 mm]); body=box([1 mm,1 mm,1 mm]); }",
        "E230",
    );
}

#[test]
fn recursion_invalid_dimensions_and_nonfinite_values_are_diagnostic() {
    let diagnostics = error("component first() -> Solid { return second(); } component second() -> Solid { return first(); } part P { material:M; body=first(); }", "E237");
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("called at")
            && diagnostic.message.contains("defined at")));
    for value in ["0 mm", "-1 mm", "1 mm/0"] {
        error(
            &format!("part P {{ material:M; body=box([{value},1 mm,1 mm]); }}"),
            if value.contains('/') { "E235" } else { "E238" },
        );
    }
    error(&format!("parameter HUGE = {} mm;", "9".repeat(400)), "E204");
    error("part P { material:M; body=box([1 mm,1 mm]); }", "E233");
    error(
        "component f() -> Solid { return 1 mm; } part P { material:M; body=f(); }",
        "E234",
    );
}

#[test]
fn expansion_depth_and_total_work_are_bounded() {
    let mut source = "component huge() -> Solid { shape0=box([1 mm,1 mm,1 mm]);".to_owned();
    for index in 1..20 {
        source.push_str(&format!(
            "shape{index}=union(shape{},shape{});",
            index - 1,
            index - 1
        ));
    }
    source.push_str("return shape19; } part P { material:M; body=huge(); }");
    error(&source, "E239");
    let mut source = String::new();
    for index in 0..60 {
        source.push_str(&format!(
            "component f{index}() -> Solid {{ return f{}(); }}",
            index + 1
        ));
    }
    source.push_str("component f60() -> Solid { return box([1 mm,1 mm,1 mm]); } part P { material:M; body=f0(); }");
    error(&source, "E239");
    let mut source = "component deep() -> Solid { shape0=box([1 mm,1 mm,1 mm]);".to_owned();
    for index in 1..60 {
        source.push_str(&format!(
            "shape{index}=translate([1 mm,0 mm,0 mm],shape{});",
            index - 1
        ));
    }
    source.push_str("return shape59; } part P { material:M; body=deep(); }");
    error(&source, "E239");
}

#[test]
fn total_expansion_budget_and_global_parameter_cycles_are_diagnostic() {
    let mut source = "component large() -> Solid { shape0=box([1 mm,1 mm,1 mm]);".to_owned();
    for index in 1..11 {
        source.push_str(&format!(
            "shape{index}=union(shape{},shape{});",
            index - 1,
            index - 1
        ));
    }
    source.push_str(
        "return shape10; } part P { material:M; first=large(); second=large(); third=large(); }",
    );
    let diagnostics = error(&source, "E239");
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("total expanded")));
    error(
        "parameter FIRST = SECOND; parameter SECOND = FIRST;",
        "E201",
    );
    let mut source = String::new();
    for index in 0..100 {
        source.push_str(&format!("parameter value{index} = value{};", index + 1));
    }
    source.push_str("parameter value100 = 1 mm;");
    error(&source, "E239");
}

#[test]
fn rotation_requires_angles_and_retains_source_provenance() -> Result<(), &'static str> {
    let model = valid("parameter TURN = 90 deg; part P { material:M; body=rotate(x:TURN,y:0 rad,z:-90 deg,shape:box([1 mm,2 mm,3 mm])); }");
    let Feature::Solid { geometry, .. } = &model.parts["P"].features[0] else {
        return Err("solid");
    };
    let SolidOperation::Rotate { angles, shape } = &geometry.operation else {
        return Err("rotation");
    };
    assert!((angles[0].degrees() - 90.0).abs() < 1e-10);
    assert!((angles[2].degrees() + 90.0).abs() < 1e-10);
    assert!(geometry.span.end > geometry.span.start);
    assert!(shape.span.start > geometry.span.start);
    for angle in ["90", "90 mm"] {
        error(
            &format!(
                "part P {{ material:M; body=rotate({angle},0 deg,0 deg,box([1 mm,2 mm,3 mm])); }}"
            ),
            "E104",
        );
    }
    error(
        "part P { material:M; body=rotate(x:0 deg,y:0 deg,shape:box([1 mm,2 mm,3 mm])); }",
        "E236",
    );
    error(
        "component rotate() -> Solid { return box([1 mm,1 mm,1 mm]); }",
        "E230",
    );
    Ok(())
}
