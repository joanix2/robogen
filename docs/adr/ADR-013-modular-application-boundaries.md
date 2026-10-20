# ADR-013 — Modular application and feature boundaries

- Status: accepted for the structural refactor (ticket 17); legacy retention
  superseded by ADR-014 on 2026-10-20
- Date: 2026-10-20

## Context

The product target is one printable mini humanoid described in `.rgn`, with
integrated topology optimization and reinforcement learning. Rust already owns
syntax, semantic types, native presentation and revision-aware computation.
Large source files currently mix document history, projections, widgets and
external-process transport. This makes independent changes and backend
replacement harder to review.

The user requests a healthy codebase and autonomous, bounded engineering work.
Julia is excluded from the target stack. A qualified Python scientific adapter
is the proposed replacement direction; installing a library is not qualification.

## Decision

Preserve the existing crate boundaries and public import paths while organizing
implementation by responsibility. The shared semantic model remains the single
source-derived model. Internal module extraction does not introduce a second
application, a new persistent schema, or new physical capabilities.

- `robogen-project`: core snapshot/report contracts, application document/history,
  compilation and optimization orchestration, and project-file infrastructure.
- `robogen-ui`: core presentation state, feature views for design/optimization/
  learning, reusable widgets/theme, and CAD-to-preview presentation conversion.
- `robogen-topopt::mass`: diagnostic value types/connectivity, numerical wire
  validation, and external-process execution in separate internal modules.

Root modules re-export existing public types and functions. Internal items gain
only the narrowest visibility necessary for their owner; no third-party backend
or toolkit type is added to a public boundary. Rust privacy must be used rather
than making every helper public.

Keep functions and files focused, preferably below 350 lines. Large feature
views or historical test groups require a documented exception until they can
be separated along real responsibilities. Do not split a function mechanically
or add abstractions solely to satisfy a line count.

## Preserved contracts

- Source edits remain undoable, including invalid edits; the last valid snapshot
  can remain visible while export is disabled.
- Worker cancellation and revision checks guard final and intermediate results.
- Preparation meshes, mass diagnostics and mechanically accepted meshes remain
  distinct; unsupported constraints and unavailable backends are explicit.
- IDs, units, serialized formats, numerical settings and physical hypotheses are
  unchanged by the module extraction.
- Existing examples, UI labels, shortcuts, exports and public imports keep their
  behavior. Moved tests retain their fixtures and ignored scientific gates.

## Backend transition

The historical Julia implementation is isolated as a legacy adapter during this
structural step. It is not extended or represented as the target architecture.
Removal from execution requires a separately qualified replacement and an
explicit capability check. Do not replace a real result with synthetic output
or silently reinterpret minimization of mass as fixed-volume compliance.

Future Python adapters are local supervised processes with versioned RoboGen
requests/results, bounded messages, cancellation, revisions and locked runtime
versions. Their capabilities and limitations must pass contract fixtures before
application dispatch changes. PPO remains an existing-library integration after
the robotics/physics and M9 prerequisites; this ADR implements no RL backend.

## Verification and handoff

Run focused package tests, the existing vertical integrations, workspace fmt,
Clippy/all-targets with denied warnings, and workspace tests. Inspect the native
Design/Optimization/Learning navigation and mini-biped preparation. Scientific
Julia/CUDA tests remain separate qualification evidence, never silently passed.

The parent owns manifests, shared documentation, task records, final validation
and Git checkpoints. Delegated sessions have disjoint file ownership and return
changes, contracts, exact checks, risks and required handoffs.
