# OEM Li-ion 1S battery pack

Nominal assembly model of the yellow RC pack described by the user, marked
"Li-ion 3.7V 3000mAh 15C", with red/black leads and a red two-position connector.
The body envelope is **diameter 18 mm x length 68 mm**, not 21700 or 70 mm long.
This is an approximate external fit/visualisation model, not a manufacturer CAD file.

## Parameters

All lengths below are millimetres. They are editable as document parameters
or per-instance named arguments to `li_ion_battery(...)`.

| Document parameter | Argument | Default | Basis |
| --- | --- | --- | --- |
| PACK_DIAMETER | diameter | 18 | User-selected nominal pack envelope |
| PACK_LENGTH | length | 68 | User-selected nominal pack envelope |
| WIRE_LENGTH | wire_length | 20 | Estimated, straight leads |
| WIRE_DIAMETER | wire_diameter | 1.4 | Estimated, insulation included |
| WIRE_SPACING | wire_spacing | 3 | Estimated centre spacing |
| CONNECTOR_WIDTH | connector_width | 6 | Estimated |
| CONNECTOR_DEPTH | connector_depth | 4 | Estimated |
| CONNECTOR_LENGTH | connector_length | 10 | Estimated |

The pack axis is +Z, from 0 to 68 mm; X/Y are centred on its axis. Leads leave
the same end at X = +/-1.5 mm and run to Z = 88 mm. The connector ends at
Z = 98 mm. The complete default envelope is 18 x 18 x 98 mm.
The red lead is on negative X; polarity placement is illustrative, not verified.
Two blind rectangular sockets open at the connector's +Z face. Their proportions
are estimated; no connector family or mating compatibility is asserted.

`battery_body(diameter, length)` isolates the pack without wires. Two 0.2 mm
end layers use a darker yellow; these are a visual convention within the nominal
68 mm envelope, not measured shrink-wrap thickness. `compound` retains separate
closed shells, including touching interfaces; it is not a Boolean union.
Use positive dimensions with pack length greater than 0.4 mm. Arbitrary wire
spacing and connector dimensions are not constrained to form a valid harness.
Printed text/logos, flexible cable bends, terminals and internal chemistry are
not modelled. The DSL currently has no surface-label texture operation.

## Source provenance and limits

References supplied by the user, not independently verified in this iteration:

- [eBay item 298604746866](https://www.ebay.com/itm/298604746866): described as an
  unbranded/OEM pack with matching appearance and label.
- [VICMILE comparison](https://www.findthisbest.com/brand/727278-vicmile): user
  reports a similar 3.7 V / 3000 mAh / 15C pack at 68 x 18 mm and 53 g. This does
  not identify this pack as VICMILE; 53 g is not assigned to this model.
- [Bare-cell comparison](https://www.lithiumlifepo4battery.com/sale-12824677-15c-discharge-lithium-ion-rechargeable-batteries-3-7v-3000mah-samsung-inr18650-30q.html):
  the cited approximately 18.3 x 64.9 mm cell is a different reference. An
  18.3 mm cell cannot fit inside an 18 mm outside diameter; these are not a
  consistent measured stack. No internal cell is generated.

Measure the actual pack before designing a housing. The suggested 18.5-19 mm
diameter x 69 mm reservation is a starting clearance, not guaranteed fit, and
does not include wire routing or connector access. No housing is generated.
Electrical label values are descriptive only. `DisplayOnly` contains required
placeholder material constants, not validated mass, strength, thermal or battery
simulation properties. This geometry must not be used to infer electrical limits.

## Use and checks

Choose **Exemples > Batterie Li-ion (18 x 68 mm)** to replace the document, or
the **+** next to **Batterie Li-ion 1S** in the left Library to add an undoable
instance. The Library scrolls and supports searching for `batterie`.

```sh
cargo run -p robogen-cli -- check examples/li_ion_battery/main.rgn
cargo run -p robogen-cli -- export-stl examples/li_ion_battery/main.rgn /tmp/li-ion-battery.stl
cargo run -p robogen-desktop -- examples/li_ion_battery/main.rgn
```

STL is exported in millimetres without colors. Automated tests check envelope
dimensions and parameter variations, invalid diameter, shell closure/orientation,
colors, STL geometry, library insertion/undo and preview visibility at two sizes.