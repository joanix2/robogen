# RoboGen DSL

`.rgn` files are the human-readable source of truth. The milestone grammar is
documented by the executable example in
[`examples/constrained_bracket/main.rgn`](../../examples/constrained_bracket/main.rgn).

The parser is recovery-oriented: it retains a partial spanned AST and returns
structured diagnostics for editor display. Semantic compilation resolves names
and physical units before CAD evaluation. Renderer, CAD-kernel and physics
types are deliberately absent from the syntax layer.

## User-Defined Geometry

Requested on 2026-09-22: define a procedural servo once in `.rgn`, then use its
name as a reusable geometry constructor. This is a user-defined symbol, not a
new reserved keyword or a Rust plugin. The same mechanism must work for brackets,
wheels and components composed from other components.

Same-file components, defaults, named arguments, arithmetic and nested calls
are executable. This constructor can be used by a part with a declared material:

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
These conventions are implemented by the mesh CSG adapter in ADR-010.
Dimensions above are illustrative, not a manufacturer's servo specification.
The clearance parameter is reserved for the mounting geometry in the full
example; it is deliberately not applied to this simplified body and shaft.

Imports remain unavailable (E109). The following cross-file syntax is a
future proposal, not executable:

```text
module robot;
import actuators.servos;

part HipServoGeometry {
		body = actuators.servos.servo(width: 25 mm, clearance: 0.2 mm);
}
```

The [complete servo example](../../examples/servo/main.rgn) demonstrates
mounting tabs, holes, nested constructors and clearance. Real mesh booleans
implement union and difference; closed-mesh and volume tests guard them.
`color(red: 48, green: 108, blue: 224, shape: solid)` adds an opaque appearance.
Channels are unitless integers in 0..255. Nested explicit colors override an
outer color; outer colors fill uncolored Boolean cut faces. STL omits colors.

`compound(first, second, ...)` retains separate bodies without fusing their
surfaces. It accepts 1 to 128 positional operands and can be wrapped in
`translate` or `color`. It cannot be an operand of `union` or `difference`;
the current adapter reports a diagnostic instead. Compound output is bounded
to 128 leaf bodies, 32 wrapper levels, 4096 wrapper visits, 262144 vertices
and 524288 triangles. Each leaf keeps the existing CSG budgets.

## Library And Taxonomy

The Library lists `servo`, `raspberry_pi_camera` and `jetson_nano_super`.
Adding an instance copies its function definitions into the current document
once, with prefixed internal symbols, then appends a uniquely named `part`.
Existing functions are reused rather than overwritten. Conflicting imported
names return E331 without changing the source. This is source insertion, not
the still-unavailable `import` syntax.

For example, two parts calling `servo()` produce two independent taxonomy
entries and one shared function definition. Arguments and placement can be
edited in the source; the insertion button uses default arguments and places
the new part beyond the current X bounds. Existing instance materials are
reused. The board generator groups the existing separate solids with compound.
The Taxonomy above the Library reads compiled parts/features, with links to
their calls and definitions. Invalid/pending documents retain the last valid
taxonomy with an explicit stale-result status; insertion/export are disabled.

### Delivery And Acceptance

The supplementary servo ticket of 2026-10-07 authorizes this DSL/CAD path
without declaring M1, M2 or M3 complete. Import resolution, a block editor,
stable Boolean face naming and a dedicated wgpu viewport remain separate work.
Architecture and backend contracts are recorded in ADR-010.

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


## Rigid Placement

`rotate(x: 90 deg, y: 0 deg, z: 0 deg, shape: component())` rotates a solid or
compound about the origin, right-handed, X then Y then Z. All four arguments
are required. Angles must use `deg`, `rad` or an `Angle` parameter; unitless
numbers and lengths produce diagnostics. Nested translations define a pivot:
translate to put the local anchor at zero, rotate, then translate to its
assembly position. Colors, separate shells and source provenance are retained.

The [mini biped](../../examples/mini_biped/README.md) uses this operation to
place existing components through shared dimensional equations. These are
placement equations, not executable mechanical/TopOpt constraints. Supports
remain absent until the declarative constraint contracts and M7/M8 solver are
implemented. See [ADR-011](../adr/ADR-011-component-placement.md).
