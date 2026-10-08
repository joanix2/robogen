using TopOpt, Test
include("mass.jl")

@testset "Minimum mass: 3D library qualification, not robot certification" begin
    cells = (8, 4, 4)
    spacing_m = (0.01, 0.01, 0.01)
    density_kg_m3 = 2700.0
    problem = PointLoadCantilever(cells, spacing_m, 69.0e9, 0.3, 1.0)
    solid_cells = collect(1:8:prod(cells))
    void_cells = [4, 5]
    projector = get_fixed_element_projector(problem, solid_cells, void_cells)
    free_count = get_free_variable_count(projector)
    solver = FEASolver(DirectSolver, problem; xmin=1e-6, penalty=PowerPenaltyFun(3.0))
    compliance = ComplianceFun(solver)
    volume_fraction = VolumeFun(solver)
    physical = free -> PseudoDensities(projector(free))
    reference = ones(free_count)
    reference_compliance = compliance(physical(reference))
    compliance_limit_j = 4.0 * reference_compliance
    full_mass_kg = prod(cells) * prod(spacing_m) * density_kg_m3
    objective = free -> volume_fraction(physical(free))
    constraint = free -> compliance(physical(free)) / compliance_limit_j - 1.0
    model = Model(objective)
    addvar!(model, zeros(free_count), ones(free_count))
    add_ineq_constraint!(model, constraint)
    result = optimize(model, MMA87(), reference;
        options=MMAOptions(; maxiter=200, tol=Nonconvex.Tolerance(; kkt=1e-4)))
    densities = projector(result.minimizer)
    final_constraint = constraint(result.minimizer)
    @test all(isfinite, densities)
    @test all(value -> 0.0 <= value <= 1.0, densities)
    @test all(densities[solid_cells] .== 1.0)
    @test all(densities[void_cells] .== 0.0)
    @test final_constraint <= 1e-3
    @test 0.0 < objective(result.minimizer) < 0.95 * objective(reference)
    @test_throws ArgumentError get_fixed_element_projector(problem, [1], [1])
    println((backend="TopOpt", version=string(pkgversion(TopOpt)),
        reference_mass_kg=full_mass_kg * objective(reference),
        density_mass_kg=full_mass_kg * objective(result.minimizer),
        compliance_limit_j, normalized_violation=final_constraint,
        publishable=false))
end

@testset "Direct displacement limits, filtered regions, independent cases" begin
    problems = [PointLoadCantilever((8, 4, 4), (0.01, 0.01, 0.01),
        69.0e9, 0.3, force_n) for force_n in (1.0, -2.0)]
    solid_cells = sort(union(collect(1:8:128), collect(8:8:128)))
    void_cells = [4, 5]
    prepared = prepare_mass(problems; density_kg_m3=2700.0, solid_cells, void_cells,
        displacement_limits_m=[8e-8, 1.6e-7], filter_radius_m=0.015)
    reference_responses = [sqrt(maximum(measure(prepared.reference)))
        for measure in prepared.squared_displacements]
    @test reference_responses[2] ≈ 2 * reference_responses[1]
    @test all(constraint(prepared.reference) < 0 for constraint in prepared.constraints)
    filtered = prepared.physical(fill(0.5, length(prepared.reference))).x
    @test all(filtered[solid_cells] .== 1)
    @test all(filtered[void_cells] .== 0)
    events = []
    result = solve_mass(prepared; max_iterations=500, progress=event -> push!(events, event))
    @test result.feasible
    @test result.converged
    @test result.kkt_residual <= 1e-4
    @test 0 < result.mass_kg < 0.95 * result.reference_mass_kg
    @test all(result.densities[solid_cells] .== 1)
    @test all(result.densities[void_cells] .== 0)
    @test result.displacements_m[2] ≈ 2 * result.displacements_m[1]
    @test !result.publishable
    @test last(events).final
    @test_throws InterruptException solve_mass(prepared; cancelled=() -> true)
    @test_throws ArgumentError solve_mass(prepared; max_iterations=0)
    @test_throws ArgumentError prepare_mass(problems; density_kg_m3=NaN,
        solid_cells, void_cells, displacement_limits_m=[8e-8, 1.6e-7], filter_radius_m=0.015)
    println((; qualification="direct displacement", result.mass_kg,
        result.reference_mass_kg, result.displacements_m, result.violations,
        result.feasible, result.converged, result.kkt_residual, result.publishable))
end