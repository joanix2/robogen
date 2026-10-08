# Topology-optimised beam

Standalone M7 CPU benchmark, authorized ahead of M1-M6 on 2026-10-08.
It does not generate robot supports or enable the desktop TopOpt backend.

Run from the workspace root after activating the local Rust toolchain:

```sh
cargo run -p robogen-topopt --example beam > /tmp/beam-history.csv
cargo run -p robogen-topopt --example beam -- --surface-stl /tmp/beam-approximate.stl
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

Stress/frequency constraints, arbitrary geometric regions, FDM/CNC
constraints, post-operation revalidation and manufacturing proof remain absent.
`topology(...)` still fails with E330 until its complete requested capabilities
are implemented. M7 desktop acceptance and M5/M8 robot mechanics remain open.

## Approximate Surface (2026-10-08)

`BeamOptimization::reconstruct_surface` carries the full request, caller revision
and solver identifier into a `BeamSurface`. Its `ReconstructedSurface` stores
typed Length vertices, triangle indices, threshold, reconstruction backend and
sampling subdivision count. Mesh arrays are read-only to callers. Volume is
reported in cubic metres through a typed wrapper. No CAD kernel type is exposed.

The `SurfaceReconstructor` port uses fast-surface-nets 0.2.1. Densities are sampled
at cell centres with two outside zero-density layers, then trilinearly sampled
at twice the original resolution. This refines only extraction, not FEM or the
optimization result. The level field is threshold minus density, not a metric
signed distance. Thresholds [0.5,1) keep the surface inside the design envelope;
the example selects 0.5. Ambiguous or degenerate output is rejected, not repaired.
One outside layer initially left open boundary faces; coarse unrefined extraction
also produced a non-manifold edge on the actual benchmark. Both failures were
detected before export and are covered by the full-field/benchmark tests.

Checks cover finite in-domain coordinates, triangle area, two oppositely oriented
faces per edge, a connected circular link at each vertex, surface components and
positive total signed volume. These are topological checks, not a proof against
self-intersection. Cavity shells count as surface components too. Cooperative
cancellation surrounds the bounded extraction call and runs during sampling and
validation; the library call itself is not interruptible. Input budgets match
the standalone grid (64 cells per axis, 2048 total).

The example reuses robogen-export's binary STL writer via development-only
dependencies. It converts typed metres to the existing Mesh DTO; the writer
emits millimetres. `--surface-stl PATH` explicitly requests an approximate
visualization artifact, not a mechanically revalidated part. It replaces an
existing destination atomically using the existing exporter. No desktop export
guard is bypassed and no new persistent project format is introduced.

Measured reference export: 208 vertices, 404 triangles, **three disconnected
surface components**. Enclosed mesh volume is 13,463.265650 mm3 versus filtered
material volume 33,600 mm3. Independent STL reimport with trimesh confirmed
closure, consistent winding, three components and volume 13,463.265672 mm3.
Bounds in millimetres: [1.696579,4.796383,1.677495] to
[76.402588,35.203793,18.322504]. Neither the clamp at X=0 nor the loaded face at
X=120 survives this thresholding. **This is not a usable cantilever support.**
The density-field compliance must not be assigned to this surface. The next
mechanical step is preservation of interfaces/load paths and reanalysis after
reconstruction, not a claim that this STL retains the SIMP stiffness or volume.

[Diagnostic figure](../../docs/screenshots/m7-beam-surface-20261008.png) is
rendered from the actual exported STL, not a native desktop screenshot.
Four new surface tests and the extended SIMP/STL integration pass. Workspace:
118 tests, fmt and Clippy all-targets -D warnings pass.


## Explicit Regions And Binary Reanalysis (iteration 5)

`BeamRequest.regions` is either empty (all Design) or one `CellRegion` per Hex8,
ordered X slowest, Z fastest: `(x*ny+y)*nz+z`. Preserve cells remain exactly 1,
Void cells exactly 0. The density filter mixes only Design cells. Its transpose
excludes fixed variables, while the volume includes preserved material. The
uniform reference now means uniform *free* density with identical fixed regions.
This OC implementation targets an equality volume: infeasible targets are
rejected, and fully fixed masks require direct binary FEM rather than SIMP.
Regions are explicit caller inputs; no interface thickness or connecting rail is
invented. The SIMP floor remains an optimization regularization, including Void
cells, and cannot establish that a thresholded solid carries the loads.

`BeamOptimization::reconstruct_solid` thresholds the field, checks fixed/void
regions, requires the complete clamped and loaded cell layers, and verifies
face-connected material. `BeamModel::solve_solid_mask` omits empty elements and
unused DOFs entirely. It reuses the original forces and constitutive law without
an artificial stiffness floor. Absent grid nodes have zero displacement
placeholders; they are not solved DOFs. Edge/corner-only joins are insufficient.

`VoxelBoundaryReconstructor` emits exposed faces of exactly those cells, with
shared lattice vertices and outward winding. It checks manifold edges/vertex
links, domain bounds and signed volume against the occupied-cell volume. Cavities
are allowed; surface component count includes inner shells. This staircase
boundary is exactly the reanalysed Hex8 union, **not a smoothed surface**. A
successful analysis is linear elastic stiffness evidence, not stress, fatigue,
printability or strength certification. The separate Surface Nets preview is
unchanged and must not inherit these mechanical results.

The returned `BeamSolid` has its own analysis, caller revision, backend, actual
volume fraction and `within_volume_budget`. A mechanically analysable candidate
can still exceed the requested budget. The example refuses to export it.
Cancellation propagates through reanalysis and surface extraction. Tests cover
masked compliance/volume derivatives, fixed regions, infeasibility, removed DOFs,
missing interfaces, disconnected masks, cavities and non-manifold contacts.

### Reproducible Binary Example

```sh
cargo run -p robogen-topopt --example beam -- --solid-stl /tmp/beam-solid.stl
```

This is an explicitly different benchmark: same grid/material/load, complete
first and last X cell layers preserved, **volume target 0.85**, reconstruction
**threshold 0.65**. It does not replace the original 0.35 reference above.
Local debug run 2026-10-08: 33 updates, DesignChange, 10.609 s (excluding build).

| Measure | Value |
| --- | --- |
| Uniform free-density reference compliance | 4.402968327304e-7 J |
| Final filtered compliance | 3.156725678745e-7 J |
| Filtered volume fraction | 0.85 |
| Actual binary volume fraction | 0.8333333333333334 |
| Reanalysed binary compliance | 2.964559539151e-7 J |
| Reanalysis relative residual | 1.414025e-12 |
| Boundary | 168 vertices, 336 triangles, one component |
| Enclosed volume | 80,000 mm3 |
| Bounds | [0,0,0] to [120,40,20] mm |

Independent decoding of the exported STL verifies paired opposite edges, volume
and bounds. No STL, generated cache or local toolchain is added to Git.

Threshold selection matters: the same run at 0.5 yields volume fraction 0.875
and fails the budget; at 0.75 it loses connectivity. Both are regression tests.
Earlier trials with preserved end layers and volume targets 0.60 and 0.80 also
failed connectivity or binary volume checks. These are not repaired or presented
as usable optimized supports. A robust projection/continuation or threshold
selection strategy remains future work; no general feasibility is promised.
The 0.85 benchmark proves the validation path on one explicit case.

All 124 workspace tests, formatting and Clippy all-targets -D warnings pass.
Arbitrary geometric regions, multiple load cases, constraint capability dispatch,
DSL result publication and the desktop workflow remain open; E330 is unchanged.
