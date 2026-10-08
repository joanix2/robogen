use robogen_domain::{Length, LoadCaseId, SourceSpan};
use thiserror::Error;

use crate::{
    fem::{
        Beam, BeamModel, BoundaryConditions, Compliance, FemError, FemResult, GridDefinition,
        GridModel, GridResult,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellRegion {
    Design,
    Preserve,
    Void,
}

#[derive(Debug, Clone)]
pub struct BeamRequest {
    pub beam: Beam,
    pub options: SimpOptions,
    pub document_revision: u64,
    /// Empty means all cells are design variables. Otherwise one role per cell,
    /// ordered X slowest, Z fastest: (x * ny + y) * nz + z.
    pub regions: Vec<CellRegion>,
}

#[derive(Debug, Clone)]
pub struct GridLoadCase {
    pub id: LoadCaseId,
    pub span: SourceSpan,
    pub boundary: BoundaryConditions,
}

#[derive(Debug, Clone)]
pub struct GridRequest {
    pub grid: GridDefinition,
    pub load_cases: Vec<GridLoadCase>,
    pub options: SimpOptions,
    pub regions: Vec<CellRegion>,
    pub document_revision: u64,
}

#[derive(Debug)]
pub struct CaseAnalysis {
    pub id: LoadCaseId,
    pub span: SourceSpan,
    pub analysis: GridResult,
}

#[derive(Debug, Clone)]
pub struct CaseMetrics {
    pub id: LoadCaseId,
    pub compliance: Compliance,
    pub relative_residual: f64,
}

#[derive(Debug, Clone)]
pub struct GridIteration {
    /// Compliance is summed; residual is the maximum across cases.
    pub aggregate: Iteration,
    pub cases: Vec<CaseMetrics>,
}

#[derive(Debug)]
pub struct GridOptimization {
    pub request: GridRequest,
    pub solver_backend: String,
    pub densities: Vec<f64>,
    pub design_variables: Vec<f64>,
    pub uniform_reference: Compliance,
    pub analyses: Vec<CaseAnalysis>,
    pub history: Vec<GridIteration>,
    pub stop_reason: StopReason,
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
    FixedRegions,
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
    #[error("Expected 1..16 load cases with unique IDs")]
    InvalidCases,
    #[error("Load case {id} failed: {source}")]
    Case {
        id: LoadCaseId,
        span: SourceSpan,
        source: FemError,
    },
    #[error("Volume target is incompatible with preserved/void regions")]
    InfeasibleVolume,
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
    validate_options(&request.options)?;
    if cancelled() {
        return Err(FemError::Cancelled.into());
    }
    let model = BeamModel::new(request.beam.clone())?;
    let generic = GridRequest {
        grid: model.grid_definition().clone(),
        load_cases: vec![GridLoadCase {
            id: LoadCaseId::from_name("robogen.benchmark.beam"),
            span: SourceSpan::default(),
            boundary: model.boundary_conditions().clone(),
        }],
        options: request.options.clone(),
        regions: request.regions.clone(),
        document_revision: request.document_revision,
    };
    let mut result = optimize_grid(&generic, solver, cancelled, &mut |entry| {
        progress(&entry.aggregate)
    })
    .map_err(|error| match error {
        SimpError::Case { source, .. } => SimpError::Fem(source),
        other => other,
    })?;
    let case = result.analyses.pop().ok_or(SimpError::InvalidCases)?;
    Ok(BeamOptimization {
        request: request.clone(),
        solver_backend: result.solver_backend,
        densities: result.densities,
        design_variables: result.design_variables,
        uniform_reference: result.uniform_reference,
        analysis: model.with_tip(case.analysis),
        history: result
            .history
            .into_iter()
            .map(|entry| entry.aggregate)
            .collect(),
        stop_reason: result.stop_reason,
    })
}

pub(crate) fn validate_cases(cases: &[GridLoadCase]) -> Result<(), SimpError> {
    if cases.is_empty()
        || cases.len() > 16
        || cases
            .iter()
            .map(|case| case.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != cases.len()
    {
        return Err(SimpError::InvalidCases);
    }
    Ok(())
}

fn solve_cases(
    model: &GridModel,
    request: &GridRequest,
    densities: &[f64],
    solver: &dyn SparseLinearSolver,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<CaseAnalysis>, SimpError> {
    let analyses: Vec<CaseAnalysis> = request
        .load_cases
        .iter()
        .map(|case| {
            if cancelled() {
                return Err(FemError::Cancelled.into());
            }
            let analysis = model
                .solve(
                    densities,
                    request.options.penalty,
                    request.options.stiffness_floor,
                    &case.boundary,
                    solver,
                    cancelled,
                )
                .map_err(|source| SimpError::Case {
                    id: case.id,
                    span: case.span,
                    source,
                })?;
            Ok(CaseAnalysis {
                id: case.id,
                span: case.span,
                analysis,
            })
        })
        .collect::<Result<_, SimpError>>()?;
    if !Compliance::sum(analyses.iter().map(|case| case.analysis.compliance))
        .joules()
        .is_finite()
    {
        return Err(SimpError::Update);
    }
    Ok(analyses)
}

pub fn optimize_grid(
    request: &GridRequest,
    solver: &dyn SparseLinearSolver,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(&GridIteration),
) -> Result<GridOptimization, SimpError> {
    let options = &request.options;
    validate_options(options)?;
    if cancelled() {
        return Err(FemError::Cancelled.into());
    }
    validate_cases(&request.load_cases)?;
    let model = GridModel::new(request.grid.clone())?;
    let filter = DensityFilter::new(
        model.definition(),
        options.filter_radius,
        &request.regions,
        cancelled,
    )?;
    let preserved = filter
        .regions
        .iter()
        .filter(|role| **role == CellRegion::Preserve)
        .count();
    let free = filter
        .regions
        .iter()
        .filter(|role| **role == CellRegion::Design)
        .count();
    let free_density = if free == 0 {
        if preserved as f64 / model.element_count() as f64 > options.volume_fraction + 1e-10 {
            return Err(SimpError::InfeasibleVolume);
        }
        0.0
    } else {
        let value = (options.volume_fraction * model.element_count() as f64 - preserved as f64)
            / free as f64;
        if !value.is_finite() || !(MIN_DENSITY - 1e-12..=1.0 + 1e-12).contains(&value) {
            return Err(SimpError::InfeasibleVolume);
        }
        value.clamp(MIN_DENSITY, 1.0)
    };
    let mut design: Vec<_> = filter
        .regions
        .iter()
        .map(|role| match role {
            CellRegion::Design => free_density,
            CellRegion::Preserve => 1.0,
            CellRegion::Void => 0.0,
        })
        .collect();
    let mut densities = filter.apply(&design);
    let mut analyses = solve_cases(&model, request, &densities, solver, cancelled)?;
    let uniform_reference = Compliance::sum(analyses.iter().map(|case| case.analysis.compliance));
    let mut history = vec![grid_iteration(0, &analyses, &densities, 0.0)];
    progress(&history[0]);
    if cancelled() {
        return Err(FemError::Cancelled.into());
    }
    let mut stop_reason = if free == 0 {
        StopReason::FixedRegions
    } else {
        StopReason::IterationLimit
    };
    let volume_derivative = filter.transpose(&vec![1.0 / design.len() as f64; design.len()]);
    for number in 1..=if free == 0 { 0 } else { options.max_iterations } {
        if cancelled() {
            return Err(FemError::Cancelled.into());
        }
        let sensitivities = combined_sensitivities(&filter, &analyses, &densities, options);
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
        analyses = solve_cases(&model, request, &densities, solver, cancelled)?;
        design = next;
        history.push(grid_iteration(number, &analyses, &densities, max_change));
        progress(&history[history.len() - 1]);
        if cancelled() {
            return Err(FemError::Cancelled.into());
        }
        if max_change < options.change_tolerance {
            stop_reason = StopReason::DesignChange;
            break;
        }
    }
    Ok(GridOptimization {
        request: request.clone(),
        solver_backend: solver.backend_id().to_owned(),
        densities,
        design_variables: design,
        uniform_reference,
        analyses,
        history,
        stop_reason,
    })
}

fn validate_options(options: &SimpOptions) -> Result<(), SimpError> {
    if !options.volume_fraction.is_finite()
        || options.volume_fraction <= 0.0
        || options.volume_fraction > 1.0
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

fn grid_iteration(
    number: usize,
    analyses: &[CaseAnalysis],
    densities: &[f64],
    max_design_change: f64,
) -> GridIteration {
    GridIteration {
        aggregate: Iteration {
            number,
            compliance: Compliance::sum(analyses.iter().map(|case| case.analysis.compliance)),
            volume_fraction: mean(densities),
            max_design_change,
            relative_residual: analyses
                .iter()
                .map(|case| case.analysis.relative_residual)
                .fold(0.0, f64::max),
        },
        cases: analyses
            .iter()
            .map(|case| CaseMetrics {
                id: case.id,
                compliance: case.analysis.compliance,
                relative_residual: case.analysis.relative_residual,
            })
            .collect(),
    }
}

fn combined_sensitivities(
    filter: &DensityFilter,
    analyses: &[CaseAnalysis],
    densities: &[f64],
    options: &SimpOptions,
) -> Vec<f64> {
    let mut combined = vec![0.0; densities.len()];
    for case in analyses {
        for (total, partial) in
            combined
                .iter_mut()
                .zip(sensitivities(filter, &case.analysis, densities, options))
        {
            *total += partial;
        }
    }
    combined
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sensitivities(
    filter: &DensityFilter,
    analysis: &GridResult,
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
            .zip(&filter.regions)
            .any(|(value, role)| {
                !value.is_finite() || (*role == CellRegion::Design && *value <= 0.0)
            })
    {
        return Err(SimpError::Update);
    }
    let ratios: Vec<_> = sensitivities
        .iter()
        .zip(volume_derivative)
        .map(|(sensitivity, volume)| {
            if *volume > 0.0 {
                -sensitivity / volume
            } else {
                0.0
            }
        })
        .collect();
    let candidate = |multiplier: f64| {
        design
            .iter()
            .zip(&ratios)
            .zip(&filter.regions)
            .map(|((density, ratio), role)| {
                if *role == CellRegion::Preserve {
                    return 1.0;
                }
                if *role == CellRegion::Void {
                    return 0.0;
                }
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
    regions: Vec<CellRegion>,
    rows: Vec<Vec<(usize, f64)>>,
}

impl DensityFilter {
    fn new(
        beam: &GridDefinition,
        radius: Length,
        regions: &[CellRegion],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, FemError> {
        let count = beam.cells.iter().product();
        let regions = if regions.is_empty() {
            vec![CellRegion::Design; count]
        } else {
            if regions.len() != count {
                return Err(FemError::InvalidInput);
            }
            regions.to_vec()
        };
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
            if regions[target] != CellRegion::Design {
                rows.push(Vec::new());
                continue;
            }
            let center = position(target);
            let mut row = Vec::new();
            for (source, role) in regions.iter().enumerate() {
                if *role != CellRegion::Design {
                    continue;
                }
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
        Ok(Self { rows, regions })
    }

    fn apply(&self, values: &[f64]) -> Vec<f64> {
        self.rows
            .iter()
            .zip(&self.regions)
            .map(|(row, role)| {
                if *role == CellRegion::Preserve {
                    return 1.0;
                }
                if *role == CellRegion::Void {
                    return 0.0;
                }
                row.iter()
                    .map(|(index, weight)| values[*index] * weight)
                    .sum::<f64>()
                    // Normalized convex weights can round just above one.
                    .clamp(0.0, 1.0)
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
            regions: Vec::new(),
        }
    }

    fn multi_request() -> Result<GridRequest, SimpError> {
        let mut original = request();
        original.beam.cells = [4, 2, 1];
        original.options.max_iterations = 4;
        let model = BeamModel::new(original.beam)?;
        let mut other = model.boundary_conditions().clone();
        for load in &mut other.forces {
            load.force = load.force.map(|f| Force::from_newtons(-f.newtons()));
        }
        Ok(GridRequest {
            grid: model.grid_definition().clone(),
            options: original.options,
            regions: Vec::new(),
            document_revision: 71,
            load_cases: vec![
                GridLoadCase {
                    id: LoadCaseId::from_name("down"),
                    span: SourceSpan::new(10, 20),
                    boundary: model.boundary_conditions().clone(),
                },
                GridLoadCase {
                    id: LoadCaseId::from_name("up"),
                    span: SourceSpan::new(30, 40),
                    boundary: other,
                },
            ],
        })
    }

    #[test]
    fn opposing_load_cases_are_not_cancelled_and_preserve_individual_results()
    -> Result<(), SimpError> {
        let request = multi_request()?;
        let result = optimize_grid(&request, &FaerSolver, &|| false, &mut |_| {})?;
        let mut single = request.clone();
        single.load_cases.pop();
        let reference = optimize_grid(&single, &FaerSolver, &|| false, &mut |_| {})?;
        assert_eq!(result.analyses.len(), 2);
        assert_eq!(result.request.document_revision, 71);
        for (a, b) in result.densities.iter().zip(reference.densities) {
            assert!((a - b).abs() < 1e-10);
        }
        for entry in &result.history {
            assert_eq!(entry.cases.len(), 2);
            assert!(
                (entry.aggregate.compliance.joules() / entry.cases[0].compliance.joules() - 2.0)
                    .abs()
                    < 1e-10
            );
        }
        for (down, up) in result.analyses[0]
            .analysis
            .displacements
            .iter()
            .zip(&result.analyses[1].analysis.displacements)
        {
            for axis in 0..3 {
                assert!((down[axis].metres() + up[axis].metres()).abs() < 1e-14);
            }
        }
        Ok(())
    }

    #[test]
    fn multi_case_gradient_matches_finite_differences() -> Result<(), SimpError> {
        let mut request = multi_request()?;
        let end_offset =
            request.grid.cells[0] * (request.grid.cells[1] + 1) * (request.grid.cells[2] + 1);
        for constraint in &mut request.load_cases[1].boundary.constraints {
            constraint.node.0 += end_offset;
        }
        for load in &mut request.load_cases[1].boundary.forces {
            load.node.0 -= end_offset;
        }
        for load in &mut request.load_cases[1].boundary.forces {
            load.force = [
                load.force[2],
                Force::from_newtons(0.0),
                Force::from_newtons(0.0),
            ];
        }
        let model = GridModel::new(request.grid.clone())?;
        let filter =
            DensityFilter::new(&request.grid, request.options.filter_radius, &[], &|| false)?;
        let design = vec![0.55; 8];
        let evaluate =
            |x: &[f64]| solve_cases(&model, &request, &filter.apply(x), &FaerSolver, &|| false);
        let analyses = evaluate(&design)?;
        let gradient =
            combined_sensitivities(&filter, &analyses, &filter.apply(&design), &request.options);
        for i in 0..8 {
            let mut plus = design.clone();
            plus[i] += 1e-5;
            let mut minus = design.clone();
            minus[i] -= 1e-5;
            let objective = |cases: Vec<CaseAnalysis>| {
                Compliance::sum(cases.iter().map(|c| c.analysis.compliance)).joules()
            };
            let numerical = (objective(evaluate(&plus)?) - objective(evaluate(&minus)?)) / 2e-5;
            assert!((numerical / gradient[i] - 1.0).abs() < 1e-5);
        }
        Ok(())
    }

    #[test]
    fn invalid_case_and_cancellation_never_return_partial_results() -> Result<(), SimpError> {
        let mut request = multi_request()?;
        request.load_cases[1].boundary.constraints.truncate(1);
        assert!(
            matches!(optimize_grid(&request,&FaerSolver,&||false,&mut |_|{}),Err(SimpError::Case {id,span,source:FemError::RigidModes}) if id==request.load_cases[1].id && span==request.load_cases[1].span)
        );
        request = multi_request()?;
        request.load_cases[1].id = request.load_cases[0].id;
        assert!(matches!(
            optimize_grid(&request, &FaerSolver, &|| false, &mut |_| {}),
            Err(SimpError::InvalidCases)
        ));
        request = multi_request()?;
        let cancelled = std::cell::Cell::new(false);
        assert!(matches!(
            optimize_grid(&request, &FaerSolver, &|| cancelled.get(), &mut |_| {
                cancelled.set(true)
            }),
            Err(SimpError::Fem(FemError::Cancelled))
        ));
        Ok(())
    }

    #[test]
    fn filtered_sensitivities_match_finite_differences() -> Result<(), SimpError> {
        let mut request = request();
        request.beam.cells = [3, 2, 1];
        request.options.filter_radius = Length::from_metres(0.05);
        let model = BeamModel::new(request.beam.clone())?;
        let filter = DensityFilter::new(
            model.grid_definition(),
            request.options.filter_radius,
            &[],
            &|| false,
        )?;
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
    fn masked_filter_has_exact_regions_and_consistent_derivatives() -> Result<(), SimpError> {
        let mut request = request();
        request.beam.cells = [4, 2, 1];
        let roles = [
            CellRegion::Preserve,
            CellRegion::Preserve,
            CellRegion::Design,
            CellRegion::Void,
            CellRegion::Design,
            CellRegion::Design,
            CellRegion::Preserve,
            CellRegion::Preserve,
        ];
        let filter = DensityFilter::new(
            BeamModel::new(request.beam.clone())?.grid_definition(),
            Length::from_metres(0.05),
            &roles,
            &|| false,
        )?;
        let model = BeamModel::new(request.beam.clone())?;
        let design = vec![0.6; 8];
        let evaluate = |x: &[f64]| model.solve(&filter.apply(x), 3.0, 1e-6, &FaerSolver, &|| false);
        let densities = filter.apply(&design);
        let analytic = sensitivities(
            &filter,
            &evaluate(&design)?.grid,
            &densities,
            &request.options,
        );
        let dv = filter.transpose(&[0.125; 8]);
        for i in 0..8 {
            let mut plus = design.clone();
            plus[i] += 1e-5;
            let mut minus = design.clone();
            minus[i] -= 1e-5;
            let dc = (evaluate(&plus)?.compliance.joules() - evaluate(&minus)?.compliance.joules())
                / 2e-5;
            if roles[i] == CellRegion::Design {
                assert!((dc / analytic[i] - 1.0).abs() < 1e-5);
            } else {
                assert_eq!(dc, 0.0);
                assert_eq!(analytic[i], 0.0);
                assert_eq!(dv[i], 0.0);
                assert_eq!(
                    densities[i],
                    if roles[i] == CellRegion::Preserve {
                        1.0
                    } else {
                        0.0
                    }
                );
            }
            let numerical_volume =
                (mean(&filter.apply(&plus)) - mean(&filter.apply(&minus))) / 2e-5;
            assert!((dv[i] - numerical_volume).abs() < 1e-10);
        }
        Ok(())
    }

    #[test]
    fn masks_hold_during_optimization_and_infeasible_budget_is_rejected() -> Result<(), SimpError> {
        let mut request = request();
        request.beam.cells = [4, 2, 1];
        request.options.volume_fraction = 0.65;
        request.options.max_iterations = 4;
        request.regions = vec![
            CellRegion::Preserve,
            CellRegion::Preserve,
            CellRegion::Void,
            CellRegion::Design,
            CellRegion::Design,
            CellRegion::Design,
            CellRegion::Preserve,
            CellRegion::Preserve,
        ];
        let result = optimize_beam(&request, &FaerSolver, &|| false, &mut |_| {})?;
        for (role, density) in request.regions.iter().zip(&result.densities) {
            match role {
                CellRegion::Preserve => assert_eq!(*density, 1.0),
                CellRegion::Void => assert_eq!(*density, 0.0),
                _ => {}
            }
        }
        assert!(
            result
                .history
                .iter()
                .all(|entry| (entry.volume_fraction - 0.65).abs() < 1e-6)
        );
        for fraction in [0.4, 0.95] {
            request.options.volume_fraction = fraction;
            assert!(matches!(
                optimize_beam(&request, &FaerSolver, &|| false, &mut |_| {}),
                Err(SimpError::InfeasibleVolume)
            ));
        }
        request.regions.pop();
        assert!(matches!(
            optimize_beam(&request, &FaerSolver, &|| false, &mut |_| {}),
            Err(SimpError::Fem(FemError::InvalidInput))
        ));
        Ok(())
    }

    #[test]
    fn simp_improves_uniform_reference_at_same_volume() -> Result<(), SimpError> {
        let mut request = request();
        request.options.max_iterations = 100;
        let result = optimize_beam(&request, &FaerSolver, &|| false, &mut |_| {})?;
        let surface = result.reconstruct_surface(0.5, &|| false);
        assert!(surface.is_ok(), "benchmark surface: {surface:?}");
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
