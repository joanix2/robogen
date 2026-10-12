using TopOpt

# Conservative smooth maximum of nonnegative squared, normalized displacements.
# No division by the node count: the p-norm bounds every individual node.
# Scaling avoids overflow; the algebra cancels the derivative of the maximum,
# including at tied critical nodes. The zero case also has a finite gradient.
function displacement_envelope(values)
    scale = maximum(values)
    iszero(scale) && return sum(values)
    return scale * sum((values ./ scale).^16)^(1 / 16)
end

function prepare_mass(problems; density_kg_m3, solid_cells, void_cells,
    displacement_limits_m, filter_radius_m, solver_type=DirectSolver)
    1 <= length(problems) <= 16 || throw(ArgumentError("Expected 1..16 independent cases"))
    length(displacement_limits_m) == length(problems) ||
        throw(ArgumentError("Each case needs its own displacement limit"))
    all(value -> isfinite(value) && value > 0,
        [density_kg_m3, filter_radius_m, displacement_limits_m...]) ||
        throw(ArgumentError("Density, filter radius and limits must be positive SI values"))
    grid = first(problems).ch.dh.grid
    1 <= length(grid.cells) <= 32768 || throw(ArgumentError("Cell budget exceeded"))
    length(grid.cells) * length(problems) <= 65536 || throw(ArgumentError("Cell-case budget exceeded"))
    3 * length(grid.nodes) <= 150000 || throw(ArgumentError("Grid DOF budget exceeded"))
    for problem in problems
        size(problem.metadata.node_dofs, 1) == 3 ||
            throw(ArgumentError("Only 3D structural problems are qualified"))
        problem.ch.dh.grid.nodes == grid.nodes && problem.ch.dh.grid.cells == grid.cells ||
            throw(ArgumentError("Cases must share coordinates and cell ordering"))
    end
    projector = get_fixed_element_projector(first(problems), solid_cells, void_cells)
    free_count = get_free_variable_count(projector)
    free_count > 0 || throw(ArgumentError("No free design elements"))
    solvers = [FEASolver(solver_type, problem;
        xmin=1e-6, penalty=PowerPenaltyFun(3.0)) for problem in problems]
    density_filter = DensityFilterFun(first(solvers); rmin=filter_radius_m)
    physical = free -> PseudoDensities(projector(
        density_filter(PseudoDensities(projector(free))).x[projector.free]))
    volume = VolumeFun(first(solvers); fraction=false)
    reference = ones(free_count)
    reference_mass_kg = density_kg_m3 * volume(physical(reference))
    mass = free -> density_kg_m3 * volume(physical(free))
    displacement_functions = [DisplacementFun(solver) for solver in solvers]
    squared_displacements = [let displacement = displacement_functions[case_index],
        node_dofs = problems[case_index].metadata.node_dofs
        free -> vec(sum(abs2, displacement(physical(free)).u[node_dofs]; dims=1))
    end for case_index in eachindex(problems)]
    constraints = [let measure = squared_displacements[case_index],
        limit = displacement_limits_m[case_index]
        free -> displacement_envelope(measure(free) ./ limit^2) - 1.0
    end for case_index in eachindex(problems)]
    # Diagnostic callbacks are outside AD. Reuse the forward state just evaluated
    # by MMA only when its physical density matches exactly. Adjoint solves use
    # solver.lhs, leaving solver.u intact in the pinned TopOpt adapter.
    responses = free -> begin
        field = physical(free)
        [let displacement = displacement_functions[index],
            node_dofs = problems[index].metadata.node_dofs
            if displacement.fevals == 0 || displacement.solver.vars != field.x
                displacement(field)
            end
            sqrt(maximum(vec(sum(abs2, displacement.solver.u[node_dofs]; dims=1))))
        end for index in eachindex(problems)]
    end
    return (; physical, mass, reference_mass_kg, reference, constraints,
        squared_displacements, projector, displacement_limits_m, responses)
end

function solve_mass(prepared; max_iterations=200, cancelled=() -> false,
    progress=identity, candidate=identity)
    1 <= max_iterations <= 500 || throw(ArgumentError("Iteration budget must be 1..500"))
    cancelled() && throw(InterruptException())
    model = Model(free -> prepared.mass(free) / prepared.reference_mass_kg)
    addvar!(model, zeros(length(prepared.reference)), ones(length(prepared.reference)))
    for constraint in prepared.constraints
        add_ineq_constraint!(model, constraint)
    end
    callback = function (solution; update=false)
        cancelled() && throw(InterruptException())
        progress((; normalized_mass=solution.f,
            infeasibility=solution.convstate.infeas, final=update))
        candidate(solution.x)
    end
    result = optimize(model, MMA87(), copy(prepared.reference); callback,
        options=MMAOptions(; maxiter=max_iterations,
            tol=Nonconvex.Tolerance(; kkt=1e-4, infeas=1e-5)))
    cancelled() && throw(InterruptException())
    densities = prepared.physical(result.minimizer).x
    displacements_m = prepared.responses(result.minimizer)
    # Report the declared physical limits, not the conservative surrogate.
    violations = (displacements_m ./ prepared.displacement_limits_m).^2 .- 1.0
    finite = all(isfinite, densities) && all(isfinite, displacements_m) &&
        all(isfinite, violations)
    return (; densities, mass_kg=prepared.mass(result.minimizer),
        reference_mass_kg=prepared.reference_mass_kg, displacements_m, violations,
        feasible=finite && maximum(violations) <= 1e-5,
        converged=result.convstate.converged,
        kkt_residual=result.convstate.kkt_residual, publishable=false)
end
