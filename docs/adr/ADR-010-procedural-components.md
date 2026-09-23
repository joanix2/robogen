# ADR-010 - Procedural geometry components

- Status: accepted for the first executable component slice, backend validation pending
- Date: 2026-09-22

## Decision

User-defined `component` declarations are typed, bounded geometry constructors,
not reserved words, Rust plugins or arbitrary executable code. Parameters use
physical units, defaults and lexical scope. Calls expand into RoboGen-owned
`SolidGeometry` in robogen-ir. Every node carries a source span; component call
diagnostics must retain the call and definition context. Instance feature IDs
are qualified by module, part and feature. Cycles and expansion limits produce
diagnostics instead of unbounded recursion.

The first geometry contract covers corner-origin boxes, +Z cylinders starting
at z=0, translations, unions and differences. Lengths stay typed until the CAD
adapter converts them to its private backend units. No third-party geometry
types cross crate boundaries. The existing extrusion adapter remains available.

Real boolean operations require an established geometry library and executable
volume/closure tests; appending meshes is not a union implementation. A mesh CSG
adapter is a limited STL-oriented addition, not a replacement for the proposed
Truck B-Rep backend in ADR-002, nor a STEP or stable-face-naming implementation.
Backend selection and measured limitations must be recorded before delivery.

Project compilation and SourceDocument undo/redo remain the shared entry point.
This slice does not implement physical actuator behavior, a visual block editor,
or the deferred asynchronous task system. Existing synchronous rebuild limits
must remain explicit. The user's implementation request authorizes this focused
DSL/CAD addition while M1 remains open; no milestone is marked complete by it.

## Mesh CSG Backend

The candidate adapter pins `csgrs = 0.15.0` (MIT), with default features disabled
and only `f64`, `earcut-io`, `earclip-io` and `chull-io` enabled. The latter two
are required by incompletely gated upstream imports even though the adapter does
not use convex hulls. Compilation and
geometry contracts on Rust 1.88 must pass before this selection is validated.
Versions 0.20.1 and 0.16.0 cannot resolve because they require yanked core2 0.4.0;
0.15.0 predates that dependency. Boolmesh 0.1.10 uses MPL-2.0 and was not selected
under this slice's permissive-license requirement.
The upstream BSP union/difference and triangulation implement the core geometry;
RoboGen does not introduce its own Boolean algorithm. Optional file formats,
text, images, fields and parallel execution are disabled. Upstream still
depends on nalgebra and the f64 Parry/Rapier libraries; this does not enable
RoboGen physics. Backend types remain private to robogen-cad.

The adapter will retain native rectangle extrusion and use millimetres internally
for mesh CSG, converting to SI metres at the Mesh boundary. Cylinders are faceted,
not exact analytic surfaces. The synchronous API has no cancellation or revision
publication guard; the caller owns revision checks. Node spans identify geometry
errors; the existing Mesh DTO does not acquire persistent face provenance.
STEP, B-Rep, stable Boolean face naming and general manufacturing certification
remain unavailable. Input complexity and intermediate polygon costs must be
bounded before calling the backend; unwinding backend panics become CadError,
not fabricated geometry. Fatal allocation failures/abort are not unwindable.