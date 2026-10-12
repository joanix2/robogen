using TopOpt, Test
include("native.jl")

function binary_fixture()
    cells = [4,2,2]
    node(x,y,z) = (x*3+y)*3+z
    locks = [Dict("node"=>node(0,y,z), "fixed"=>[true,true,true]) for y in 0:2 for z in 0:1]
    cases = [Dict("constraints"=>deepcopy(locks),
        "forces"=>[Dict("node"=>node(4,1,1), "force_n"=>[0.,0.,f])]) for f in (-1.,2.)]
    regions = [z==0 ? (x==0 || x==3 ? "preserve" : "design") : "void"
        for x in 0:3 for y in 0:1 for z in 0:1]
    Dict("cells"=>cells,"dimensions_m"=>[.04,.02,.02], "young_pa"=>2e9,"poisson"=>.35,
        "density_kg_m3"=>1240., "regions"=>regions, "cases"=>cases, "limits_m"=>[.001,.002])
end

@testset "Binary reanalysis removes empty elements and unused DOFs" begin
    request = binary_fixture()
    densities = [role == "void" ? 0. : 1. for role in request["regions"]]
    result = reanalyse_binary(request,densities,.5)
    @test result.feasible
    @test !result.publishable
    @test result.element_counts == [8,8]
    @test result.node_counts == [30,30]
    @test result.mass_kg ≈ .04*.02*.01*1240
    @test result.displacements_m[2] ≈ 2*result.displacements_m[1]
    # Independent full-grid half-height model, not ersatz void verification.
    thin = deepcopy(request)
    thin["cells"]=[4,2,1];thin["dimensions_m"]=[.04,.02,.01]
    for case in thin["cases"]
        for lock in case["constraints"]
            n=lock["node"];lock["node"]=(n÷9*3+n÷3%3)*2+n%3
        end
        for force in case["forces"]
            n=force["node"];force["node"]=(n÷9*3+n÷3%3)*2+n%3
        end
    end
    for (index,case) in enumerate(thin["cases"])
        problem,_ = native_problem(thin,case)
        solver=FEASolver(DirectSolver,problem;xmin=1e-6,penalty=PowerPenaltyFun(3.))
        u=DisplacementFun(solver)(PseudoDensities(ones(8))).u
        maximum_u=sqrt(maximum(vec(sum(abs2,u[problem.metadata.node_dofs];dims=1))))
        @test result.displacements_m[index] ≈ maximum_u rtol=1e-10
    end
    broken=copy(densities);broken[5:8].=0
    @test_throws ErrorException reanalyse_binary(request,broken,.5)
    lost=copy(densities);lost[1]=0
    @test_throws ErrorException reanalyse_binary(request,lost,.5)
    keepout=copy(densities);keepout[2]=1
    @test_throws ErrorException reanalyse_binary(request,keepout,.5)
    tight=deepcopy(request);tight["limits_m"]=[1e-15,1e-15]
    @test !reanalyse_binary(tight,densities,.5).feasible
    @test binary_components([true,false,false,true],[2,2,1])==2
    @test_throws ErrorException reanalyse_binary(request,densities,NaN)
end
