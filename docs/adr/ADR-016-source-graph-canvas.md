# ADR-016 — One declared mechanical model for graph, schematic and geometry

- Status: accepted for ticket 17, corrected iteration 10
- Date: 2026-10-20

The user's vise illustration distinguishes the relationship graph (rigid bodies
and typed joints) from a geometric source-dependency graph. The uncommitted
reference-graph implementation is replaced; its interactive canvas is reused.

Domain owns serializable neutral MechanicalModel records with typed body/joint
IDs, part memberships, joint kinds, physical origins, direction axes and helical
pitch. IR resolves new `body` and `joint` declarations, names and quantities.
Robotics validates mechanical record invariants. Project invokes validation on
compilation and rebuild, introducing Project -> Robotics (no reverse dependency).
UI owns only layout, projection, symbols, picking and transient view state.
SemanticModel remains the common source-derived record. No backend types escape.

Joint origins and axes are expressed in the document rest frame; they are not
relative FK transforms. Body part memberships refer to the same PartIds as CAD.
Optional actuator ObjectInstanceId links a declared joint to its placed motor;
the parent carries the housing, the child the rotating output by convention.
This does not split the purchased motor mesh or qualify its mounting interface.

The model permits closed chains, including the provided vise triangle. Validation
checks identities, memberships, endpoints, axes and required pitch; it does not
solve loop closure, infer mobility from edge counts, simulate motion, certify
clearance or establish load paths. Fixed edges must not indirectly lock a declared
moving joint; that structural check belongs to Robotics.

The mini-biped declarations describe design intent using its existing shaft
origins/orientations. Unqualified support preparations are not silently assigned
to rigid bodies or accepted as manufacturable attachments. The vise fixture is
an explanatory mechanism, not an engineering-qualified product.

No persistent graph coordinates are added. Source edits use existing document
commands/history; node dragging affects only the view. Missing mechanical
metadata yields an explicit empty state, never inferred geometric connections.
Existing serialized SemanticModels default to an empty MechanicalModel; no stored
project migration is required because .rgn remains authoritative.
