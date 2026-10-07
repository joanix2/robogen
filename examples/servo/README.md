# Parametric Servo

This example transcribes the supplied block-model screenshots, not a
manufacturer's dimensional specification. Units are millimetres. Boxes use
their minimum corner as origin; cylinders start at z=0 along +Z.

Open **Exemples > Servomoteur** in the desktop, or run:

```sh
cargo run -p robogen-desktop -- examples/servo/main.rgn
cargo run -p robogen-cli -- check examples/servo/main.rgn
cargo run -p robogen-cli -- export-stl examples/servo/main.rgn servo.stl
```

The blue housing is 11.8 x 22.5 x 22.7 mm. The mounting tabs extend from
y=-4.7 to 27.2 mm, at z=15.9..18.4 mm. The output shaft reaches z=29.9 mm.
Two radius-1 mm cuts and two 1.3 mm slots open the tabs. The black connector
lies below the front tab, so the hole is not a free passage through the entire
height of the assembly.

`servo_main(jeu)` constructs the unperforated model. `servo(jeu)` subtracts
the mounting openings. Change `parameter JEU = 0 mm;` to `0.2 mm` to expand
only the rectangular housing by 0.2 mm on each side, exactly as in the
reference. This is not a global offset of the motor or a certified fit allowance.
An excessive clearance can interfere with the mounting openings.

Use Code DSL to edit the parameter and the existing undo/redo controls to
restore it. Compilation runs in a bounded background worker; the previous
preview stays visible but export is disabled while building or invalid.
The bottom progress strip can cancel a build. After cancellation, edit the
source or use undo/redo to request a new build.

The desktop export button writes `robogen-export.stl` in the working directory.
STL coordinates are millimetres and contain no colors. The three source RGB
appearances survive Boolean operations and are displayed in the preview.
No physics, block editor, STEP, analytic B-Rep or library imports are included.
Cylinders have 32 segments. The preview remains a CPU projection drawn by egui,
not a dedicated CAD wgpu renderer.