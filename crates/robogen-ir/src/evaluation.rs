use super::{parse_quantity, LowerContext, SolidGeometry, SolidOperation};
use robogen_domain::{Length, Quantity, QuantityKind, SourceSpan};
use robogen_dsl::{ComponentDecl, Expr, ExprKind, FeatureDecl};
use std::collections::{BTreeMap, BTreeSet};

const NODE_LIMIT: usize = 4096;
const DEPTH_LIMIT: usize = 48;
const STEP_LIMIT: usize = 32768;

#[derive(Clone, Copy)]
pub(super) enum Value {
    Quantity(Quantity),
    Vector([Length; 3]),
    Solid(usize),
    Unbound,
}

pub(super) struct Node {
    operation: Operation,
    span: SourceSpan,
    expanded: usize,
    height: usize,
}

enum Operation {
    Box([Length; 3]),
    Cylinder(Length, Length),
    Translate([Length; 3], usize),
    Union(Vec<usize>),
    Difference(usize, Vec<usize>),
}

type Scope = BTreeMap<String, Value>;

impl LowerContext<'_> {
    pub(super) fn validate_component(&mut self, component: &ComponentDecl) {
        if matches!(component.name.as_str(), "box" | "cylinder" | "translate" | "union" | "difference" | "extrude") {
            self.error("E230", "component name conflicts with a built-in constructor", component.span);
        }
        let mut names = BTreeSet::new();
        for parameter in &component.parameters {
            if !names.insert(&parameter.name) {
                self.error("E230", format!("duplicate parameter `{}`", parameter.name), parameter.span);
            }
            if type_kind(&parameter.type_name).is_none() {
                self.error("E231", format!("unsupported parameter type `{}`", parameter.type_name), parameter.span);
            }
        }
        for binding in &component.bindings {
            if !names.insert(&binding.name) {
                self.error("E230", format!("duplicate local binding `{}`", binding.name), binding.span);
            }
        }
    }

    pub(super) fn solid_feature(&mut self, feature: &FeatureDecl) -> Option<SolidGeometry> {
        let before = self.diagnostics.len();
        let value = self.call(&feature.operation, &feature.args, &Scope::new(), feature.span);
        let result = value.and_then(|value| self.expect_solid(value, feature.span)).and_then(|index| {
            let count = self.arena[index].expanded;
            if count > NODE_LIMIT.saturating_sub(self.emitted) {
                self.error("E239", "total expanded solid node budget exceeded", feature.span);
                None
            } else {
                self.emitted += count;
                Some(self.materialize(index))
            }
        });
        if self.diagnostics.len() != before {
            self.error("E240", format!("failed to expand feature `{}`", feature.name), feature.span);
        }
        result
    }

    pub(super) fn evaluate(&mut self, expression: &Expr, scope: &Scope) -> Option<Value> {
        self.steps += 1;
        if self.steps > STEP_LIMIT || self.depth >= DEPTH_LIMIT {
            self.error("E239", "evaluation step or depth limit exceeded", expression.span);
            return None;
        }
        self.depth += 1;
        let value = self.evaluate_inner(expression, scope);
        self.depth -= 1;
        value
    }

    fn evaluate_inner(&mut self, expression: &Expr, scope: &Scope) -> Option<Value> {
        let span = expression.span;
        match &expression.kind {
            ExprKind::Number { value, unit } => {
                let quantity = parse_quantity(*value, unit.as_deref());
                match quantity {
                    Some(quantity) if magnitude(quantity).is_finite() => Some(Value::Quantity(quantity)),
                    _ => { self.error("E204", "unsupported unit or nonfinite quantity", span); None }
                }
            }
            ExprKind::Reference(name) => match scope.get(name).copied() {
                Some(Value::Unbound) => { self.error("E232", format!("parameter `{name}` is not bound yet"), span); None }
                Some(value) => Some(value),
                None => self.resolve_parameter(name, span).map(Value::Quantity),
            },
            ExprKind::Unary { operator, value } => {
                let value = self.evaluate(value, scope)?;
                let quantity = self.expect_quantity(value, span)?;
                let factor = if *operator == '-' { -1.0 } else { 1.0 };
                Some(Value::Quantity(with_magnitude(quantity.kind(), magnitude(quantity) * factor)))
            }
            ExprKind::Binary { operator, left, right } => {
                let left = self.evaluate(left, scope)?;
                let right = self.evaluate(right, scope)?;
                let left = self.expect_quantity(left, span)?;
                let right = self.expect_quantity(right, span)?;
                self.arithmetic(*operator, left, right, span).map(Value::Quantity)
            }
            ExprKind::Vector(expressions) => {
                if expressions.len() != 3 {
                    self.error("E233", "expected a three-length vector", span);
                    return None;
                }
                let mut values = [Length::ZERO; 3];
                for (target, expression) in values.iter_mut().zip(expressions) {
                    let value = self.evaluate(expression, scope)?;
                    *target = self.expect_length(value, expression.span)?;
                }
                Some(Value::Vector(values))
            }
            ExprKind::Call { name, args } => self.call(name, args, scope, span),
            ExprKind::String(_) | ExprKind::Named { .. } => {
                self.error("E203", "expected a quantity, vector or solid expression", span);
                None
            }
        }
    }

    pub(super) fn expect_quantity(&mut self, value: Value, span: SourceSpan) -> Option<Quantity> {
        if let Value::Quantity(value) = value { Some(value) } else {
            self.error("E203", "expected physical quantity", span); None
        }
    }

    fn expect_length(&mut self, value: Value, span: SourceSpan) -> Option<Length> {
        let value = self.expect_quantity(value, span)?;
        if let Quantity::Length(value) = value { Some(value) } else {
            self.type_error(QuantityKind::Length, value.kind(), span); None
        }
    }

    fn expect_vector(&mut self, value: Value, span: SourceSpan) -> Option<[Length; 3]> {
        if let Value::Vector(value) = value { Some(value) } else {
            self.error("E233", "expected a three-length vector", span); None
        }
    }

    fn expect_solid(&mut self, value: Value, span: SourceSpan) -> Option<usize> {
        if let Value::Solid(value) = value { Some(value) } else {
            self.error("E234", "expected Solid", span); None
        }
    }

    fn arithmetic(&mut self, operator: char, left: Quantity, right: Quantity, span: SourceSpan) -> Option<Quantity> {
        let left_kind = left.kind();
        let right_kind = right.kind();
        let kind = match operator {
            '+' | '-' if left_kind == right_kind => left_kind,
            '*' if left_kind == QuantityKind::Scalar => right_kind,
            '*' | '/' if right_kind == QuantityKind::Scalar => left_kind,
            '/' if left_kind == right_kind => QuantityKind::Scalar,
            _ => {
                self.error("E104", format!("incompatible units for `{operator}`: {left_kind:?} and {right_kind:?}"), span);
                return None;
            }
        };
        let left = magnitude(left);
        let right = magnitude(right);
        let result = match operator {
            '+' => left + right, '-' => left - right, '*' => left * right, '/' => left / right,
            _ => return None,
        };
        if !result.is_finite() {
            self.error("E235", "nonfinite arithmetic result (including division by zero)", span);
            return None;
        }
        Some(with_magnitude(kind, result))
    }

    fn arguments<'expression>(&mut self, args: &'expression [Expr], names: &[&str], span: SourceSpan) -> Option<Vec<Option<&'expression Expr>>> {
        let mut values = vec![None; names.len()];
        let mut position = 0;
        let mut named_seen = false;
        for argument in args {
            let (index, value) = if let ExprKind::Named { name, value } = &argument.kind {
                named_seen = true;
                let Some(index) = names.iter().position(|expected| *expected == name) else {
                    self.error("E236", format!("unknown argument `{name}`"), argument.span);
                    return None;
                };
                (index, value.as_ref())
            } else {
                if named_seen || position >= names.len() {
                    self.error("E236", "unexpected positional argument (positional arguments must precede named arguments)", argument.span);
                    return None;
                }
                let index = position;
                position += 1;
                (index, argument)
            };
            if values[index].replace(value).is_some() {
                self.error("E236", format!("duplicate argument `{}`", names[index]), span);
                return None;
            }
        }
        Some(values)
    }

    fn call(&mut self, name: &str, args: &[Expr], caller: &Scope, span: SourceSpan) -> Option<Value> {
        if let Some(component) = self.components.get(name).copied() {
            if self.calls.iter().any(|(active, _, _)| active == name) {
                self.error("E237", format!("recursive component `{name}` (definition {}..{})", component.span.start, component.span.end), span);
                return None;
            }
            if self.calls.len() >= 24 {
                self.error("E239", "component call depth limit exceeded", span);
                return None;
            }
            self.calls.push((name.to_owned(), span, component.span));
            let result = self.component_call(component, args, caller, span);
            self.calls.pop();
            return result;
        }
        self.builtin(name, args, caller, span)
    }

    fn component_call(&mut self, component: &ComponentDecl, args: &[Expr], caller: &Scope, span: SourceSpan) -> Option<Value> {
        let names: Vec<_> = component.parameters.iter().map(|parameter| parameter.name.as_str()).collect();
        let arguments = self.arguments(args, &names, span)?;
        let mut scope: Scope = names.iter().map(|name| ((*name).to_owned(), Value::Unbound)).collect();
        for (parameter, argument) in component.parameters.iter().zip(arguments) {
            let value = if let Some(argument) = argument {
                self.evaluate(argument, caller)?
            } else if let Some(default) = &parameter.default {
                self.evaluate(default, &scope)?
            } else {
                self.error("E236", format!("missing argument `{}`", parameter.name), span);
                return None;
            };
            let quantity = self.expect_quantity(value, span)?;
            let expected = type_kind(&parameter.type_name)?;
            if quantity.kind() != expected {
                self.type_error(expected, quantity.kind(), span);
                return None;
            }
            scope.insert(parameter.name.clone(), value);
        }
        for binding in &component.bindings {
            let value = self.evaluate(&binding.value, &scope)?;
            scope.insert(binding.name.clone(), value);
        }
        let value = self.evaluate(&component.result, &scope)?;
        self.expect_solid(value, component.result.span).map(Value::Solid)
    }

    fn builtin(&mut self, name: &str, args: &[Expr], scope: &Scope, span: SourceSpan) -> Option<Value> {
        if matches!(name, "union" | "difference") {
            let indices = if args.iter().any(|arg| matches!(arg.kind, ExprKind::Named { .. })) {
                let names: &[&str] = if name == "union" { &["left", "right"] } else { &["base", "tool"] };
                let values = self.required_arguments(args, names, scope, span)?;
                values.into_iter().map(|value| self.expect_solid(value, span)).collect::<Option<Vec<_>>>()?
            } else {
                if args.len() < 2 {
                    self.error("E236", "union/difference require at least two solids", span);
                    return None;
                }
                let mut indices = Vec::new();
                for argument in args {
                    let value = self.evaluate(argument, scope)?;
                    indices.push(self.expect_solid(value, argument.span)?);
                }
                indices
            };
            let operation = if name == "union" { Operation::Union(indices) } else {
                Operation::Difference(indices[0], indices[1..].to_vec())
            };
            return self.node(operation, span).map(Value::Solid);
        }
        let operation = match name {
            "box" => {
                let values = self.required_arguments(args, &["size"], scope, span)?;
                let size = self.expect_vector(values[0], span)?;
                for length in size { self.positive(length, span)?; }
                Operation::Box(size)
            }
            "cylinder" => {
                let values = self.required_arguments(args, &["radius", "height"], scope, span)?;
                let radius = self.expect_length(values[0], span)?;
                let height = self.expect_length(values[1], span)?;
                self.positive(radius, span)?;
                self.positive(height, span)?;
                Operation::Cylinder(radius, height)
            }
            "translate" => {
                let values = self.required_arguments(args, &["offset", "shape"], scope, span)?;
                let offset = self.expect_vector(values[0], span)?;
                let shape = self.expect_solid(values[1], span)?;
                Operation::Translate(offset, shape)
            }
            _ => {
                self.error("E221", format!("unknown constructor `{name}`; only same-file components are available"), span);
                return None;
            }
        };
        self.node(operation, span).map(Value::Solid)
    }

    fn required_arguments(&mut self, args: &[Expr], names: &[&str], scope: &Scope, span: SourceSpan) -> Option<Vec<Value>> {
        let arguments = self.arguments(args, names, span)?;
        let mut values = Vec::new();
        for (name, argument) in names.iter().zip(arguments) {
            let Some(argument) = argument else {
                self.error("E236", format!("missing argument `{name}`"), span);
                return None;
            };
            values.push(self.evaluate(argument, scope)?);
        }
        Some(values)
    }

    fn positive(&mut self, length: Length, span: SourceSpan) -> Option<()> {
        if length.metres().is_finite() && length.metres() > 0.0 { Some(()) } else {
            self.error("E238", "solid dimensions must be finite and strictly positive", span); None
        }
    }

    fn node(&mut self, operation: Operation, span: SourceSpan) -> Option<usize> {
        let children: Vec<usize> = match &operation {
            Operation::Box(_) | Operation::Cylinder(..) => Vec::new(),
            Operation::Translate(_, shape) => vec![*shape],
            Operation::Union(shapes) => shapes.clone(),
            Operation::Difference(base, tools) => std::iter::once(*base).chain(tools.iter().copied()).collect(),
        };
        let mut expanded = 1usize;
        let mut height = 1;
        for child in children {
            expanded = expanded.saturating_add(self.arena[child].expanded);
            height = height.max(self.arena[child].height + 1);
        }
        if expanded > NODE_LIMIT || height > DEPTH_LIMIT || self.arena.len() >= NODE_LIMIT {
            self.error("E239", "solid expansion node or depth limit exceeded", span);
            return None;
        }
        let index = self.arena.len();
        self.arena.push(Node { operation, span, expanded, height });
        Some(index)
    }

    fn materialize(&self, index: usize) -> SolidGeometry {
        let node = &self.arena[index];
        let operation = match &node.operation {
            Operation::Box(size) => SolidOperation::Box { size: *size },
            Operation::Cylinder(radius, height) => SolidOperation::Cylinder { radius: *radius, height: *height },
            Operation::Translate(offset, shape) => SolidOperation::Translate { offset: *offset, shape: Box::new(self.materialize(*shape)) },
            Operation::Union(shapes) => SolidOperation::Union { shapes: shapes.iter().map(|index| self.materialize(*index)).collect() },
            Operation::Difference(base, tools) => SolidOperation::Difference { base: Box::new(self.materialize(*base)), tools: tools.iter().map(|index| self.materialize(*index)).collect() },
        };
        SolidGeometry { operation, span: node.span }
    }
}

fn type_kind(name: &str) -> Option<QuantityKind> {
    Some(match name {
        "Length" => QuantityKind::Length, "Scalar" => QuantityKind::Scalar,
        "Angle" => QuantityKind::Angle, "Density" => QuantityKind::Density,
        "Pressure" => QuantityKind::Pressure, _ => return None,
    })
}

fn magnitude(value: Quantity) -> f64 {
    match value {
        Quantity::Scalar(value) => value,
        Quantity::Length(value) => value.metres(),
        Quantity::Angle(value) => value.radians(),
        Quantity::Density(value) => value.kg_per_m3(),
        Quantity::Pressure(value) => value.pascals(),
    }
}

fn with_magnitude(kind: QuantityKind, value: f64) -> Quantity {
    match kind {
        QuantityKind::Scalar => Quantity::Scalar(value),
        QuantityKind::Length => Quantity::Length(Length::from_metres(value)),
        QuantityKind::Angle => Quantity::Angle(robogen_domain::Angle::from_radians(value)),
        QuantityKind::Density => Quantity::Density(robogen_domain::Density::from_kg_per_m3(value)),
        QuantityKind::Pressure => Quantity::Pressure(robogen_domain::Pressure::from_pascals(value)),
    }
}