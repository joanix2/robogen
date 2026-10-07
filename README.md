# RoboGen

RoboGen is a native, declarative robotics design environment written in Rust. A
`.rgn` document is the source of truth; CAD, rendering, robot models and future
simulation/optimisation pipelines are projections of the same semantic model.

## Current vertical slice

The current milestone provides:

- a native dark desktop shell for Design, Optimisation and Training;
- a typed, diagnostic-producing parser for the constrained-bracket DSL subset;
- parameter-aware sketch evaluation and deterministic extrusion to a triangle mesh;
- reusable same-file geometry components, mesh booleans and RGB appearances;
- live rebuild after editing `WIDTH` or `HEIGHT` in the DSL editor;
- background desktop compilation with cancellation and stale-revision rejection;
- binary STL export through the CLI or desktop export action;
- explicit unavailable states for AI, topology optimisation and RL.

In Design, the left column shows **Taxonomie** (compiled project instances)
above **Bibliotheque** (DSL generators). The **+** next to Servomoteur,
Camera Raspberry Pi, Jetson Nano Super or Batterie Li-ion 1S adds an instance without replacing
the document. The chat stays on the right. Instance/function links open the
corresponding DSL definition; source changes and additions support undo/redo.
The Jetson label refers to the existing approximate 104 x 90 x 37 mm board,
not a verified manufacturer model.

Run the UI with `cargo run -p robogen-desktop`, or compile/export the example:

```sh
cargo run -p robogen-cli -- check examples/constrained_bracket/main.rgn
cargo run -p robogen-cli -- export-stl examples/constrained_bracket/main.rgn bracket.stl
```

For the complete parametric servo, choose **Exemples > Servomoteur**, or run
`cargo run -p robogen-desktop -- examples/servo/main.rgn`.
See [the servo example](examples/servo/README.md) for dimensions, clearance,
colors, export and limitations.

The [compute board example](examples/compute_board/README.md) reconstructs the
104 x 90 x 37 mm reference photograph. Open **Exemples > Carte embarquee**;
undimensioned components are approximate and not manufacturing specifications.

The [Raspberry Pi camera example](examples/raspberry_pi_camera/README.md)
reconstructs the dimensioned 2013 module with four mounting holes, optics,
underside connector and a short illustrative ribbon. Open
**Exemples > Camera Raspberry Pi (2013)**. The PCB is 23.9 x 25 x 0.95 mm;
undimensioned optical and connector details are explicitly approximate.

The [Li-ion battery example](examples/li_ion_battery/README.md) models a yellow
OEM pack with a nominal 18 mm diameter and 68 mm body length. Wires and the red
two-position connector are separately parameterised estimates. Open it from
**Exemples > Batterie Li-ion**, or add it from the Library. Printed markings and
internal cell/electrical properties are not modelled.

The [mini biped layout](examples/mini_biped/README.md) places 14 servos, the
board, battery and camera with shared dimensional parameters. Open
**Exemples > Mini bipède — disposition**. Mechanical supports will be generated
from constraints in the planned TopOpt path; they are not fabricated in this
initial layout.

The [declarative topology example](examples/topology_battery_support/README.md)
defines a battery support's domain, preserved interfaces, exclusions, loads,
limits and manufacturing profile through `topology(...)`. Inspect it with
`cargo run -p robogen-cli -- inspect-topology examples/topology_battery_support/main.rgn`,
or open **Exemples > Support batterie (contraintes TopOpt)**. Semantic validation
is implemented; numerical optimization of these declarations is not. Check/export explicitly fail
until a real backend can generate and validate the support.

A separate [M7 cantilever benchmark](examples/topopt_beam/README.md) now performs
real CPU Hex8 FEM and filtered SIMP with sparse faer factorization:
`cargo run -p robogen-topopt --example beam`. It reports measured compliance,
volume, residual and convergence; it does not yet reconstruct a surface or
enable robot-support optimization in the desktop.

Quality gates are `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace`.

