//! Diagnostic values and face-connectivity queries.

use crate::{execution::ComputeBackend, simp::CellRegion};
use robogen_domain::{Length, LoadCaseId, Mass, OptimizationId, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
pub struct MassIteration {
    pub mass: Mass,
    pub infeasibility: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryMassDiagnostic {
    pub threshold: f64,
    pub mass: Mass,
    pub displacements: Vec<(LoadCaseId, Length)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassDiagnostic {
    pub compute_backend: ComputeBackend,
    pub id: OptimizationId,
    pub name: String,
    pub span: SourceSpan,
    pub document_revision: u64,
    pub origin: [Length; 3],
    pub dimensions: [Length; 3],
    pub cells: [usize; 3],
    pub regions: Vec<CellRegion>,
    pub densities: Vec<f64>,
    pub reference_densities: Vec<f64>,
    pub mass: Mass,
    pub reference_mass: Mass,
    pub cases: Vec<(LoadCaseId, Length, Length)>,
    pub history: Vec<MassIteration>,
    pub finished: bool,
    pub feasible: bool,
    pub converged: bool,
    pub kkt_residual: Option<f64>,
    pub binary: Option<BinaryMassDiagnostic>,
    pub reconstruction_error: Option<String>,
}

/// Face-connected components of a thresholded diagnostic, not mechanical approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityConnectivity {
    pub components: usize,
    pub preserved_components: usize,
    pub selected_cells: usize,
    pub preserved_cells: usize,
}

impl MassDiagnostic {
    pub fn connectivity(
        &self,
        threshold: f64,
        reference: bool,
    ) -> Result<DensityConnectivity, String> {
        let densities = if reference {
            &self.reference_densities
        } else {
            &self.densities
        };
        let count = self
            .cells
            .iter()
            .try_fold(1usize, |n, axis| n.checked_mul(*axis));
        if self.cells.contains(&0)
            || count != Some(densities.len())
            || densities.len() != self.regions.len()
            || densities.len() > 32768
            || !threshold.is_finite()
            || !(0.0..=1.0).contains(&threshold)
            || densities.iter().zip(&self.regions).any(|(value, role)| {
                !value.is_finite()
                    || !(0.0..=1.0).contains(value)
                    || (*role == CellRegion::Preserve && *value != 1.0)
                    || (*role == CellRegion::Void && *value != 0.0)
            })
        {
            return Err("Invalid density connectivity input".into());
        }
        let active = |index: usize| densities[index] > 0.0 && densities[index] >= threshold;
        let mut seen = vec![false; densities.len()];
        let mut report = DensityConnectivity {
            components: 0,
            preserved_components: 0,
            selected_cells: 0,
            preserved_cells: 0,
        };
        let strides = [self.cells[1] * self.cells[2], self.cells[2], 1];
        let mut queue = Vec::new();
        for start in 0..densities.len() {
            if seen[start] || !active(start) {
                continue;
            }
            report.components += 1;
            let mut has_preserve = false;
            seen[start] = true;
            queue.push(start);
            while let Some(index) = queue.pop() {
                report.selected_cells += 1;
                if self.regions[index] == CellRegion::Preserve {
                    has_preserve = true;
                    report.preserved_cells += 1;
                }
                for (axis, stride) in strides.iter().enumerate() {
                    let coordinate = index / stride % self.cells[axis];
                    let neighbors = [
                        (coordinate > 0).then(|| index - stride),
                        (coordinate + 1 < self.cells[axis]).then(|| index + stride),
                    ];
                    for neighbor in neighbors.into_iter().flatten() {
                        if !seen[neighbor] && active(neighbor) {
                            seen[neighbor] = true;
                            queue.push(neighbor);
                        }
                    }
                }
            }
            report.preserved_components += usize::from(has_preserve);
        }
        Ok(report)
    }

    /// Highest existing density threshold connecting every preserved cell by faces.
    /// Merely proposes a view setting: no density is changed, no bridge is added,
    /// and detached non-preserved islands are neither hidden nor removed.
    pub fn connection_threshold(&self, reference: bool) -> Result<Option<f64>, String> {
        let full = self.connectivity(0.0, reference)?;
        if full.preserved_cells == 0 || full.preserved_components != 1 {
            return Ok(None);
        }
        let mut levels: Vec<_> = if reference {
            self.reference_densities.clone()
        } else {
            self.densities.clone()
        };
        levels.retain(|density| *density > 0.0);
        levels.sort_by(f64::total_cmp);
        levels.dedup();
        let mut lower = 0;
        let mut upper = levels.len();
        while lower + 1 < upper {
            let middle = lower + (upper - lower) / 2;
            if self
                .connectivity(levels[middle], reference)?
                .preserved_components
                == 1
            {
                lower = middle;
            } else {
                upper = middle;
            }
        }
        Ok(Some(levels[lower]))
    }
}
