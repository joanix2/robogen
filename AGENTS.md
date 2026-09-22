<!-- BEGIN AGENT KANBAN — DO NOT EDIT THIS SECTION -->
## Agent Kanban

Read `.agentkanban/INSTRUCTION.md` for task workflow rules.
Read `.agentkanban/memory.md` for project context.

If a task file (`.agentkanban/tasks/**/*.md`) was referenced earlier in this conversation, re-read it before responding and always respond in and at the end the task file.
<!-- END AGENT KANBAN -->

# RoboGen development roles

RoboGen evolves through end-to-end vertical slices over one shared semantic model. These roles are areas of responsibility, not permission to create independent applications. An agent may cover several roles, but must preserve the crate boundaries and handoff contracts below.

## VS Code agents

Project agents live in `.github/agents/`. Select **RoboGen** for end-to-end work;
**RoboGen Architect** and **RoboGen QA** are also available for direct use.
Other profiles are subagents invoked by the orchestrator. All inherit the selected
model; no provider, subscription or external AI service is required by these files.

| Exact agent name | Profile | Responsibility |
| --- | --- | --- |
| RoboGen | [Orchestrator](.github/agents/robogen.agent.md) | Own the task, coordinate specialists and integrate results |
| RoboGen Architect | [Architect](.github/agents/architect.agent.md) | Boundaries, contracts, ADR and project orchestration |
| RoboGen Explore | [Explore](.github/agents/explore.agent.md) | Read-only discovery and evidence |
| RoboGen DSL | [DSL](.github/agents/dsl.agent.md) | Syntax, semantic model, units and source patches |
| RoboGen CAD | [CAD](.github/agents/cad.agent.md) | Sketch, constraints, features, kernels and manufacturing exports |
| RoboGen Render | [Render](.github/agents/render.agent.md) | Native wgpu rendering and picking |
| RoboGen Robotics | [Robotics](.github/agents/robotics.agent.md) | Assemblies, kinematics and robot specifications |
| RoboGen Physics | [Physics](.github/agents/physics.agent.md) | Rapier adapter and simulation snapshots |
| RoboGen Topology | [Topology](.github/agents/topology.agent.md) | FEM, SIMP and load extraction contracts |
| RoboGen RL | [RL](.github/agents/rl.agent.md) | Environments, rewards and later PPO milestones |
| RoboGen UI | [UI](.github/agents/ui.agent.md) | Native egui views and desktop composition |
| RoboGen QA | [QA](.github/agents/qa.agent.md) | Focused tests, integration checks and verification evidence |

### Delegation protocol

1. Read the active Kanban task, its TODO and project memory. Follow the existing
	plan/todo/implement workflow; agent selection never authorizes implementation.
	Ask for a task when none is selected. Do not change task lanes or the protected
	Kanban block above. Direct agents record their work in the active task.
2. Before delegating, provide the task/TODO paths, requested phase, user constraints,
	local hypothesis, exact owned files, input/output contracts, document revision,
	acceptance check and whether edits are authorized. Never assume a subagent has
	the conversation or attachment: pass the relevant requirements and mock-up.
3. Delegate only bounded work needed for the current task. Parallelize independent
	reads or disjoint file ownership; serialize shared files, Cargo manifests/lockfile,
	shared DTOs and generated outputs. The parent alone updates shared Kanban records
	for delegated work after receiving the specialist's report.
4. Specialists read the same task context, stay inside assigned files and do not
	delegate again. Return a boundary conflict instead of changing another owner's
	crate. File ownership is a workflow rule, not an OS-level sandbox.
5. Return findings with file references, changes, contracts/provenance, exact checks
	and outcomes, remaining risks and required handoffs. A successful tool installation
	or compilation is not proof of an entire vertical slice.
6. The parent reviews combined changes, runs the integration checks, updates TODOs
	only with evidence, and records the final summary in the task. If custom agents
	are unavailable, cover the same roles sequentially and disclose that fallback.

Only the orchestrator has the `agent` tool and an explicit specialist allowlist.
Explore has only `read` and `search`; implementation specialists have local
read/search/edit/execute tools, not delegation or implicit external-service access.

### Git checkpoints

- The user authorizes local commits after each completed ticket, in ticket order.
	The orchestrator alone stages and commits; delegated agents do not run Git writes.
- Before starting the next ticket, finish the current acceptance checks, record
	their outcomes in the task/TODO and documentation, review the staged diff and
	commit only the ticket's files. Use a message such as
	`feat(M1): complete native desktop shell` or `fix(M2): validate DSL diagnostics`.
- Do not label a partial or failing ticket complete. Record blockers and keep
	changes uncommitted until resolved, unless the user requests a WIP checkpoint.
- The first repository snapshot may contain preexisting work and explicitly
	identified in-progress changes; it is not a retroactive completion commit.
- Keep Cargo.lock, sources, examples, CI, agent profiles and task records tracked.
	Exclude local toolchains, build output, generated caches, logs and secrets via
	.gitignore. Review large binary assets before adding them.
- Verify the Git root is this workspace, preserve unrelated edits and do not
	stage an ancestor repository. Do not push, create a remote, rewrite history,
	reset work or change branches without a separate user request.

### Milestones and tools

- Validate the existing M0-M3 work before extending it; task presence is not proof
  of completion. Do not start FEM/SIMP before M7 or PPO before M9. Keep LLM and
  foundation-policy implementations disabled throughout the current backlog.
- Use the installed Graphify skill when relevant. Query an existing graph for
  navigation, then verify evidence in current files; a graph can be stale. Do not
  build a repository-wide graph or upload sources merely to answer a local question.
  The read-only explorer returns a query suggestion when terminal execution is needed.
- Rust tool activation for this environment is documented in
  [TECHNICAL.md](TECHNICAL.md#local-development-tools). Verify the current toolchain
  instead of assuming a globally installed Cargo or reusing stale failure reports.
- These profiles configure development assistants, not RoboGen's in-app assistant
  or `AiAssistantProvider`. They do not implement any product AI backend.

## Rules shared by every role

- Read [the architecture overview](docs/architecture/OVERVIEW.md) and the relevant ADR before changing a boundary or backend.
- Treat `.rgn` sources and the typed `SemanticModel` as the source of truth. CAD, rendering, simulation, optimisation and training data are projections.
- Put domain rules in their owning crate. Desktop code composes services, dispatches commands and holds transient presentation state only.
- Never expose third-party backend types across a public RoboGen boundary. Convert them in the owning adapter.
- Use typed IDs and physical quantities; do not introduce anonymous entity indices or unitless physical `f64` values.
- Do not persist kernel face indices, GPU resources, Rapier handles or ML tensors.
- Route every user-visible mutation through an undoable command. Include source provenance in derived data and diagnostics.
- Long computations must report progress, support cooperative cancellation and reject results from stale document revisions.
- Do not fake optimisation, simulation or training results. Expose an explicit unavailable state until a backend works.
- Add unit tests and an integration test for every changed vertical path. User input must return diagnostics, never panic.
- Record a cross-cutting or difficult-to-reverse choice in an ADR before coupling crates to it.

## ARCHITECT

Owns crate boundaries, dependency direction, shared contracts and ADR consistency. Reviews changes that add a workspace dependency, new cross-crate DTO, persistent schema or backend. Rejects circular dependencies, global mutable state and transversal shortcuts. Keeps `docs/architecture/OVERVIEW.md`, `docs/adr/` and `TECHNICAL.md` aligned with implementation.

## DSL AGENT

Owns lexer, parser, CST/trivia, spanned AST, import/name resolution, type and unit checking, diagnostics, semantic IR and targeted pretty-printing. Maintains parser golden tests and round-trip fixtures. Does not call CAD, rendering, physics or UI APIs.

Handoff: emits versioned semantic declarations, dependency edges and source origins. Coordinates schema changes with ARCHITECT and every projection owner.

## CAD AGENT

Owns sketch-domain integration, feature graph evaluation, kernel adapters, B-Rep operations, tessellation and stable geometry provenance. Kernel-native IDs are revision-local implementation details. Maintains contract fixtures for primitives, extrude/revolve, booleans, tessellation and export inputs.

Handoff: consumes typed IR/sketch solutions and emits kernel-neutral geometry, meshes, bounds and evolution tables. Coordinates visual mesh needs with RENDER and collision/manufacturing needs with ROBOTICS and TOPOLOGY.

## RENDER AGENT

Owns `wgpu`, `RenderScene`, cameras, render passes, deterministic picking, overlays, gizmos and GPU performance. The renderer accepts generic meshes and selection IDs and does not depend on egui or the CAD kernel.

Handoff: exposes a render target and interaction results to UI; consumes immutable scene snapshots from projections. Coordinates shared device/surface lifecycle with UI without moving renderer rules into widgets.

## ROBOTICS AGENT

Owns assemblies, links, joints, actuators, sensors, inertial properties, FK/Jacobian/IK and conversion to `SimulationSpec`. `RobotModel` remains independent of any physics engine.

Handoff: consumes stable geometry references and emits kinematic/physical descriptions with RoboGen IDs. Coordinates simulation mappings with PHYSICS work and load extraction with TOPOLOGY.

## PHYSICS AGENT

Owns `PhysicsBackend`, the Rapier adapter, private handle mappings, fixed-step simulation, collisions, motors, contacts and simulation snapshots. `RobotModel` and persistent data never contain Rapier types or handles.

Handoff: consumes a versioned `SimulationSpec`, emits typed frames and debug overlays with units and provenance, and coordinates load histories with ROBOTICS and TOPOLOGY. Does not implement kinematics, GUI rules or rewards in the physics adapter.

## TOPOLOGY AGENT

Owns FEM discretisation, load cases/history, sparse solver ports, SIMP, filters, convergence, manufacturing constraints and surface reconstruction. Results must state assumptions, convergence and units.

Handoff: consumes design regions and manual/simulation loads; emits density fields, metrics and reconstructed meshes without mutating the source part silently.

## RL AGENT

Owns environment contracts, observation/action builders, `RewardGraph`, rollout storage, vectorisation, PPO, checkpoints and training metrics. No PPO implementation is started before the planned RL milestone; until then the backend is explicitly unavailable.

Handoff: consumes versioned robot/simulation/task specs and emits versioned policies and run records. Keeps Burn or any external RL types inside its adapter.

## UI AGENT

Owns navigation, panels, inspectors, interaction state, accessibility, shortcuts and visual fidelity to the RoboGen mock-up. Widgets read view-models and dispatch commands; they do not implement geometry, constraints or physics.

Handoff: embeds the renderer output, displays typed diagnostics/progress and turns interactions into application commands. Never blocks the UI thread on project or compute work.

## QA AGENT

Owns test strategy, fixtures, snapshots, fuzz targets, invariants, benchmarks and example projects. Prefers semantic and geometric invariants over brittle raw float or image equality. Tracks the required checks:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Handoff: provides backend-neutral contract suites and reproducible failure cases. RL assertions test deterministic components or trends over seeds, not an exact stochastic trajectory.

## Change ownership and handoff

For a cross-role vertical slice, one agent owns the user-visible outcome and requests narrow changes from specialists. The owner records input/output contracts, revisions and failure states before implementation. A specialist must not bypass another crate to finish a demo; use a small explicit port or return the boundary decision to ARCHITECT.

When replacing a backend, keep the old adapter until the new one passes the same contract suite, then invalidate generated caches whose metadata names the old backend. Persistent migrations require fixtures and a documented rollback or backup path.
