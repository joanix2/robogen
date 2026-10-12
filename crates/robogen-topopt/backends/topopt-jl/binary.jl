# Exact regular-cell reconstruction diagnostics. Caller includes native.jl first.
# The FEM library receives only material cells, not a density floor in empty space.
function binary_components(occupied, counts)
    length(occupied) == prod(counts) || error("Invalid binary grid")
    seen = falses(length(occupied))
    strides = (counts[2]*counts[3], counts[3], 1)
    components = 0
    queue = Int[]
    for start in eachindex(occupied)
        (!occupied[start] || seen[start]) && continue
        components += 1
        push!(queue, start); seen[start] = true
        while !isempty(queue)
            index = pop!(queue)
            for axis in 1:3
                position = ((index-1) ÷ strides[axis]) % counts[axis]
                for sign in (-1,1)
                    0 <= position + sign < counts[axis] || continue
                    neighbor = index + sign * strides[axis]
                    if occupied[neighbor] && !seen[neighbor]
                        seen[neighbor] = true
                        push!(queue, neighbor)
                    end
                end
            end
        end
    end
    return components
end

function reanalyse_binary(request, densities, threshold; solver_type=DirectSolver)
    counts = Int.(request["cells"])
    length(densities) == prod(counts) || error("Invalid reconstruction density count")
    isfinite(threshold) && 0 < threshold <= 1 || error("Invalid reconstruction threshold")
    all(isfinite, densities) && all(0 .<= densities .<= 1) || error("Invalid reconstruction densities")
    occupied = densities .>= threshold
    for (index, role) in enumerate(request["regions"])
        role == "preserve" && !occupied[index] && error("Reconstruction lost preserved material")
        role == "void" && occupied[index] && error("Reconstruction intersects a keepout")
    end
    binary_components(occupied, counts) == 1 || error("Reconstruction is fragmented")
    responses = Float64[]
    element_counts = Int[]
    node_counts = Int[]
    for case in request["cases"]
        problem, _ = native_problem(request, case; occupied)
        solver = FEASolver(solver_type, problem; xmin=1e-6, penalty=PowerPenaltyFun(3.0))
        displacement = DisplacementFun(solver)(PseudoDensities(ones(count(occupied))))
        push!(responses, sqrt(maximum(vec(sum(abs2, displacement.u[problem.metadata.node_dofs]; dims=1)))))
        push!(element_counts, length(problem.ch.dh.grid.cells))
        push!(node_counts, length(problem.ch.dh.grid.nodes))
    end
    all(isfinite, responses) || error("Non-finite binary response")
    mass_kg = count(occupied) * prod(Float64.(request["dimensions_m"]) ./ counts) * request["density_kg_m3"]
    return (; threshold, mass_kg, displacements_m=responses,
        feasible=all(responses .<= request["limits_m"]),
        components=1, element_counts, node_counts, publishable=false)
end
