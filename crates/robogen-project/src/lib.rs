//! Project loading and incremental orchestration for the shared model pipeline.

mod compilation;
mod contracts;
mod dependencies;
mod diagnostic_mesh;
mod document;
mod library;
mod manifest;
mod optimization;
mod project;
mod projection;
mod rebuild;
mod topology;

pub use contracts::{
    topology_settings, DirtyNode, OptimizationState, ProjectSnapshot, RebuildReport,
    TopologyCaseReport, TopologyReport,
};
pub use document::SourceDocument;
pub use library::{FunctionDefinition, LibraryGenerator, TaxonomyFeature, TaxonomyInstance};
pub use manifest::{load_manifest, ProjectIoError, ProjectManifest};
pub use project::compile_source;
pub use robogen_ir::topology::Objective as TopologyObjective;
pub use robogen_topopt::execution::{
    grid_resolution, resolution_for_cell_size, ComputeBackend, ExecutionSettings, GridResolution,
};
pub use robogen_topopt::mass::{MassDiagnostic, MASS_BACKEND, MASS_BACKEND_AVAILABLE};

use robogen_domain::Quantity;
use robogen_dsl::Module;
use std::collections::BTreeMap;

// Private storage is shared only by the crate's responsibility-specific implementations.
#[derive(Clone)]
pub struct Project {
    source: String,
    ast: Module,
    overrides: BTreeMap<String, Quantity>,
    snapshot: ProjectSnapshot,
}

#[cfg(test)]
mod tests;
