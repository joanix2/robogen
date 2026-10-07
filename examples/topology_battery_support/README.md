# Declarative battery support

This example declares a deferred `topology(...)` solid. It does **not** contain
an optimized support, a FEM run or a printable part. M7/M8 remain unavailable.

The nominal purchased battery is 18 x 68 mm. `CLEARANCE` is a radial 0.4 mm
allowance around it; the excluded cylinder also leaves axial room. The design
domain is 30 x 24 x 74 mm by default, with explicit preserved mounting and
seat regions. These regions are not measured fastener interfaces. No mounting
hole pattern, cable access, kinematic clearance or hardware compatibility is
asserted. Region inclusion/intersection checks belong to the future backend.

The 3 N load, IllustrativePLA constants and all limits are hypothetical inputs
for a language fixture, not measurements of the battery, robot or printed PLA.
The FDM declaration is not a calibrated FlashForge profile. The source must
be replaced with verified data before a real mechanical run.

## Inspect

```sh
cargo run -p robogen-cli -- inspect-topology examples/topology_battery_support/main.rgn
```

This succeeds with a semantic-only report: one operation, two preserved
regions, one exclusion, one load case, six constraints, one FDM declaration,
and an explicit unavailable solver. `check` and `export-stl` instead fail
with E330 at the topology call. No domain mesh is substituted for the result.

In the desktop choose **Exemples > Support batterie (contraintes TopOpt)**.
The loader selects the DSL view. Editing and undo/redo use existing source
history; pending or failed compilation disables export. A previously valid
preview can remain visible with the application's stale-result indication.
It is not the generated support.

## Parameters and composition

Edit `BATTERY_DIAMETER`, `BATTERY_LENGTH`, `CLEARANCE` and `DESIGN_FORCE` in the
source. `battery_support()` can be nested in translate, rotate, color, union,
difference or compound. Each distinct call in the same feature must declare
a unique topology name; separate part/features qualify names independently.
The topology domain, interfaces, loads and manufacturing directions share the
operation's local coordinates. Outer rigid transforms place its eventual
result; they do not infer new gravity/load cases from the robot pose.

See [DSL contract](../../docs/dsl/README.md#declarative-topology) and
[ADR-012](../../docs/adr/ADR-012-declarative-topology.md). Geometric publication
checks, load distribution, per-constraint solver capabilities, convergence,
reconstruction, persistence and mechanical revalidation remain future work.