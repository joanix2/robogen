include("mass.jl")
using JSON
using Downloads

output_dir = normpath(joinpath(@__DIR__, "../../../../generated/topopt-report"))
mkpath(output_dir)
problems = [PointLoadCantilever((8, 4, 4), (0.01, 0.01, 0.01),
    69.0e9, 0.3, force_n) for force_n in (1.0, -2.0)]
solid_cells = sort(union(collect(1:8:128), collect(8:8:128)))
void_cells = [4, 5]
limits_m = [8e-8, 1.6e-7]
prepared = prepare_mass(problems; density_kg_m3=2700.0, solid_cells, void_cells,
    displacement_limits_m=limits_m, filter_radius_m=0.015)
history = []
result = solve_mass(prepared; max_iterations=500, progress=event -> push!(history,
    (; mass_g=event.normalized_mass * prepared.reference_mass_kg * 1000,
        infeasibility=isfinite(event.infeasibility) ? event.infeasibility : nothing,
        final=event.final)))
grid = first(problems).ch.dh.grid
reference_densities = prepared.physical(prepared.reference).x
cells = [let coordinates = [grid.nodes[node].x for node in cell.nodes]
    (; minimum_mm=[minimum(point[axis] for point in coordinates) * 1000 for axis in 1:3],
        maximum_mm=[maximum(point[axis] for point in coordinates) * 1000 for axis in 1:3],
        density=result.densities[index],
        reference_density=reference_densities[index],
        region=index in solid_cells ? "preserve" : index in void_cells ? "void" : "design")
end for (index, cell) in enumerate(grid.cells)]
@assert length(cells) == 128
@assert all(cell -> all(cell.maximum_mm .> cell.minimum_mm), cells)
@assert all(isfinite, result.densities)
@assert all(result.densities[solid_cells] .== 1)
@assert all(result.densities[void_cells] .== 0)
integrated_mass_kg = sum(cell.density * prod((cell.maximum_mm .- cell.minimum_mm) ./ 1000)
    for cell in cells) * 2700
@assert isapprox(integrated_mass_kg, result.mass_kg; rtol=1e-10)
@assert isapprox(sum(cell.reference_density * prod((cell.maximum_mm .- cell.minimum_mm) ./ 1000)
    for cell in cells) * 2700, result.reference_mass_kg; rtol=1e-10)
data = (; schema=1, title="TopOpt - calcul de qualification 3D",
    backend="TopOpt 0.14.0 + robogen-compat2", fixture="8 x 4 x 4 cellules, 80 x 40 x 40 mm",
    robot_part=false, publishable=result.publishable, converged=result.converged,
    feasible=result.feasible, kkt_residual=result.kkt_residual, kkt_limit=1e-4,
    mass_g=result.mass_kg * 1000, reference_mass_g=result.reference_mass_kg * 1000,
    displacement_m=result.displacements_m, displacement_limit_m=limits_m,
    forces_n=[1.0, -2.0], violations=result.violations, violation_limit=1e-5,
    cells, history)
write(joinpath(output_dir, "data.json"), JSON.json(data))
cp(joinpath(@__DIR__, "viewer.html"), joinpath(output_dir, "index.html"); force=true)
for (filename, url) in [
    ("three.module.min.js", "https://cdn.jsdelivr.net/npm/three@0.160.1/build/three.module.min.js"),
    ("OrbitControls.js", "https://cdn.jsdelivr.net/npm/three@0.160.1/examples/jsm/controls/OrbitControls.js"),
    ("lucide.min.js", "https://cdn.jsdelivr.net/npm/lucide@0.468.0/dist/umd/lucide.min.js")]
    destination = joinpath(output_dir, filename)
    if !isfile(destination)
        Downloads.download(url, destination)
    end
end
println("Report data: ", joinpath(output_dir, "data.json"))
println((; data.mass_g, data.reference_mass_g, data.feasible, data.converged))