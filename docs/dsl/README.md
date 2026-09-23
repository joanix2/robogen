# RoboGen DSL

`.rgn` files are the human-readable source of truth. The milestone grammar is
documented by the executable example in
[`examples/constrained_bracket/main.rgn`](../../examples/constrained_bracket/main.rgn).

The parser is recovery-oriented: it retains a partial spanned AST and returns
structured diagnostics for editor display. Semantic compilation resolves names
and physical units before CAD evaluation. Renderer, CAD-kernel and physics
types are deliberately absent from the syntax layer.

## Planned User-Defined Geometry

Requested on 2026-09-22: define a procedural servo once in `.rgn`, then use its
name as a reusable geometry constructor. This is a user-defined symbol, not a
new reserved keyword or a Rust plugin. The same mechanism must work for brackets,
wheels and components composed from other components.

The following syntax is a proposal, **not currently executable**:

```text
module actuators.servos;

component servo(
		width: Length = 23 mm,
		depth: Length = 12 mm,
		height: Length = 24 mm,
		shaft_radius: Length = 2 mm,
		clearance: Length = 0 mm
) -> Solid {
		body = box(size: [width, depth, height]);
		shaft = translate(
				offset: [width / 2, depth / 2, height],
				shape: cylinder(radius: shaft_radius, height: 5 mm)
		);
		return union(body, shaft);
}
```

The box uses a corner origin; the cylinder starts at z=0 with its axis along +Z.
These conventions are proposed and must be finalized with the CAD contract.
Dimensions above are illustrative, not a manufacturer's servo specification.
The clearance parameter is reserved for the mounting geometry in the full
example; it is deliberately not applied to this simplified body and shaft.

Use from another file would look like:

```text
module robot;
import actuators.servos;

part HipServoGeometry {
		body = actuators.servos.servo(width: 25 mm, clearance: 0.2 mm);
}
```

The complete servo example must additionally demonstrate mounting tabs and
holes using union, difference and transforms. Subtraction must perform a real
geometric boolean, not hide overlapping triangles or append meshes. A separate
wrapper can then reuse the servo constructor to build a mount or clearance tool.

### Delivery And Acceptance

Preserve ticket order: finish M1 before extending the DSL/CAD implementation
plan. M2 supplies definitions, parameter scope, defaults, named calls, imports,
qualified names, arithmetic and unit checks in the typed semantic model. CAD
work supplies solid primitives, transformations and actual boolean evaluation;
backend and cross-crate contract decisions require architecture review first.

- Two calls with different arguments produce independent instances with stable
	instance-qualified IDs and source provenance for both definition and call.
- Composing a constructor inside another works without compiler changes.
- Unknown names, duplicate or missing arguments, incompatible units, invalid
	dimensions and import cycles produce localized diagnostics, never panics.
- Recursive constructor calls are diagnosed in the initial implementation;
	expansion has explicit resource limits. Arbitrary code execution is excluded.
- Source edits and undo/redo rebuild the affected geometry without silently
	exporting a stale result. Tests cover definition -> call -> mesh -> STL.
- A rectangular mounting-hole fixture verifies actual subtraction, bounds and
	watertightness; changing one instance must leave the other unchanged.

This defines geometry only. A simulated servomotor additionally needs links,
joints, travel limits and actuator properties in the robotics/physics model.
An editor made of visual blocks could later edit the same source model; the
reference image does not by itself request a second language or a block editor.

