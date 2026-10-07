# Raspberry Pi camera module (2013)

Dimensional reconstruction of the supplied drawing credited to Gert van Loo,
21 May 2013, revision 1.0. The drawing describes manual measurements without
guarantees; this is not a certified model of other Raspberry Pi camera revisions.

`raspberry_pi_camera(hole_diameter)` is the reusable generator. `Camera` is its
example instance. `camera_pcb`, `camera_optics`, `camera_sensor_tail`,
`camera_connector` and `camera_ribbon` expose the subcomponents.

## Coordinates and dimensions

All dimensions below are in mm. Origin: lower-left corner of the PCB in the
top view, on its underside. X points right, Y toward the top of the drawing,
Z points out of the PCB toward the lens.

- PCB: 23.9 x 25 x 0.95.
- Four through holes: diameter 2; centers at (9.35, 2), (21.85, 2),
  (9.35, 23), (21.85, 23). The vertical spacing is 21, not 23:
  the drawing's 23 is measured from the bottom edge to the upper row.
- Optical base: 8 x 8, left edge X=5.1, centered along Y (Y=8.5..16.5).
  Its center X=9.1 is not the hole-column X=9.35.
- Optical stack: 5.2 above the PCB top, hence maximum Z=6.15.
- Underside connector: projects 2.8 below the PCB, minimum Z=-2.8.
- Ribbon width: 16.2. The 1.27 callout is interpreted as its center plane
  below the PCB underside.
- Complete envelope including illustrative ribbon: X=-5..23.9,
  Y=0..25, Z=-2.8..6.15 (28.9 x 25 x 8.95).

## Approximations

The barrel/lens diameters (7.4/5.6), optical height split (3 + 2.1 + 0.1),
connector footprint (5.6 x 19.4), sensor-tail rectangle (9.8 x 8 x 0.8),
ribbon extension (5) and thickness (0.3) are illustrative, not drawing callouts.
The ribbon is straight; the drawing's free end curve and the sensor tail's
chamfers/flexure are not reproduced. Colors are illustrative; there is no
optical simulation, texture, electronics, cable motion or transparency.

`DisplayOnly` material constants only satisfy the current DSL schema. They
are not measured mass, stiffness or simulation properties. STL is in mm and
does not preserve colors. The cylinders have 32 facets.

## Use

```sh
cargo run -p robogen-cli -- check examples/raspberry_pi_camera/main.rgn
cargo run -p robogen-cli -- export-stl examples/raspberry_pi_camera/main.rgn /tmp/raspberry-pi-camera.stl
cargo run -p robogen-desktop -- examples/raspberry_pi_camera/main.rgn
```

Change `MOUNT_DIAMETER` to adjust the four mounting holes independently of
the optical dimensions. The native Exemples menu also loads this model.