# ADR-015 — Component instances own geometric interface regions

- Status: accepted for ticket 17, iteration 7
- Date: 2026-10-20

## Problem

The mini-biped repeats poses independently for visible servos, attachment regions
and occupied/clearance volumes. A topology declaration receives copied geometry
without an explicit reference to the component instance that requires it. Moving
one occurrence can desynchronize these projections, and repeated components have
no reusable contract for their local interface regions.

## Decision

Extend the DSL with reusable Object values and named instance declarations.
An Object contains a Solid shape, explicit exclusion volumes and named geometric
interfaces in its local frame. It carries no loads, supports or solver settings.
Existing Solid components remain valid. Object and Instance are distinct from
Solid and physical quantity parameter types.

```text
component servo_object() -> Object {
    return object(shape: servo(), void: [servo_exclusion()],
        interfaces: [interface(name: "housing", shape: servo_housing_contacts())]);
}
instance LeftKneeServo = translate(offset: [0 mm, 0 mm, 100 mm],
    shape: rotate(x: 0 deg, y: 90 deg, z: 0 deg, shape: servo_object()));
```

Translate/rotate on an Object apply atomically to its shape and every region.
A named instance receives a stable typed ObjectInstanceId derived from its module
and declaration name. Its geometry and regions are projections of that one value.
No independently maintained placement expression is needed for its interfaces.

The ordinary Solid projections are `instance_shape(instance)`,
`instance_void(instance)` and `instance_interface(instance, "name")`. Topology
continues to select explicit preserve/void geometry. Its existing interface names
identify piece-local loads/supports; they must not implicitly copy a load case
from the component template. A component parameter of type Instance allows a
reusable support generator to select regions from specific occurrences.

SemanticModel stores serde-default instance records and explicit instance-use
provenance (identity, selected region/interface, feature key and source span).
Geometry remains the existing backend-neutral SolidGeometry: CAD, renderer,
export and TopOpt receive projections and do not implement instance lookup.
No new backend type crosses a public boundary. Additive semantic metadata and a
serde-default Solid return type preserve deserialization of prior snapshots;
.rgn sources remain authoritative and prior syntax needs no migration.

## Meaning of full and empty

The component's occupied/clearance region excludes material from the new printed
piece. An interface identifies a possible connection; only interfaces selected
by that piece become material to preserve. Preserve material belongs to the new
piece, never to the entire purchased component. Each instance has its own pose
and identity, while intrinsic interface definitions are reusable.

Domain, chosen interfaces, loads, supports, material and manufacturing constraints
remain specific to the optimized part. Estimated mechanical data remain labeled
as estimates. No unavailable optimizer or physical result is enabled here.

## Validation and recovery

Reject duplicate/unknown instances and interfaces, incompatible Object/Solid/
Instance types, cycles and resource-budget violations with source diagnostics.
Test two translated/rotated occurrences of one object and a support selecting
only one interface. Check stable IDs, region provenance, parameter rebuilds,
source undo/redo and unchanged unrelated instances. Migrate the mini-biped's
servo regions to shared instance poses; keep geometric and physical invariants.

Existing sources remain supported. No automatic overwrite of user .rgn files is
performed. Reverting to the previous source checkpoint is the rollback path for
the migrated example. Run focused parser/IR tests, a vertical CAD/projection
integration and the standard workspace gates, followed by native inspection.
