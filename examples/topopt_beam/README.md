# Topology-optimised beam

Standalone M7 CPU benchmark, authorized ahead of M1-M6 on 2026-10-08.
It does not generate robot supports or enable the desktop TopOpt backend.

Run from the workspace root after activating the local Rust toolchain:

```sh
cargo run -p robogen-topopt --example beam > /tmp/beam-history.csv
cargo test -p robogen-topopt
```

The executable writes iteration CSV to stdout and assumptions/results to stderr.
All inputs are explicit in [the example](../../crates/robogen-topopt/examples/beam.rs).
The public `BeamRequest` accepts typed lengths, pressure and forces, cell counts,
material Poisson ratio, SIMP settings and document revision. This is a standalone
numerical fixture, not a new persistent project format or a second product model.

## Reference Problem

- Rectangular domain: 120 x 40 x 20 mm, axial direction +X; 12 x 4 x 2 Hex8 cells.
- All three displacements fixed on X=0; total [0, 0, -1] N uniformly distributed
	over X=120 mm using consistent nodal face weights (not a corner point force).
- Linear isotropic elasticity: E=70 GPa, Poisson ratio 0.3; hypothetical benchmark
	material, not calibrated printed PLA. No gravity or robot load inference.
- Eight Gauss points per cell; small strain; engineering shear strain convention.
- Minimize compliance at volume fraction 0.35, compared to uniform density 0.35
	on the identical mesh, loads, supports and constitutive interpolation.
- SIMP: E(rho)/E = 1e-6 + (1-1e-6) rho^3. Physical densities come from a normalized
	distance filter of radius 15 mm; chain-rule derivatives apply to compliance
	and filtered volume. OC move limit 0.2; design density bounds [0.001, 1].
- Stop when maximum design-variable change is below 0.005, or after 100 updates.
	`DesignChange` is an algorithmic stop criterion, not proof of global optimality.
	`IterationLimit` is explicitly distinct. Every history entry describes an
	actually solved state; the returned final densities are reanalysed.

## Measured Run

Local Rust 1.88 debug run, 2026-10-08; faer 0.22.6 sparse LLT, single-thread
configuration, 65 updates, 20.932 seconds excluding compilation. This one run
is not a portable performance guarantee.

| Measure | Value |
| --- | --- |
| Uniform reference compliance | 6.271263227758e-6 J |
| Final compliance | 2.377550262401e-6 J |
| Final/reference ratio | 0.379118 |
| Final filtered volume fraction | 0.350000000 |
| Maximum final design change | 0.004620056 |
| Final relative linear residual | 6.616003e-13 |
| Weighted mean tip displacement Z | -0.0023775502624 mm |
| Stop reason | DesignChange |

Compliance here is force dotted with displacement (twice strain energy).
The 62.1% improvement is against the uniform porous reference, not against a
solid beam. Density volume is not an estimated print time or certified strength.

## Validation And Limits

Tests cover a known sparse solution, duplicate entries, invalid/singular/indefinite
systems, rigid modes, constant multiaxial strain energy, exact axial extension
at Poisson ratio zero, bending refinement towards Euler-Bernoulli (fine-mesh
error below 10%), discrete energy balance, filtered sensitivities against central
differences, volume within 1e-6, density bounds, cancellation, revision/backend
provenance, iteration-limit reporting and final-state reanalysis/load scaling.

Resource limits: 1..64 cells per axis, at most 2048 cells and 8000 free DOFs;
Poisson ratio in [-0.99, 0.49), no nearly incompressible guarantee. The sparse
port accepts lower-triangle triplets, adds duplicates, limits entries to four
million, and checks finite values and relative residual. It is algebraic;
FEM alone owns the physical units. The test suite uses residual <=1e-7 and
energy balance <=1e-6 for FEM, tighter tolerances for exact small fixtures.

The library is synchronous, intended for a CLI/worker, never the UI thread.
Cancellation is checked during filter construction, element assembly, OC,
after factorization and before returning a run; faer factorization itself
is not interruptible. Progress reports solved iterations starting at zero.
Results retain the request and solver identifier; revision is caller-scoped,
not a global cache key. No document publication or stale-result dispatch exists
on this standalone path yet.

No isosurface reconstruction, watertight export, stress/frequency constraints,
preserved/excluded geometric regions, FDM/CNC constraints, post-operation
revalidation or manufacturing proof is provided. `topology(...)` still fails
with E330 until its complete requested capabilities are implemented. M7's
reconstruction and desktop acceptance, and M5/M8 robot mechanics, remain open.

