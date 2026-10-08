using CUDA, Krylov, LinearAlgebra, SparseArrays

pkgversion(CUDA) == v"6.0.0" && pkgversion(Krylov) == v"0.8.4" ||
    error("Unqualified CUDA/Krylov version")

struct CudaCGSolver <: TopOpt.AbstractLinearSolver end

function cuda_solve(matrix, rhs; tolerance=1e-10, max_iterations=20000)
    CUDA.functional() || error("CUDA is unavailable; select the CPU backend")
    eltype(matrix) == Float64 && eltype(rhs) == Float64 || error("CUDA FEM requires Float64")
    host_matrix = sparse(Symmetric(matrix))
    all(isfinite, nonzeros(host_matrix)) || error("Non-finite CUDA matrix")
    diagonal = diag(host_matrix)
    all(value -> isfinite(value) && value > 0, diagonal) ||
        error("CUDA solver requires a finite positive diagonal")
    all(isfinite, rhs) || error("Non-finite CUDA right-hand side")
    device_matrix = CUDA.CUSPARSE.CuSparseMatrixCSR(host_matrix)
    preconditioner = Diagonal(CuArray(1.0 ./ diagonal))
    host_solution = zeros(length(rhs))
    correction_rhs = copy(rhs)
    absolute_matrix = abs.(host_matrix)
    row_terms = maximum(diff(host_matrix.colptr))
    rounding_factor = 8 * row_terms * eps(Float64) / (1 - row_terms * eps(Float64))
    iterations = 0
    residual = Inf
    for refinement in 1:4
        rhs_scale = norm(correction_rhs)
        if iszero(rhs_scale)
            return host_solution, (; iterations, residual=0.0, backward_error=0.0,
                roundoff_limit=0.0, refinement)
        end
        solution, stats = Krylov.cg(device_matrix, CuArray(correction_rhs ./ rhs_scale); M=preconditioner,
            atol=0.0, rtol=tolerance, itmax=max_iterations)
        CUDA.synchronize()
        stats.solved || error("CUDA CG did not converge: $(stats.status)")
        iterations += stats.niter
        host_solution .+= rhs_scale .* Array(solution)
        correction_rhs = rhs - host_matrix * host_solution
        residual = norm(correction_rhs) / max(norm(rhs), eps(Float64))
        all(isfinite, host_solution) && isfinite(residual) || error("Non-finite CUDA solution")
        scale = absolute_matrix * abs.(host_solution) + abs.(rhs)
        backward_error = maximum(abs.(correction_rhs) ./ max.(scale, floatmin(Float64)))
        roundoff_limit = rounding_factor * norm(scale) / max(norm(rhs), eps(Float64))
        if backward_error <= 1e-12 && residual <= max(1e-8, roundoff_limit)
            return host_solution, (; iterations, residual, backward_error, roundoff_limit, refinement)
        end
    end
    error("CUDA true relative residual rejected after refinement: $residual")
end

function TopOpt.FEA.solve_system!(::Type{CudaCGSolver},
    solver::TopOpt.GenericFEASolver{Float64, Physics, CudaCGSolver},
    matrix, rhs, lhs; kwargs...) where {Physics}
    solution, _ = cuda_solve(matrix, rhs)
    copyto!(lhs, solution)
    return false
end