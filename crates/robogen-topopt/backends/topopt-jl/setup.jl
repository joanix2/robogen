using Pkg, SHA

const backend_dir = @__DIR__
const workspace_dir = normpath(joinpath(backend_dir, "../../../.."))
const private_depot = joinpath(workspace_dir, "target/topopt-tools/depot")
abspath(first(DEPOT_PATH)) == private_depot ||
    error("Set JULIA_DEPOT_PATH to $private_depot; refusing to patch a shared depot")
Pkg.activate(backend_dir)
Pkg.instantiate()

const source_path = Base.find_package("TopOpt")
isnothing(source_path) && error("TopOpt is not installed")

function patch_checked(relative_path, original_sha256, replacements)
    path = joinpath(dirname(source_path), relative_path)
    startswith(realpath(path), realpath(private_depot) * "/") ||
        error("TopOpt source is outside the private depot")
    source = read(path, String)
    if bytes2hex(sha256(source)) == original_sha256
        patched = replace(source, replacements...)
        original_mode = stat(path).mode
        chmod(path, original_mode | 0o200)
        try
            write(path, patched)
        finally
            chmod(path, original_mode)
        end
    else
        restored = replace(source, (last(pair) => first(pair) for pair in replacements)...)
        bytes2hex(sha256(restored)) == original_sha256 ||
            error("Unexpected TopOpt source; refusing to apply or accept fix: $relative_path")
    end
    println(relative_path, ": ", bytes2hex(sha256(read(path))))
end

patch_checked("Functions/temperature.jl",
    "e8ca0b8e2b55d55d8500488e6c5edf182d754c745729dfa18f6249d5ed0ddfb7", (
    "mutable struct TemperatureFun{\n    T,\n    Tt<:AbstractVector{T}," =>
    "mutable struct TemperatureFun{\n    Scalar,\n    Tt<:AbstractVector{Scalar},",
    "} <: AbstractFunction{T}\n    T::Tt" => "} <: AbstractFunction{Scalar}\n    T::Tt",
    "struct TemperatureResult{T,N,A<:AbstractArray{T,N}} <: AbstractArray{T,N}" =>
    "struct TemperatureResult{Scalar,N,A<:AbstractArray{Scalar,N}} <: AbstractArray{Scalar,N}",
))
patch_checked("CheqFilters/CheqFilters.jl",
    "387cc54ca0f865ac1ea6219c45b8d51693adbc62534ee93f145d509ec8f91ab6", (
    "current_cell_id = cell.cellid" => "current_cell_id = Ferrite.cellid(cell)",
))
println("TopOpt 0.14.0 + robogen-compat2")