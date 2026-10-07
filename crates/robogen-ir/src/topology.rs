//! Backend-neutral declarations. Acceptance of a spec is not a computed result.
use crate::{Feature, SemanticModel, SolidGeometry, SolidOperation};
use robogen_domain::{
    Angle, FeatureId, Force, Frequency, InterfaceId, Length, LoadCaseId, Mass, MaterialId,
    OptimizationId, Pressure, SourceSpan, Torque,
};
use serde::{Deserialize, Serialize};

pub const TOPOLOGY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologySpec<G> {
    pub schema_version: u32,
    pub id: OptimizationId,
    pub name: String,
    pub feature: FeatureId,
    pub span: SourceSpan,
    pub domain: G,
    pub preserve: Vec<PreservedRegion<G>>,
    pub voids: Vec<G>,
    pub material: MaterialId,
    pub load_cases: Vec<LoadCase>,
    pub objective: Objective,
    pub constraints: Vec<Constraint>,
    pub manufacturing: Vec<Manufacturing>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreservedRegion<G> {
    // Only explicitly named interfaces may receive loads/supports.
    pub interface: Option<Interface>,
    pub geometry: G,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub id: InterfaceId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadCase {
    pub id: LoadCaseId,
    pub name: String,
    pub span: SourceSpan,
    pub loads: Vec<Load>,
    pub supports: Vec<Support>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Load {
    pub target: InterfaceId,
    pub span: SourceSpan,
    pub kind: LoadKind,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LoadKind {
    Force([Force; 3]),
    Moment([Torque; 3]),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Support {
    pub target: InterfaceId,
    pub span: SourceSpan,
    pub kind: SupportKind,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SupportKind {
    Fixed,
    Pin { axis: [f64; 3] },
    Frictionless { normal: [f64; 3] },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Objective {
    MinimizeMass,
    MaximizeStiffness,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub span: SourceSpan,
    pub kind: ConstraintKind,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    MaxDisplacement(Length),
    MaxStress(Pressure),
    SafetyFactor(f64),
    MaxMass(Mass),
    MinFrequency(Frequency),
    VolumeFraction(f64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manufacturing {
    pub span: SourceSpan,
    pub process: ManufacturingProcess,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ManufacturingProcess {
    Fdm {
        build_direction: [f64; 3],
        min_thickness: Length,
        min_hole: Length,
        max_overhang: Angle,
    },
    Cnc3Axis {
        tool_direction: [f64; 3],
        tool_diameter: Length,
    },
}

impl<G> TopologySpec<G> {
    pub fn geometries(&self) -> Vec<&G> {
        std::iter::once(&self.domain)
            .chain(self.preserve.iter().map(|region| &region.geometry))
            .chain(&self.voids)
            .collect()
    }

    pub fn map_geometry<H>(&self, map: impl Fn(&G) -> H) -> TopologySpec<H> {
        TopologySpec {
            schema_version: self.schema_version,
            id: self.id,
            name: self.name.clone(),
            feature: self.feature,
            span: self.span,
            domain: map(&self.domain),
            preserve: self
                .preserve
                .iter()
                .map(|region| PreservedRegion {
                    interface: region.interface.clone(),
                    geometry: map(&region.geometry),
                    span: region.span,
                })
                .collect(),
            voids: self.voids.iter().map(map).collect(),
            material: self.material,
            load_cases: self.load_cases.clone(),
            objective: self.objective,
            constraints: self.constraints.clone(),
            manufacturing: self.manufacturing.clone(),
        }
    }
}

impl SolidGeometry {
    pub fn topology_operations(&self) -> Vec<&TopologySpec<SolidGeometry>> {
        let mut pending = vec![self];
        let mut result = Vec::new();
        while let Some(geometry) = pending.pop() {
            match &geometry.operation {
                SolidOperation::Topology { specification } => {
                    result.push(specification.as_ref());
                    pending.extend(specification.geometries());
                }
                SolidOperation::Color { shape, .. }
                | SolidOperation::Translate { shape, .. }
                | SolidOperation::Rotate { shape, .. } => pending.push(shape),
                SolidOperation::Union { shapes } | SolidOperation::Compound { shapes } => {
                    pending.extend(shapes)
                }
                SolidOperation::Difference { base, tools } => {
                    pending.push(base);
                    pending.extend(tools);
                }
                SolidOperation::Box { .. } | SolidOperation::Cylinder { .. } => {}
            }
        }
        result
    }
}

impl SemanticModel {
    pub fn topology_operations(&self) -> Vec<&TopologySpec<SolidGeometry>> {
        self.parts
            .values()
            .flat_map(|part| &part.features)
            .flat_map(|feature| match feature {
                Feature::Solid { geometry, .. } => geometry.topology_operations(),
                Feature::Extrude { .. } => Vec::new(),
            })
            .collect()
    }
}
