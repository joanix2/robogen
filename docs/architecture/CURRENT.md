# Implemented architecture

Verified structural reference: 2026-10-20, ticket 17 and ADR-013.
This page describes implemented responsibilities. OVERVIEW.md also retains
historical milestone observations and the longer-term target architecture.

## Shared model and crate boundaries

The Cargo workspace has 21 packages and 36 internal production dependency edges.
The production graph is acyclic. `.rgn` text is parsed by `robogen-dsl` and lowered
by `robogen-ir` into the shared typed `SemanticModel`. `robogen-domain` owns IDs,
units, materials and diagnostics. Scientific and geometric projections consume
these RoboGen contracts; toolkit and backend-native types remain in adapters.

The existing dependency Project -> CAD/constraints/TopOpt is an orchestration
boundary. Export still consumes CAD's RoboGen Mesh. These existing edges are
explicit; the refactor adds no Cargo dependencies or persistent schemas.

[ADR-015](../adr/ADR-015-instance-owned-regions.md) extends the DSL with
`Object` values and named `instance` declarations. An instance owns its shape,
exclusion volumes and named interfaces under one pose; `instance_shape`,
`instance_void` and `instance_interface` project them with provenance recorded
in `SemanticModel.instances` / `instance_references`. `ObjectInstanceId` is
typed and name-derived. Geometry stays backend-neutral; CAD/render/export/TopOpt
still receive ordinary SolidGeometry projections.

## Document and application

`robogen-project/src/lib.rs` preserves the public facade and private Project
storage. Its internal modules separate:

- `contracts`: snapshots, optimization states and reports;
- `manifest`: project-file contracts and loading;
- `document`: source history, invalid edits, undo/redo and job lifecycle;
- `project`: source compilation and initial preparation;
- `compilation`: serial worker, latest pending request, cancellation and revisions;
- `optimization` / `topology`: explicit per-part jobs and result publication;
- `projection` / `diagnostic_mesh`: CAD preparation and diagnostic visualization;
- `rebuild` / `dependencies`: parameter overrides and dependent-part rebuilds;
- `library`: component discovery, taxonomy and source-based instantiation.

SourceDocument continues to hold the last valid snapshot when current source is
invalid. Invalid/stale work cannot enable export. Final and intermediate worker
results are filtered by request revision. Internal module extraction preserves
these behaviors; it is not a new task scheduler or a new document format.

## Presentation

`robogen-ui` retains RoboGenUi, WorkspaceMode and UiAction at their existing
public paths. The facade owns presentation state and dispatch. Feature modules
own design/taxonomy/library, editor, optimization and learning views. Shared
shell, theme, icons, widgets and preview conversion have focused owners. Mass
diagnostic and accepted-result presenters are separate from topology job controls.

The viewport still uses CPU depth rasterization from `robogen-render`, displayed
as an egui texture. Eframe composes the application through wgpu. This is not a
new native CAD GPU pipeline; CPU preview costs remain a performance concern.
The Design/Optimization/Learning navigation and unavailable states are retained.
Design tabs are ordered DSL, Graphe cinématique 2D, Vue 3D, with DSL selected at
startup. The 2D panel is a pan/zoom/picking canvas over the declared mechanical
model (ADR-016): `body`/`joint` declarations resolve in IR into Domain
`MechanicalModel`, Robotics validates it and Project invokes that validation. It
shows a joint graph (layered layout, draggable nodes) and an XY/XZ/YZ rest-pose
kinematic schematic. UI owns only layout, projection, symbols and selection. No
joint is inferred from geometry; loops are accepted, with no closure/mobility solve.


Declared object interfaces are automatically projected into
`ProjectSnapshot.instance_interfaces`, keyed by ObjectInstanceId. A separate green
mount overlay shows every instance's interfaces independently of the currently
selected topology part, also in documents without TopologySpec. Its visibility
is presentation state; it adds no mechanical load/support or accepted geometry.


## Scientific adapters

[ADR-014](../adr/ADR-014-remove-julia-backend.md) records the user's immediate
removal of the Julia engine, overriding ADR-013's temporary legacy retention.
There are no Julia project sources, environment manifests or subprocess launch
paths. Ignored local downloads are not discovered or launched by application code.

`robogen-topopt::mass` preserves its public DTOs and execution function. Its
`diagnostic` module owns typed data and connectivity; `unavailable` returns a
source-located E348 error (E345 on cancellation), without scientific events.
`MASS_BACKEND_AVAILABLE` is false and is re-exported through Project for UI use.
The historical transport and its backend-specific qualifications are removed.

Native minimum-mass launch/CPU/CUDA controls are disabled, even with complete
inputs. Direct worker/API calls fail without metrics, meshes or exportable
results. The CLI stiffness execution port rejects minimum mass with E340 before
projection. The existing Rust fixed-volume stiffness backend and its independent
integration tests remain. No minimum-mass objective is silently substituted.

Rust remains the application host; a future Python engine requires independent
qualification. This change installs no scientific package, simulator or PPO.

## Product scope and remaining limits

The intended MVP integrates DSL generation, topology optimization and learned
locomotion on the existing mini-biped. Its current source describes 14 servos,
a board, a battery, a camera and 11 support preparations. Hardware interfaces,
motor limits, inertial properties and manufacturing inputs remain incomplete.
The displayed layout is not a qualified dynamic or printable robot.

Assembly/robotics/physics/RL contracts remain partial, and production physics/RL
backends remain unavailable. Existing parameter overrides do not patch the source,
manifest loading is single-source with lexical path checks, and native save is
still unavailable. These are explicit follow-up work, not solved by moving files.

## Architecture checks

Run from the repository root with Python 3.11 or newer:

```sh
python3 -m unittest discover -s scripts/tests
python3 scripts/check_architecture.py
```

The checker reads production, build and target-specific Cargo dependencies,
including package aliases; dev dependencies are deliberately excluded. It rejects
production cycles, backend/toolkit imports into the core, adapter dependencies
on the application, direct solver dispatch from UI and toolkit/kernel coupling
in the renderer. New legitimate cross-crate dependencies require a reviewed
boundary decision and corresponding policy/test update.

The existing Rust fmt, Clippy and workspace tests remain the integration gates.
Deleted-engine qualifications were removed with that engine. Deterministic API,
worker and UI tests now verify unavailability; these are not solver qualification.
