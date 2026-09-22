use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyRequest {
    pub target_name: String,
    pub volume_fraction: f32,
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
