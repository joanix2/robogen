//! Object/instance evaluation: reusable object values, named instances and
//! their ordinary solid projections with source provenance.

use super::{LowerContext, ObjectNode, Operation, Scope, Value};
use crate::instances::{InstanceRegion, InstanceRegionReference, ObjectInstance};
use robogen_domain::{Angle, FeatureId, Length, ObjectInstanceId, SourceSpan};
use robogen_dsl::{Expr, ExprKind};
use std::collections::BTreeMap;

impl LowerContext<'_> {
    pub(crate) fn resolve_instance(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Option<ObjectInstanceId> {
        let id = ObjectInstanceId::from_name(&format!("{}::{}", self.module_name, name));
        if self.instance_objects.contains_key(&id) {
            return Some(id);
        }
        if self.instance_resolving.len() >= 32 {
            self.error("E239", "instance dependency depth limit exceeded", span);
            return None;
        }
        if !self.instance_resolving.insert(name.to_owned()) {
            self.error("E201", format!("cyclic instance `{name}`"), span);
            return None;
        }
        let declaration = self.instance_declarations.get(name).copied();
        let result = match declaration {
            Some(declaration) => {
                let value = self.evaluate(&declaration.value, &BTreeMap::new())?;
                let index = self.expect_object(value, declaration.value.span)?;
                self.instance_values.insert(name.to_owned(), index);
                self.instance_objects.insert(id, index);
                Some(id)
            }
            None => {
                self.error("E202", format!("unknown instance `{name}`"), span);
                None
            }
        };
        self.instance_resolving.remove(name);
        result
    }

    pub(crate) fn materialize_instances(
        &self,
        module_name: &str,
    ) -> BTreeMap<String, ObjectInstance> {
        let mut result = BTreeMap::new();
        for (name, &index) in &self.instance_values {
            let object = &self.objects[index];
            let span = self
                .instance_declarations
                .get(name)
                .map_or_else(SourceSpan::default, |declaration| declaration.span);
            result.insert(
                name.clone(),
                ObjectInstance {
                    id: ObjectInstanceId::from_name(&format!("{module_name}::{name}")),
                    name: name.clone(),
                    shape: self.materialize(object.shape),
                    voids: object
                        .voids
                        .iter()
                        .map(|&index| self.materialize(index))
                        .collect(),
                    interfaces: object
                        .interfaces
                        .iter()
                        .map(|(name, &index)| (name.clone(), self.materialize(index)))
                        .collect(),
                    span,
                },
            );
        }
        result
    }

    pub(crate) fn object_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        scope: &Scope,
        span: SourceSpan,
    ) -> Option<Value> {
        match name {
            "object" => {
                let fields = self.topology_args(args, &["shape", "void", "interfaces"], span)?;
                let shape = self.topology_solid(fields[0], scope)?;
                let mut voids = Vec::new();
                for entry in self.topology_list(fields[1], 0, 64)? {
                    voids.push(self.topology_solid(entry, scope)?);
                }
                let mut interfaces = BTreeMap::new();
                for entry in self.topology_list(fields[2], 0, 64)? {
                    let (constructor, interface_args) = self.object_call(entry)?;
                    if constructor != "interface" {
                        self.error(
                            "E250",
                            "expected interface(...) in interfaces list",
                            entry.span,
                        );
                        return None;
                    }
                    let values =
                        self.topology_args(interface_args, &["name", "shape"], entry.span)?;
                    let name = self.topology_name(values[0])?;
                    if interfaces.contains_key(&name) {
                        self.error("E254", format!("duplicate interface `{name}`"), entry.span);
                        return None;
                    }
                    let shape = self.topology_solid(values[1], scope)?;
                    interfaces.insert(name, shape);
                }
                let index = self.objects.len();
                self.objects.push(ObjectNode {
                    shape,
                    voids,
                    interfaces,
                });
                Some(Value::Object(index))
            }
            "instance_shape" => {
                let fields = self.topology_args(args, &["instance"], span)?;
                let id = self.instance_value(fields[0], scope)?;
                let object = self.object_of(id, span)?;
                self.record_reference(id, InstanceRegion::Shape, span);
                Some(Value::Solid(object.shape))
            }
            "instance_void" => {
                let fields = self.topology_args(args, &["instance"], span)?;
                let id = self.instance_value(fields[0], scope)?;
                let object = self.object_of(id, span)?;
                let index = match object.voids.as_slice() {
                    [] => {
                        self.error("E234", "instance has no void regions", span);
                        return None;
                    }
                    [single] => *single,
                    multiple => self.node(Operation::Compound(multiple.to_vec()), span)?,
                };
                self.record_reference(id, InstanceRegion::Void, span);
                Some(Value::Solid(index))
            }
            "instance_interface" => {
                let fields = self.topology_args(args, &["instance", "name"], span)?;
                let id = self.instance_value(fields[0], scope)?;
                let name = self.topology_name(fields[1])?;
                let object = self.object_of(id, span)?;
                let Some(&shape) = object.interfaces.get(&name) else {
                    self.error(
                        "E253",
                        format!("unknown instance interface `{name}`"),
                        fields[1].span,
                    );
                    return None;
                };
                self.record_reference(id, InstanceRegion::Interface(name), span);
                Some(Value::Solid(shape))
            }
            _ => {
                self.error("E221", format!("unknown constructor `{name}`"), span);
                None
            }
        }
    }

    pub(crate) fn transform_object(
        &mut self,
        index: usize,
        offset: Option<[Length; 3]>,
        angles: Option<[Angle; 3]>,
        span: SourceSpan,
    ) -> Option<usize> {
        let object = self.objects[index].clone();
        let shape = self.transform_solid(object.shape, offset, angles, span)?;
        let mut voids = Vec::with_capacity(object.voids.len());
        for &void in &object.voids {
            voids.push(self.transform_solid(void, offset, angles, span)?);
        }
        let mut interfaces = BTreeMap::new();
        for (name, &interface) in &object.interfaces {
            interfaces.insert(
                name.clone(),
                self.transform_solid(interface, offset, angles, span)?,
            );
        }
        let index = self.objects.len();
        self.objects.push(ObjectNode {
            shape,
            voids,
            interfaces,
        });
        Some(index)
    }

    fn expect_object(&mut self, value: Value, span: SourceSpan) -> Option<usize> {
        if let Value::Object(index) = value {
            Some(index)
        } else {
            self.error("E234", "expected Object", span);
            None
        }
    }

    fn instance_value(&mut self, expr: &Expr, scope: &Scope) -> Option<ObjectInstanceId> {
        let value = self.evaluate(expr, scope)?;
        if let Value::Instance(id) = value {
            Some(id)
        } else {
            self.error("E234", "expected Instance", expr.span);
            None
        }
    }

    fn object_of(&mut self, id: ObjectInstanceId, span: SourceSpan) -> Option<ObjectNode> {
        let Some(&index) = self.instance_objects.get(&id) else {
            self.error("E234", "instance is not resolved", span);
            return None;
        };
        Some(self.objects[index].clone())
    }

    fn record_reference(
        &mut self,
        instance: ObjectInstanceId,
        region: InstanceRegion,
        span: SourceSpan,
    ) {
        self.instance_references.push(InstanceRegionReference {
            instance,
            region,
            feature: FeatureId::from_name(&self.feature_key),
            span,
        });
    }

    fn transform_solid(
        &mut self,
        solid: usize,
        offset: Option<[Length; 3]>,
        angles: Option<[Angle; 3]>,
        span: SourceSpan,
    ) -> Option<usize> {
        match (offset, angles) {
            (Some(offset), _) => self.node(Operation::Translate(offset, solid), span),
            (None, Some(angles)) => self.node(Operation::Rotate(angles, solid), span),
            (None, None) => Some(solid),
        }
    }

    fn object_call<'e>(&mut self, expr: &'e Expr) -> Option<(&'e str, &'e [Expr])> {
        if let ExprKind::Call { name, args } = &expr.kind {
            return Some((name, args));
        }
        self.error("E250", "expected a constructor call", expr.span);
        None
    }
}
