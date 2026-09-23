use robogen_domain::{Diagnostic, Length};
use robogen_ir::{lower, Feature, LowerOutput, SolidOperation};

const MATERIAL: &str = "material M { density: 1240 kg/m3; young: 3.5 GPa; poisson: 0.36; yield_strength: 50 MPa; }";
const SERVO: &str = "component servo(width: Length = 23 mm, depth: Length = 12 mm, height: Length = 24 mm, shaft_radius: Length = 2 mm) -> Solid { body = box(size:[width,depth,height]); shaft = translate(offset:[width/2,depth/2,height],shape:cylinder(radius:shaft_radius,height:5 mm)); return union(body,shaft); }";

fn compile(body: &str) -> LowerOutput {
    let parsed = robogen_dsl::parse(&format!("module test; {MATERIAL} {body}"));
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
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{body}: {:?}", output.diagnostics);
    output.diagnostics
}

#[test]
fn defaults_named_positional_independent_instances_and_provenance() -> Result<(), &'static str> {
    let body = format!("part P {{ material: M; first = servo(); second = servo(width:25 mm); third = servo(26 mm); }} {SERVO}");
    let model = valid(&body);
    let features = &model.parts["P"].features;
    let mut ids = Vec::new();
    for (feature, width) in features.iter().zip([23.0, 25.0, 26.0]) {
        let Feature::Solid { id, geometry, span, .. } = feature else { return Err("expected solid"); };
        ids.push(*id);
        assert_ne!(*span, geometry.span);
        let SolidOperation::Union { shapes } = &geometry.operation else { return Err("expected union"); };
        let SolidOperation::Box { size } = shapes[0].operation else { return Err("expected box"); };
        assert_eq!(size[0], Length::from_millimetres(width));
        let SolidOperation::Translate { offset, .. } = shapes[1].operation else { return Err("expected translation"); };
        assert_eq!(offset[0], Length::from_millimetres(width / 2.0));
    }
    assert_ne!(ids[0], ids[1]);
    let changed = valid(&body.replace("25 mm", "29 mm"));
    assert_eq!(features[0], changed.parts["P"].features[0]);
    let Feature::Solid { id, .. } = changed.parts["P"].features[1] else { return Err("expected solid"); };
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
    let Feature::Solid { geometry, .. } = &model.parts["P"].features[0] else { return Err("expected solid"); };
    let SolidOperation::Box { size } = geometry.operation else { return Err("expected box"); };
    assert_eq!(size[0], Length::from_millimetres(7.0));
    Ok(())
}

#[test]
fn reports_unknown_missing_duplicate_arguments_symbols_and_types() {
    error("part P { material:M; body=unknown(); }", "E221");
    for (call, code) in [("servo(unknown:1 mm)", "E236"), ("servo(width:1 mm,width:2 mm)", "E236"), ("servo(1 mm,width:2 mm)", "E236"), ("servo(width:1 mm,2 mm)", "E236"), ("servo(width:3 deg)", "E104"), ("servo(width:1 mm+2 deg)", "E104"), ("servo(width:1 mm*2 mm)", "E104")] {
        error(&format!("{SERVO} part P {{ material:M; body={call}; }}"), code);
    }
    error("component f(width: Length) -> Solid { return box([width,width,width]); } part P { material:M; body=f(); }", "E236");
    error("component f(width: Length, width: Length) -> Solid { return box([width,width,width]); }", "E230");
    error("component f(width: Nope) -> Solid { return box([1 mm,1 mm,1 mm]); }", "E231");
    error("parameter width = 1 mm; parameter width = 2 mm;", "E230");
    error("component f() -> Solid { a=1 mm; a=2 mm; return box([a,a,a]); }", "E230");
    error("component box() -> Solid { return cylinder(1 mm,1 mm); }", "E230");
    error("part P { body=box([1 mm,1 mm,1 mm]); }", "E220");
    error("part P { material:M; body=box([1 mm,1 mm,1 mm]); body=box([1 mm,1 mm,1 mm]); }", "E230");
}

#[test]
fn recursion_invalid_dimensions_and_nonfinite_values_are_diagnostic() {
    let diagnostics = error("component first() -> Solid { return second(); } component second() -> Solid { return first(); } part P { material:M; body=first(); }", "E237");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.message.contains("called at") && diagnostic.message.contains("defined at")));
    for value in ["0 mm", "-1 mm", "1 mm/0"] {
        error(&format!("part P {{ material:M; body=box([{value},1 mm,1 mm]); }}"), if value.contains('/') { "E235" } else { "E238" });
    }
    error(&format!("parameter HUGE = {} mm;", "9".repeat(400)), "E204");
    error("part P { material:M; body=box([1 mm,1 mm]); }", "E233");
    error("component f() -> Solid { return 1 mm; } part P { material:M; body=f(); }", "E234");
}

#[test]
fn expansion_depth_and_total_work_are_bounded() {
    let mut source = "component huge() -> Solid { shape0=box([1 mm,1 mm,1 mm]);".to_owned();
    for index in 1..20 { source.push_str(&format!("shape{index}=union(shape{},shape{});", index-1,index-1)); }
    source.push_str("return shape19; } part P { material:M; body=huge(); }");
    error(&source, "E239");
    let mut source = String::new();
    for index in 0..60 { source.push_str(&format!("component f{index}() -> Solid {{ return f{}(); }}", index+1)); }
    source.push_str("component f60() -> Solid { return box([1 mm,1 mm,1 mm]); } part P { material:M; body=f0(); }");
    error(&source, "E239");
    let mut source = "component deep() -> Solid { shape0=box([1 mm,1 mm,1 mm]);".to_owned();
    for index in 1..60 { source.push_str(&format!("shape{index}=translate([1 mm,0 mm,0 mm],shape{});",index-1)); }
    source.push_str("return shape59; } part P { material:M; body=deep(); }");
    error(&source, "E239");
}

#[test]
fn total_expansion_budget_and_global_parameter_cycles_are_diagnostic() {
    let mut source = "component large() -> Solid { shape0=box([1 mm,1 mm,1 mm]);".to_owned();
    for index in 1..11 {
        source.push_str(&format!("shape{index}=union(shape{},shape{});", index-1,index-1));
    }
    source.push_str("return shape10; } part P { material:M; first=large(); second=large(); third=large(); }");
    let diagnostics = error(&source, "E239");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.message.contains("total expanded")));
    error("parameter FIRST = SECOND; parameter SECOND = FIRST;", "E201");
    let mut source = String::new();
    for index in 0..100 { source.push_str(&format!("parameter value{index} = value{};", index+1)); }
    source.push_str("parameter value100 = 1 mm;");
    error(&source, "E239");
}