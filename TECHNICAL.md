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

## Architecture decisions

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
