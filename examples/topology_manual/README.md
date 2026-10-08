# Executable DSL Topology

This example executes the typed `topology(...)` declaration in `main.rgn` through
CSG discretisation, multi-case SIMP, exact voxel reconstruction, binary FEM
reanalysis and the existing STL exporter. It uses an illustrative isotropic
material and manual loads; it does not derive loads from the biped.

After activating the local Rust tools as described in `TECHNICAL.md`, run:

```sh
cargo run -p robogen-cli -- inspect-topology examples/topology_manual/main.rgn
cargo run -p robogen-cli -- optimize-topology examples/topology_manual/main.rgn manual_support /tmp/manual-support.stl 12 4 2 12 0.65
```

The arguments after the output path are NX NY NZ, filter radius in **millimetres**,
and density threshold. Maximum iterations are 100, SIMP penalty 3, stiffness
floor 1e-6, move limit 0.2 and stopping tolerance 0.005. The API additionally exposes
maximum iterations. These numerical controls are outside the DSL; the `.rgn`
defines the geometry, material, loads, objective and constraints. An operation ID
from `inspect-topology` can replace the name when names are ambiguous.

The exported STL contains the selected operation in its **local frame**, including
its domain offset. Outer transforms, compounds and Boolean operations in the
surrounding document are not executed by this command. Standard `check` and
desktop compilation only prepare domain/preserve/keepout meshes, without calling
the solver. Standard `export-stl` refuses uncomputed topology (E330); use the
explicit command above or the desktop per-part calculation/export. Computed
results compose translation, rotation, color and compounds; downstream Booleans
remain refused pending mechanical revalidation.

Open **Exemples > TopOpt calculable (charges manuelles)**, or run:

```sh
cargo run -p robogen-desktop -- examples/topology_manual/main.rgn
```

Conception displays domain (blue), preserved regions (green) and keepouts (red)
as independently switchable contours. Optimisation selects a part and exposes
grid/filter/threshold/iteration controls; **Lancer cette piece** explicitly starts
that part in the cancellable worker. The preset above is editable, not silently
tuned for robot interfaces. Charges, interfaces, materials and constraints remain
editable through the undoable DSL editor, opened from the control panel.

Results show binary measurements and the settings actually used. Calculating a
second part preserves the first. Editing source (including undo/redo) invalidates
all TopOpt results conservatively without recalculation. States distinguish not
calculated, running, calculated, obsolete, failed and cancelled. In Optimisation,
export contains only the selected calculated part; whole-document export in
Conception requires all declared topology parts to be calculated. Preparation
layers never enter an export, even when all displayed layers are hidden.

## Projection And Mechanical Assumptions

- Primitive boxes/cylinders, color, translation, Euler rotation (Rx then Ry then
  Rz), union/compound and difference are classified conservatively against the
  whole cell. A cell-centre occupancy test is not used to certify containment.
- Only cells wholly inside the domain can carry material. All cells potentially
  intersecting a void are excluded; all cells potentially intersecting preserve
  are mandatory. Conflicts produce a localized error, not a bridge or a clipped
  interface. Classification can reject a feasible geometry at coarse resolution.
  The bounds/classification use floating-point arithmetic, not certified exact
  predicates. Contacts between regularized solid boundaries have zero volume.
- Volume fraction is evaluated against the conservatively discretised domain
  before keepouts; mass uses the actual reconstructed volume and declared density.
  A curved boundary therefore needs a resolution study. Domain cells, grid origin
  and voxelised interface centroids remain available in the result.
- An interface is a **volume** in this DSL contract. `fixed` locks all nodes of its
  preserved cells. A total force is distributed uniformly through the voxelised
  interface volume with Hex8 nodal weights (1/8 per corner per cell). No mounting
  face is inferred. Its reported centroid is the application point of that total
  force. This differs from the standalone beam benchmark's loaded end face.
- Moments become a zero-resultant nodal force couple about the weighted centroid.
  The geometric inertia tensor determines the distribution. Tests check total
  force and torque. Cartesian frictionless supports lock only the normal axis;
  pin and oblique-normal projections are explicitly unsupported.
- Every load case is solved independently. `maximize_stiffness` minimizes summed
  compliance at a budget obtained from volume_fraction and/or max_mass. Displacement
  limits are checked on **every reanalysed binary case**, not on the averaged forces
  or density-field response. These checks reject a violation; they do not guarantee
  that every feasible constrained problem will be found by this OC algorithm.
- Unimplemented capabilities are rejected: minimize_mass, stress, safety factor,
  modal frequency, FDM/CNC constraints and nested unresolved topology. The existing
  battery-support declaration consequently remains unsupported. They remain work
  required for the overall system, not constraints silently omitted from a run.

The result retains its original TopologyRequest/material/revision, projected
interfaces, optimization history and each reconstructed-case analysis. Numerical
limits: 2048 cells, 64 per axis, 8000 free DOFs per case, 16 cases, bounded region
expression depth/count. Cancellation is checked during projection/iterations and
between reanalyses. Sparse factorization remains internally non-interruptible.
This synchronous API is intended for CLI/background-worker execution.

## Measured Example, 2026-10-08

Domain 120 x 40 x 20 mm, first/last 10 mm regions preserved. The first region is
fixed and two independent ±1 N vertical loads act on the last region, centred at
X=115 mm. Budget 85%, max mass 230 g, maximum displacement 0.01 mm. Explicit
12 x 4 x 2 cells, 12 mm filter and 0.65 threshold:

| Quantity | Measured value |
| --- | --- |
| SIMP updates / stop | 22 / DesignChange |
| Initial summed compliance | 6.540018216047e-7 J |
| Final summed filtered compliance | 4.133799448315e-7 J |
| Binary compliance, each case | 2.014606055233476e-7 J |
| Maximum binary displacement, each case | 0.000219890239692 mm |
| Solid volume | 80,000 mm3 (80 of 96 cells) |
| Mass | 216 g |
| Surface | 168 vertices, 336 triangles, one component |
| Bounds | [0,0,0] to [120,40,20] mm |

Independent STL decoding verifies paired oppositely oriented edges, volume and
bounds. The artifact is generated locally and not committed. Tests also exercise
thin keepouts smaller than a cell, CSG differences, rotations, conflicts, forces
and moments, unsupported constraints, cancellation, iteration limits, a fully
prescribed region and a violated displacement limit. CLI failure leaves an
existing destination untouched rather than replacing it with invalid geometry.

Exploration is recorded: a 15 mm filter with threshold 0.65 exceeded the binary
volume budget; threshold 0.70 broke connectivity. With the 12 mm filter, thresholds
0.50–0.60 still exceeded volume, 0.65–0.70 produced this validated candidate, and
0.75 or higher broke connectivity. The CLI never adjusts a threshold silently.
These findings motivate a later robust projection/continuation strategy; this
fixture is not evidence of general feasibility, printability or certified strength.

Projection regression: translating the domain and every interface together by
[1,2,3] m preserves their cell masks and load centroids. Common translations are
combined before local-grid classification; this avoids a false conflict caused
by subtracting small cell coordinates from world coordinates repeatedly.
