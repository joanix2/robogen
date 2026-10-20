//! Stable object-instance records and source provenance for ordinary geometry projections.

use crate::SolidGeometry;
use robogen_domain::{FeatureId, ObjectInstanceId, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A placed purchased component and its explicit regions, all in the document frame.
/// Interfaces contain geometry only; loads and supports belong to each TopologySpec.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectInstance {
    pub id: ObjectInstanceId,
    pub name: String,
    pub shape: SolidGeometry,
    pub voids: Vec<SolidGeometry>,
    pub interfaces: BTreeMap<String, SolidGeometry>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceRegion {
    Shape,
    Void,
    Interface(String),
}

/// Records the feature selecting an instance region without exposing evaluator node indices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InstanceRegionReference {
    pub instance: ObjectInstanceId,
    pub region: InstanceRegion,
    pub feature: FeatureId,
    pub span: SourceSpan,
}
