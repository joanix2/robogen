using TopOpt, Test, LinearAlgebra
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

@testset "Conservative smooth displacement constraint and composed gradients" begin
    for values in ([0.0, 0.0], [1.0, 1.0], [0.1, 0.8, 0.2], [1e100, 2e100], [1e-100, 2e-100])
        bound = displacement_envelope(values)
        @test isfinite(bound)
        @test bound >= maximum(values)
        @test bound <= length(values)^(1/16) * maximum(values) * (1 + 1e-14)
    end
    value_gradient = TopOpt.Nonconvex.NonconvexCore.value_gradient
    _, gradient = value_gradient(displacement_envelope, [1.0, 1.0])
    @test gradient[1] ≈ gradient[2]
    @test all(isfinite, last(value_gradient(displacement_envelope, [0.0, 0.0])))
    problems = [PointLoadCantilever((8,4,4), (.01,.01,.01), 69e9, .3, f) for f in (1., -2.)]
    prepared = prepare_mass(problems; density_kg_m3=2700.,
        solid_cells=sort(union(collect(1:8:128),collect(8:8:128))), void_cells=[4,5],
        displacement_limits_m=[8e-8,1.6e-7], filter_radius_m=.015)
    design = [.55+.1*sin(i) for i in eachindex(prepared.reference)]
    direction = sin.(collect(eachindex(design))); direction ./= norm(direction)
    for f in (x -> prepared.mass(x)/prepared.reference_mass_kg, prepared.constraints...)
        _, gradient = value_gradient(f, design)
        for h in (1e-4, 1e-5)
            numerical = (f(design+h*direction)-f(design-h*direction))/(2h)
            @test dot(gradient,direction) ≈ numerical rtol=1e-5 atol=1e-9
        end
    end
    for (f, measure, limit) in zip(prepared.constraints, prepared.squared_displacements,
        prepared.displacement_limits_m)
        @test f(design) >= maximum(measure(design))/limit^2 - 1
    end
end


@testset "Diagnostic reuse never changes independent forward responses" begin
    problems = [PointLoadCantilever((8,4,4), (.01,.01,.01), 69e9, .3, f) for f in (1., -2.)]
    p = prepare_mass(problems; density_kg_m3=2700., solid_cells=[1,8], void_cells=[4,5],
        displacement_limits_m=[8e-8,1.6e-7], filter_radius_m=.015)
    for value in (1.0, 0.6, 0.4, 0.6)
        design = fill(value, length(p.reference))
        # Includes first use (solver variables alone are not a valid cache key).
        expected = p.responses(design)
        direct = [sqrt(maximum(measure(design))) for measure in p.squared_displacements]
        @test expected ≈ direct
        for f in p.constraints
            TopOpt.Nonconvex.NonconvexCore.value_gradient(f, design)
        end
        @test p.responses(design) ≈ direct
        @test p.responses(design)[2] ≈ 2 * p.responses(design)[1]
    end
end
