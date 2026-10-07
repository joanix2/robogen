# Compute Board - Visual Envelope

A schematic reconstruction of the supplied board photograph. The only
dimensioned information is the **104 x 90 x 37 mm overall envelope**. The exact
product reference is not established. This is not a manufacturer CAD model.

Open **Exemples > Carte embarquee (104 x 90 mm)** in RoboGen, or run:

```sh
cargo run -p robogen-desktop -- examples/compute_board/main.rgn
cargo run -p robogen-cli -- check examples/compute_board/main.rgn
cargo run -p robogen-cli -- export-stl examples/compute_board/main.rgn compute-board.stl
```

The model contains a drilled main PCB, elevated module, twelve heatsink fins,
fan frame with a circular opening and corner holes, a static four-blade rotor,
front connector shells, two stacked USB-style housings, a 40-pin illustrative
header, chips and capacitors. Geometry is split into seven named parts. Colors
distinguish boards, metal, plastic and contacts.

`BOARD_LENGTH` and `BOARD_WIDTH` control the PCB outline and mounting holes;
they do not scale the entire component layout. `TOTAL_HEIGHT` changes the
heatsink fin height and the fan elevation. Coordinates are in millimetres with
Z up and the front connectors opening toward negative Y. All undimensioned
sizes, positions, hole patterns, connector types and rotor details are estimates.
The fan blades are straight schematic paddles, not an aerodynamic replica.

`DisplayOnly` supplies placeholder physical quantities required by the DSL.
Do not use them for mass, inertia, strength, electrical or thermal calculations.
There is no electronics, cooling simulation or functional pin assignment.

The STL uses millimetres and has no colors. Each body is closed, but the export
contains separate touching or overlapping bodies: it is not a certified,
single fused printable solid or a validated mounting/drilling template.