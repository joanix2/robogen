use robogen_domain::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum JointType {
    Fixed,
    Revolute,
    Continuous,
    Prismatic,
    Ball,
    Planar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: EntityId,
    pub name: String,
    pub mass_kg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joint {
    pub id: EntityId,
    pub parent: EntityId,
    pub child: EntityId,
    pub kind: JointType,
    pub axis: [f64; 3],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotModel {
    pub links: Vec<Link>,
    pub joints: Vec<Joint>,
}
