use robogen_domain::Length;
use thiserror::Error;

use crate::{
    fem::{Beam, BeamModel, Compliance, FemError, FemResult},
    linear::SparseLinearSolver,
};

const MIN_DENSITY: f64 = 0.001;

#[derive(Debug, Clone)]
pub struct SimpOptions {
    pub volume_fraction: f64,
    pub penalty: f64,
    pub stiffness_floor: f64,
    pub filter_radius: Length,
    pub move_limit: f64,
    pub change_tolerance: f64,
    pub max_iterations: usize,
}

#[derive(Debug, Clone)]
pub struct BeamRequest {
    pub beam: Beam,
    pub options: SimpOptions,
    pub document_revision: u64,
}

#[derive(Debug, Clone)]
pub struct Iteration {
    pub number: usize,
    pub compliance: Compliance,
    pub volume_fraction: f64,
    pub max_design_change: f64,
    pub relative_residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    DesignChange,
    IterationLimit,
}

#[derive(Debug)]
pub struct BeamOptimization {
    pub request: BeamRequest,
    pub solver_backend: String,
    pub densities: Vec<f64>,
    pub design_variables: Vec<f64>,
    pub uniform_reference: Compliance,
    pub analysis: FemResult,
    pub history: Vec<Iteration>,
    pub stop_reason: StopReason,
}

#[derive(Debug, Error)]
pub enum SimpError {
    #[error("Invalid SIMP settings")]
    InvalidOptions,
    #[error("Optimality criteria update failed its finite-value or volume checks")]
    Update,
    #[error(transparent)]
    Fem(#[from] FemError),
}

pub fn optimize_beam(
    request: &BeamRequest,
    solver: &dyn SparseLinearSolver,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(&Iteration),
) -> Result<BeamOptimization, SimpError> {
    let options = &request.options;
    validate_options(options)?;
    if cancelled() {
        return Err(FemError::Cancelled.into());
    }
    let model = BeamModel::new(request.beam.clone())?;
    let filter = DensityFilter::new(model.beam(), options.filter_radius, cancelled)?;
    let mut design = vec![options.volume_fraction; model.element_count()];
    let mut densities = filter.apply(&design);
    let mut analysis = model.solve(
        &densities,
        options.penalty,
        options.stiffness_floor,
        solver,
        cancelled,
    )?;
    let uniform_reference = analysis.compliance;
    let mut history = vec![iteration(0, &analysis, &densities, 0.0)];
    progress(&history[0]);
    let mut stop_reason = StopReason::IterationLimit;
    let volume_derivative = filter.transpose(&vec![1.0 / design.len() as f64; design.len()]);
    for number in 1..=options.max_iterations {
        if cancelled() {
            return Err(FemError::Cancelled.into());
        }
        let sensitivities = sensitivities(&filter, &analysis, &densities, options);
        let next = oc_update(
            &design,
            &sensitivities,
            &volume_derivative,
            &filter,
            options,
            cancelled,
        )?;
        let max_change = design
            .iter()
            .zip(&next)
            .map(|(old, new)| (old - new).abs())
            .fold(0.0, f64::max);
        densities = filter.apply(&next);
        analysis = model.solve(
            &densities,
            options.penalty,
            options.stiffness_floor,
            solver,
            cancelled,
        )?;
        design = next;
        history.push(iteration(number, &analysis, &densities, max_change));
        progress(&history[history.len() - 1]);
        if cancelled() {
            return Err(FemError::Cancelled.into());
        }
        if max_change < options.change_tolerance {
            stop_reason = StopReason::DesignChange;
            break;
        }
    }
    Ok(BeamOptimization {
        request: request.clone(),
        solver_backend: solver.backend_id().to_owned(),
        densities,
        design_variables: design,
        uniform_reference,
        analysis,
        history,
        stop_reason,
    })
}

fn validate_options(options: &SimpOptions) -> Result<(), SimpError> {
    if !options.volume_fraction.is_finite()
        || !(0.01..0.99).contains(&options.volume_fraction)
        || !options.penalty.is_finite()
        || !(1.0..=5.0).contains(&options.penalty)
        || !options.stiffness_floor.is_finite()
        || !(1e-9..=0.1).contains(&options.stiffness_floor)
        || !options.filter_radius.metres().is_finite()
        || options.filter_radius.metres() <= 0.0
        || !options.move_limit.is_finite()
        || !(0.001..=0.5).contains(&options.move_limit)
        || !options.change_tolerance.is_finite()
        || !(1e-6..=0.1).contains(&options.change_tolerance)
        || options.max_iterations == 0
        || options.max_iterations > 500
    {
        return Err(SimpError::InvalidOptions);
    }
    Ok(())
}

fn iteration(
    number: usize,
    analysis: &FemResult,
    densities: &[f64],
    max_design_change: f64,
) -> Iteration {
    Iteration {
        number,
        compliance: analysis.compliance,
        volume_fraction: mean(densities),
        max_design_change,
        relative_residual: analysis.relative_residual,
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sensitivities(
    filter: &DensityFilter,
    analysis: &FemResult,
    densities: &[f64],
    options: &SimpOptions,
) -> Vec<f64> {
    filter.transpose(
        &densities
            .iter()
            .zip(&analysis.element_energies)
            .map(|(density, energy)| {
                -options.penalty
                    * (1.0 - options.stiffness_floor)
                    * density.powf(options.penalty - 1.0)
                    * energy
            })
            .collect::<Vec<_>>(),
    )
}

fn oc_update(
    design: &[f64],
    sensitivities: &[f64],
    volume_derivative: &[f64],
    filter: &DensityFilter,
    options: &SimpOptions,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<f64>, SimpError> {
    if sensitivities
        .iter()
        .any(|value| !value.is_finite() || *value > 0.0)
        || volume_derivative
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(SimpError::Update);
    }
    let ratios: Vec<_> = sensitivities
        .iter()
        .zip(volume_derivative)
        .map(|(sensitivity, volume)| -sensitivity / volume)
        .collect();
    let candidate = |multiplier: f64| {
        design
            .iter()
            .zip(&ratios)
            .map(|(density, ratio)| {
                (density * (ratio / multiplier).sqrt()).clamp(
                    (density - options.move_limit).max(MIN_DENSITY),
                    (density + options.move_limit).min(1.0),
                )
            })
            .collect::<Vec<_>>()
    };
    let mut upper = ratios.iter().copied().fold(0.0, f64::max);
    if !upper.is_finite() || upper <= 0.0 {
        return Err(SimpError::Update);
    }
    let mut lower = 0.0;
    let mut next = candidate(upper);
    if mean(&filter.apply(&next)) > options.volume_fraction + 1e-10 {
        return Err(SimpError::Update);
    }
    for _ in 0..80 {
        if cancelled() {
            return Err(FemError::Cancelled.into());
        }
        let middle = (upper + lower) * 0.5;
        let trial = candidate(middle);
        if mean(&filter.apply(&trial)) > options.volume_fraction {
            lower = middle;
        } else {
            upper = middle;
            next = trial;
        }
    }
    if next.iter().any(|value| !value.is_finite())
        || (mean(&filter.apply(&next)) - options.volume_fraction).abs() > 1e-6
    {
        return Err(SimpError::Update);
    }
    Ok(next)
}

struct DensityFilter {
    rows: Vec<Vec<(usize, f64)>>,
}

impl DensityFilter {
    fn new(beam: &Beam, radius: Length, cancelled: &dyn Fn() -> bool) -> Result<Self, FemError> {
        let count = beam.cells.iter().product();
        let spacing: [f64; 3] =
            std::array::from_fn(|axis| beam.dimensions[axis].metres() / beam.cells[axis] as f64);
        let position = |index: usize| {
            [
                index / (beam.cells[1] * beam.cells[2]),
                index / beam.cells[2] % beam.cells[1],
                index % beam.cells[2],
            ]
        };
        let mut rows = Vec::with_capacity(count);
        for target in 0..count {
            if cancelled() {
                return Err(FemError::Cancelled);
            }
            let center = position(target);
            let mut row = Vec::new();
            for source in 0..count {
                let other = position(source);
                let distance = (0..3).fold(0.0_f64, |norm, axis| {
                    norm.hypot((center[axis] as f64 - other[axis] as f64) * spacing[axis])
                });
                let weight = (1.0 - distance / radius.metres()).max(0.0);
                if weight > 0.0 {
                    row.push((source, weight));
                }
            }
            let sum: f64 = row.iter().map(|(_, weight)| weight).sum();
            for (_, weight) in &mut row {
                *weight /= sum;
            }
            rows.push(row);
        }
        Ok(Self { rows })
    }

    fn apply(&self, values: &[f64]) -> Vec<f64> {
        self.rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(index, weight)| values[*index] * weight)
                    .sum()
            })
            .collect()
    }

    fn transpose(&self, values: &[f64]) -> Vec<f64> {
        let mut result = vec![0.0; self.rows.len()];
        for (row, value) in self.rows.iter().zip(values) {
            for (index, weight) in row {
                result[*index] += weight * value;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear::FaerSolver;
    use robogen_domain::{Force, Pressure};

    fn request() -> BeamRequest {
        BeamRequest {
            beam: Beam {
                dimensions: [0.12, 0.04, 0.02].map(Length::from_metres),
                cells: [12, 4, 2],
                young_modulus: Pressure::from_pascals(70e9),
                poisson_ratio: 0.3,
                end_force: [0.0, 0.0, -1.0].map(Force::from_newtons),
            },
            options: SimpOptions {
                volume_fraction: 0.35,
                penalty: 3.0,
                stiffness_floor: 1e-6,
                filter_radius: Length::from_metres(0.015),
                move_limit: 0.2,
                change_tolerance: 0.005,
                max_iterations: 40,
            },
            document_revision: 7,
        }
    }

    #[test]
    fn filtered_sensitivities_match_finite_differences() -> Result<(), SimpError> {
        let mut request = request();
        request.beam.cells = [3, 2, 1];
        request.options.filter_radius = Length::from_metres(0.05);
        let model = BeamModel::new(request.beam.clone())?;
        let filter = DensityFilter::new(&request.beam, request.options.filter_radius, &|| false)?;
        let design = vec![0.4, 0.5, 0.6, 0.45, 0.55, 0.65];
        let evaluate = |variables: &[f64]| {
            model.solve(&filter.apply(variables), 3.0, 1e-6, &FaerSolver, &|| false)
        };
        let analysis = evaluate(&design)?;
        let analytic = sensitivities(&filter, &analysis, &filter.apply(&design), &request.options);
        let volume_gradient = filter.transpose(&[1.0 / 6.0; 6]);
        for index in 0..design.len() {
            let mut plus = design.clone();
            plus[index] += 1e-5;
            let mut minus = design.clone();
            minus[index] -= 1e-5;
            let numerical = (evaluate(&plus)?.compliance.joules()
                - evaluate(&minus)?.compliance.joules())
                / 2e-5;
            assert!(
                (numerical / analytic[index] - 1.0).abs() < 1e-5,
                "element {index}: {numerical} vs {}",
                analytic[index]
            );
            let volume = (mean(&filter.apply(&plus)) - mean(&filter.apply(&minus))) / 2e-5;
            assert!((volume - volume_gradient[index]).abs() < 1e-10);
        }
        Ok(())
    }

    #[test]
    fn simp_improves_uniform_reference_at_same_volume() -> Result<(), SimpError> {
        let request = request();
        let result = optimize_beam(&request, &FaerSolver, &|| false, &mut |_| {})?;
        assert!(result.analysis.compliance.joules() < 0.95 * result.uniform_reference.joules());
        assert!(
            result
                .densities
                .iter()
                .all(|density| (MIN_DENSITY..=1.0).contains(density))
        );
        for entry in &result.history {
            assert!((entry.volume_fraction - request.options.volume_fraction).abs() < 1e-6);
            assert!(entry.relative_residual < 1e-7);
        }
        assert_eq!(result.request.document_revision, 7);
        let model = BeamModel::new(request.beam)?;
        let recheck = model.solve(&result.densities, 3.0, 1e-6, &FaerSolver, &|| false)?;
        assert!(
            (recheck.compliance.joules() / result.analysis.compliance.joules() - 1.0).abs() < 1e-10
        );
        Ok(())
    }

    #[test]
    fn cancellation_after_progress_prevents_publication() {
        let cancelled = std::cell::Cell::new(false);
        let result = optimize_beam(&request(), &FaerSolver, &|| cancelled.get(), &mut |_| {
            cancelled.set(true)
        });
        assert!(matches!(result, Err(SimpError::Fem(FemError::Cancelled))));
        let mut invalid = request();
        invalid.options.volume_fraction = f64::NAN;
        assert!(matches!(
            optimize_beam(&invalid, &FaerSolver, &|| false, &mut |_| {}),
            Err(SimpError::InvalidOptions)
        ));
    }
}
