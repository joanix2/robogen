use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod fem;
pub mod linear;
pub mod simp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyRequest {
    pub specification: robogen_ir::topology::TopologySpec<robogen_ir::SolidGeometry>,
    pub material: robogen_domain::Material,
    pub document_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResult {
    pub densities: Vec<f32>,
    pub compliance: f64,
}

#[derive(Debug, Error)]
pub enum TopologyError {
    #[error("Solver unavailable")]
    Unavailable,
}

pub trait TopologyOptimizer: Send + Sync {
    fn optimize(&self, request: &TopologyRequest) -> Result<TopologyResult, TopologyError>;
}

pub struct UnavailableTopologyOptimizer;
impl TopologyOptimizer for UnavailableTopologyOptimizer {
    fn optimize(&self, _request: &TopologyRequest) -> Result<TopologyResult, TopologyError> {
        Err(TopologyError::Unavailable)
    }
}
