# RoboGen technical notes

## Current backend status (2026-10-20)

Julia project sources, environment manifests, viewer, transport and subprocess
launch code have been removed by explicit user request, recorded in ADR-014.
Minimum-mass CPU/CUDA computation is unavailable: UI launch is disabled and API/
worker/CLI calls return diagnostics without results or export. Rust stiffness
optimization is retained. No Python replacement or RL runtime is installed.
OpenCode/Oh My OpenAgent 5.1.38 on the authorized DeepSeek-v4-pro provider
completed audit `ses_ea6243118ffeG4Fc9WGSP2qvxN` (exit 0, 273740 ms) and final
review `ses_ea61f65aaffe8pFEKwzEkb8STJ` (exit 0, 200442 ms, no blocking findings).
Both were read-only; parent/native specialists implemented and verified changes.
These replace the earlier insufficient-balance attempt as current session evidence.

Final removal validation: workspace fmt, Clippy/all-targets/-D warnings,
163 Rust tests passed with zero ignored, desktop/CLI builds, four Python
architecture tests, boundary checker, Ruff and actionlint passed locally.
A CLI minimum-mass request failed with source-located E340 and created no STL.
The rebuilt native client displayed the disabled minimum-mass path for complete
inputs; see [capture](docs/screenshots/mass-backend-unavailable-20261020.png).
Graphify was refreshed locally without LLM extraction (1475 nodes, 3374 edges,
139 communities). GitHub CI was not run by this local validation.

The dated Julia/CUDA sections below are historical measurements only; they do
not describe current capabilities. Ignored local downloads are left untouched
and no application code launches them.

## Native CUDA Mass Solver (2026-10-08, Iteration 16)

Optimisation now has an explicit CPU / GPU CUDA choice for minimum mass, CPU by
default. ComputeBackend travels in ExecutionSettings and each JSON event; Rust
rejects backend mismatches and retains the actual backend in MassDiagnostic.
The historical stiffness path refuses CUDA rather than silently running on CPU.
Julia loads the optional adapter only for CUDA requests. CPU code is retained.

CUDA 6.0.0 / Krylov 0.8.4 implement sparse preconditioned CG in Float64, forward
and adjoint, with validated true residual/backward error and bounded refinement.
CUDA 6.4.2/GPUCompiler 2.13 did not compile on Julia 1.10.10. The working versions
are pinned; see ADR-008 and the removed backend qualification notes (Git history) for the explicit roundoff
criterion and reproducible benchmark. No GPU types cross the Rust boundary.

Actual left-shin grid 24192 cells/80475 DOFs, unchanged PLA/loads/filter, two full
iterations: CPU 300.27 s, CUDA 105.18 s (~2.85x, warm session). Both produce
56.134595 g, displacement 1.887961/1.243991 mm >0.5 mm: not converged/feasible,
never publishable. The GPU speeds the linear solve; assembly and sensitivity
contraction remain CPU. Startup and very small grids may still be slower.

CUDA tests cover solutions/refusals/scaling, forward/adjoint finite differences,
independent cases, masks, filtered mass, actual biped equivalence and native UI
without export. No mechanical tolerance, grid or physical input was weakened.

Workspace fmt/Clippy/all-targets -D warnings/tests and desktop/CLI builds pass.
Native 1440x831 inspection confirms LeftShinSupport selected with GPU CUDA and
2.5 mm grid: docs/screenshots/biped-cuda-native.png. The new window is prepared
for an explicit run, not showing the offline benchmark as a persisted result.

## Estimated PLA Shin Inputs (2026-10-08, Iteration 15)

The user explicitly authorized estimated inputs pending real measurements.
Only LeftShinSupport now uses `left_shin_PLA_estimated` in the actual mini_biped
DSL. PLA_Estimated: 1240 kg/m3, E=2 GPa, nu=0.35, nominal strength 25 MPa.
The strength is metadata, not an enforced constraint. Forces are [10,0,-40] N
and [0,10,-40] N in separate cases, with a 0.5 mm nodal displacement limit.
The vertical load assumes 2 kg, one-leg support and a factor of two, rounded up.
No walking, stress, buckling, fatigue or FDM qualification is inferred.

Two distal offset pads are clamped; a proximal offset cylinder receives loads.
Both explicitly named estimated rigid-transfer interfaces substitute idealized
load transfer for unmeasured fastener/horn mechanics. Purchased servo geometry
and keepouts remain unchanged. Exact contact rings are not used as FEM supports
for this model: the conservative uniform projector cannot preserve their cells
while also excluding intersecting hole/housing cells. No projector rule is relaxed.

The projection test checks 28x24x36 cells at 2.5 mm, PLA properties, two independent
force resultants, nonempty interfaces and preserve/design/void regions. The project
integration test keeps ten incomplete preparations, seventeen purchased meshes
and no automatic optimized result. Detailed editable assumptions are documented
in examples/mini_biped/README.md. These inputs permit exploration only; source
completeness must never be presented as a validated or printable tibia.

Verification: workspace Clippy/all-targets -D warnings and tests, desktop/CLI
builds pass. Runtime-dependent ignored Julia tests were not rerun for this
input-only change; no optimization was launched. Native inspection shows the
estimated PLA/cases/supports on LeftShinSupport without missing-input warnings:
docs/screenshots/biped-left-shin-pla-estimated-native.png (1440x831).

## Physical Grid Resolution (2026-10-08, Iteration 14)

The mass workflow now defaults to a 2.5 mm target cell size in native Optimisation,
with an alternative explicit XYZ mode. Counts use ceil(domain extent / target),
and actual per-axis sizes, cell count and DOFs are shown before launch. Invalid
resource requests disable launch; no silent coarsening occurs. The regularization
filter retains its physical radius when the grid changes.

The common projector no longer constructs the historical FEM GridModel just to
enumerate nodes. Mass projection/Julia accept up to 32768 cells, 256 per axis,
150000 DOFs and 65536 cell-cases. The historical solver remains at 2048 cells and
64 per axis. These are resource bounds, not qualified accuracy guarantees or an
OS-level memory sandbox. Fine grids can use several GiB; small fixations may need
a locally refined/conforming mesh beyond this regular-grid budget.

Measured internal control: 120x40x20 mm, grid 48x16x8 (6144 Hex8 cells) instead of
12x4x2 (96). Two real Julia iterations took 78.6 s including startup, with peak
RSS 3269848 KiB (~3.12 GiB). Reference density mass remained 259.2 g; the 72.008 g
candidate was NOT converged. This is neither a mesh-independence study nor a
biped result. Forces, masks and per-case responses remain checked.

Diagnostic rendering now omits internal faces and shares grid vertices, without
smoothing or changing the density field. A full 48x16x8 grid needs 5120 boundary
triangles rather than 73728 cube triangles. Export remains disabled for diagnostics.

GPU permission was granted. nvidia-smi confirms an RTX 3060 with 12 GiB VRAM.
TopOpt 0.14.0's active DirectSolver is CPU sparse Cholesky/QR; its old CuVector
assembly code is commented out and CUDA is not an active dependency. No GPU
compute acceleration is claimed. A GPU adapter must pass numerical/gradient
contracts and measured timing comparisons before being enabled.

Validation: refined-grid execution test (explicitly run with --ignored), new
projection/UI/boundary-mesh tests, existing native Julia/UI test, workspace tests,
Clippy -D warnings, fmt and desktop/CLI builds pass. The native 1440x831 capture
is docs/screenshots/topology-grid-resolution-native.png. Existing biped input and
mechanical qualification blockers are unchanged.

## Native Minimum-Mass Diagnostics (2026-10-08, Iteration 13)

The native Optimisation workspace now runs a selected, complete minimize_mass
declaration through Project's existing background worker and a local Julia
process. The projector preserves the source geometry, material and independent
nodal loads/supports. It does not substitute the historical OC objective.
The external adapter currently accepts max_displacement only, with explicit
refusal of other constraints/manufacturing. Incomplete biped declarations still
return E347 and cannot launch; no hardware properties are filled by guesswork.

`robogen-topopt::mass::MassDiagnostic` carries operation/source/revision, grid,
regions, reference/current densities, density mass, per-case displacement limits,
history and final convergence status. ProjectSnapshot.mass_diagnostics and the
Diagnostic state are separate from validated topology reports and export meshes.
The existing native renderer displays the selected diagnostic, with a visual
threshold and reference toggle. The curve reports mass evaluations, not a certified
final-solid mass. Changes invalidate diagnostics; failed/cancelled jobs retain
the last accepted field but cannot publish geometry. Booleans downstream of
topology remain unsupported, including in diagnostic geometry composition.

`native.jl` owns Ferrite/TopOpt types; the Rust port owns a versioned JSON protocol,
local temp input, one-MiB events, bounded queue, 30-minute timeout and child cleanup.
The worker keeps only the latest diagnostic update and rejects stale revisions.
`serde_json` and `tempfile` are adapter dependencies. Setup remains explicit and
developer-local; see the backend README for pinned runtime installation/tests.
No browser server, WebView, Python application or second rendering stack is used.

The three runtime-dependent tests passed separately (TopOpt transport, Project
worker and UI rendering/export guards). Standard workspace tests skip these when
Julia is not installed. Full Rust tests, Clippy and desktop/CLI build pass.
This does not resolve the external qualification failures or create a computed
LeftShinSupport; exact transmission interfaces, physical data and binary
reanalysis remain mandatory before publication. The earlier iteration notes below
describe historical states.

## External Minimum-Mass Qualification (2026-10-08, Iteration 11)

TopOpt.jl 0.14.0 is installed with Julia 1.10.10 in ignored target/topopt-tools.
The experimental kernel, dependency lock, source-checked compatibility fixes and
reproduction commands live in
the removed backend qualification directory (available in Git history).
No Rust dependency, product dispatch, source schema or desktop behavior changed.
The homogeneous-mass/compliance smoke test passes seven assertions. Direct nodal
displacement with filtered masks and two independent force cases passes thirteen
assertions but fails three feasibility/convergence assertions at 500 iterations.
This is an intentionally visible failing qualification, not a production backend.
Do not publish the candidate or relax tolerances. Gradient/refinement checks,
stress/manufacturing, isolated process supervision and binary reanalysis remain.

Hardware research is recorded in examples/mini_biped/README.md. Servo identity,
horn attachment and printed PLA properties are not established by the candidate
manufacturer pages. LeftShinSupport remains incomplete, E347 remains appropriate.
Rust fmt, Clippy all-targets with denied warnings and workspace tests passed.

## Biped Support Preparation (2026-10-08, Iteration 10)

The mini_biped source now retains its 17 purchased components and adds eleven
separate topology preparations for camera, torso, pelvis and paired limbs/feet.
Their domain dimensions are design proposals, not optimized geometry. Contact
rings use the existing housing/board hole coordinates; horn transmission
interfaces, swept volumes and physical loading are still unresolved. Camera
mounting on the board is a design proposal, not a validated PCB load path.

TopologySpec schema 2 uses Option<MaterialId>. Explicit `material: unknown`
allows empty preserve/load/constraint lists for incomplete preparation; ordinary
known-material declarations retain their checks. Recompile derived IR from .rgn.
`preparation_issues()` is shared by the execution boundaries. Project, CLI and
TopOpt reject incomplete requests with E347 before any solve, including attempts
to supply the Part/display material as a fallback. The UI reports missing data
and disables launch, while retaining components and region previews. Existing
export guards exclude preparations; the global biped export is unavailable.

This is the real biped scene, not a standalone numerical fixture. It has no
computed support, no inferred walking load and no new minimize_mass capability.
Fixtures remain internal regression checks. Domain overlap and complete movement,
cable/tool keepouts must be resolved before mechanical acceptance.

Validation: focused IR/Project/UI/TopOpt tests, full workspace tests, fmt check,
Clippy all-targets with warnings denied and desktop/CLI builds passed. Native
1080x680 biped preparation captured; synthetic menu/tab clicks were inconclusive,
so manual Optimisation navigation is not claimed. See screenshots/README.md.

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

### Modular refactor and architecture checks (2026-10-20)

The structural refactor preserves the 21-crate workspace and all existing public
imports. Project separates document history, snapshot contracts, manifest IO,
projection and optimization orchestration. UI separates feature views and shared
presentation helpers. The mass adapter separates diagnostics, protocol validation
and historical process execution. See [current architecture](docs/architecture/CURRENT.md)
and [ADR-013](docs/adr/ADR-013-modular-application-boundaries.md).

Production dependency boundaries are checked without compiling or networking:

```sh
python3 -m unittest discover -s scripts/tests
python3 scripts/check_architecture.py
```

These checks run in CI before the existing Rust gates. The Python checker uses
only the standard library (Python >=3.11); it is development tooling, not a new
scientific backend. Local lint/format checks are `ruff check scripts` and
`ruff format --check scripts`. Numerical settings, physical hypotheses, wire
schema, cancellation/revision filtering and export guards are unchanged.

The target is Rust host plus qualified Python scientific adapters, with Julia
excluded from the future stack. The legacy Julia path was initially isolated by the structural step and was
subsequently removed under ADR-014 at the user's request. pyMOTO/MuJoCo/PPO are not installed or implemented here.
The mini-biped still requires actual hardware and physical input qualification.

Oh My OpenAgent 5.1.38 was invoked with the user's explicit DeepSeek authorization.
Session `ses_ea63790c1ffeXxEUB3v1zZ9NjP` failed on its first request with
`Insufficient Balance` (exit 1); it made no source edits. Disjoint native sessions
performed Project, UI and mass extraction under parent integration instead.
No alternate external provider was configured. Local execution logs are under
ignored `.agentkanban/logs/refactor-17`; transient `.omo/run-continuation` is ignored.

Verification of the combined refactor: `cargo fmt --check`,
`cargo clippy --workspace --all-targets --offline -- -D warnings`,
`cargo test --workspace --offline` (161 passed, 8 scientific qualifications
ignored), and desktop/CLI builds passed. Four architecture unit tests, the
21-crate/36-edge check, Ruff and actionlint also passed locally. GitHub-hosted
CI was not executed by this local run.
Native X11 verification loaded the mini-biped through Exemples and navigated
Design/Optimization/Learning after compilation completed. The 1440x831 captures
in [screenshots](docs/screenshots/README.md) show preparation overlays, blocked
incomplete supports and unavailable training. They do not qualify mechanics/RL.

### Local development tools

Graphify 0.9.15 is installed as the isolated `uv` tool package `graphifyy`,
matching the installed Graphify skill. Its executable is in `~/.local/bin`.
The previous executable linked to a removed VS Code Snap revision; reinstall
with `UV_TOOL_BIN_DIR="$HOME/.local/bin" uv tool install --force graphifyy==0.9.15`
if that link breaks again. Installing the tool does not generate a project graph.

The local Rust 1.88.0 toolchain includes Clippy, rustfmt, rust-analyzer and rust-src,
declared in `rust-toolchain.toml`.
`.vscode/settings.json` supplies its environment to new Linux integrated
terminals and rust-analyzer, and selects Clippy for editor checks. The official
`rust-lang.rust-analyzer` extension is installed in the current VS Code profile.
The workspace explicitly selects the local rustup rust-analyzer proxy through
`rust-analyzer.server.path`. The installed extension discovers toolchains using
the VS Code process environment, before applying `server.extraEnv`; relying on
automatic discovery can therefore select its incompatible bundled server.
After updating the components/settings, run **Developer: Reload Window** in the
VS Code command palette. If a component is missing, activate the environment below
and run `rustup component add rust-analyzer rust-src --toolchain 1.88.0`.
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

## Standalone M7 Beam (2026-10-08)

The user explicitly authorized a manual-load M7 scientific slice before closing
M1-M6, tracked in ticket 15 iteration 3 and ADR-008. `robogen-topopt` now owns
`linear`, `fem` and `simp`: a backend-neutral sparse port with a private faer
0.22.6 LLT adapter; regular 3D Hex8 Gauss-integrated elasticity; filtered SIMP,
chain-rule sensitivities, OC volume update and solved-state history. faer uses
std/sparse-linalg without rayon. No third-party types escape the crate API.

Beam inputs have domain Length/Pressure/Force values; Compliance exposes joules.
Full request/revision and backend identifier accompany results. DOF indices are
typed algebraic indices, not persisted geometry identifiers. The bounded API is
synchronous for CLI/worker use, with cooperative cancellation and per-iteration
progress; direct factorization cannot be interrupted internally. No UI wiring,
new project schema, persistent numerical cache or document publication is added.
`UnavailableTopologyOptimizer`, E330 and the CAD refusal path remain active.

`cargo run -p robogen-topopt --example beam` produces reproducible CSV progress.
The 120x40x20 mm, 12x4x2 benchmark stops at iteration 65 (design change 0.004620056),
volume 0.35, residual 6.616003e-13. Compliance decreases from 6.271263227758e-6 J
to 2.377550262401e-6 J against uniform density at the same volume. Local debug
run: 20.932 seconds, not a general performance guarantee. The earlier 60-step
run correctly reported IterationLimit. Full assumptions and numerical tolerances
are in [the benchmark](examples/topopt_beam/README.md).

Ten unit tests plus one integration test cover sparse failures, element energy,
analytical traction, rigid modes, bending refinement, finite-difference gradients,
volume, cancellation, final-state reanalysis and quadratic compliance load scaling.
All 114 workspace tests pass; fmt and workspace all-targets Clippy -D warnings pass.
M7 reconstruction/desktop publication and constraint capability checking remain
open; these results do not certify stress, fabrication or any robot support.

## M7 Surface Reconstruction (2026-10-08)

Ticket 15 iteration 4 adds `surface` in robogen-topopt: backend-neutral
SurfaceReconstructor/DensityGrid and a private fast-surface-nets 0.2.1 adapter.
Two outside zero-density layers close chunk boundaries; trilinear subdivision
by two avoids the benchmark's coarse-grid non-manifold case. No FEM densities
or threshold are changed to obtain the mesh. Non-manifold/degenerate outputs
remain errors; empty/invalid/cancelled states are distinct. Threshold [0.5,1),
typed coordinates/volume, immutable mesh arrays, component count and backend/
sampling metadata accompany the result. BeamSurface retains request/revision and
solver provenance. No global artifact identity or persistent cache is claimed.

Validation checks finite coordinates/bounds, triangle area, opposite edge pairs,
vertex links and signed volume. It does not prove absence of self-intersections,
mechanical performance or preservation of interfaces. The bounded extraction
call is not internally cancellable; surrounding sampling/validation is.

The standalone example optionally writes `--surface-stl PATH` using existing
robogen-export and Mesh DTO through dev-dependencies only; production TopOpt
does not depend on CAD/export. Native UI and E330 guards are unchanged. STL
coordinates are millimetres and are tested by decoding the binary records.

Actual converged benchmark: 208 vertices/404 triangles, three components,
surface volume 13463.265650 mm3 vs filtered material 33600 mm3. Independent
trimesh STL reimport agrees on closure/winding/components/volume. X extent is
1.696579..76.402588 mm: neither the fixed nor the loaded face survives. This
is explicitly a failed mechanical reconstruction, despite valid surface
topology. The density-field compliance is not a property of that mesh.
The diagnostic figure in docs/screenshots is an offline plot of the real STL,
not a desktop capture. Interface preservation and post-reconstruction reanalysis
must precede any robot-support publication.

Four surface tests added, existing benchmark and integration extended to
reconstruction/STL/provenance; all 118 workspace tests and fmt/Clippy gates pass.
Full assumptions and reproduction command: examples/topopt_beam/README.md.

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

## M7 Region Masks And Exact Binary Reanalysis (iteration 5, 2026-10-08)

The standalone BeamRequest accepts explicit Design/Preserve/Void masks. The
filter/transpose and OC preserve constants, count mandatory material in the
volume equality and reject infeasible targets. There is no automatic preserve
thickness. Fully fixed masks use FEM directly. Filter roundoff at density one
is clamped to the mathematical convex-combination range [0,1].

Binary reanalysis removes void Hex8 elements and unused DOFs, requires complete
beam end interfaces and a face-connected load path, then checks residual and
energy balance without the SIMP stiffness floor. The exact voxel boundary uses
shared lattice vertices, manifold validation and a volume invariant. BeamSolid
retains request/revision, solver, reanalysed metrics and actual volume; budget
status is separate from mechanical solvability. Surface Nets remains a separate
approximate preview. No production cross-crate dependency or persistent schema
is added, and no desktop export guard is bypassed.

`--solid-stl PATH` runs the explicitly documented 0.85-volume case with preserved
end layers and threshold 0.65. It exports only a connected, reanalysed candidate
within budget: 80/96 cells, 80,000 mm3, 168 vertices/336 triangles, preserved
X=0/X=120 faces, compliance 2.964559539151e-7 J. Thresholds 0.5 (over budget)
and 0.75 (disconnected) are rejected by the respective checks. Neither this case
nor the validation machinery guarantees arbitrary constraints are feasible.
124 workspace tests/fmt/Clippy pass. Full reproduction and trial limitations:
examples/topopt_beam/README.md. No new UI route is delivered in this slice;
TopologySpec/desktop publication stays unavailable pending its real adapter.

## General Hex8 And Multi-Case SIMP (iteration 6)

`GridModel`/`GridDefinition` replace beam-specific FEM assembly. BoundaryConditions
carries revision-local NodeId, per-axis translation locks and typed nodal forces.
Rigid-mode rank, input/resource, residual/energy and binary connectivity/interface
checks apply. Loads on constrained components are rejected explicitly. BeamModel
is the reference-problem adapter and exposes its GridDefinition/boundary conditions;
FemResult wraps GridResult and preserves convenient access to existing quantities.

`GridRequest`/`optimize_grid` support 1..16 independently solved cases carrying
LoadCaseId and source span. Shared-density OC minimizes summed compliance, with
summed derivatives; force vectors are never merged across cases. Per-case metrics
and analyses survive in the result, and errors retain the offending ID/span.
`optimize_beam` now delegates to this same implementation. `GridSolid` reanalyses
every case on exact binary cells, tracks completed-case progress and cancels
without publishing partial analyses. DTOs remain TopOpt-owned, nonpersistent,
with no third-party or CAD/UI dependencies.

Regression evidence and reproduction: examples/topopt_beam/README.md and
crates/robogen-topopt/tests/grid.rs. The general-grid patch test checks traction
along X/Y/Z using directional supports; multi-case finite differences use distinct
support sets and load directions. Opposed loads remain two nonzero responses.
The actual DSL geometry/interface projector, per-case constraint enforcement and
revision-guarded product publication remain required; E330 is still active.

Iteration 6 gates: 130 tests workspace, fmt --check and Clippy workspace
all-targets -D warnings pass; no product UI change in this numerical slice.

## Typed Topology Execution From DSL (iteration 7)

TopOpt now owns a conservative CSG-to-grid projector (`geometry`/`execution`),
reading the existing SolidGeometry/TopologySpec. It classifies whole cells for
box/cylinder/transforms/union/compound/difference, excludes uncertain domain/void
cells and preserves all potential preserve intersections. Resolution/conflicts
produce diagnostics with source spans. Region trees and cell grids are bounded.
Interfaces are voxelised volumes: fixed locks their nodes, forces use consistent
uniform-volume nodal weights, moments use a weighted force couple. Cartesian
frictionless is supported; pin/oblique supports are refused. ADR-012 specifies
the approximation, coordinate frames, load distribution and regularized boundary
semantics; this is not an exact CAD intersection or a surface-selection tool.

`prepare_topology` carries source/material/revision and returns GridRequest,
origin/domain-cell count and interface centroids. `execute_topology` requires
convergence, reconstructs/reanalyses all cases, checks actual volume/mass and
per-case maximum displacement, then returns ComputedTopology. Errors E340–E346
distinguish capabilities, geometry, interface mapping, solve, constraints,
cancellation and iteration limit. Unsupported objective/stress/frequency/safety/
manufacturing requests are never silently dropped. CPU settings are execution
inputs, not manual geometry inserted into the DSL. SIMP now accepts a volume
budget up to 1; fully prescribed masks are genuinely analysed with FixedRegions
and zero OC updates, rather than rejected merely for having no design variables.

CLI `optimize-topology SOURCE NAME_OR_ID OUTPUT NX NY NZ FILTER_MM THRESHOLD`
compiles the DSL and executes one selected node. It exports in the node's local
frame with the grid origin restored, using the existing atomic STL exporter.
`inspect-topology` now prints operation IDs and distinguishes declaration
inspection from computation. CLI adds local TopOpt/domain deps; TopOpt adds DSL
only as a test dependency. No production CAD dependency or persistent schema
is introduced. Project/desktop E330 remains until graph publication is integrated;
outer transforms/Booleans are not included in this explicit node export.

Reproducible `.rgn`, full assumptions, measured values and negative trials:
examples/topology_manual/README.md. Measured actual run: 22 updates, two cases,
80,000 mm3, 216 g, 336 triangles, maximum displacement 0.000219890239692 mm per
case. Unit/integration/CLI tests cover projection, force/torque conservation,
limits, cancellation, capability refusal and source-to-reanalysis-to-STL.

Iteration 7 gates: 139 tests workspace, fmt --check and Clippy workspace
all-targets -D warnings pass. A common-translation regression also verifies
unchanged cell masks after normalizing geometry into the local grid frame.
No desktop integration or native UI validation is claimed for this CLI slice.

## Topology Document Publication (iteration 8)

Project now executes supported topology and converts accepted binary results
into CAD Mesh, restoring the grid origin. ResolvedTopologyKernel composes outer
rigid transforms, colors and compounds with ordinary geometry. Missing results,
malformed meshes and downstream Booleans fail explicitly. No solver types enter
CAD and no generated source is substituted.

ProjectSnapshot atomically includes TopologyReport with source span/material,
worker-scoped revision (0 synchronously), grid origin, settings, backend,
iteration count, mass/volume and each binary case's compliance, displacement
and residual. Rebuild recalculates topology; no persistent result cache exists.
The worker rejects stale revisions and supports cancellation; faer factorization
remains non-interruptible. Progress counts topology nodes and CAD parts, not time
remaining. Previous meshes/reports survive invalid edits, with export disabled.

Document builds use the fixture preset [12,4,2] cells, 12 mm filter, 0.65 threshold
and 100 iterations. These settings are visible in Optimisation, not automatically
tuned or sufficient evidence for tiny servo interfaces. Configurable numerical
controls remain CLI/API-only. E330 wraps the localized backend failure rather
than reporting all topology unavailable. Minimize-mass, stress/safety/frequency
and manufacturing remain unsupported. Publication is not certification.

Native entry: Exemples > TopOpt calculable (charges manuelles), or start desktop
with examples/topology_manual/main.rgn and select Optimisation. Tests cover
composition, invalid mesh data, actual published measurements, load scaling,
failed-limit atomicity, cancellation, asynchronous history and export guards.

Mini-biped hardware boundary: servo DSL provides casing/tabs, diameter-2 mm cuts
at (5.9,-2.4) and (5.9,24.9) mm, and a diameter-4.6 x 3.2 mm shaft. It contains
no output horn, spline profile or retention screw. User requested exact interfaces,
not provisional ones; articulated supports are not generated from assumed horns.
Robot loads and calibrated materials remain separate requirements. The existing
17-component layout is unchanged.

Iteration 8 verification: 144 workspace tests, fmt/Clippy -D warnings, desktop
build and editor diagnostics passed. Native results inspected at 1080x831 and
1080x680; capture in docs/screenshots/topology-publication-native.png. Hardware
interfaces remain incomplete, so the biped-support acceptance stays open.

## Preparation And Explicit Per-Part Optimization (iteration 9)

This supersedes iteration 8's automatic execution. Project::from_source and
rebuild only compile and tessellate declarative regions. CAD topology_preview
retains enclosing rigid transforms and projects three separate layers without
requiring solver capabilities. ProjectSnapshot.preparation is keyed by PartId;
these meshes never enter the exportable meshes table. Semantic validation still
applies: the existing DSL declaration syntax has not become a draft schema.

SourceDocument::start_optimization(PartId, ExecutionSettings) sends an explicit
job to the existing bounded worker. Only that part's operations run. Other
computed meshes/reports remain intact. Settings are editable per part in the
native control panel; reports retain the settings used, not the next-run edits.
Loads/material/interfaces/constraints stay source-owned and editable through
the undoable DSL editor. The worker serializes jobs; concurrent launches are
rejected, not silently enqueued. Cancellation remains cooperative.

Each part has NotCalculated/Running/Calculated/Obsolete/Failed/Cancelled state.
Calculation diagnostics are distinct from compilation diagnostics: an unsupported
objective does not invalidate geometric preparation. Every source edit currently
invalidates all TopOpt results conservatively, including undo/redo, without
automatic recomputation. Job revisions prevent stale publication. Numerical
settings are transient next-run controls, not persisted source changes.

Conception shows domain/preserve/keepout contours (blue/green/red), components
and computed results with independent visibility. Optimisation shows selected
part regions and per-part calculation controls/reports. Regions share one scene
frame; hiding them does not recenter the camera. Whole-document export requires
all topology parts calculated. Optimization export contains only the selected
calculated part, independently of visibility or other pending parts. CLI check
accepts preparation; export-stl refuses unresolved parts; optimize-topology stays
an explicit node-local execution/export command.

No new solver capability, mechanical interface, stress field, smoothed surface
or robot support is inferred from the reference mock-ups. The current region
display uses mesh contours, not transparent shaded volumes or face picking.

Iteration 9 verification: 149 workspace tests, fmt, Clippy all-targets -D warnings,
desktop build and editor diagnostics passed. Native preparation/selection/result
captures and XTest launch/masking checks are documented in docs/screenshots/README.md.

## External Topology Backend Direction (2026-10-08)

ADR-008 now records the requested shift from further in-house FEM/SIMP development
to an existing-library adapter. Scikit-Topt is the first prototype candidate,
not an installed or validated backend. The current Rust execution path remains
active until replacement contracts pass. TopologySpec/TopologyRequest already
separate DSL semantics from execution; the next boundary is a versioned numerical
problem/result protocol for a local isolated Python process. The existing
TopologyOptimizer trait is still a skeleton, not the production dispatcher.

Preserve iteration 9's preparation-only compilation and explicit per-part jobs.
The prototype must verify independent load cases/supports, preserve/void handling
through filters, units/index ordering, cancellation and post-reconstruction
reanalysis. Minimum compliance at a volume budget is not minimum mass under
strength limits. Capabilities remain explicit and unsupported constraints fail.
See ADR-008 for primary sources, package-version caveat and migration gates.


## Mass convergence and connected candidate diagnostics (iteration 17)

The mass adapter now uses a conservative smooth nodal-displacement envelope;
exact physical maxima still control feasibility in the wire protocol. The
previous 500-iteration failing fixture converges in 29 iterations. The actual
estimated-PLA tibia also converges, with no change to loads, grid or limits.
See the backend README and ADR-008 for the mathematical bound and evidence.

After convergence, bounded descending thresholds are tested for face connectivity
and exact reduced-cell FEM reanalysis. BinaryMassDiagnostic is kept separate
from gray-field mass and from exportable meshes. Rust checks the received mask,
mass and case limits. Native Optimisation exposes the reanalysed candidate and
its actual threshold/mass; a visual connectivity proposal never grants export.
Product inputs remain the original .rgn; no generated robot geometry is inserted
into the DSL. Callback response reuse avoids duplicate FEM solves outside AD.
