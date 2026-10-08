using TopOpt, Test, LinearAlgebra, SparseArrays
include("native.jl")
include("gpu.jl")

CUDA.allowscalar(false)

@testset "CUDA sparse solve and rejection" begin
    matrix = sparse([4.0 -1.0 0.0; -1.0 4.0 -1.0; 0.0 -1.0 3.0])
    expected = [1.0, 2.0, 3.0]
    solution, report = cuda_solve(matrix, matrix * expected)
    @test solution ≈ expected rtol=1e-9
    @test report.residual <= 1e-8
    @test report.backward_error <= 1e-12
    for amplitude in (1e-16, 1e16)
        scaled, scaled_report = cuda_solve(matrix, amplitude .* (matrix * expected))
        @test scaled ≈ amplitude .* expected rtol=1e-9
        @test scaled_report.backward_error <= 1e-12
    end
    @test_throws ErrorException cuda_solve(spdiagm(0 => [0.0, -1.0]), ones(2))
    @test_throws ErrorException cuda_solve(sparse([1.0 1.0; 1.0 1.0]), [1.0, -1.0])
    @test_throws ErrorException cuda_solve(matrix, ones(3); max_iterations=1)
    zero_solution, _ = cuda_solve(matrix, zeros(3))
    @test iszero(zero_solution)
end

function compare_fem(problem, densities)
    direct = FEASolver(DirectSolver, problem; xmin=1e-6, penalty=PowerPenaltyFun(3.0))
    gpu = FEASolver(CudaCGSolver, problem; xmin=1e-6, penalty=PowerPenaltyFun(3.0))
    direct_displacement = DisplacementFun(direct)
    gpu_displacement = DisplacementFun(gpu)
    input = PseudoDensities(densities)
    cpu_time = @elapsed cpu_result, cpu_pullback = TopOpt.Functions.ChainRulesCore.rrule(direct_displacement, input)
    gpu_time = @elapsed gpu_result, gpu_pullback = TopOpt.Functions.ChainRulesCore.rrule(gpu_displacement, input)
    @test gpu_result.u ≈ cpu_result.u rtol=1e-6
    weights = copy(cpu_result.u) ./ norm(cpu_result.u)
    cpu_adjoint_time = @elapsed cpu_gradient = cpu_pullback(weights)[2].x
    gpu_adjoint_time = @elapsed gpu_gradient = gpu_pullback(weights)[2].x
    @test gpu_gradient ≈ cpu_gradient rtol=1e-5
    direction = [0 < densities[index] < 1 ? sin(index) : 0.0 for index in eachindex(densities)]
    direction ./= norm(direction)
    step = 1e-5
    plus = gpu_displacement(PseudoDensities(densities + step * direction)).u
    minus = gpu_displacement(PseudoDensities(densities - step * direction)).u
    finite_difference = dot(weights, plus - minus) / (2 * step)
    @test dot(gpu_gradient, direction) ≈ finite_difference rtol=1e-3
    println((; cells=length(densities), dofs=length(cpu_result.u), cpu_time, gpu_time,
        cpu_adjoint_time, gpu_adjoint_time,
        relative_displacement_error=norm(gpu_result.u-cpu_result.u)/norm(cpu_result.u),
        relative_gradient_error=norm(gpu_gradient-cpu_gradient)/norm(cpu_gradient)))
end

@testset "CUDA FEM forward and adjoint" begin
    for force in (1.0, -2.0)
        problem = PointLoadCantilever((8, 4, 4), (0.01, 0.01, 0.01), 2e9, 0.35, force)
        compare_fem(problem, [0.4 + 0.2 * abs(sin(index)) for index in 1:128])
    end
end

@testset "Filtered mass CPU versus CUDA" begin
    problems = [PointLoadCantilever((8, 4, 4), (0.01, 0.01, 0.01), 2e9, 0.35, force)
        for force in (1.0, -2.0)]
    results = [solve_mass(prepare_mass(problems; density_kg_m3=1240.0,
        solid_cells=[1, 8], void_cells=[4, 5], displacement_limits_m=[1e-5, 2e-5],
        filter_radius_m=0.015, solver_type); max_iterations=2)
        for solver_type in (DirectSolver, CudaCGSolver)]
    @test results[2].mass_kg ≈ results[1].mass_kg rtol=1e-5
    @test results[2].displacements_m ≈ results[1].displacements_m rtol=1e-5
    @test results[2].densities ≈ results[1].densities rtol=1e-5
    @test all(results[2].densities[[1, 8]] .== 1)
    @test all(results[2].densities[[4, 5]] .== 0)
    @test !results[2].publishable
end

if !isempty(ARGS)
    request = JSON.parse(read(only(ARGS), String))
    @testset "Actual native request CPU versus CUDA" begin
        problem, cell_map = native_problem(request, first(request["cases"]))
        densities = zeros(length(cell_map))
        for (index, role) in enumerate(request["regions"])
            densities[cell_map[index]] = role == "void" ? 0.0 : role == "preserve" ? 1.0 : 0.6
        end
        compare_fem(problem, densities)
    end
    @testset "Actual native mass run CPU versus CUDA" begin
        request["max_iterations"] = 2
        results = []
        for solver_type in (DirectSolver, CudaCGSolver)
            events = []
            seconds = @elapsed run_native(request, event -> push!(events, event); solver_type)
            result = last(events)
            @test result.kind == "result"
            @test !result.publishable
            push!(results, result)
            println((; solver=string(solver_type), cells=request["cells"], seconds,
                iterations=2, result.mass_kg, result.displacements_m, result.converged))
        end
        @test results[2].mass_kg ≈ results[1].mass_kg rtol=1e-5
        @test results[2].densities ≈ results[1].densities rtol=1e-5
        @test results[2].displacements_m ≈ results[1].displacements_m rtol=1e-5
    end
end