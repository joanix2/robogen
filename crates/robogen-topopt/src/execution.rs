//! Explicit execution of a typed DSL topology node. Numerical options are not DSL geometry.
use crate::{
    TopologyRequest,
    fem::{BoundaryConditions, GridDefinition, NodalForce, NodeConstraint, NodeId},
    geometry::{Bounds, Region, Relation},
    linear::SparseLinearSolver,
    simp::{
        CellRegion, GridIteration, GridLoadCase, GridOptimization, GridRequest, SimpError,
        SimpOptions, StopReason, optimize_grid,
    },
    surface::{GridSolid, GridSolidError},
};
use robogen_domain::{Diagnostic, Force, InterfaceId, Length, LoadCaseId, Mass, SourceSpan};
use robogen_ir::topology::{
    ConstraintKind, LoadKind, Objective, SupportKind, TOPOLOGY_SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{diagnostic:?}")]
pub struct ExecutionError {
    pub diagnostic: Diagnostic,
}
pub(crate) fn failure(code: &str, span: SourceSpan, message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        diagnostic: Diagnostic::error(code, message, span),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeBackend {
    #[default]
    Cpu,
    Cuda,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionSettings {
    pub compute_backend: ComputeBackend,
    pub cells: [usize; 3],
    pub filter_radius: Length,
    pub threshold: f64,
    pub max_iterations: usize,
}

pub const MASS_MAX_CELLS: usize = 32_768;
pub const MASS_MAX_AXIS: usize = 256;
pub const MASS_MAX_CASE_CELLS: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub struct GridResolution {
    pub cells: [usize; 3],
    pub cell_size: [Length; 3],
    pub total_cells: usize,
    pub degrees_of_freedom: usize,
}

pub fn grid_resolution(
    specification: &robogen_ir::topology::TopologySpec<robogen_ir::SolidGeometry>,
    cells: [usize; 3],
) -> Result<GridResolution, ExecutionError> {
    let (axis_limit, cell_limit) = if specification.objective == Objective::MinimizeMass {
        (MASS_MAX_AXIS, MASS_MAX_CELLS)
    } else {
        (64, 2048)
    };
    if cells.iter().any(|count| *count == 0 || *count > axis_limit) {
        return Err(failure(
            "E340",
            specification.span,
            format!("Grid axes must be within 1..={axis_limit}"),
        ));
    }
    let total_cells = cells.iter().product::<usize>();
    let degrees_of_freedom = 3 * cells.iter().map(|count| count + 1).product::<usize>();
    if total_cells > cell_limit
        || (specification.objective == Objective::MinimizeMass
            && (degrees_of_freedom > 150_000
                || total_cells * specification.load_cases.len().max(1) > MASS_MAX_CASE_CELLS))
    {
        return Err(failure(
            "E340",
            specification.span,
            format!(
                "Requested grid: {total_cells} cells, {degrees_of_freedom} DOFs; backend limit {cell_limit} cells (mass: 150000 DOFs, {MASS_MAX_CASE_CELLS} cell-cases)"
            ),
        ));
    }
    let (_, _, bounds) = Region::compile(&specification.domain, &mut 4096, 0)?
        .into_grid_frame(specification.domain.span)?;
    Ok(GridResolution {
        cells,
        total_cells,
        degrees_of_freedom,
        cell_size: std::array::from_fn(|axis| {
            Length::from_metres(bounds.dimensions()[axis] / cells[axis] as f64)
        }),
    })
}

pub fn resolution_for_cell_size(
    specification: &robogen_ir::topology::TopologySpec<robogen_ir::SolidGeometry>,
    size: Length,
) -> Result<GridResolution, ExecutionError> {
    if !size.metres().is_finite() || size.metres() <= 0.0 {
        return Err(failure(
            "E340",
            specification.span,
            "Cell size must be finite and positive",
        ));
    }
    let (_, _, bounds) = Region::compile(&specification.domain, &mut 4096, 0)?
        .into_grid_frame(specification.domain.span)?;
    let cells = bounds
        .dimensions()
        .map(|length| (length / size.metres()).ceil().max(1.0) as usize);
    grid_resolution(specification, cells)
}
#[derive(Debug, Clone)]
pub struct InterfaceProjection {
    pub id: InterfaceId,
    pub name: String,
    pub cell_count: usize,
    /// Centroid of the voxelised volume to which the total load is applied.
    pub centroid: [Length; 3],
}
pub struct PreparedTopology {
    pub request: TopologyRequest,
    pub grid: GridRequest,
    pub origin: [Length; 3],
    pub domain_cells: usize,
    pub interfaces: Vec<InterfaceProjection>,
}
pub struct ComputedTopology {
    pub request: TopologyRequest,
    pub origin: [Length; 3],
    pub domain_cells: usize,
    pub interfaces: Vec<InterfaceProjection>,
    pub optimization: GridOptimization,
    pub solid: GridSolid,
    pub mass: Mass,
    pub max_displacements: Vec<(LoadCaseId, Length)>,
}
pub enum ExecutionProgress<'a> {
    Projection { completed: usize, total: usize },
    Optimization(&'a GridIteration),
    Reanalysis(LoadCaseId),
}

pub fn prepare_topology(
    request: &TopologyRequest,
    settings: &ExecutionSettings,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(ExecutionProgress<'_>),
) -> Result<PreparedTopology, ExecutionError> {
    let spec = &request.specification;
    let error = |message| failure("E340", spec.span, message);
    if settings.compute_backend == ComputeBackend::Cuda && spec.objective != Objective::MinimizeMass
    {
        return Err(error(
            "Stiffness projection requires CPU settings; no CUDA engine is connected",
        ));
    }
    if cancelled() {
        return Err(failure("E345", spec.span, "Topology execution cancelled"));
    }
    let issues = spec.preparation_issues();
    if !issues.is_empty() {
        return Err(failure(
            "E347",
            spec.span,
            format!("Incomplete topology preparation: {}", issues.join("; ")),
        ));
    }
    if spec.schema_version != TOPOLOGY_SCHEMA_VERSION || spec.material != Some(request.material.id)
    {
        return Err(error("Schema or material identity mismatch"));
    }
    grid_resolution(spec, settings.cells)?;
    if !settings.filter_radius.metres().is_finite()
        || settings.filter_radius.metres() <= 0.0
        || !settings.threshold.is_finite()
        || !(0.5..1.0).contains(&settings.threshold)
        || settings.max_iterations == 0
        || settings.max_iterations > 500
    {
        return Err(error("Invalid numerical settings or resource budget"));
    }
    if spec.preserve.len() > 256
        || spec.voids.len() > 256
        || spec.constraints.len() > 32
        || spec.load_cases.is_empty()
        || spec.load_cases.len() > 16
    {
        return Err(error(
            "Topology declaration exceeds supported resource limits",
        ));
    }
    if let Some(process) = spec.manufacturing.first() {
        return Err(failure(
            "E340",
            process.span,
            "CPU backend does not yet enforce manufacturing constraints",
        ));
    }
    let density = request.material.density.kg_per_m3();
    if !density.is_finite()
        || density <= 0.0
        || !request.material.young_modulus.pascals().is_finite()
        || request.material.young_modulus.pascals() <= 0.0
        || !request.material.poisson_ratio.is_finite()
        || !(-0.99..0.49).contains(&request.material.poisson_ratio)
    {
        return Err(error("Invalid material density or elastic properties"));
    }
    let mut fraction: Option<f64> = None;
    let mut max_mass: Option<f64> = None;
    for constraint in &spec.constraints {
        if spec.objective == Objective::MinimizeMass
            && !matches!(constraint.kind, ConstraintKind::MaxDisplacement(_))
        {
            return Err(failure(
                "E340",
                constraint.span,
                "Experimental mass backend supports only max_displacement constraints",
            ));
        }
        let bad = || {
            failure(
                "E340",
                constraint.span,
                "Invalid or unsupported topology constraint",
            )
        };
        match constraint.kind {
            ConstraintKind::VolumeFraction(v) if v.is_finite() && v > 0.0 && v <= 1.0 => {
                fraction = Some(fraction.map_or(v, |old| old.min(v)));
            }
            ConstraintKind::MaxMass(m) if m.kilograms().is_finite() && m.kilograms() > 0.0 => {
                max_mass = Some(max_mass.map_or(m.kilograms(), |old| old.min(m.kilograms())));
            }
            ConstraintKind::MaxDisplacement(l) if l.metres().is_finite() && l.metres() > 0.0 => {}
            ConstraintKind::MaxStress(_) => {
                return Err(failure(
                    "E340",
                    constraint.span,
                    "CPU backend does not yet enforce max_stress",
                ));
            }
            ConstraintKind::SafetyFactor(_) => {
                return Err(failure(
                    "E340",
                    constraint.span,
                    "CPU backend does not yet enforce safety_factor",
                ));
            }
            ConstraintKind::MinFrequency(_) => {
                return Err(failure(
                    "E340",
                    constraint.span,
                    "CPU backend does not yet enforce min_frequency",
                ));
            }
            _ => return Err(bad()),
        }
    }
    if spec.objective == Objective::MaximizeStiffness && fraction.is_none() && max_mass.is_none() {
        return Err(error(
            "maximize_stiffness requires a volume_fraction or max_mass budget",
        ));
    }
    let mut budget = 4096;
    let (domain, origin, bounds) =
        Region::compile(&spec.domain, &mut budget, 0)?.into_grid_frame(spec.domain.span)?;
    let preserves = spec
        .preserve
        .iter()
        .map(|p| {
            Region::compile(&p.geometry, &mut budget, 0).map(|r| r.shifted(origin.map(|v| -v)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let voids = spec
        .voids
        .iter()
        .map(|g| Region::compile(g, &mut budget, 0).map(|r| r.shifted(origin.map(|v| -v))))
        .collect::<Result<Vec<_>, _>>()?;
    for (region, source) in voids.iter().zip(&spec.voids) {
        region.checked_bounds(source.span)?;
    }
    for (region, source) in preserves.iter().zip(&spec.preserve) {
        if !bounds.contains(region.checked_bounds(source.span)?) {
            return Err(failure(
                "E341",
                source.span,
                "Preserved region bounds extend beyond the domain grid",
            ));
        }
    }
    let definition = GridDefinition {
        dimensions: bounds.dimensions().map(Length::from_metres),
        cells: settings.cells,
        young_modulus: request.material.young_modulus,
        poisson_ratio: request.material.poisson_ratio,
    };
    let [nx, ny, nz] = settings.cells;
    let total = nx * ny * nz;
    let mut roles = Vec::with_capacity(total);
    let mut counts = vec![0; preserves.len()];
    let mut nodes: Vec<BTreeMap<NodeId, f64>> = vec![BTreeMap::new(); preserves.len()];
    let spacing: [f64; 3] =
        std::array::from_fn(|a| bounds.dimensions()[a] / settings.cells[a] as f64);
    let mut domain_cells = 0;
    for x in 0..nx {
        for y in 0..ny {
            for z in 0..nz {
                if cancelled() {
                    return Err(failure("E345", spec.span, "Topology execution cancelled"));
                }
                let p = [x, y, z];
                let cell = Bounds {
                    min: std::array::from_fn(|a| bounds.min[a] + p[a] as f64 * spacing[a]),
                    max: std::array::from_fn(|a| {
                        if p[a] + 1 == settings.cells[a] {
                            bounds.max[a]
                        } else {
                            bounds.min[a] + (p[a] + 1) as f64 * spacing[a]
                        }
                    }),
                };
                let inside = domain.classify(cell) == Relation::Inside;
                if inside {
                    domain_cells += 1;
                }
                let excluded = voids.iter().any(|v| v.classify(cell) != Relation::Outside);
                let mut mandatory = false;
                for (index, region) in preserves.iter().enumerate() {
                    if region.classify(cell) == Relation::Outside {
                        continue;
                    }
                    if !inside || excluded {
                        return Err(failure(
                            "E341",
                            spec.preserve[index].span,
                            format!(
                                "Preserve/domain/void conflict at cell [{x},{y},{z}]; refine the grid or correct the regions"
                            ),
                        ));
                    }
                    mandatory = true;
                    counts[index] += 1;
                    for corner in 0..8 {
                        let position: [usize; 3] =
                            std::array::from_fn(|axis| p[axis] + ((corner >> axis) & 1));
                        let node =
                            NodeId((position[0] * (ny + 1) + position[1]) * (nz + 1) + position[2]);
                        *nodes[index].entry(node).or_default() += 0.125;
                    }
                }
                roles.push(if mandatory {
                    CellRegion::Preserve
                } else if inside && !excluded {
                    CellRegion::Design
                } else {
                    CellRegion::Void
                });
                progress(ExecutionProgress::Projection {
                    completed: roles.len(),
                    total,
                });
            }
        }
    }
    if cancelled() {
        return Err(failure("E345", spec.span, "Topology execution cancelled"));
    }
    if domain_cells == 0 {
        return Err(failure(
            "E341",
            spec.domain.span,
            "No complete cells fit inside the domain at this resolution",
        ));
    }
    let mut interface_nodes = BTreeMap::new();
    let mut interfaces = Vec::new();
    for (i, source) in spec.preserve.iter().enumerate() {
        if counts[i] == 0 {
            return Err(failure(
                "E341",
                source.span,
                "Preserved region has no volume at this resolution",
            ));
        }
        for weight in nodes[i].values_mut() {
            *weight /= counts[i] as f64;
        }
        if let Some(interface) = &source.interface {
            if interface_nodes
                .insert(interface.id, nodes[i].clone())
                .is_some()
            {
                return Err(failure("E342", source.span, "Duplicate interface ID"));
            }
            let mut centroid = [0.0; 3];
            for (node, w) in &nodes[i] {
                let pos = node_position(*node, settings.cells, spacing, origin);
                for a in 0..3 {
                    centroid[a] += w * pos[a];
                }
            }
            interfaces.push(InterfaceProjection {
                id: interface.id,
                name: interface.name.clone(),
                cell_count: counts[i],
                centroid: centroid.map(Length::from_metres),
            });
        }
    }
    let mut cases = Vec::new();
    let mut ids = BTreeSet::new();
    for case in &spec.load_cases {
        if cancelled() {
            return Err(failure("E345", spec.span, "Topology execution cancelled"));
        }
        if !ids.insert(case.id)
            || case.loads.is_empty()
            || case.supports.is_empty()
            || case.loads.len() > 256
            || case.supports.len() > 256
        {
            return Err(failure(
                "E342",
                case.span,
                "Invalid case IDs, loads, supports or resource budget",
            ));
        }
        let mut fixed: BTreeMap<NodeId, [bool; 3]> = BTreeMap::new();
        let mut forces: BTreeMap<NodeId, [f64; 3]> = BTreeMap::new();
        for support in &case.supports {
            let selected = interface_nodes
                .get(&support.target)
                .ok_or_else(|| failure("E342", support.span, "Unknown support interface"))?;
            let axes = match support.kind {
                SupportKind::Fixed => [true; 3],
                SupportKind::Frictionless { normal } => {
                    let axis = (0..3)
                        .find(|a| {
                            (normal[*a].abs() - 1.0).abs() < 1e-12
                                && (0..3).filter(|b| b != a).all(|b| normal[b].abs() < 1e-12)
                        })
                        .ok_or_else(|| {
                            failure(
                                "E340",
                                support.span,
                                "Oblique frictionless support is not implemented",
                            )
                        })?;
                    std::array::from_fn(|a| a == axis)
                }
                SupportKind::Pin { .. } => {
                    return Err(failure(
                        "E340",
                        support.span,
                        "Pin support projection is not implemented",
                    ));
                }
            };
            for node in selected.keys() {
                let locks = fixed.entry(*node).or_default();
                for a in 0..3 {
                    locks[a] |= axes[a];
                }
            }
        }
        for load in &case.loads {
            if cancelled() {
                return Err(failure("E345", load.span, "Topology execution cancelled"));
            }
            let selected = interface_nodes
                .get(&load.target)
                .ok_or_else(|| failure("E342", load.span, "Unknown load interface"))?;
            match load.kind {
                LoadKind::Force(vector) => {
                    let vector = vector.map(|f| f.newtons());
                    if vector.iter().any(|f| !f.is_finite()) {
                        return Err(failure("E342", load.span, "Non-finite force"));
                    }
                    for (node, weight) in selected {
                        let target = forces.entry(*node).or_default();
                        for a in 0..3 {
                            target[a] += weight * vector[a];
                        }
                    }
                }
                LoadKind::Moment(vector) => {
                    let couple = moment_forces(
                        selected,
                        vector.map(|m| m.newton_metres()),
                        settings.cells,
                        spacing,
                        [0.0; 3],
                        load.span,
                    )?;
                    for (node, value) in couple {
                        let target = forces.entry(node).or_default();
                        for a in 0..3 {
                            target[a] += value[a];
                        }
                    }
                }
            }
        }
        cases.push(GridLoadCase {
            id: case.id,
            span: case.span,
            boundary: BoundaryConditions {
                constraints: fixed
                    .into_iter()
                    .map(|(node, fixed)| NodeConstraint { node, fixed })
                    .collect(),
                forces: forces
                    .into_iter()
                    .map(|(node, force)| NodalForce {
                        node,
                        force: force.map(Force::from_newtons),
                    })
                    .collect(),
            },
        });
    }
    let bounding_volume = spacing.iter().product::<f64>() * total as f64;
    if !bounding_volume.is_finite()
        || bounding_volume <= 0.0
        || !(density * bounding_volume).is_finite()
        || density * bounding_volume <= 0.0
    {
        return Err(error("Invalid physical grid volume"));
    }
    let available = roles.iter().filter(|r| **r != CellRegion::Void).count() as f64 / total as f64;
    let target = fraction
        .map_or(f64::INFINITY, |f| f * domain_cells as f64 / total as f64)
        .min(max_mass.map_or(f64::INFINITY, |m| m / (density * bounding_volume)))
        .min(available);
    Ok(PreparedTopology {
        request: request.clone(),
        origin: origin.map(Length::from_metres),
        domain_cells,
        interfaces,
        grid: GridRequest {
            grid: definition,
            load_cases: cases,
            options: SimpOptions {
                volume_fraction: target,
                penalty: 3.0,
                stiffness_floor: 1e-6,
                filter_radius: settings.filter_radius,
                move_limit: 0.2,
                change_tolerance: 0.005,
                max_iterations: settings.max_iterations,
            },
            regions: roles,
            document_revision: request.document_revision,
        },
    })
}

pub fn execute_topology(
    request: &TopologyRequest,
    settings: &ExecutionSettings,
    solver: &dyn SparseLinearSolver,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(ExecutionProgress<'_>),
) -> Result<ComputedTopology, ExecutionError> {
    let span = request.specification.span;
    if cancelled() {
        return Err(failure("E345", span, "Topology execution cancelled"));
    }
    if request.specification.objective != Objective::MaximizeStiffness {
        return Err(failure("E340", span, crate::mass::MASS_BACKEND));
    }
    let prepared = prepare_topology(request, settings, cancelled, progress)?;
    let optimization = optimize_grid(&prepared.grid, solver, cancelled, &mut |entry| {
        progress(ExecutionProgress::Optimization(entry))
    })
    .map_err(|e| simp_error(span, e))?;
    if optimization.stop_reason == StopReason::IterationLimit {
        return Err(failure(
            "E346",
            span,
            "SIMP reached the iteration limit; no converged solid published",
        ));
    }
    let solid = optimization
        .reconstruct_solid(settings.threshold, solver, cancelled, &mut |id| {
            progress(ExecutionProgress::Reanalysis(id))
        })
        .map_err(|e| match e {
            GridSolidError::Solve(error) => simp_error(span, error),
            GridSolidError::Surface(crate::surface::SurfaceError::Cancelled) => {
                failure("E345", span, "Topology execution cancelled")
            }
            other => error_text(span, other),
        })?;
    if !solid.within_volume_budget {
        return Err(failure(
            "E344",
            span,
            format!(
                "Binary volume fraction {} exceeds the projected budget {}",
                solid.volume_fraction, prepared.grid.options.volume_fraction
            ),
        ));
    }
    let mass = Mass::from_kilograms(
        solid.surface.enclosed_volume.cubic_metres() * request.material.density.kg_per_m3(),
    );
    let max_displacements: Vec<_> = solid
        .analyses
        .iter()
        .map(|case| {
            (
                case.id,
                Length::from_metres(
                    case.analysis
                        .displacements
                        .iter()
                        .map(|p| p.iter().fold(0.0_f64, |norm, u| norm.hypot(u.metres())))
                        .fold(0.0, f64::max),
                ),
            )
        })
        .collect();
    for constraint in &request.specification.constraints {
        match constraint.kind {
            ConstraintKind::MaxDisplacement(limit) => {
                if let Some((id, value)) = max_displacements
                    .iter()
                    .find(|(_, value)| value.metres() > limit.metres())
                {
                    return Err(failure(
                        "E344",
                        constraint.span,
                        format!(
                            "Load case {id}: maximum displacement {} m exceeds {} m",
                            value.metres(),
                            limit.metres()
                        ),
                    ));
                }
            }
            ConstraintKind::MaxMass(limit)
                if mass.kilograms() > limit.kilograms() * (1.0 + 1e-10) =>
            {
                return Err(failure(
                    "E344",
                    constraint.span,
                    "Reconstructed mass exceeds max_mass",
                ));
            }
            _ => {}
        }
    }
    if cancelled() {
        return Err(failure("E345", span, "Topology execution cancelled"));
    }
    Ok(ComputedTopology {
        request: request.clone(),
        origin: prepared.origin,
        domain_cells: prepared.domain_cells,
        interfaces: prepared.interfaces,
        optimization,
        solid,
        mass,
        max_displacements,
    })
}
fn simp_error(span: SourceSpan, error: SimpError) -> ExecutionError {
    match error {
        SimpError::Case {
            source: crate::fem::FemError::Cancelled,
            ..
        }
        | SimpError::Fem(crate::fem::FemError::Cancelled) => {
            failure("E345", span, "Topology execution cancelled")
        }
        SimpError::Case { id, span, source } => {
            failure("E343", span, format!("Load case {id}: {source}"))
        }
        other => error_text(span, other),
    }
}
fn error_text(span: SourceSpan, error: impl std::fmt::Display) -> ExecutionError {
    failure("E343", span, error.to_string())
}
fn node_position(node: NodeId, cells: [usize; 3], spacing: [f64; 3], origin: [f64; 3]) -> [f64; 3] {
    let p = [
        node.0 / ((cells[1] + 1) * (cells[2] + 1)),
        node.0 / (cells[2] + 1) % (cells[1] + 1),
        node.0 % (cells[2] + 1),
    ];
    std::array::from_fn(|a| origin[a] + p[a] as f64 * spacing[a])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn moment_forces(
    nodes: &BTreeMap<NodeId, f64>,
    moment: [f64; 3],
    cells: [usize; 3],
    spacing: [f64; 3],
    origin: [f64; 3],
    span: SourceSpan,
) -> Result<Vec<(NodeId, [f64; 3])>, ExecutionError> {
    let mut center = [0.0; 3];
    for (node, w) in nodes {
        let p = node_position(*node, cells, spacing, origin);
        for a in 0..3 {
            center[a] += w * p[a];
        }
    }
    let points: Vec<_> = nodes
        .iter()
        .map(|(node, w)| {
            let p = node_position(*node, cells, spacing, origin);
            (
                *node,
                *w,
                std::array::from_fn::<_, 3, _>(|a| p[a] - center[a]),
            )
        })
        .collect();
    let mut tensor = [[0.0; 4]; 3];
    for (_, w, r) in &points {
        let norm: f64 = r.iter().map(|v| v * v).sum();
        for a in 0..3 {
            for b in 0..3 {
                tensor[a][b] += w
                    * (if a == b {
                        norm - r[a] * r[b]
                    } else {
                        -r[a] * r[b]
                    });
            }
        }
    }
    let scale = tensor
        .iter()
        .flat_map(|row| &row[..3])
        .copied()
        .fold(0.0_f64, |m, v| m.max(v.abs()));
    for a in 0..3 {
        tensor[a][3] = moment[a];
    }
    for col in 0..3 {
        let pivot = (col..3)
            .max_by(|a, b| tensor[*a][col].abs().total_cmp(&tensor[*b][col].abs()))
            .ok_or_else(|| failure("E342", span, "Invalid moment interface"))?;
        tensor.swap(col, pivot);
        if !tensor[col][col].is_finite() || tensor[col][col].abs() <= scale * 1e-12 {
            return Err(failure(
                "E342",
                span,
                "Interface cannot carry the requested moment",
            ));
        }
        let divisor = tensor[col][col];
        for j in col..4 {
            tensor[col][j] /= divisor;
        }
        for row in 0..3 {
            if row == col {
                continue;
            }
            let factor = tensor[row][col];
            for j in col..4 {
                tensor[row][j] -= factor * tensor[col][j];
            }
        }
    }
    let rotation = [tensor[0][3], tensor[1][3], tensor[2][3]];
    let forces: Vec<_> = points
        .iter()
        .map(|(node, w, r)| (*node, cross(rotation, *r).map(|f| f * w)))
        .collect();
    if forces.iter().any(|(_, f)| f.iter().any(|v| !v.is_finite())) {
        return Err(failure("E342", span, "Non-finite moment forces"));
    }
    Ok(forces)
}
