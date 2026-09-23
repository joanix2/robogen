//! Resolution and type/unit checking from syntax into the shared semantic model.

use robogen_domain::{
    Density, Diagnostic, FeatureId, Length, Material, MaterialId, ParameterId, PartId, Point2,
    Pressure, Quantity, QuantityKind, SketchId, SourceSpan,
};
use robogen_dsl::{Declaration, Expr, ExprKind, Module};
use robogen_sketch::{ConstraintKind, Plane, Rectangle, Sketch, SketchConstraint, SketchEntity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod evaluation;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticParameter {
    pub id: ParameterId,
    pub name: String,
    pub value: Quantity,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolidGeometry {
    pub operation: SolidOperation,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SolidOperation {
    Box { size: [Length; 3] },
    Cylinder { radius: Length, height: Length },
    Translate { offset: [Length; 3], shape: Box<SolidGeometry> },
    Union { shapes: Vec<SolidGeometry> },
    Difference { base: Box<SolidGeometry>, tools: Vec<SolidGeometry> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Feature {
    Extrude {
        id: FeatureId,
        name: String,
        sketch: SketchId,
        depth: Length,
        span: SourceSpan,
    },
    Solid {
        id: FeatureId,
        name: String,
        geometry: SolidGeometry,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub id: PartId,
    pub name: String,
    pub material: MaterialId,
    pub features: Vec<Feature>,
    pub source_span: SourceSpan,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticModel {
    pub module: String,
    pub parameters: BTreeMap<String, SemanticParameter>,
    pub materials: BTreeMap<String, Material>,
    pub sketches: BTreeMap<String, Sketch>,
    pub parts: BTreeMap<String, Part>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LowerOutput {
    pub model: SemanticModel,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lower(module: &Module) -> LowerOutput {
    let mut context = LowerContext {
        diagnostics: Vec::new(),
        parameters: BTreeMap::new(),
        resolving: BTreeSet::new(),
        components: BTreeMap::new(),
        resolved: BTreeMap::new(),
        calls: Vec::new(),
        arena: Vec::new(),
        steps: 0,
        emitted: 0,
        depth: 0,
    };
    let mut names = BTreeSet::new();
    for declaration in &module.declarations {
        let (name, span) = match declaration {
            Declaration::Parameter(value) => (&value.name, value.span),
            Declaration::Material(value) => (&value.name, value.span),
            Declaration::Sketch(value) => (&value.name, value.span),
            Declaration::Part(value) => (&value.name, value.span),
            Declaration::Component(value) => (&value.name, value.span),
        };
        if !names.insert(name) {
            context.error("E230", format!("duplicate declaration `{name}`"), span);
        }
        if let Declaration::Parameter(parameter) = declaration {
            context
                .parameters
                .insert(parameter.name.clone(), &parameter.value);
        }
            if let Declaration::Component(component) = declaration {
                context.validate_component(component);
                context.components.insert(component.name.clone(), component);
            }
    }

    let mut model = SemanticModel {
        module: module.name.clone(),
        ..SemanticModel::default()
    };
    for declaration in &module.declarations {
        match declaration {
            Declaration::Parameter(parameter) => {
                if let Some(value) = context.resolve_parameter(&parameter.name, parameter.span) {
                    let qualified = format!("{}::{}", module.name, parameter.name);
                    model.parameters.insert(
                        parameter.name.clone(),
                        SemanticParameter {
                            id: ParameterId::from_name(&qualified),
                            name: parameter.name.clone(),
                            value,
                            span: parameter.span,
                        },
                    );
                }
            }
            Declaration::Material(material) => {
                let qualified = format!("{}::{}", module.name, material.name);
                let density = context
                    .field(material, "density", QuantityKind::Density)
                    .and_then(as_density);
                let young = context
                    .field(material, "young", QuantityKind::Pressure)
                    .and_then(as_pressure);
                let poisson = context
                    .field(material, "poisson", QuantityKind::Scalar)
                    .and_then(as_scalar);
                let yield_strength = context
                    .field(material, "yield_strength", QuantityKind::Pressure)
                    .and_then(as_pressure);
                if let (
                    Some(density),
                    Some(young_modulus),
                    Some(poisson_ratio),
                    Some(yield_strength),
                ) = (density, young, poisson, yield_strength)
                {
                    model.materials.insert(
                        material.name.clone(),
                        Material {
                            id: MaterialId::from_name(&qualified),
                            name: material.name.clone(),
                            density,
                            young_modulus,
                            poisson_ratio,
                            yield_strength,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    for declaration in &module.declarations {
        if let Declaration::Sketch(sketch) = declaration {
            let plane = match sketch.plane.as_str() {
                "XY" => Some(Plane::XY),
                "XZ" => Some(Plane::XZ),
                "YZ" => Some(Plane::YZ),
                _ => {
                    context.error("E210", "expected sketch plane XY, XZ or YZ", sketch.span);
                    None
                }
            };
            let mut entities = Vec::new();
            for rectangle in &sketch.rectangles {
                let x = context.length(&rectangle.origin_x);
                let y = context.length(&rectangle.origin_y);
                let width = context.length(&rectangle.width);
                let height = context.length(&rectangle.height);
                if let (Some(x), Some(y), Some(width), Some(height)) = (x, y, width, height) {
                    entities.push(SketchEntity::Rectangle(Rectangle {
                        name: rectangle.name.clone(),
                        origin: Point2 { x, y },
                        width,
                        height,
                        span: rectangle.span,
                    }));
                }
            }
            let constraints = sketch
                .constraints
                .iter()
                .filter_map(|constraint| {
                    let reference = |expr: &Expr| {
                        if let ExprKind::Reference(path) = &expr.kind {
                            Some(path.clone())
                        } else {
                            None
                        }
                    };
                    let kind = match (constraint.name.as_str(), constraint.args.as_slice()) {
                        ("horizontal", [path]) => ConstraintKind::Horizontal(reference(path)?),
                        ("vertical", [path]) => ConstraintKind::Vertical(reference(path)?),
                        ("fix", [path]) => ConstraintKind::Fix(reference(path)?),
                        ("distance_x", [from, to, _]) => ConstraintKind::DistanceX {
                            from: reference(from)?,
                            to: reference(to)?,
                        },
                        ("distance_y", [from, to, _]) => ConstraintKind::DistanceY {
                            from: reference(from)?,
                            to: reference(to)?,
                        },
                        _ => {
                            context.error(
                                "E211",
                                format!(
                                    "unsupported or malformed constraint `{}`",
                                    constraint.name
                                ),
                                constraint.span,
                            );
                            return None;
                        }
                    };
                    let value = if matches!(
                        kind,
                        ConstraintKind::DistanceX { .. } | ConstraintKind::DistanceY { .. }
                    ) {
                        constraint.args.get(2).and_then(|expr| context.length(expr))
                    } else {
                        None
                    };
                    Some(SketchConstraint {
                        kind,
                        value,
                        span: constraint.span,
                    })
                })
                .collect();
            if let Some(plane) = plane {
                let qualified = format!("{}::{}", module.name, sketch.name);
                model.sketches.insert(
                    sketch.name.clone(),
                    Sketch {
                        id: SketchId::from_name(&qualified),
                        name: sketch.name.clone(),
                        plane,
                        entities,
                        constraints,
                        source_span: sketch.span,
                    },
                );
            }
        }
    }

    for declaration in &module.declarations {
        if let Declaration::Part(part) = declaration {
            let material = match part.material.as_ref() {
                Some(name) => model.materials.get(name).map(|value| value.id).or_else(|| {
                    context.error("E220", format!("unknown material `{name}`"), part.span);
                    None
                }),
                None => {
                    context.error(
                        "E220",
                        format!("part `{}` requires a material", part.name),
                        part.span,
                    );
                    None
                }
            };
            let mut features = Vec::new();
            let mut feature_names = BTreeSet::new();
            for feature in &part.features {
                if !feature_names.insert(&feature.name) {
                    context.error("E230", format!("duplicate feature `{}`", feature.name), feature.span);
                    continue;
                }
                let qualified = format!("{}::{}::{}", module.name, part.name, feature.name);
                if feature.operation != "extrude" {
                    if let Some(geometry) = context.solid_feature(feature) {
                        features.push(Feature::Solid {
                            id: FeatureId::from_name(&qualified),
                            name: feature.name.clone(), geometry, span: feature.span,
                        });
                    }
                    continue;
                }
                if feature.args.len() != 2 {
                    context.error("E222", "extrude expects exactly two positional arguments", feature.span);
                    continue;
                }
                let Some(Expr {
                    kind: ExprKind::Reference(sketch_name),
                    ..
                }) = feature.args.first()
                else {
                    context.error("E222", "extrude expects a sketch reference", feature.span);
                    continue;
                };
                let Some(sketch_id) = model.sketches.get(sketch_name).map(|value| value.id) else {
                    context.error(
                        "E223",
                        format!("unknown sketch `{sketch_name}`"),
                        feature.span,
                    );
                    continue;
                };
                let Some(depth) = feature.args.get(1).and_then(|expr| context.length(expr)) else {
                    context.error("E224", "extrude expects a length depth", feature.span);
                    continue;
                };
                let qualified = format!("{}::{}::{}", module.name, part.name, feature.name);
                features.push(Feature::Extrude {
                    id: FeatureId::from_name(&qualified),
                    name: feature.name.clone(),
                    sketch: sketch_id,
                    depth,
                    span: feature.span,
                });
            }
            if let Some(material) = material {
                let qualified = format!("{}::{}", module.name, part.name);
                model.parts.insert(
                    part.name.clone(),
                    Part {
                        id: PartId::from_name(&qualified),
                        name: part.name.clone(),
                        material,
                        features,
                        source_span: part.span,
                    },
                );
            }
        }
    }
    LowerOutput {
        model,
        diagnostics: context.diagnostics,
    }
}

struct LowerContext<'a> {
    diagnostics: Vec<Diagnostic>,
    parameters: BTreeMap<String, &'a Expr>,
    resolving: BTreeSet<String>,
    components: BTreeMap<String, &'a robogen_dsl::ComponentDecl>,
    resolved: BTreeMap<String, Quantity>,
    calls: Vec<(String, SourceSpan, SourceSpan)>,
    arena: Vec<evaluation::Node>,
    steps: usize,
    emitted: usize,
    depth: usize,
}

impl LowerContext<'_> {
    fn resolve_parameter(&mut self, name: &str, span: SourceSpan) -> Option<Quantity> {
        if let Some(value) = self.resolved.get(name) { return Some(*value); }
        if self.resolving.len() >= 32 {
            self.error("E239", "parameter dependency depth limit exceeded", span);
            return None;
        }
        if !self.resolving.insert(name.to_owned()) {
            self.error("E201", format!("cyclic parameter `{name}`"), span);
            return None;
        }
        let expression = self.parameters.get(name).copied();
        let result = match expression {
            Some(expression) => self.quantity(expression),
            None => {
                self.error("E202", format!("unknown parameter `{name}`"), span);
                None
            }
        };
        self.resolving.remove(name);
        if let Some(value) = result { self.resolved.insert(name.to_owned(), value); }
        result
    }

    fn quantity(&mut self, expression: &Expr) -> Option<Quantity> {
        let value = self.evaluate(expression, &BTreeMap::new())?;
        self.expect_quantity(value, expression.span)
    }

    fn length(&mut self, expression: &Expr) -> Option<Length> {
        let value = self.quantity(expression)?;
        if let Quantity::Length(value) = value {
            Some(value)
        } else {
            self.type_error(QuantityKind::Length, value.kind(), expression.span);
            None
        }
    }

    fn field(
        &mut self,
        material: &robogen_dsl::MaterialDecl,
        name: &str,
        expected: QuantityKind,
    ) -> Option<Quantity> {
        let Some(field) = material.fields.iter().find(|field| field.name == name) else {
            self.error(
                "E205",
                format!("material `{}` requires `{name}`", material.name),
                material.span,
            );
            return None;
        };
        let value = self.quantity(&field.value)?;
        if value.kind() != expected {
            self.type_error(expected, value.kind(), field.value.span);
            None
        } else {
            Some(value)
        }
    }

    fn type_error(&mut self, expected: QuantityKind, found: QuantityKind, span: SourceSpan) {
        self.error(
            "E104",
            format!("expected {expected:?}, found {found:?}"),
            span,
        );
    }
    fn error(&mut self, code: &str, message: impl Into<String>, span: SourceSpan) {
        let mut message = message.into();
        for (name, call, definition) in &self.calls {
            message.push_str(&format!("; component `{name}` called at {}..{}, defined at {}..{}", call.start, call.end, definition.start, definition.end));
        }
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }
}

fn parse_quantity(value: f64, unit: Option<&str>) -> Option<Quantity> {
    Some(match unit {
        None => Quantity::Scalar(value),
        Some("mm") => Quantity::Length(Length::from_millimetres(value)),
        Some("cm") => Quantity::Length(Length::from_metres(value / 100.0)),
        Some("m") => Quantity::Length(Length::from_metres(value)),
        Some("deg") => Quantity::Angle(robogen_domain::Angle::from_degrees(value)),
        Some("rad") => Quantity::Angle(robogen_domain::Angle::from_radians(value)),
        Some("kg/m3") => Quantity::Density(Density::from_kg_per_m3(value)),
        Some("g/cm3") => Quantity::Density(Density::from_kg_per_m3(value * 1000.0)),
        Some("Pa") => Quantity::Pressure(Pressure::from_pascals(value)),
        Some("kPa") => Quantity::Pressure(Pressure::from_pascals(value * 1e3)),
        Some("MPa") => Quantity::Pressure(Pressure::from_pascals(value * 1e6)),
        Some("GPa") => Quantity::Pressure(Pressure::from_pascals(value * 1e9)),
        Some(_) => return None,
    })
}

fn as_density(value: Quantity) -> Option<Density> {
    if let Quantity::Density(value) = value {
        Some(value)
    } else {
        None
    }
}
fn as_pressure(value: Quantity) -> Option<Pressure> {
    if let Quantity::Pressure(value) = value {
        Some(value)
    } else {
        None
    }
}
fn as_scalar(value: Quantity) -> Option<f64> {
    if let Quantity::Scalar(value) = value {
        Some(value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_unit_mismatch_without_panicking() -> Result<(), &'static str> {
        let parsed = robogen_dsl::parse("module x; parameter W = 10 deg; material M { density: 1 kg/m3; young: 2 Pa; poisson: 0.3; yield_strength: 2 Pa; } sketch S on XY { rectangle r { origin: (0 mm, 0 mm); width: W; height: 2 mm; } } part P { material: M; f = extrude(S, 2 mm); }");
        let module = parsed.module.as_ref().ok_or("expected a parsed module")?;
        let output = lower(module);
        assert!(output.diagnostics.iter().any(|d| d.code == "E104"));
        Ok(())
    }
}
