//! Source resolution and quantity checking for declared rigid bodies and joints.
use super::{LowerContext, SemanticModel};
use robogen_domain::{mechanics::*, JointId, Length, Quantity, RigidBodyId, SourceSpan};
use robogen_dsl::{Declaration, Expr, ExprKind, Module, PropertyBlock};
use std::collections::BTreeMap;

impl LowerContext<'_> {
    pub(super) fn lower_mechanics(&mut self, module: &Module, model: &mut SemanticModel) {
        for declaration in &module.declarations {
            if let Declaration::Body(body) = declaration {
                let fields = self.mechanical_fields(body, &["parts", "grounded"]);
                let mut parts = Vec::new();
                if let Some(expression) = fields.get("parts") {
                    if let ExprKind::Vector(values) = &expression.kind {
                        for value in values {
                            if let Some(name) = self.mechanical_name(value) {
                                if let Some(part) = model.parts.get(name) {
                                    parts.push(part.id);
                                } else {
                                    self.error(
                                        "E361",
                                        format!("unknown part `{name}`"),
                                        value.span,
                                    );
                                }
                            }
                        }
                    } else {
                        self.error(
                            "E360",
                            "parts expects a list of part names",
                            expression.span,
                        );
                    }
                }
                let grounded =
                    fields
                        .get("grounded")
                        .is_some_and(|value| match self.mechanical_name(value) {
                            Some("true") => true,
                            Some("false") => false,
                            _ => {
                                self.error("E360", "grounded expects true or false", value.span);
                                false
                            }
                        });
                let id = RigidBodyId::from_name(&format!("{}::{}", module.name, body.name));
                model.mechanics.bodies.insert(
                    id,
                    RigidBody {
                        id,
                        name: body.name.clone(),
                        parts,
                        grounded,
                        span: body.span,
                    },
                );
            }
        }
        for declaration in &module.declarations {
            if let Declaration::Joint(joint) = declaration {
                if let Some(record) = self.lower_joint(module, joint, model) {
                    model.mechanics.joints.insert(record.id, record);
                }
            }
        }
    }

    fn lower_joint(
        &mut self,
        module: &Module,
        joint: &PropertyBlock,
        model: &SemanticModel,
    ) -> Option<KinematicJoint> {
        let fields = self.mechanical_fields(
            joint,
            &[
                "kind", "parent", "child", "origin", "axis", "pitch", "actuator",
            ],
        );
        let kind_expr = self.mechanical_required(&fields, "kind", joint.span)?;
        let kind = match self.mechanical_name(kind_expr)? {
            "fixed" => JointKind::Fixed,
            "revolute" => JointKind::Revolute,
            "prismatic" => JointKind::Prismatic,
            "cylindrical" => JointKind::Cylindrical,
            "helical" => JointKind::Helical,
            name => {
                self.error(
                    "E362",
                    format!("unsupported joint kind `{name}`"),
                    kind_expr.span,
                );
                return None;
            }
        };
        let parent_expr = self.mechanical_required(&fields, "parent", joint.span)?;
        let parent = self.mechanical_body(parent_expr, model)?;
        let child_expr = self.mechanical_required(&fields, "child", joint.span)?;
        let child = self.mechanical_body(child_expr, model)?;
        let origin_expr = self.mechanical_required(&fields, "origin", joint.span)?;
        let origin = self.mechanical_vector(origin_expr, true)?;
        let axis = if let Some(expr) = fields.get("axis") {
            let direction = self.mechanical_vector(expr, false)?;
            // Normalize finite nonzero directions; invalid data stays explicit for Robotics.
            let norm = direction
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            Some(AxisDirection(if norm.is_finite() && norm > 0.0 {
                direction.map(|value| value / norm)
            } else {
                direction
            }))
        } else {
            None
        };
        let pitch = if let Some(expr) = fields.get("pitch") {
            match self.quantity(expr) {
                Some(Quantity::Length(value)) => Some(value),
                _ => {
                    self.error("E360", "helical pitch expects a length per turn", expr.span);
                    return None;
                }
            }
        } else {
            None
        };
        let actuator = if let Some(expr) = fields.get("actuator") {
            let name = self.mechanical_name(expr)?;
            match model.instances.get(name) {
                Some(instance) => Some(instance.id),
                None => {
                    self.error(
                        "E361",
                        format!("unknown actuator instance `{name}`"),
                        expr.span,
                    );
                    return None;
                }
            }
        } else {
            None
        };
        Some(KinematicJoint {
            id: JointId::from_name(&format!("{}::{}", module.name, joint.name)),
            name: joint.name.clone(),
            parent,
            child,
            kind,
            origin: origin.map(Length::from_metres),
            axis,
            pitch,
            actuator,
            span: joint.span,
        })
    }

    fn mechanical_fields<'a>(
        &mut self,
        block: &'a PropertyBlock,
        allowed: &[&str],
    ) -> BTreeMap<&'a str, &'a Expr> {
        let mut fields = BTreeMap::new();
        for field in &block.fields {
            if !allowed.contains(&field.name.as_str()) {
                self.error(
                    "E360",
                    format!("unknown mechanical property `{}`", field.name),
                    field.span,
                );
            }
            if fields.insert(field.name.as_str(), &field.value).is_some() {
                self.error(
                    "E360",
                    format!("duplicate mechanical property `{}`", field.name),
                    field.span,
                );
            }
        }
        fields
    }

    fn mechanical_required<'a>(
        &mut self,
        fields: &BTreeMap<&str, &'a Expr>,
        name: &str,
        span: SourceSpan,
    ) -> Option<&'a Expr> {
        let expression = fields.get(name).copied();
        if expression.is_none() {
            self.error("E360", format!("joint requires `{name}`"), span);
        }
        expression
    }

    fn mechanical_name<'a>(&mut self, expression: &'a Expr) -> Option<&'a str> {
        match &expression.kind {
            ExprKind::Reference(name) | ExprKind::String(name) => Some(name),
            _ => {
                self.error("E360", "expected a mechanical name", expression.span);
                None
            }
        }
    }

    fn mechanical_body(&mut self, expression: &Expr, model: &SemanticModel) -> Option<RigidBodyId> {
        let name = self.mechanical_name(expression)?;
        let id = model
            .mechanics
            .bodies
            .values()
            .find(|body| body.name == name)
            .map(|body| body.id);
        if id.is_none() {
            self.error(
                "E361",
                format!("unknown rigid body `{name}`"),
                expression.span,
            );
        }
        id
    }

    fn mechanical_vector(&mut self, expression: &Expr, lengths: bool) -> Option<[f64; 3]> {
        if let ExprKind::Vector(values) = &expression.kind {
            if values.len() == 3 {
                let mut result = [0.0; 3];
                for (slot, value) in result.iter_mut().zip(values) {
                    *slot = match (self.quantity(value)?, lengths) {
                        (Quantity::Length(value), true) => value.metres(),
                        (Quantity::Scalar(value), false) => value,
                        _ => {
                            self.error(
                                "E360",
                                if lengths {
                                    "origin requires lengths"
                                } else {
                                    "axis requires dimensionless scalars"
                                },
                                value.span,
                            );
                            return None;
                        }
                    };
                }
                return Some(result);
            }
        }
        self.error(
            "E360",
            "expected a vector of three coordinates",
            expression.span,
        );
        None
    }
}
