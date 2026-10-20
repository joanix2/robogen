use crate::{Project, SourceDocument};
use robogen_domain::{Diagnostic, FeatureId, PartId, SourceSpan};
use robogen_dsl::{Declaration, Expr, ExprKind};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryGenerator {
    Servo,
    Camera,
    JetsonNanoSuper,
    Battery,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    pub signature: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaxonomyFeature {
    pub id: FeatureId,
    pub name: String,
    pub functions: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaxonomyInstance {
    pub id: PartId,
    pub name: String,
    pub features: Vec<TaxonomyFeature>,
    pub span: SourceSpan,
}

impl Project {
    pub fn function_definitions(&self) -> Vec<FunctionDefinition> {
        self.ast
            .declarations
            .iter()
            .filter_map(|declaration| {
                let Declaration::Component(component) = declaration else {
                    return None;
                };
                let parameters = component
                    .parameters
                    .iter()
                    .map(|parameter| {
                        let default = parameter
                            .default
                            .as_ref()
                            .map(|value| {
                                format!(" = {}", &self.source[value.span.start..value.span.end])
                            })
                            .unwrap_or_default();
                        format!("{}: {}{default}", parameter.name, parameter.type_name)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(FunctionDefinition {
                    name: component.name.clone(),
                    signature: format!("{}({parameters})", component.name),
                    span: component.span,
                })
            })
            .collect()
    }

    pub fn taxonomy_instances(&self) -> Vec<TaxonomyInstance> {
        let functions: BTreeSet<_> = self
            .function_definitions()
            .into_iter()
            .map(|function| function.name)
            .collect();
        self.snapshot
            .model
            .parts
            .values()
            .map(|part| {
                let syntax =
                    self.ast
                        .declarations
                        .iter()
                        .find_map(|declaration| match declaration {
                            Declaration::Part(value) if value.name == part.name => Some(value),
                            _ => None,
                        });
                let features = part
                    .features
                    .iter()
                    .map(|feature| {
                        let (id, name, span) = match feature {
                            robogen_ir::Feature::Solid { id, name, span, .. }
                            | robogen_ir::Feature::Extrude { id, name, span, .. } => {
                                (*id, name, *span)
                            }
                        };
                        let mut called = BTreeSet::new();
                        if let Some(feature) = syntax.and_then(|part| {
                            part.features.iter().find(|feature| feature.name == *name)
                        }) {
                            if functions.contains(&feature.operation) {
                                called.insert(feature.operation.clone());
                            }
                            let mut pending: Vec<&Expr> = feature.args.iter().collect();
                            while let Some(expression) = pending.pop() {
                                match &expression.kind {
                                    ExprKind::Call { name, args } => {
                                        if functions.contains(name) {
                                            called.insert(name.clone());
                                        }
                                        pending.extend(args);
                                    }
                                    ExprKind::Vector(values) => pending.extend(values),
                                    ExprKind::Named { value, .. }
                                    | ExprKind::Unary { value, .. } => pending.push(value),
                                    ExprKind::Binary { left, right, .. } => {
                                        pending.push(left);
                                        pending.push(right);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        TaxonomyFeature {
                            id,
                            name: name.clone(),
                            functions: called.into_iter().collect(),
                            span,
                        }
                    })
                    .collect();
                TaxonomyInstance {
                    id: part.id,
                    name: part.name.clone(),
                    features,
                    span: part.source_span,
                }
            })
            .collect()
    }
}

impl LibraryGenerator {
    pub const ALL: [Self; 4] = [
        Self::Servo,
        Self::Camera,
        Self::JetsonNanoSuper,
        Self::Battery,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Servo => "servo",
            Self::Camera => "raspberry_pi_camera",
            Self::JetsonNanoSuper => "jetson_nano_super",
            Self::Battery => "li_ion_battery",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Servo => "Servomoteur",
            Self::Camera => "Camera Raspberry Pi",
            Self::JetsonNanoSuper => "Jetson Nano Super",
            Self::Battery => "Batterie Li-ion 1S",
        }
    }

    fn template(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Servo => (
                include_str!("../../../examples/servo/main.rgn"),
                "lib_servo_",
                "ServoPlastic",
            ),
            Self::Camera => (
                include_str!("../../../examples/raspberry_pi_camera/main.rgn"),
                "lib_camera_",
                "DisplayOnly",
            ),
            Self::JetsonNanoSuper => (
                include_str!("../../../examples/compute_board/main.rgn"),
                "lib_jetson_",
                "DisplayOnly",
            ),
            Self::Battery => (
                include_str!("../../../examples/li_ion_battery/main.rgn"),
                "lib_battery_",
                "DisplayOnly",
            ),
        }
    }

    pub fn definitions(self) -> Result<String, Vec<Diagnostic>> {
        let (source, prefix, _) = self.template();
        robogen_dsl::component_library(source, prefix, self.name())
    }
}

pub fn declaration_name(declaration: &Declaration) -> &str {
    match declaration {
        Declaration::Parameter(value) => &value.name,
        Declaration::Material(value) => &value.name,
        Declaration::Sketch(value) => &value.name,
        Declaration::Component(value) => &value.name,
        Declaration::Part(value) => &value.name,
        Declaration::Instance(value) => &value.name,
        Declaration::Body(value) | Declaration::Joint(value) => &value.name,
    }
}

impl SourceDocument {
    pub fn instantiate_generator(
        &mut self,
        generator: LibraryGenerator,
    ) -> Result<String, Vec<Diagnostic>> {
        if !self.is_valid() {
            return Err(vec![Diagnostic::error(
                "E331",
                "wait for a valid document before inserting an instance",
                SourceSpan::default(),
            )]);
        }
        let parsed = robogen_dsl::parse(self.source());
        let Some(module) = parsed.module else {
            return Err(parsed.diagnostics);
        };
        let mut names: BTreeSet<_> = module
            .declarations
            .iter()
            .map(|declaration| declaration_name(declaration).to_owned())
            .collect();
        let existing = module.declarations.iter().any(|declaration| matches!(declaration, Declaration::Component(component) if component.name == generator.name()));
        let mut source = self.source().to_owned();
        let material = if existing {
            let previous = self.project().and_then(|project| {
                project.taxonomy_instances().into_iter().find(|instance| {
                    instance.features.iter().any(|feature| {
                        feature
                            .functions
                            .iter()
                            .any(|name| name == generator.name())
                    })
                })
            });
            let previous_material = previous.and_then(|instance| {
                module
                    .declarations
                    .iter()
                    .find_map(|declaration| match declaration {
                        Declaration::Part(part) if part.name == instance.name => {
                            part.material.clone()
                        }
                        _ => None,
                    })
            });
            previous_material
                .or_else(|| {
                    module.declarations.iter().find_map(|declaration| {
                        if let Declaration::Material(value) = declaration {
                            Some(value.name.clone())
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| {
                    vec![Diagnostic::error(
                        "E331",
                        "an instance needs a material declaration",
                        module.span,
                    )]
                })?
        } else {
            let definitions = generator.definitions()?;
            let imported = robogen_dsl::parse(&format!("module library;\n{definitions}"));
            let Some(imported) = imported.module else {
                return Err(imported.diagnostics);
            };
            for declaration in &imported.declarations {
                if !names.insert(declaration_name(declaration).to_owned()) {
                    return Err(vec![Diagnostic::error(
                        "E331",
                        format!("library name conflict: {}", declaration_name(declaration)),
                        module.span,
                    )]);
                }
            }
            source.push_str("\n\n");
            source.push_str(&definitions);
            let (_, prefix, material) = generator.template();
            format!("{prefix}{material}")
        };
        let mut number = 1;
        let instance = loop {
            let name = format!("{}_{number}", generator.name());
            if !names.contains(&name) {
                break name;
            }
            number += 1;
        };
        let x_mm = self
            .project()
            .into_iter()
            .flat_map(|project| project.snapshot().meshes.values())
            .flat_map(|mesh| &mesh.vertices)
            .map(|vertex| f64::from(vertex.position[0]) * 1000.0)
            .reduce(f64::max)
            .map_or(0.0, |maximum| maximum + 10.0);
        source.push_str(&format!("\npart {instance} {{\n    material: {material};\n    body = translate([{x_mm:.4} mm, 0 mm, 0 mm], {}());\n}}\n", generator.name()));
        self.replace_source(source);
        Ok(instance)
    }
}

#[cfg(test)]
mod tests;
