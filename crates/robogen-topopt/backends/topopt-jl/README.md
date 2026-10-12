# TopOpt.jl native diagnostics (not qualified for publication)

Status, iteration 17: the previously failing displacement qualification now
passes using a conservative smooth displacement envelope. Binary candidates
are reconstructed and independently reanalysed with empty cells removed.
The native UI offers their explicit display. They remain diagnostic geometry,
not manufacturing exports: manifold surface checks, real attachment validation,
stress/frequency/manufacturing capabilities remain separate work.
LeftShinSupport in mini_biped is the demonstrated target.

## Native Integration

The regular-grid mass adapter now permits 32768 cells / 256 per axis, at most
150000 DOFs and 65536 cell-cases. Native controls default to a 2.5 mm target and
show actual XYZ spacing. Increasing resolution does not change the physical
filter radius or mechanical tolerances. Excessive requests are refused.
The historical Rust solver's smaller limits are unchanged.

Refinement check:

```sh
cargo test -p robogen-topopt --test declared native_mass_refined_grid -- --ignored --nocapture
```

Measured 48x16x8 (6144 cells), two iterations: 78.6 seconds including startup,
~3.12 GiB peak RSS. Reference 259.2 g, candidate 72.008 g, not converged.
This is a runtime/refinement transport check, not a mesh-convergence certificate.
The CPU path remains available. The optional native CUDA adapter below now uses
the RTX 3060; it does not rely on TopOpt's old commented-out CUDA assembly code.

## Optional CUDA Solver

Native Optimisation offers CPU (default) and GPU CUDA for minimum-mass diagnostics.
CUDA.jl 6.0.0 and Krylov.jl 0.8.4 are pinned and checked at load time. GPUCompiler
from CUDA 6.4.2 failed on the pinned Julia runtime; do not upgrade that dependency
without rerunning qualification. No driver/toolkit installation or privilege is
required here. CPU execution does not import CUDA.

The adapter uses cuSPARSE and library CG with diagonal preconditioning in Float64
for forward and adjoint solves. Assembly, filtering, sensitivity contraction and
MMA remain on CPU. Both backends keep the same grid, loads and physical limits.
CUDA unavailable/nonconverged/invalid results are explicit errors, never a silent
CPU fallback. ComputeBackend is checked on every native protocol event and shown
with the diagnostic, independently of the next-run setting.

CG requests rtol=1e-10, normalizes the RHS and permits at most four residual
corrections of 20000 iterations. Acceptance requires componentwise backward error
<=1e-12 and true relative residual <=max(1e-8, measured roundoff bound).
The bound is 8*gamma_n*norm(abs(K)*abs(u)+abs(f))/norm(f), gamma_n=n*eps/(1-n*eps),
where n is the maximum terms per symmetric matrix row. A strict 1e-8 raw residual
alone rejected a near-void trial even on CPU (3.49e-7 CPU / 3.11e-7 GPU).
This criterion does not guarantee forward accuracy in ill-conditioned designs.
Mechanical acceptance/publication rules are unchanged; see ADR-008.

Measured on the actual estimated PLA left-shin request, 28x24x36 cells, 80475 DOFs,
two independent loads, 2.5 mm target and unchanged 12 mm filter:

| Work | CPU | CUDA |
|---|---:|---:|
| FEM evaluation at design density 0.6 | 11.90 s | 1.47 s |
| Adjoint including sensitivity contraction | 2.43 s | 3.44 s |
| Full two-iteration mass run | 300.27 s | 105.18 s |

Full-run speedup ~2.85x, within one Julia session after qualification warmup;
not a promise for every grid or iteration and not cold-start timing. Forward
relative error 1.11e-12, gradient error 2.55e-12. Both runs give 56.134595 g,
displacements 1.887961 / 1.243991 mm: **not converged or feasible** against 0.5 mm.
These are the real biped hypotheses, not a certified printable support.

Reproduce (after local setup):

```sh
cargo test -p robogen-topopt export_biped_gpu_qualification_request -- --ignored
JULIA_DEPOT_PATH="$PWD/target/topopt-tools/depot" OPENBLAS_NUM_THREADS=1 \
  target/topopt-tools/julia-1.10.10/bin/julia --startup-file=no \
  --project=crates/robogen-topopt/backends/topopt-jl \
  crates/robogen-topopt/backends/topopt-jl/gpu_qualification.jl \
  target/left-shin-gpu-request.json
cargo test -p robogen-topopt --test declared native_mass_cuda_process -- --ignored
cargo test -p robogen-ui native_mass_view -- --ignored
```

Without the JSON argument the Julia script runs the smaller solver/gradient/mass
suite only. The request exporter regenerates numerical inputs from the current
DSL into ignored target storage. Tests cover singular/invalid systems, RHS scale,
CPU/GPU displacements/gradients, finite differences, masks, two independent cases,
native transport and rendering without export. Small systems can be slower on GPU.

## Native Protocol

`native.jl` accepts the versioned numerical request produced by `src/mass.rs`.
It constructs a library problem from the projected grid, per-axis supports and
nodal forces. Cell/node indices are mapped by coordinates, not assumed to match
Ferrite ordering. Each load case keeps its own solve. No robot load, material or
missing attachment is inferred. Only minimize_mass under max_displacement is
accepted; unsupported constraints are refused before launching Julia.

Project's existing worker supervises Julia locally and exposes reference/current
densities, mass history, per-case displacements and convergence in the native
Optimisation view. Diagnostic meshes never enter ProjectSnapshot.meshes or
TopologyReport, and never enable STL export, even if numerical convergence is
reported. Source changes discard diagnostics; cancelled/obsolete jobs cannot
publish further events. A cancelled or failed job keeps its last accepted field
as a diagnostic, with the corresponding failure/cancelled state.

The process uses a private temporary JSON input and JSONL output, a four-message
bounded queue and a one-MiB message limit. Rust checks schema/operation/revision,
sequence, density masks, integrated mass and displacement violations. Only
32 machine epsilons of density-bound roundoff are clamped; mechanical tolerances
are unchanged. Cancellation kills/reaps the child, including during Julia startup;
the wall-clock budget is 30 minutes. This is not an OS memory sandbox.
The pinned local runtime/depot must already be installed; ROBOGEN_JULIA can
override the executable. There is no automatic package installation in the UI.

Native transport/integration checks (require the installed runtime):

```sh
cargo test -p robogen-topopt --test declared native_mass_process -- --ignored
cargo test -p robogen-project --test components native_mass_worker -- --ignored
cargo test -p robogen-ui native_mass_view -- --ignored
```

All three have passed on 2026-10-08, including live fields, independent opposite
loads, reference/threshold rendering, cancellation during startup, invalidation
and non-export. They are integration checks, not replacements for the failing
qualification suite below. The browser report is a historical developer artifact,
not the product interface or a biped result.

## Reproduce

Tested on Linux x86_64 with Julia 1.10.10, TopOpt 0.14.0, Ferrite 1.7.0 and
NonconvexMMA 1.1.2. Project.toml pins TopOpt; Manifest.toml locks the full graph.
No Python service, source upload or privileged installation is used. CUDA is optional.

From the workspace root, install the official runtime in ignored build storage:

```sh
mkdir -p target/topopt-tools
curl -fL -o target/topopt-tools/julia-1.10.10.tar.gz \
  https://julialang-s3.julialang.org/bin/linux/x64/1.10/julia-1.10.10-linux-x86_64.tar.gz
printf '%s  %s\n' \
  6a78a03a71c7ab792e8673dc5cedb918e037f081ceb58b50971dfb7c64c5bf81 \
  target/topopt-tools/julia-1.10.10.tar.gz | sha256sum -c -
tar -xzf target/topopt-tools/julia-1.10.10.tar.gz -C target/topopt-tools
export JULIA_DEPOT_PATH="$PWD/target/topopt-tools/depot"
export JULIA_PKG_PRECOMPILE_AUTO=0 OPENBLAS_NUM_THREADS=1
JULIA="$PWD/target/topopt-tools/julia-1.10.10/bin/julia"
BACKEND=crates/robogen-topopt/backends/topopt-jl
"$JULIA" --startup-file=no --project="$BACKEND" "$BACKEND/setup.jl"
"$JULIA" --startup-file=no --project="$BACKEND" "$BACKEND/qualification.jl"
```

Expected qualification exit code is now **0** (iteration 17).
The first dependency compilation takes several minutes. Process supervision and
timeouts are implemented for the native path; portable deployment is not.

Runtime checksum source: [Julia official checksums](https://julialang-s3.julialang.org/bin/checksums/julia-1.10.10.sha256).

## Upstream compatibility fixes

TopOpt v0.14.0 is MIT licensed; tag commit
`ca90253960194b18cfdd10521712bd90ba8d65dd`, registry tree
`9023008e38ea2e23135f965d9d9630d2d4ab2eac`.
The original package fails to load on the declared Julia 1.10 baseline.
setup.jl applies two source-checked fixes **only inside the private depot**:

- Rename the type parameter (not the field) of TemperatureFun/TemperatureResult:
  Julia rejects a field and type parameter both named T.
- Use Ferrite.cellid(cell) in the density filter instead of reading cell.cellid,
  which is a vector in Ferrite 1.7.0 rather than the required scalar cell ID.

No FEM, sensitivity, filtering formula or MMA algorithm is replaced. Nevertheless
the tested identity is **TopOpt 0.14.0 + robogen-compat2**, not the unmodified release.
Pkg's registry hash alone does not describe these patched bytes. setup.jl checks
the entire original file or the reversibly patched file, restores permissions,
and rejects unexpected sources. Re-run it after reinstantiating a depot.
Upstream license and sources remain in the installed package; no issue or fork
has been published on the user's behalf. A first-build Symbolics extension cache
warning also occurred; it is not proof of a numerical failure or a passed gate.

| Patched file | SHA-256 after correction |
|---|---|
| Functions/temperature.jl | 33f82693fbeaee89149ea8454e8b9cb705b02296df3877c6e4e8964d4ac93306 |
| CheqFilters/CheqFilters.jl | 142f9d3b8eaba4bc475f60275bfc750ec90fc0e0488fb1b45276c15a0e1fcc38 |

## Historical measurements before iteration 17

mass.jl composes TopOpt's volume, displacement and filter functions with MMA.
For homogeneous material, minimizing volume minimizes mass. Displacement limits
use the maximum Euclidean norm over nodes, using Ferrite's node-to-DOF mapping;
they are **not compliance limits**. Each case owns a solver and constraint.
The density filter includes fixed regions as neighbors, then the projector
reapplies exact solid/void values. This is not a minimum-thickness guarantee.

All numerical constants below are synthetic test inputs, never robot properties.
The first test is an 8x4x4 grid, 10 mm cells, E=69 GPa, Poisson=0.3,
density=2700 kg/m3, 1 N point load and explicit fixed/void cells. Its compliance
limit is four times the reference response. Seven assertions pass: density mass
340.2 -> 97.1661 g, compliance limit 7.98804e-8 J, normalized violation 3.74335e-10.
No binary reconstruction or physical support is represented by that mass gain.

The second test preserves both end columns, excludes two cells and filters at
15 mm. Independent +1/-2 N cases have nodal limits 8e-8/1.6e-7 m. At 500 iterations:

- Reference density mass: 337.4625 g; candidate density mass: 183.2942 g.
- Maximum displacements: 8.0003217e-8 and 1.6000643e-7 m.
- Squared-displacement normalized violations: 8.04336e-5, above the 1e-5 tolerance.
- KKT residual: 0.00243068, above the 1e-4 tolerance; not converged.
- 13 assertions pass; 3 fail. The result remains feasible=false, converged=false,
  publishable=false. The 200-iteration run also failed; no limits were relaxed.

| Capability | Evidence / remaining gate |
|---|---|
| 3D minimum mass with compliance bound | Smoke test passes |
| Direct nodal displacement constraints | Evaluated, but optimization not qualified |
| Independent force cases | Separate solvers; opposite-load response scaling passes |
| Different supports between cases | Supported by problem separation in principle, not tested |
| Exact masks after filtering | Initial and final field assertions pass |
| Cancellation / progress | Live native fields, pre-start and subprocess-startup cancellation tested; mechanical callback cancellation uses the same supervisor |
| Input refusal | Native schema, revision, sequence, masks, mass and displacement consistency tests pass |
| Stress / safety / frequency / manufacturing | Not implemented in this kernel; no silent support claim |
| Gradients / refinement / analytic comparison | Still required, particularly to diagnose stalled convergence |
| Binary reanalysis / export / robot publication | Not implemented; always publishable=false |

## Sources

- [TopOpt license](https://github.com/JuliaTopOpt/TopOpt.jl/blob/v0.14.0/LICENSE.md)
- [Minimum volume](https://juliatopopt.github.io/TopOpt.jl/stable/tutorials/volume_minimization.html)
- [Fixed regions](https://juliatopopt.github.io/TopOpt.jl/stable/tutorials/fixed_elements.html)
- [3D SIMP](https://juliatopopt.github.io/TopOpt.jl/stable/tutorials/simp.html)
- [Stress relaxation and its limits](https://juliatopopt.github.io/TopOpt.jl/stable/tutorials/global_stress.html)

The stress tutorial's relaxed feasible values are not final-solid stress proof.
The next numerical investigation should check composed adjoint gradients against
finite differences and constraint duplication/nonsmooth maxima before accepting
any further solver tuning. Do not weaken the existing failing qualification.

## Iteration 17: convergence and connected reconstruction

The old raw maximum switched between tied critical nodes. Directional finite
differences of the complete mass/filter/projector/displacement composition agreed
(<4e-8 relative), while MMA02 and another dual optimizer still failed. Replacing
the raw maximum by the unnormalized p=16 norm of squared displacements divided
by their squared limits passes: 29 iterations, KKT 9.6894e-5, unchanged tolerances
and physical limits. This is a **conservative surrogate**: it bounds every node,
but is not mesh independent or a proof of global optimality. Reported violations
remain those of the exact maximum. All cases are independently checked.

The regression qualification now includes bounds, ties, extreme scales, composed
gradients and cached diagnostic responses after forward/adjoint solves. Callback
response reuse is outside AD, requires matching physical densities and a prior
forward solve, and otherwise computes the requested response normally.

`binary.jl` reconstructs exact occupied cells, rejects detached components and
lost preserved regions, and removes empty elements AND unused nodes before FEM.
`binary_qualification.jl` compares that reduced mesh to an independent full-grid
half-height model; it also checks load scaling, mass and refusal cases.

After numerical convergence, the native path tries at most nine thresholds:
`t * 0.8^i`, i=0..8, starting at the selected reconstruction threshold. The first
single-component candidate satisfying every declared displacement is returned
with its actual threshold and binary mass. No bridges are invented and no islands
are deleted. Failure is reported; no unverified candidate is exported. This
bounded reconstruction search is not a discrete minimum-mass optimizer.

Rust independently checks the binary occupancy, face connectivity, integrated
mass, case counts and displacement limits. The UI reports the reconstructed mass
separately from gray-field mass and offers **Afficher le solide réanalysé**.
The visual density threshold also reports disconnected preserved regions and an
explicit proposed connection threshold; this proposal alone is not FEM approval.

Reproduce binary qualification with the same Julia environment:

```sh
"$JULIA" --startup-file=no --project="$BACKEND" "$BACKEND/binary_qualification.jl"
```

Actual tibia run with unchanged 28x24x36 grid, 12 mm filter, PLA and loads:
reference density mass 406.868 g; converged field 40.23988 g, KKT 8.8124e-5;
maximum displacements 0.425656 and 0.455829 mm against 0.5 mm.
The 0.35 visual threshold fragments this field. An explicit 0.25 reconstruction
has 3127 cells, 4694 nodes, one component, 58.20067 g and independently reanalysed
displacements 0.042848 and 0.034614 mm. At 0.20 it has 4170 cells and 77.61330 g.
These are estimated PLA/ideal-transfer hypotheses, not measured servo attachments.
