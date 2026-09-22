//! Core value types shared by RoboGen's projections.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v5(
                    &Uuid::NAMESPACE_OID,
                    Uuid::new_v4().as_bytes(),
                ))
            }
            /// Creates a stable id from a module-qualified declaration name.
            pub fn from_name(name: &str) -> Self {
                Self(Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes()))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

typed_id!(ParameterId);
typed_id!(MaterialId);
typed_id!(SketchId);
typed_id!(PartId);
typed_id!(FeatureId);
typed_id!(EntityId);

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Length(f64);
impl Length {
    pub const ZERO: Self = Self(0.0);
    pub fn from_metres(value: f64) -> Self {
        Self(value)
    }
    pub fn from_millimetres(value: f64) -> Self {
        Self(value / 1000.0)
    }
    pub fn metres(self) -> f64 {
        self.0
    }
    pub fn millimetres(self) -> f64 {
        self.0 * 1000.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Angle(f64);
impl Angle {
    pub fn from_radians(value: f64) -> Self {
        Self(value)
    }
    pub fn from_degrees(value: f64) -> Self {
        Self(value.to_radians())
    }
    pub fn radians(self) -> f64 {
        self.0
    }
    pub fn degrees(self) -> f64 {
        self.0.to_degrees()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Density(f64);
impl Density {
    pub fn from_kg_per_m3(value: f64) -> Self {
        Self(value)
    }
    pub fn kg_per_m3(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pressure(f64);
impl Pressure {
    pub fn from_pascals(value: f64) -> Self {
        Self(value)
    }
    pub fn pascals(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Quantity {
    Scalar(f64),
    Length(Length),
    Angle(Angle),
    Density(Density),
    Pressure(Pressure),
}

impl Quantity {
    pub fn kind(self) -> QuantityKind {
        match self {
            Self::Scalar(_) => QuantityKind::Scalar,
            Self::Length(_) => QuantityKind::Length,
            Self::Angle(_) => QuantityKind::Angle,
            Self::Density(_) => QuantityKind::Density,
            Self::Pressure(_) => QuantityKind::Pressure,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantityKind {
    Scalar,
    Length,
    Angle,
    Density,
    Pressure,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: Length,
    pub y: Length,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: Length,
    pub y: Length,
    pub z: Length,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub density: Density,
    pub young_modulus: Pressure,
    pub poisson_ratio: f64,
    pub yield_strength: Pressure,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}
impl SourceSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub span: SourceSpan,
    pub severity: Severity,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span,
            severity: Severity::Error,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaxonomyEntry {
    pub id: EntityId,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaxonomyCategory {
    pub id: EntityId,
    pub label: String,
    pub entries: Vec<TaxonomyEntry>,
}

/// Domain-owned component catalog with globally unique category and entry IDs.
/// M1 extensions register categories directly; module loading is future work.
/// Search matching belongs to presentation, not this registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentTaxonomy {
    categories: Vec<TaxonomyCategory>,
}

impl ComponentTaxonomy {
    pub fn categories(&self) -> &[TaxonomyCategory] {
        &self.categories
    }

    /// Appends a category only after validating all its IDs and labels.
    /// IDs share one namespace; labels must contain non-whitespace text.
    /// Errors leave the catalog unchanged and have no source span.
    pub fn register_category(&mut self, category: TaxonomyCategory) -> Result<(), Diagnostic> {
        let mut ids: BTreeSet<_> = self
            .categories
            .iter()
            .flat_map(|existing| {
                std::iter::once(existing.id).chain(existing.entries.iter().map(|entry| entry.id))
            })
            .collect();
        for (id, label) in std::iter::once((category.id, category.label.as_str())).chain(
            category
                .entries
                .iter()
                .map(|entry| (entry.id, entry.label.as_str())),
        ) {
            if label.trim().is_empty() {
                return Err(Diagnostic::error(
                    "taxonomy.empty_label",
                    format!("Taxonomy label for {id} must not be empty"),
                    SourceSpan::default(),
                ));
            }
            if !ids.insert(id) {
                return Err(Diagnostic::error(
                    "taxonomy.duplicate_id",
                    format!("Duplicate taxonomy ID {id}"),
                    SourceSpan::default(),
                ));
            }
        }
        self.categories.push(category);
        Ok(())
    }
}

impl Default for ComponentTaxonomy {
    fn default() -> Self {
        type BuiltinEntries = &'static [(&'static str, &'static str)];
        let groups: &[(&str, &str, BuiltinEntries)] = &[
            (
                "body",
                "Corps",
                &[
                    ("chassis", "Ch\u{e2}ssis"),
                    ("shell", "Coque"),
                    ("support", "Support"),
                ],
            ),
            (
                "limb",
                "Membre",
                &[
                    ("leg", "Patte"),
                    ("arm", "Bras"),
                    ("wheel", "Roue"),
                    ("wing", "Aile"),
                    ("tail", "Queue"),
                ],
            ),
            (
                "joint",
                "Articulation",
                &[
                    ("ball", "Rotule"),
                    ("revolute", "Pivot"),
                    ("prismatic", "Prismatique"),
                ],
            ),
            (
                "actuator",
                "Actionneur",
                &[
                    ("servo", "Servo"),
                    ("bldc_motor", "Moteur BLDC"),
                    ("cylinder", "V\u{e9}rin"),
                ],
            ),
            (
                "sensor",
                "Capteur",
                &[
                    ("imu", "IMU"),
                    ("camera", "Cam\u{e9}ra"),
                    ("force_sensor", "Capteur de force"),
                ],
            ),
            (
                "material",
                "Mat\u{e9}riau",
                &[
                    ("pla", "PLA"),
                    ("resin", "R\u{e9}sine"),
                    ("aluminium", "Aluminium"),
                    ("titanium", "Titane"),
                    ("carbon", "Carbone"),
                ],
            ),
        ];
        Self {
            categories: groups
                .iter()
                .map(|(name, label, entries)| {
                    let path = format!("robogen::taxonomy::builtin::{name}");
                    TaxonomyCategory {
                        id: EntityId::from_name(&path),
                        label: (*label).into(),
                        entries: entries
                            .iter()
                            .map(|(name, label)| TaxonomyEntry {
                                id: EntityId::from_name(&format!("{path}::{name}")),
                                label: (*label).into(),
                            })
                            .collect(),
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn units_have_explicit_conversions() {
        assert_eq!(Length::from_millimetres(12.0).metres(), 0.012);
        assert!((Angle::from_degrees(180.0).radians() - std::f64::consts::PI).abs() < 1e-12);
    }
    #[test]
    fn named_ids_are_stable() {
        assert_eq!(
            PartId::from_name("sample::Part"),
            PartId::from_name("sample::Part")
        );
    }

    #[test]
    fn taxonomy_defaults_include_all_french_groups_and_resin() {
        let taxonomy = ComponentTaxonomy::default();
        let actual: Vec<_> = taxonomy
            .categories()
            .iter()
            .map(|category| {
                (
                    category.label.as_str(),
                    category
                        .entries
                        .iter()
                        .map(|entry| entry.label.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            actual,
            vec![
                ("Corps", vec!["Ch\u{e2}ssis", "Coque", "Support"]),
                ("Membre", vec!["Patte", "Bras", "Roue", "Aile", "Queue"]),
                ("Articulation", vec!["Rotule", "Pivot", "Prismatique"]),
                ("Actionneur", vec!["Servo", "Moteur BLDC", "V\u{e9}rin"]),
                ("Capteur", vec!["IMU", "Cam\u{e9}ra", "Capteur de force"]),
                (
                    "Mat\u{e9}riau",
                    vec!["PLA", "R\u{e9}sine", "Aluminium", "Titane", "Carbone"]
                ),
            ]
        );
    }

    #[test]
    fn taxonomy_builtin_ids_are_stable_qualified_and_unique() -> Result<(), Diagnostic> {
        let taxonomy = ComponentTaxonomy::default();
        assert_eq!(taxonomy, ComponentTaxonomy::default());
        let paths: &[(&str, &[&str])] = &[
            ("body", &["chassis", "shell", "support"]),
            ("limb", &["leg", "arm", "wheel", "wing", "tail"]),
            ("joint", &["ball", "revolute", "prismatic"]),
            ("actuator", &["servo", "bldc_motor", "cylinder"]),
            ("sensor", &["imu", "camera", "force_sensor"]),
            (
                "material",
                &["pla", "resin", "aluminium", "titanium", "carbon"],
            ),
        ];
        let mut validated = ComponentTaxonomy {
            categories: Vec::new(),
        };
        assert_eq!(taxonomy.categories().len(), paths.len());
        for (category, (name, entry_names)) in taxonomy.categories().iter().zip(paths) {
            let path = format!("robogen::taxonomy::builtin::{name}");
            assert_eq!(category.id, EntityId::from_name(&path));
            assert_eq!(category.entries.len(), entry_names.len());
            for (entry, name) in category.entries.iter().zip(*entry_names) {
                assert_eq!(entry.id, EntityId::from_name(&format!("{path}::{name}")));
            }
            validated.register_category(category.clone())?;
        }
        assert_eq!(validated, taxonomy);
        Ok(())
    }

    fn extension_category() -> TaxonomyCategory {
        TaxonomyCategory {
            id: EntityId::from_name("example::taxonomy::tools"),
            label: "Outil".into(),
            entries: vec![TaxonomyEntry {
                id: EntityId::from_name("example::taxonomy::tools::gripper"),
                label: "Pince".into(),
            }],
        }
    }

    #[test]
    fn taxonomy_accepts_category_extensions() -> Result<(), Diagnostic> {
        let mut taxonomy = ComponentTaxonomy::default();
        let original = taxonomy.clone();
        let extension = extension_category();
        taxonomy.register_category(extension.clone())?;
        assert_eq!(
            &taxonomy.categories()[..original.categories().len()],
            original.categories()
        );
        assert_eq!(taxonomy.categories().last(), Some(&extension));
        Ok(())
    }

    #[test]
    fn taxonomy_rejects_all_id_collisions_atomically() {
        let mut taxonomy = ComponentTaxonomy::default();
        let original = taxonomy.clone();
        let existing_category = taxonomy.categories()[0].id;
        let existing_entry = taxonomy.categories()[0].entries[0].id;
        let extension = extension_category();
        let fresh_entry = extension.entries[0].id;
        for (category_id, entry_id) in [
            (existing_category, EntityId::from_name("example::new_entry")),
            (existing_entry, EntityId::from_name("example::new_entry")),
            (extension.id, existing_category),
            (extension.id, existing_entry),
            (extension.id, extension.id),
            (extension.id, fresh_entry),
        ] {
            let mut candidate = extension.clone();
            candidate.id = category_id;
            candidate.entries.push(TaxonomyEntry {
                id: entry_id,
                label: "Autre".into(),
            });
            let result = taxonomy.register_category(candidate);
            assert!(matches!(result, Err(ref diagnostic)
                if diagnostic.code == "taxonomy.duplicate_id"
                    && diagnostic.severity == Severity::Error));
            assert_eq!(taxonomy, original);
        }
        assert!(taxonomy.register_category(extension).is_ok());
    }

    #[test]
    fn taxonomy_rejects_empty_labels_atomically() {
        let mut taxonomy = ComponentTaxonomy::default();
        let original = taxonomy.clone();
        for label in ["", " \t\n"] {
            for empty_category in [true, false] {
                let mut candidate = extension_category();
                if empty_category {
                    candidate.label = label.into();
                } else {
                    candidate.entries[0].label = label.into();
                }
                let result = taxonomy.register_category(candidate);
                assert!(matches!(result, Err(ref diagnostic)
                    if diagnostic.code == "taxonomy.empty_label"
                        && diagnostic.severity == Severity::Error));
                assert_eq!(taxonomy, original);
            }
        }
        assert!(taxonomy.register_category(extension_category()).is_ok());
    }
}
