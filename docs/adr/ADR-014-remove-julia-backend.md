# ADR-014 — Remove the Julia backend immediately

- Status: accepted by explicit user instruction
- Date: 2026-10-20
- Supersedes: ADR-013's temporary retention of the Julia adapter and the
  Julia execution/qualification portions of ADR-008

## Decision

Remove the Julia sources, environment manifests, launch scripts, viewer and
Rust subprocess bridge now. The user explicitly accepts removal before a
replacement has passed scientific qualification. Historical measurements remain
historical documentation; they describe no currently available backend.

Retain Rust's existing fixed-volume stiffness optimization, typed minimum-mass
DSL declarations, neutral diagnostic DTOs and geometry/load projection. No
minimum-mass CPU or CUDA engine is available after removal. An explicit
capability flag disables the corresponding native launch controls. Direct API,
CLI and worker calls must return source-located diagnostics without emitting
progress results, meshes or fabricated metrics. Cancellation remains distinct.
Do not reinterpret minimum mass as stiffness optimization or install a new
scientific package as an implicit replacement.

Remove tests and transport fixtures that exclusively qualify the deleted
backend. Replace them with deterministic tests for unavailability, cancellation,
worker failure, export guards and UI refusal. Keep backend-neutral density
connectivity tests and the existing stiffness integration suite.

## Consequences and recovery

Minimum-mass computation is intentionally unavailable, even for fully specified
parts. Incomplete preparations remain incomplete. Existing `.rgn` sources need
no migration: source syntax, IDs, units and revision contracts are unchanged.
Generated Julia results/caches must not be reused as current computed geometry.
Ignored local runtime downloads are not project sources and are left untouched;
no application code discovers or launches them. Git history contains the former
adapter for an explicit future recovery; no persistent source data is deleted.

A future Python adapter needs its own pinned environment and capability/contract
qualification before minimum-mass dispatch or UI launch is enabled. No Python
optimizer, simulator or RL algorithm is implemented by this removal.

## Verification

Check absence of Julia project sources and executable references. Run focused
minimum-mass API/worker/UI refusal tests and the existing stiffness integration,
then workspace fmt, Clippy and tests, desktop/CLI builds and native inspection.
Use OpenCode/Oh My OpenAgent on the authorized DeepSeek provider for an audit
and a final review; the parent verifies findings and records actual outcomes.
