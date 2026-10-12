using TopOpt, JSON, SHA
include("mass.jl")

const Problems = TopOpt.TopOptProblems
const FerriteApi = TopOpt.Ferrite

struct NativeProblem{Handler, Meta} <: Problems.StiffnessTopOptProblem{3, Float64}
    E::Float64
    ν::Float64
    ch::Handler
    metadata::Meta
    loads::Dict{Int, Vector{Float64}}
end
Problems.nnodespercell(::NativeProblem) = 8
Problems.getcloaddict(problem::NativeProblem) = problem.loads

function native_problem(request, load_case; occupied=nothing)
    counts = Tuple(Int.(request["cells"]))
    spacing = Tuple(Float64.(request["dimensions_m"]) ./ collect(counts))
    rect = Problems.RectilinearGrid(counts, spacing; celltype=:Linear)
    grid = rect.grid
    minimums = [minimum(node.x[axis] for node in grid.nodes) for axis in 1:3]
    node_map = zeros(Int, prod(collect(counts) .+ 1))
    for (native_index, node) in enumerate(grid.nodes)
        position = [round(Int, (node.x[axis] - minimums[axis]) / spacing[axis]) for axis in 1:3]
        index = (position[1] * (counts[2] + 1) + position[2]) * (counts[3] + 1) + position[3] + 1
        node_map[index] == 0 || error("Duplicate grid node")
        node_map[index] = native_index
    end
    all(node_map .> 0) || error("Incomplete node mapping")
    cell_map = zeros(Int, prod(counts))
    for (native_index, cell) in enumerate(grid.cells)
        position = [round(Int, (minimum(grid.nodes[node].x[axis] for node in cell.nodes) -
            minimums[axis]) / spacing[axis]) for axis in 1:3]
        index = (position[1] * counts[2] + position[2]) * counts[3] + position[3] + 1
        cell_map[index] == 0 || error("Duplicate grid cell")
        cell_map[index] = native_index
    end
    all(cell_map .> 0) || error("Incomplete cell mapping")
    if occupied !== nothing
        length(occupied) == length(cell_map) && any(occupied) || error("Empty binary solid")
        # Remove void cells AND unused nodes. No ersatz stiffness remains in the
        # reconstructed-solid model. Retain the request-to-Ferrite maps.
        selected_cells = sort(cell_map[occupied])
        used_nodes = sort(unique(vcat([collect(grid.cells[i].nodes) for i in selected_cells]...)))
        remap = zeros(Int, length(grid.nodes))
        remap[used_nodes] = eachindex(used_nodes)
        cell_type = typeof(first(grid.cells))
        cells = [cell_type(Tuple(remap[collect(grid.cells[i].nodes)])) for i in selected_cells]
        grid = FerriteApi.Grid(cells, grid.nodes[used_nodes])
        node_map = remap[node_map]
        cell_remap = zeros(Int, length(cell_map))
        cell_remap[selected_cells] = eachindex(selected_cells)
        cell_map = cell_remap[cell_map]
    end
    handler = FerriteApi.DofHandler(grid)
    interpolation = FerriteApi.Lagrange{FerriteApi.RefHexahedron, 1}()
    FerriteApi.add!(handler, :u, interpolation^3)
    FerriteApi.close!(handler)
    constraints = FerriteApi.ConstraintHandler(handler)
    for axis in 1:3
        nodes = Set(node_map[Int(lock["node"]) + 1] for lock in load_case["constraints"] if lock["fixed"][axis])
        0 in nodes && error("Binary solid lost a supported node")
        if !isempty(nodes)
            FerriteApi.add!(constraints, FerriteApi.Dirichlet(:u, nodes, (position, time) -> 0.0, [axis]))
        end
    end
    FerriteApi.close!(constraints)
    FerriteApi.update!(constraints, 0.0)
    loads = Dict{Int, Vector{Float64}}()
    for load in load_case["forces"]
        node = node_map[Int(load["node"]) + 1]
        node > 0 || error("Binary solid lost a loaded node")
        loads[node] = get(loads, node, zeros(3)) + Float64.(load["force_n"])
    end
    problem = NativeProblem(Float64(request["young_pa"]), Float64(request["poisson"]),
        constraints, Problems.Metadata(handler), loads)
    return problem, cell_map
end

include("binary.jl")

function run_native(request, emit; solver_type=DirectSolver)
    VERSION == v"1.10.10" && pkgversion(TopOpt) == v"0.14.0" || error("Unqualified Julia/TopOpt version")
    for (relative_path, expected) in (
        ("Functions/temperature.jl", "33f82693fbeaee89149ea8454e8b9cb705b02296df3877c6e4e8964d4ac93306"),
        ("CheqFilters/CheqFilters.jl", "142f9d3b8eaba4bc475f60275bfc750ec90fc0e0488fb1b45276c15a0e1fcc38"))
        bytes2hex(sha256(read(joinpath(dirname(pathof(TopOpt)), relative_path)))) == expected ||
            error("Backend compatibility source mismatch: $relative_path")
    end
    request["schema"] == 1 || error("Unsupported request schema")
    request["objective"] == "minimize_mass" || error("Unsupported objective")
    counts = Int.(request["cells"])
    length(counts) == 3 && all(1 .<= counts .<= 256) && prod(counts) <= 32768 || error("Invalid grid budget")
    3 * prod(counts .+ 1) <= 150000 || error("Grid DOF budget exceeded")
    length(request["regions"]) == prod(counts) || error("Invalid region count")
    all(region in ("design", "preserve", "void") for region in request["regions"]) || error("Invalid region")
    1 <= length(request["cases"]) <= 16 || error("Invalid case budget")
    prod(counts) * length(request["cases"]) <= 65536 || error("Cell-case budget exceeded")
    mapped = [native_problem(request, load_case) for load_case in request["cases"]]
    problems = first.(mapped)
    cell_map = last(first(mapped))
    all(last(entry) == cell_map for entry in mapped) || error("Case grid mismatch")
    solid_cells = cell_map[findall(==("preserve"), request["regions"])]
    void_cells = cell_map[findall(==("void"), request["regions"])]
    prepared = prepare_mass(problems; density_kg_m3=Float64(request["density_kg_m3"]),
        solid_cells, void_cells, displacement_limits_m=Float64.(request["limits_m"]),
        filter_radius_m=Float64(request["filter_radius_m"]), solver_type)
    emit((; kind="reference", densities=prepared.physical(prepared.reference).x[cell_map],
        mass_kg=prepared.reference_mass_kg, reference_mass_kg=prepared.reference_mass_kg,
        displacements_m=prepared.responses(prepared.reference)))
    result = solve_mass(prepared; max_iterations=Int(request["max_iterations"]),
        progress=event -> emit((; kind="progress", mass_kg=event.normalized_mass * prepared.reference_mass_kg,
            infeasibility=event.infeasibility)),
        candidate=free -> emit((; kind="iteration", densities=prepared.physical(free).x[cell_map],
            mass_kg=prepared.mass(free), reference_mass_kg=prepared.reference_mass_kg,
            displacements_m=prepared.responses(free))))
    binary = nothing
    reconstruction_error = nothing
    if result.converged && result.feasible
        first_threshold = Float64(get(request, "reconstruction_threshold", 0.65))
        isfinite(first_threshold) && 0 < first_threshold <= 1 || error("Invalid reconstruction threshold")
        for attempt in 0:8
            threshold = first_threshold * 0.8^attempt
            try
                trial = reanalyse_binary(request, result.densities[cell_map], threshold; solver_type)
                if trial.feasible
                    binary = (; trial.threshold, trial.mass_kg, trial.displacements_m)
                    reconstruction_error = nothing
                    break
                end
                reconstruction_error = "Binary displacement limit exceeded at threshold $threshold"
            catch failure
                failure isa InterruptException && rethrow()
                reconstruction_error = sprint(showerror, failure)
            end
        end
    end
    emit((; kind="result", densities=result.densities[cell_map], mass_kg=result.mass_kg,
        reference_mass_kg=result.reference_mass_kg, displacements_m=result.displacements_m,
        violations=result.violations, feasible=result.feasible, converged=result.converged,
        kkt_residual=result.kkt_residual, binary, reconstruction_error, publishable=false))
end

if abspath(PROGRAM_FILE) == @__FILE__
    request = JSON.parse(read(only(ARGS), String))
    compute_backend = get(request, "compute_backend", "cpu")
    protocol = stdout
    emit = event -> begin
        println(protocol, JSON.json(merge((; schema=1, operation=request["operation"],
            revision=request["revision"], compute_backend), event)))
        flush(protocol)
    end
    try
        compute_backend in ("cpu", "cuda") || error("Unknown compute backend")
        if compute_backend == "cuda"
            include("gpu.jl")
            CUDA.functional() || error("CUDA is unavailable; select CPU explicitly")
            CUDA.allowscalar(false)
        end
        solver_type = compute_backend == "cuda" ? CudaCGSolver : DirectSolver
        redirect_stdout(stderr) do
            Base.invokelatest(run_native, request, emit; solver_type)
        end
    catch failure
        emit((; kind="error", message=sprint(showerror, failure)))
        exit(1)
    end
end
