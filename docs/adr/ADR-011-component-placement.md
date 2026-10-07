# ADR-011 - Rigid component placement for the biped layout

- Status: accepted for the component layout slice; optimization contracts deferred
- Date: 2026-10-07

## Decision

`rotate(x: Angle, y: Angle, z: Angle, shape: Solid)` applies right-handed
rotations about the origin, in X then Y then Z order. All four arguments are
required; degrees/radians are accepted, untyped angles are rejected. Nesting
with `translate` defines pivot placement explicitly. `SolidOperation::Rotate`
stores `[Angle; 3]` and child geometry, retaining source spans and existing
feature/part IDs. The DSL remains backend-neutral. No new dependency is added.

The CAD adapter privately computes the rotation. It rotates CSG vertex positions,
vertex normals and plane normals consistently; a rotation about zero preserves
plane distance. It does not use the upstream general transform path implicated
in ADR-010. Compounds retain separate shells and ordered transform stacks.
Colors, winding and volumes are preserved. Finite angles, coordinate bounds,
existing complexity limits and cooperative cancellation remain enforced.

The source project schema is unchanged. Serialized derived IR has an additive
Rotate variant; consumers built before this change cannot read it and must
recompile the .rgn source with the new backend. No generated IR is persisted
in tracked projects and no source migration is required. Rollback consists of
keeping the source and opening it with a rotation-capable version; older
compilers diagnose an unknown constructor. Future caches must include producer
version; no backend-owned matrix becomes persistent assembly identity.

## Initial biped versus optimization

The example places existing bought-component models through shared parameters
and equations. It does not contain invented brackets, fake optimized meshes,
FEM properties, joint dynamics or an assembly constraint solver. Component
geometry stays inside copied library definitions because imports are unavailable.
A test compares these definitions to their library origins to catch drift.

Ticket 15 must establish the actual declarative assembly/OptimizationSpec
contracts before introducing regions, preserved interfaces, keep-outs, loads
and objectives to the language. Final support geometry will be derived from
those constraints under ADR-008, not authored as bracket primitives in .rgn.
The present parameter equations constrain placement only; they do not enforce
collision avoidance or mechanical strength. M7/M8 dependencies remain in force.

## Verification

Typed angle diagnostics; 90-degree axis and noncommuting transformation checks;
compound versus individual-shell transforms; rotated Boolean volume and closure;
source -> semantic model -> meshes -> STL and undo/redo; native biped display.
