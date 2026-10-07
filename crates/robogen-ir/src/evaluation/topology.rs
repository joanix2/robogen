use super::{magnitude, LowerContext, Scope};
use crate::topology::*;
use robogen_domain::{
    FeatureId, Force, InterfaceId, LoadCaseId, OptimizationId, Quantity, QuantityKind, SourceSpan,
    Torque,
};
use robogen_dsl::{Expr, ExprKind};
use std::collections::{BTreeMap, BTreeSet};

impl LowerContext<'_> {
    fn topology_args<'e>(
        &mut self,
        args: &'e [Expr],
        names: &[&str],
        span: SourceSpan,
    ) -> Option<Vec<&'e Expr>> {
        let values = self.arguments(args, names, span)?;
        let mut result = Vec::new();
        for (value, name) in values.into_iter().zip(names) {
            let Some(value) = value else {
                self.error("E236", format!("missing argument `{name}`"), span);
                return None;
            };
            result.push(value);
        }
        Some(result)
    }

    fn topology_list<'e>(&mut self, expr: &'e Expr, min: usize, max: usize) -> Option<&'e [Expr]> {
        if let ExprKind::Vector(values) = &expr.kind {
            if (min..=max).contains(&values.len()) {
                return Some(values);
            }
        }
        self.error(
            "E250",
            format!("expected a list with {min} to {max} entries"),
            expr.span,
        );
        None
    }

    fn topology_name(&mut self, expr: &Expr) -> Option<String> {
        if let ExprKind::String(name) = &expr.kind {
            if !name.is_empty()
                && name.len() <= 64
                && name.bytes().enumerate().all(|(index, byte)| {
                    byte == b'_'
                        || byte.is_ascii_alphabetic()
                        || (index > 0 && byte.is_ascii_digit())
                })
            {
                return Some(name.clone());
            }
        }
        self.error(
            "E251",
            "expected a quoted identifier (1..64 ASCII letters/digits/underscores)",
            expr.span,
        );
        None
    }

    fn topology_call<'e>(&mut self, expr: &'e Expr) -> Option<(&'e str, &'e [Expr])> {
        if let ExprKind::Call { name, args } = &expr.kind {
            return Some((name, args));
        }
        self.error("E250", "expected a declarative constructor call", expr.span);
        None
    }

    fn topology_quantity(
        &mut self,
        expr: &Expr,
        kind: QuantityKind,
        scope: &Scope,
    ) -> Option<Quantity> {
        let value = self.evaluate(expr, scope)?;
        let value = self.expect_quantity(value, expr.span)?;
        if value.kind() != kind {
            self.type_error(kind, value.kind(), expr.span);
            return None;
        }
        Some(value)
    }

    fn topology_vector(
        &mut self,
        expr: &Expr,
        kind: QuantityKind,
        scope: &Scope,
    ) -> Option<[f64; 3]> {
        let entries = self.topology_list(expr, 3, 3)?;
        let mut values = [0.0; 3];
        for (entry, value) in entries.iter().zip(&mut values) {
            *value = magnitude(self.topology_quantity(entry, kind, scope)?);
        }
        if values.iter().all(|value| *value == 0.0) {
            self.error("E252", "vector must be nonzero", expr.span);
            return None;
        }
        if kind == QuantityKind::Scalar
            && (values.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() > 1e-6
        {
            self.error(
                "E252",
                "direction must be a dimensionless unit vector",
                expr.span,
            );
            return None;
        }
        Some(values)
    }

    fn topology_solid(&mut self, expr: &Expr, scope: &Scope) -> Option<usize> {
        let value = self.evaluate(expr, scope)?;
        self.expect_solid(value, expr.span)
    }

    fn topology_target(
        &mut self,
        expr: &Expr,
        interfaces: &BTreeMap<String, InterfaceId>,
    ) -> Option<InterfaceId> {
        let name = self.topology_name(expr)?;
        if let Some(id) = interfaces.get(&name) {
            return Some(*id);
        }
        self.error(
            "E253",
            format!("unknown preserved interface `{name}`"),
            expr.span,
        );
        None
    }

    pub(super) fn topology(
        &mut self,
        args: &[Expr],
        scope: &Scope,
        span: SourceSpan,
    ) -> Option<TopologySpec<usize>> {
        let fields = self.topology_args(
            args,
            &[
                "name",
                "domain",
                "preserve",
                "void",
                "material",
                "load_cases",
                "objective",
                "constraints",
                "manufacturing",
            ],
            span,
        )?;
        let name = self.topology_name(fields[0])?;
        if !self.topology_names.insert(name.clone()) {
            self.error(
                "E254",
                format!("duplicate topology name `{name}` in this feature"),
                fields[0].span,
            );
            return None;
        }
        let qualified = format!("{}::topology::{name}", self.feature_key);
        let domain = self.topology_solid(fields[1], scope)?;
        let mut preserve = Vec::new();
        let mut interfaces = BTreeMap::new();
        for entry in self.topology_list(fields[2], 1, 64)? {
            let (interface, geometry) = if let ExprKind::Call { name, args } = &entry.kind {
                if name == "interface" {
                    let values = self.topology_args(args, &["name", "shape"], entry.span)?;
                    let name = self.topology_name(values[0])?;
                    let id = InterfaceId::from_name(&format!("{qualified}::interface::{name}"));
                    if interfaces.insert(name.clone(), id).is_some() {
                        self.error(
                            "E254",
                            format!("duplicate preserved interface `{name}`"),
                            entry.span,
                        );
                        return None;
                    }
                    (
                        Some(Interface { id, name }),
                        self.topology_solid(values[1], scope)?,
                    )
                } else {
                    (None, self.topology_solid(entry, scope)?)
                }
            } else {
                (None, self.topology_solid(entry, scope)?)
            };
            preserve.push(PreservedRegion {
                interface,
                geometry,
                span: entry.span,
            });
        }
        let mut voids = Vec::new();
        for entry in self.topology_list(fields[3], 0, 64)? {
            voids.push(self.topology_solid(entry, scope)?);
        }
        let ExprKind::Reference(material_name) = &fields[4].kind else {
            self.error(
                "E255",
                "expected a declared material reference",
                fields[4].span,
            );
            return None;
        };
        let Some(material) = self.materials.get(material_name) else {
            self.error(
                "E255",
                format!("unknown topology material `{material_name}`"),
                fields[4].span,
            );
            return None;
        };
        if material.density.kg_per_m3() <= 0.0
            || material.young_modulus.pascals() <= 0.0
            || material.yield_strength.pascals() <= 0.0
            || !(-1.0..0.5).contains(&material.poisson_ratio)
            || material.poisson_ratio == -1.0
        {
            self.error("E255", "topology material requires positive density/Young/yield strength and -1 < poisson < 0.5", fields[4].span);
            return None;
        }
        let material = material.id;
        let objective = match &fields[6].kind {
            ExprKind::Reference(name) if name == "minimize_mass" => Objective::MinimizeMass,
            ExprKind::Reference(name) if name == "maximize_stiffness" => {
                Objective::MaximizeStiffness
            }
            _ => {
                self.error(
                    "E256",
                    "objective must be minimize_mass or maximize_stiffness",
                    fields[6].span,
                );
                return None;
            }
        };
        let mut load_cases = Vec::new();
        let mut names = BTreeSet::new();
        for entry in self.topology_list(fields[5], 1, 16)? {
            let (constructor, args) = self.topology_call(entry)?;
            if constructor != "load_case" {
                self.error("E250", "expected load_case(...) declaration", entry.span);
                return None;
            }
            let values = self.topology_args(args, &["name", "loads", "supports"], entry.span)?;
            let name = self.topology_name(values[0])?;
            if !names.insert(name.clone()) {
                self.error("E254", "duplicate load-case name", values[0].span);
                return None;
            }
            let mut loads = Vec::new();
            for entry in self.topology_list(values[1], 1, 64)? {
                let (name, args) = self.topology_call(entry)?;
                let kind = match name {
                    "force" => QuantityKind::Force,
                    "moment" => QuantityKind::Torque,
                    _ => {
                        self.error("E257", format!("unsupported load `{name}`"), entry.span);
                        return None;
                    }
                };
                let values = self.topology_args(args, &["on", "vector"], entry.span)?;
                let target = self.topology_target(values[0], &interfaces)?;
                let vector = self.topology_vector(values[1], kind, scope)?;
                loads.push(Load {
                    target,
                    span: entry.span,
                    kind: if kind == QuantityKind::Force {
                        LoadKind::Force(vector.map(Force::from_newtons))
                    } else {
                        LoadKind::Moment(vector.map(Torque::from_newton_metres))
                    },
                });
            }
            let mut supports = Vec::new();
            let mut supported = BTreeSet::new();
            for entry in self.topology_list(values[2], 1, 64)? {
                let (name, args) = self.topology_call(entry)?;
                let keys: &[&str] = match name {
                    "fixed" => &["on"],
                    "pin" => &["on", "axis"],
                    "frictionless" => &["on", "normal"],
                    _ => {
                        self.error("E257", format!("unsupported support `{name}`"), entry.span);
                        return None;
                    }
                };
                let values = self.topology_args(args, keys, entry.span)?;
                let target = self.topology_target(values[0], &interfaces)?;
                if !supported.insert(target) {
                    self.error(
                        "E254",
                        "duplicate/conflicting support on interface in load case",
                        entry.span,
                    );
                    return None;
                }
                let kind = if name == "fixed" {
                    SupportKind::Fixed
                } else {
                    let axis = self.topology_vector(values[1], QuantityKind::Scalar, scope)?;
                    if name == "pin" {
                        SupportKind::Pin { axis }
                    } else {
                        SupportKind::Frictionless { normal: axis }
                    }
                };
                supports.push(Support {
                    target,
                    span: entry.span,
                    kind,
                });
            }
            load_cases.push(LoadCase {
                id: LoadCaseId::from_name(&format!("{qualified}::load_case::{name}")),
                name,
                span: entry.span,
                loads,
                supports,
            });
        }
        let mut constraints = Vec::new();
        let mut seen = BTreeSet::new();
        for entry in self.topology_list(fields[7], 1, 16)? {
            let (name, args) = self.topology_call(entry)?;
            if !seen.insert(name) {
                self.error("E254", "duplicate constraint", entry.span);
                return None;
            }
            let kind = match name {
                "max_displacement" => QuantityKind::Length,
                "max_stress" => QuantityKind::Pressure,
                "safety_factor" | "volume_fraction" => QuantityKind::Scalar,
                "max_mass" => QuantityKind::Mass,
                "min_frequency" => QuantityKind::Frequency,
                _ => {
                    self.error(
                        "E258",
                        format!("unsupported constraint `{name}`"),
                        entry.span,
                    );
                    return None;
                }
            };
            let values = self.topology_args(args, &["value"], entry.span)?;
            let value = self.topology_quantity(values[0], kind, scope)?;
            if magnitude(value) <= 0.0
                || (name == "volume_fraction" && magnitude(value) > 1.0)
                || (name == "safety_factor" && magnitude(value) < 1.0)
            {
                self.error(
                    "E252",
                    "constraint must be positive; volume_fraction <= 1 and safety_factor >= 1",
                    entry.span,
                );
                return None;
            }
            let kind = match (name, value) {
                ("max_displacement", Quantity::Length(value)) => {
                    ConstraintKind::MaxDisplacement(value)
                }
                ("max_stress", Quantity::Pressure(value)) => ConstraintKind::MaxStress(value),
                ("safety_factor", Quantity::Scalar(value)) => ConstraintKind::SafetyFactor(value),
                ("volume_fraction", Quantity::Scalar(value)) => {
                    ConstraintKind::VolumeFraction(value)
                }
                ("max_mass", Quantity::Mass(value)) => ConstraintKind::MaxMass(value),
                ("min_frequency", Quantity::Frequency(value)) => {
                    ConstraintKind::MinFrequency(value)
                }
                _ => return None,
            };
            constraints.push(Constraint {
                span: entry.span,
                kind,
            });
        }
        let mut manufacturing = Vec::new();
        for entry in self.topology_list(fields[8], 0, 1)? {
            let (name, args) = self.topology_call(entry)?;
            let process = match name {
                "fdm" => {
                    let values = self.topology_args(
                        args,
                        &[
                            "build_direction",
                            "min_thickness",
                            "min_hole",
                            "max_overhang",
                        ],
                        entry.span,
                    )?;
                    let build_direction =
                        self.topology_vector(values[0], QuantityKind::Scalar, scope)?;
                    let min_thickness = self.topology_positive_length(values[1], scope)?;
                    let min_hole = self.topology_positive_length(values[2], scope)?;
                    let Quantity::Angle(max_overhang) =
                        self.topology_quantity(values[3], QuantityKind::Angle, scope)?
                    else {
                        return None;
                    };
                    if max_overhang.degrees() < 0.0 || max_overhang.degrees() > 90.0 {
                        self.error(
                            "E252",
                            "FDM overhang angle must be in 0..90 deg",
                            values[3].span,
                        );
                        return None;
                    }
                    ManufacturingProcess::Fdm {
                        build_direction,
                        min_thickness,
                        min_hole,
                        max_overhang,
                    }
                }
                "cnc_3axis" => {
                    let values =
                        self.topology_args(args, &["tool_direction", "tool_diameter"], entry.span)?;
                    ManufacturingProcess::Cnc3Axis {
                        tool_direction: self.topology_vector(
                            values[0],
                            QuantityKind::Scalar,
                            scope,
                        )?,
                        tool_diameter: self.topology_positive_length(values[1], scope)?,
                    }
                }
                _ => {
                    self.error(
                        "E258",
                        format!("unsupported manufacturing process `{name}`"),
                        entry.span,
                    );
                    return None;
                }
            };
            manufacturing.push(Manufacturing {
                span: entry.span,
                process,
            });
        }
        Some(TopologySpec {
            schema_version: TOPOLOGY_SCHEMA_VERSION,
            id: OptimizationId::from_name(&qualified),
            name,
            feature: FeatureId::from_name(&self.feature_key),
            span,
            domain,
            preserve,
            voids,
            material,
            load_cases,
            objective,
            constraints,
            manufacturing,
        })
    }

    fn topology_positive_length(
        &mut self,
        expr: &Expr,
        scope: &Scope,
    ) -> Option<robogen_domain::Length> {
        let Quantity::Length(value) = self.topology_quantity(expr, QuantityKind::Length, scope)?
        else {
            return None;
        };
        self.positive(value, expr.span)?;
        Some(value)
    }
}
