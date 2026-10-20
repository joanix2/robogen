//! Neutral, source-derived mechanical descriptions in the document rest frame.
use crate::{JointId, Length, ObjectInstanceId, PartId, RigidBodyId, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MechanicalModel {
    pub bodies: BTreeMap<RigidBodyId, RigidBody>,
    pub joints: BTreeMap<JointId, KinematicJoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RigidBody {
    pub id: RigidBodyId,
    pub name: String,
    pub parts: Vec<PartId>,
    pub grounded: bool,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointKind {
    Fixed,
    Revolute,
    Prismatic,
    Cylindrical,
    Helical,
}

/// Dimensionless direction. Validation rejects nonfinite, zero and nonunit axes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisDirection(pub [f64; 3]);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KinematicJoint {
    pub id: JointId,
    pub name: String,
    pub parent: RigidBodyId,
    pub child: RigidBodyId,
    pub kind: JointKind,
    pub origin: [Length; 3],
    pub axis: Option<AxisDirection>,
    /// Signed travel per full turn; required only for a helical joint.
    pub pitch: Option<Length>,
    pub actuator: Option<ObjectInstanceId>,
    pub span: SourceSpan,
}
