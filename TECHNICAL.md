# RoboGen technical notes

## Source-of-truth flow

The `.rgn` source is parsed into a spanned syntax model, resolved and checked
before becoming the semantic document consumed by projections. The MVP CAD
projection evaluates explicit physical units, solves the constrained rectangle
subset, and tessellates an extrusion. Rendering and STL export consume the same
mesh; neither parses DSL text independently.

```text
.rgn source -> parser/diagnostics -> semantic document -> sketch solution
                                                    |-> CAD mesh -> renderer
                                                    `-> CAD mesh -> STL exporter
```

The desktop owns orchestration and transient UI state only. Domain crates do
not depend on egui, eframe, wgpu, or a CAD kernel.

## Workspace boundaries

- `robogen-domain`: IDs, units and source diagnostics.
- `robogen-dsl` / `robogen-ir`: syntax and semantic projection.
- `robogen-sketch` / `robogen-constraints`: solver-neutral sketch model.
- `robogen-cad`: feature evaluation, stable selectors and mesh generation.
- `robogen-render`: CPU-projected mesh previews, camera and viewport data; dedicated
  wgpu CAD passes, RenderScene and picking IDs remain M3 work.
- `robogen-ui`: egui components and UI state.
- `robogen-export`: manufacturing/export adapters.
- `robogen-project`: synchronous source compilation, parameter overrides and
  dependent-part mesh rebuilds; commands, events and revision-aware tasks remain
  M3 work.
- `robogen-robotics`, `robogen-physics`, `robogen-topopt`, `robogen-rl`: future
  vertical slices behind backend traits; unavailable states are intentional.
- `robogen-agent-api`: disabled AI/foundation-policy provider interfaces only.

## Build and validation

The pinned toolchain is Rust 1.88.0, matching the MSRV of the current native
windowing dependency graph. Standard validation is:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

During initial construction in the provided environment, Rust was bootstrapped
locally under ignored `.cargo-home/` and `.rustup-local/` directories because no
toolchain was initially present.

### Local development tools

Graphify 0.9.15 is installed as the isolated `uv` tool package `graphifyy`,
matching the installed Graphify skill. Its executable is in `~/.local/bin`.
The previous executable linked to a removed VS Code Snap revision; reinstall
with `UV_TOOL_BIN_DIR="$HOME/.local/bin" uv tool install --force graphifyy==0.9.15`
if that link breaks again. Installing the tool does not generate a project graph.

The local Rust 1.88.0 toolchain already includes Clippy and rustfmt.
`.vscode/settings.json` supplies its environment to new Linux integrated
terminals and rust-analyzer, and selects Clippy for editor checks. The official
`rust-lang.rust-analyzer` extension is installed in the current VS Code profile.
For an existing or external terminal, run from the repository root:

```sh
export CARGO_HOME="$PWD/.cargo-home"
export RUSTUP_HOME="$PWD/.rustup-local"
export PATH="$CARGO_HOME/bin:$PATH"
```

This environment-specific configuration requires the local toolchain directories;
other machines can use a standard rustup installation instead.

### Development agents

Project-scoped VS Code Copilot profiles live in `.github/agents/*.agent.md`.
Select `RoboGen` in the chat agent picker to coordinate the active Kanban task.
`RoboGen Architect` and `RoboGen QA` are also directly selectable; the nine other
profiles are available only as subagents. If the picker has not refreshed, reload
the VS Code window. No user-profile customization or model selection is imposed.

The orchestrator has an explicit allowlist of eleven subagents. Specialists have
no delegation tools and an empty subagent allowlist; `RoboGen Explore` has only
read/search access. File ownership and milestone limits remain behavioral rules,
not filesystem sandboxing. The shared protocol and role-to-profile map are in
`AGENTS.md`; the orchestrator alone updates Kanban records for delegated work.
Selecting an agent does not bypass the plan/todo/implement authorization rules.

Validation covers YAML parsing, unique names, allowlist resolution, tool sets,
visibility flags, local links and preservation of the protected Kanban block.
These static checks do not establish runtime discovery or successful model
delegation in every VS Code/Copilot version. No application AI backend is enabled.

## Deliberate milestone limits

### M0 validation (2026-09-22)

All 21 workspace packages inherit the shared Rust/Clippy lint policy. Existing
tests now propagate errors instead of using unwrap. Missing native UI presentation
helpers were restored; a headless egui test exercises seven workflow/tab states
at two viewport sizes while checking that no command is emitted.

Local validation passed: `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace` (21 tests, including the CAD/STL integration test).
The existing Linux GitHub Actions workflow runs the same three gates; no hosted
CI run is claimed. This checkout has no `.git` directory.

`cargo build -p robogen-desktop` passed. The native X11 shell was launched on
DISPLAY=:1 using eframe/wgpu with the NVIDIA RTX 3060 Vulkan adapter. The
1440x831 client capture is [m0-native.png](docs/screenshots/m0-native.png).
The driver emitted present-mode and missing validation-layer warnings, but the
captured window rendered successfully. This proves shell composition, not the
dedicated CAD GPU pipeline planned for M3.

The architecture overview and ADR status sections distinguish current code from
target contracts. In particular, lexer trivia preservation, accurate constraint
DOF, source patches/undo, asynchronous tasks and stable geometry resolution are
not complete. Current project->CAD/constraints and export->CAD dependencies are
documented discrepancies to resolve in M3, not newly approved layering rules.

The topology and training views never invent metrics: they report their solver
as unavailable. AI is represented by a disabled provider. Rapier, FEM/SIMP,
Burn/PPO, STEP and production B-Rep integration remain later milestones.

### M1 header revision (2026-09-22)

The native shell now has two full-width headers shared by every workspace:
`application_bar` contains only RoboGen at the left and the Profile menu at the
right; `workspace_bar` contains Design, Optimisation and Apprentissage plus
project actions. The profile menu explicitly reports an unconfigured profile;
account settings remain disabled and no authentication backend was added.

The headless UI suite verifies separate header rows, right-aligned profile,
profile menu interaction and undo/camera commands at 1080x680. All 12 UI tests
and all 41 workspace tests pass, as do workspace fmt/Clippy and the desktop
build. Native X11 captures were inspected at 1440x831 and 1080x680; see
`docs/screenshots/m1-design.png` and `docs/screenshots/m1-design-compact.png`.
This validates the header revision, not the full M1 acceptance checklist.

## Parametric servo slice (2026-10-07)

The executable example is `examples/servo/main.rgn`, also available from the
desktop Exemples menu or as an optional `.rgn` command-line argument.
Same-file procedural components support boxes, cylinders, translation,
union, difference and opaque RGB decoration. ADR-010 records the mesh backend
and appearance contracts; imports remain explicitly unavailable.

The csgrs translation defect was isolated by plane orientation tests. The
adapter now uses polygon translation, welds positions at 1 nm and splits
T-junction edges in the output triangulation. Tests verify signed box/Boolean
volumes, oriented edge closure, servo bounds and open mounting tabs. The
servo produces 2870 triangles, with three source colors; build time is about
0.8 seconds in local debug mode. Cylinders have 32 segments.

SourceDocument keeps its synchronous entry points for CLI/tests. The desktop
enables a single background compilation worker with one replaceable pending
job, source-only undo/redo, atomic revisions, per-part progress and cooperative
cancellation. CAD checks cancellation between nodes, polygon triangulations
and mesh conformance steps; an individual library Boolean call is not
interruptible. Only the current revision publishes a result. Invalid, pending
or cancelled documents retain the last preview and prohibit export.

RGB is optional per CAD triangle, including serialized backward-compatible
defaults. The CPU renderer shades source colors and rasterizes surfaces with
per-pixel inverse depth and near-plane clipping. The UI caches an RGBA texture
per view, invalidated by camera, mesh or viewport size changes. Raster images
are limited to 2048 pixels per axis, with nearest sampling and no antialiasing.
The older centroid-sorted primitive API is retained but does not draw UI surfaces.
Rasterization remains synchronous on view changes; sustained orbit frame rate
has not been benchmarked. CAD Z is up
and the preview is normalized to its bounds. This is still CPU projection on
egui/eframe, not the dedicated M3 GPU renderer or stable-face picking.

Clearance expands only the rectangular housing. The front connector is below
the tab and intersects its vertical axis; hole assertions therefore cover the
tab thickness rather than the entire height. The STL exporter writes mm and
omits appearance. Tests cover clearance 0 -> 0.2 mm, IDs, undo/redo and STL.

Local `cargo check` initially used pre-component metadata for robogen-dsl and
robogen-domain despite passing test builds. Cleaning only those two packages'
generated artifacts restored the desktop build; no source rollback was needed.

Validation: `cargo fmt --check`, workspace Clippy with `-D warnings`, and all
69 workspace tests passed. CLI check reports 1437 vertices and 2870 triangles.
Native captures at 1440x831 and 1080x680 were inspected; the compact client
required temporary X11 override-redirect because the WM ignored resize requests.
Synthetic native clicks were not accepted in this session. Menu loading,
source history, camera controls and export guards are covered by automated
tests, but a manual native edit/orbit/export walkthrough remains to be signed
off in the ticket. Editor-only Serde macro diagnostics were reported in the
untouched domain crate despite the passing Cargo gates; they were not fixed
by changing domain source.

## Compute board example (2026-10-07)

`examples/compute_board/main.rgn` reconstructs the supplied photograph as seven
colored parts in a 104 x 90 x 37 mm envelope. The Exemples menu loads it through
the existing source history and background compilation pipeline, with a
three-quarter camera preset. The model includes PCB mounting holes, hollow
connector shells, twelve heatsink fins, a static four-paddle fan and header pins.
Only the overall envelope is dimensioned in the reference. Component placement
and physical material values are illustrative, not manufacturing or simulation
data; see the example README. STL retains separate bodies and omits colors.

This larger model exposed edge-conformance failures after premature f32
rounding. The CAD adapter now retains backend f64 positions for welding and
edge matching, while publishing the same f32 Mesh API. A four-hole 104 x 90 mm
board fixture verifies signed volume and closure. Integration verifies closed
part meshes, colors and exported mm bounds at heights 37 and 42 mm. No backend
budget was increased and no public boundary or dependency changed.

Validation passed: fmt, workspace Clippy with `-D warnings`, 72 workspace
tests, and desktop/CLI builds. The final board has 3707 vertices and 7206
triangles. The 1440x831 native capture in
`docs/screenshots/compute-board-native.png` was visually inspected. The normal
desktop instance was left open on the model; no forced window sizing was used.

Display correction: the initial board capture did not establish correct
occlusion. Per-pixel depth now replaces centroid sorting in every mesh preview
(ADR-001). Regression tests cover crossing faces, near clipping, invalid faces,
cache invalidation and actual board fan visibility at 600x400 and 320x240.
All 76 workspace tests, fmt, Clippy with `-D warnings`, and desktop build pass.
`docs/screenshots/compute-board-depth.png` shows the corrected native 1440x831
view after an XTest camera drag. Fan, fins and connectors occlude the PCB
correctly in this view. This is not a dedicated GPU pipeline or a measurement
of continuous camera performance.

## Raspberry Pi camera example (2026-10-07)

`examples/raspberry_pi_camera/main.rgn` implements the supplied Gert van Loo
2013 drawing as reusable solid components and one `Camera` instance. The
`raspberry_pi_camera(hole_diameter)` generator composes the PCB, optics,
sensor tail, underside connector and ribbon with six illustrative RGB colors.
It uses the existing mesh CSG backend without changes to domain contracts.

The 23.9 x 25 x 0.95 mm PCB has four holes at X=9.35/21.85, Y=2/23 mm.
The optical stack reaches 5.2 mm above the PCB; the connector projects 2.8 mm
below it. The complete illustrative envelope is 28.9 x 25 x 8.95 mm including
a 5 mm ribbon stub. Unspecified lens diameters, height split, connector
footprint and cable geometry are documented in the example README; the
material constants are schema placeholders, not physical measurements.

The Exemples menu loads the source through existing history and background
compilation, with a camera preset. Focused tests verify a closed oriented mesh,
open holes and radii at diameters 2 and 3 mm, six colors, STL bounds in mm,
asynchronous UI load/export/undo and visible lens pixels at two preview sizes.
Default CLI output: 1773 vertices, 3558 triangles.

Validation: fmt, workspace Clippy with `-D warnings`, all 78 workspace tests,
desktop build and CLI STL export passed. The native 1440x831 capture
`docs/screenshots/camera-native.png` was inspected: four open holes, visible
lens, PCB and ribbon. Automated tests cover the example load/export/undo;
the capture alone does not establish a manual edit/export walkthrough.

## Generated Taxonomy And Library (2026-10-07)

Design now uses a resizable left column: compiled Taxonomy above Library,
with an adjustable horizontal separator and independent searches. The
assistant remains on the right. `Project::taxonomy_instances` projects typed
PartId/FeatureId entries and call names from the matching semantic snapshot
and AST; `function_definitions` exposes signatures and source spans.
Invalid or pending source retains explicitly marked previous data. Selection
is transient; double-clicking an instance or following a function link opens
and selects its DSL declaration only when the snapshot is current.

`SourceDocument::instantiate_generator` is one undoable source replacement
that preserves the existing document. The three library entries import
servo/camera/board definitions through the DSL's token-aware `component_library`
transformation. Helper declarations receive separate prefixes; name conflicts
are diagnosed before mutation. Subsequent instances reuse existing functions
and instance materials. Arguments are edited in DSL; the UI button uses defaults
and places the new instance 10 mm beyond the current maximum X. The full
generated source compiles independently of the UI. The Jetson Nano Super label
is user supplied and does not certify the existing board's exact product identity.

Compound is a new explicit separate-body IR operation, not a Boolean union.
The adapter bounds body count/output/depth and supports cancellation and outer
color/translation; compounds nested inside Boolean operands are rejected.
The board generator preserves the exact prior vertex positions and triangle
counts. Existing CSG budgets are unchanged. See ADR-010 for the boundary and
API change: arbitrary `RoboGenUi::set_taxonomy` injection has been removed.

Checks passed: 85 workspace tests, fmt, Clippy with `-D warnings`, desktop and
CLI builds. After the left/right correction all 17 UI tests and fmt/Clippy
were rerun successfully, including layout assertions at 1440x831 and 1080x680.

Native validation used XTest clicks on all three Library add buttons. The
original bracket remained and three generated instances appeared in the
taxonomy and viewport. `docs/screenshots/taxonomy-library-left-native.png`
records the final 1440x680 client, with left-hand sections and right-hand chat.
No forced resize was used. The instance list is scrollable independently of
the function list. Native undo/source editing were not retested in this pass;
their contracts are covered by the automated tests above.

## Li-ion battery example (2026-10-07)

`examples/li_ion_battery/main.rgn` adds `li_ion_battery(...)`, `battery_body(...)`
and `battery_connector(...)` using existing cylinder/box/color/translate,
compound and difference operations. No CAD or IR boundary changes are needed.
`LibraryGenerator::Battery` imports the definitions with `lib_battery_` prefixes.
The Library now has four entries; the earlier three-entry capture remains
historical. A menu loader sets a battery camera preset and preserves undo.

The nominal pack is diameter 18 x 68 mm, on +Z with a centred X/Y axis. Eight
length parameters expose pack, wires and connector dimensions. Straight leads
and connector proportions are estimates; full default Z extent is 98 mm.
The bare-cell reference diameter of 18.3 mm is not inserted inside the 18 mm
pack. The example README distinguishes supplied provenance from measured facts.
Colors are visual, markings are not meshed, and DisplayOnly constants are not
validated battery physics. Compound exports separate closed shells, not one
fused manufacturing solid. UI arguments remain editable in DSL.

Validation: 88 workspace tests, fmt, Clippy with `-D warnings`, desktop/CLI
builds and CLI STL export passed. Default mesh: 370 vertices, 716 triangles.
Tests cover dimensions, parameter variation, invalid diameter, colors, closed
oriented shells, STL coordinates/normals, insertion/undo and preview visibility.
`docs/screenshots/battery-native.png` records the inspected 1440x831 native
client; no manual edit/export walkthrough is claimed.

## Architecture decision summary

The normative target architecture is documented in
`docs/architecture/OVERVIEW.md`. Backend decisions and replacement seams are in
`docs/adr/ADR-001` through `ADR-009`.

The current decisions are: egui/eframe over wgpu for the desktop shell, a
kernel-neutral CAD port with a native extrusion backend before the provisional
Truck spike, a limited native sketch constraint solver, a recovery-oriented
manual parser before a later Logos plus LALRPOP migration,
a versioned TOML project manifest, Rapier3D for initial rigid-body physics,
Burn with a RoboGen-owned PPO loop for the later RL milestone, CPU SIMP with a
replaceable sparse solver for initial topology optimisation, and
provenance-based stable geometry references. Provisional choices must pass the
spikes and contract tests defined by their ADR before being treated as proven.

## Mini biped component layout (2026-10-07)

Ticket 14 adds `examples/mini_biped/main.rgn` and the **Mini bipède — disposition**
menu loader. Seventeen named parts reuse the four library generators: 14 servos,
board, battery and camera. Shared typed parameters derive limb/torso/head heights
and bilateral spacing. No structural support is authored, no design domain is
shown as a fake part, and no dynamics/optimization result is claimed.

`rotate(x: Angle, y: Angle, z: Angle, shape: Solid)` extends neutral IR/CAD;
see ADR-011 for right-handed X/Y/Z order and source/serialization contracts.
The private adapter rotates CSG positions, vertex normals and plane normals,
keeping signed plane distances. Compound transforms use an ordered stack,
applying child transforms before parent transforms. Existing bounded work,
colors, winding, cancellation and diagnostic spans remain in force. There
are no new dependencies and no schema migration for source project manifests.

The example is an initial arrangement, not a RobotModel or OptimizationSpec.
Ticket 15 specifies the remaining constraints/interfaces/regions and links to
M7 (FEM/SIMP), M8 (robot loads) and ticket 13 (parameterized print profiles).
UnavailableTopologyOptimizer remains unchanged. Component dimensions and
physical properties retain their original example limitations; DisplayOnly
must not be used for mechanical validation. Arbitrary edited layouts are not
automatically collision-checked; the default pose has a conservative AABB test.

Validation includes library-copy drift, original component dimensions, 17
separated default envelopes, stable IDs and parameter-dependent displacement,
closed colored shells, millimetre STL, asynchronous UI load, undo/redo and
preview visibility at two sizes. Results and native evidence are recorded in
the ticket after execution. Existing open M0-M3 acceptance items are not closed
by this supplementary slice.

Ticket 14 verification: baseline 88 tests; after implementation 94 workspace
tests pass, fmt --check and Clippy all-targets -D warnings pass, CLI/desktop
builds pass. AABB testing detected elbow/hip overlap at 172 mm shoulder spacing;
190 mm resolves it. Native menu inspection then detected head clipping; after
correcting the camera target, the focused asynchronous load/undo/redo/framing
test, fmt, Clippy and desktop build pass again. Final 1440x831 native capture:
`docs/screenshots/mini-biped-native.png`. CLI produces 25968 vertices / 51660
triangles and a 2583084-byte STL in /tmp. Compilation takes about 13 s in the
local debug configuration and remains off the UI thread.

## Declarative topology contract (2026-10-07)

Ticket 15 iteration 2 implements ADR-012, not M7 FEM/SIMP. Domain owns Force,
Torque, Mass, Frequency and OptimizationId/InterfaceId/LoadCaseId. IR owns
`TopologySpec<G>` schema 1 and `SolidOperation::Topology`; the existing bounded
evaluator lowers contextual calls/lists into a typed deferred geometry node.
Specifications preserve local source spans, material identity, explicit regions,
nonzero loads, supports, objective/limits and optional FDM/CNC declarations.
Unknown or incompatible fields fail; semantic success does not prove geometry,
mechanical feasibility, support sufficiency or printer compatibility.

TopOpt now depends on IR/domain; Project depends on TopOpt for orchestration.
`TopologyRequest` replaces its unused target-name/volume-fraction stub with a
specification, material and compilation revision. The worker passes its scoped
revision; standalone/synchronous builds use revision 0. No global cache identity
is implied. The unavailable optimizer still produces no numerical result.
Project reports E330 at each unresolved call before meshing; CAD rejects such
nodes directly under transforms, Booleans and compounds. No domain fallback is
used. The old successful snapshot may remain for display but export is blocked.
Future result publication must add capability checks, revision/hash identity,
progress, cancellation, convergence and post-reconstruction/post-operation
validation; even an unexpected successful placeholder response is not published.

CLI `inspect-topology` uses semantic compilation only, reports counts/spans and
backend unavailability, and does not authorize check/export. The battery support
example uses hypothetical loads/material properties, not DisplayOnly values as
mechanical evidence. Its menu action opens the DSL view with undoable source
replacement. The existing worker handles stale completion and cancellation.

Verification: 103 workspace tests pass, fmt and Clippy all-targets -D warnings
pass, CLI/desktop builds and actual inspect-topology run pass. The native menu
was exercised using XTest; `docs/screenshots/topology-declaration-native.png`
shows the inspected 1440x831 DSL/diagnostic state with export disabled. This
validates declarations and refusal, not FEM, supports or any numerical result.

## Repository hygiene

`.gitignore` excludes nested Rust target directories, local toolchains/Python
caches, generated/ and root exports/, logs/profiling output, downloaded research
PDFs, local environment files and editor/OS temporary files. Source projects,
Cargo.lock, shared Cargo/VS Code settings, CI, agent/task records and documented
screenshots remain versioned. Environment examples/templates are retained.
STL/PDF/PNG extensions are not globally ignored: source fixtures and references
can live outside generated/download directories. The 2026-10-07 review checked
28 excluded paths and 18 retained paths with `git check-ignore --no-index`;
no already indexed file matched the exclusions.
