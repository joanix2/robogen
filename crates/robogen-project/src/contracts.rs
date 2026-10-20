//! Snapshot, optimization status and rebuild reports shared with callers.

use crate::{ComputeBackend, ExecutionSettings, MassDiagnostic};
use robogen_cad::Mesh;
use robogen_domain::{
    Length, LoadCaseId, Mass, MaterialId, ObjectInstanceId, OptimizationId, PartId, SourceSpan,
};
use robogen_ir::SemanticModel;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectSnapshot {
    pub model: SemanticModel,
    pub meshes: BTreeMap<PartId, Mesh>,
    pub topology: BTreeMap<OptimizationId, TopologyReport>,
    pub mass_diagnostics: BTreeMap<OptimizationId, MassDiagnostic>,
    pub preparation: BTreeMap<PartId, [Mesh; 3]>,
    pub instance_interfaces: BTreeMap<ObjectInstanceId, Mesh>,
    pub optimization_states: BTreeMap<PartId, OptimizationState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizationState {
    NotCalculated,
    Running,
    Calculated,
    Diagnostic,
    Obsolete,
    Failed,
    Cancelled,
}

pub fn topology_settings() -> ExecutionSettings {
    ExecutionSettings {
        compute_backend: ComputeBackend::Cpu,
        cells: [12, 4, 2],
        filter_radius: Length::from_millimetres(12.0),
        threshold: 0.65,
        max_iterations: 100,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopologyReport {
    pub name: String,
    pub span: SourceSpan,
    pub material: MaterialId,
    pub document_revision: u64,
    pub settings: ExecutionSettings,
    pub solver_backend: String,
    pub origin: [Length; 3],
    pub domain_cells: usize,
    pub iterations: usize,
    pub volume_fraction: f64,
    pub mass: Mass,
    pub cases: Vec<TopologyCaseReport>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopologyCaseReport {
    pub id: LoadCaseId,
    pub span: SourceSpan,
    pub compliance: robogen_topopt::fem::Compliance,
    pub relative_residual: f64,
    pub max_displacement: Length,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirtyNode {
    Parameter(String),
    Sketch(String),
    Part(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RebuildReport {
    pub rebuilt: Vec<DirtyNode>,
}
