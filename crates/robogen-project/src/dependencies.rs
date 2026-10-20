//! Pure source dependency discovery and typed quantity lowering.

use crate::DirtyNode;
use robogen_domain::{Quantity, SourceSpan};
use robogen_dsl::{Declaration, Expr, ExprKind, Module};
use std::collections::BTreeSet;

pub(super) fn dependencies_for<'a>(
    ast: &Module,
    parameters: impl Iterator<Item = &'a String>,
) -> BTreeSet<DirtyNode> {
    let mut parameters: BTreeSet<&str> = parameters.map(String::as_str).collect();
    loop {
        let before = parameters.len();
        for declaration in &ast.declarations {
            if let Declaration::Parameter(parameter) = declaration {
                if references_any(&parameter.value, &parameters) {
                    parameters.insert(parameter.name.as_str());
                }
            }
        }
        if parameters.len() == before {
            break;
        }
    }
    let mut dirty = BTreeSet::new();
    for parameter in &parameters {
        dirty.insert(DirtyNode::Parameter((*parameter).to_owned()));
    }
    let mut sketches = BTreeSet::new();
    for declaration in &ast.declarations {
        if let Declaration::Sketch(sketch) = declaration {
            let depends = sketch.rectangles.iter().any(|rect| {
                [&rect.origin_x, &rect.origin_y, &rect.width, &rect.height]
                    .into_iter()
                    .any(|expr| references_any(expr, &parameters))
            }) || sketch
                .constraints
                .iter()
                .flat_map(|constraint| &constraint.args)
                .any(|expr| references_any(expr, &parameters));
            if depends {
                sketches.insert(sketch.name.as_str());
                dirty.insert(DirtyNode::Sketch(sketch.name.clone()));
            }
        }
    }
    for declaration in &ast.declarations {
        if let Declaration::Part(part) = declaration {
            let dependencies: BTreeSet<_> = sketches.union(&parameters).copied().collect();
            let depends = part.features.iter().any(|feature| {
                (!parameters.is_empty() && feature.operation != "extrude")
                    || feature
                        .args
                        .iter()
                        .any(|expression| references_any(expression, &dependencies))
            });
            if depends {
                dirty.insert(DirtyNode::Part(part.name.clone()));
            }
        }
    }
    dirty
}

fn references_any(expression: &Expr, names: &BTreeSet<&str>) -> bool {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Reference(name) if names.contains(name.as_str()) => return true,
            ExprKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Unary { value, .. } | ExprKind::Named { value, .. } => pending.push(value),
            ExprKind::Vector(values) | ExprKind::Call { args: values, .. } => {
                pending.extend(values)
            }
            _ => {}
        }
    }
    false
}

pub(super) fn quantity_expr(value: Quantity, span: SourceSpan) -> Expr {
    let (value, unit) = match value {
        Quantity::Scalar(value) => (value, None),
        Quantity::Length(value) => (value.metres(), Some("m".into())),
        Quantity::Angle(value) => (value.radians(), Some("rad".into())),
        Quantity::Density(value) => (value.kg_per_m3(), Some("kg/m3".into())),
        Quantity::Pressure(value) => (value.pascals(), Some("Pa".into())),
        Quantity::Force(value) => (value.newtons(), Some("N".into())),
        Quantity::Torque(value) => (value.newton_metres(), Some("Nm".into())),
        Quantity::Mass(value) => (value.kilograms(), Some("kg".into())),
        Quantity::Frequency(value) => (value.hertz(), Some("Hz".into())),
    };
    Expr {
        kind: ExprKind::Number { value, unit },
        span,
    }
}
