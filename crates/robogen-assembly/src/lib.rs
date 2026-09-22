//! Assembly graph contracts. Kernel and physics handles never enter this model.

use robogen_domain::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInstance {
    pub id: EntityId,
    pub source_part: EntityId,
    pub transform: [[f64; 4]; 4],
}
