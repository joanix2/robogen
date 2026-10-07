# Screenshots

Release and milestone screenshots belong here. `docs/maquettes.jpeg` is the
original product direction reference and is not a claim of implemented solver
output.

M1 header revision, 2026-09-22, native X11/eframe captures:

- [Design, 1440x831](m1-design.png): application name/profile header above
	workspace tabs and project actions.
- [Design, 1080x680](m1-design-compact.png): the same two headers at the minimum
	supported window size, after removing window-manager maximization.

Both were visually inspected. These captures do not close the remaining M1
Optimisation and Apprentissage visual checks.

Servo CAD slice, 2026-10-07:

- [Servo, 1440x831](servo-native.png): real mesh CSG servo, source appearances,
	depth-sorted preview and grid behind the model.
- [Servo, 1080x680](servo-compact.png): native compact client, not a resized
	image. X11 override-redirect was temporarily needed to force the client size
	because the WM ignored ordinary resize requests.

Both captures were inspected. An earlier capture exposed polygon-stroke mitre
artifacts on thin triangles; indexed triangle drawing fixed them before these
captures. The native session did not accept synthetic clicks, so these images
prove layout/rendering, not a completed manual edit/orbit/export walkthrough.

Compute board, 2026-10-07:

- [Board, 1440x831](compute-board-native.png): seven-part model with PCB,
  heatsink, fan, hollow connectors and header pins. Captured from the normal
  native client and visually inspected. The reference only specifies the
  104 x 90 x 37 mm envelope; component detail is an approximation.

- [Corrected board, 1440x831](compute-board-depth.png): per-pixel depth replaces
	centroid sorting. Native client captured after an XTest camera drag; inspected
	fan hub, fins and connectors in front of the PCB without the earlier spurious
	triangles. The original board capture above is historical and did not prove
	correct occlusion. No forced client size was used for this corrected capture.

Raspberry Pi camera, 2026-10-07:

- [Camera, 1440x831](camera-native.png): native model loaded from the .rgn
	command-line argument. Four mounting holes, optics, sensor tail and ribbon
	are visible. Captured without forced sizing and visually inspected.
	Lens diameters, tail, connector footprint and cable details are approximate;
	see the example README. This records rendering, not a manual edit/export
	walkthrough. Automated tests cover the menu loader and export/undo path.

Generated taxonomy and library, 2026-10-07:

- [Left taxonomy/library, 1440x680](taxonomy-library-left-native.png): final
	layout after the user's left/right correction. Taxonomy is above Library
	on the left; the assistant stays on the right. Native XTest clicks on all
	three generator buttons added servo, camera and board alongside the original
	bracket. Four compiled instances and their meshes are present. The taxonomy
	scrolls when its allocated height is insufficient. Final client dimensions
	were measured at capture time; no forced resize was used for this image.
	The board is approximate, not a certified Jetson product model.

Li-ion battery, 2026-10-07:

- [Battery, 1440x831](battery-native.png): native X11 client loaded from the
	example source, without forced resizing. Visually inspected: yellow pack,
	red/black leads, red connector and fourth Library entry. Wire/connector
	dimensions are estimated; printed markings are not modelled. Automated
	tests cover loading, insertion, export/undo and visibility at two sizes.


Mini biped layout, 2026-10-07:

- [Biped, 1440x831](mini-biped-native.png): native client after XTest clicks on
  Exemples -> Mini bipède — disposition. Visually inspected: camera, vertical
  board, yellow battery, arms and both legs fully visible; 17 taxonomy instances.
  No forced resize or image compositing. An initial menu capture clipped the
  head; the loader now targets mid-height and a test checks all projected
  faces fit with margin at 600x400 and 320x240. Supports are intentionally absent
  pending the constraint/optimization pipeline. This demonstrates placement,
  not a walking robot or a TopOpt result. The corrected test window was left
  open for inspection; the earlier test instance was closed.

Declarative topology, 2026-10-07:

- [Topology declaration, 1440x831](topology-declaration-native.png): native
	X11 client after selecting Exemples > Support batterie (contraintes TopOpt).
	Visually inspected: DSL view selected, E330 at 23:12 with solver unavailable,
	previous taxonomy explicitly marked stale, export and insertion disabled.
	No forced resizing and no optimized geometry. Automated tests cover the
	semantic contract, CAD refusal, CLI inspection, history and worker revisions.

Prerequisite audit, 2026-10-08:

- [Optimisation, 1440x831](m1-optimisation-20261008.png): native tab navigation,
	actual design preview and explicit unavailable solver; no computed metrics.
	Disabled indicative settings are not a projection of TopologySpec.
- [Apprentissage, 1440x831](m1-apprentissage-20261008.png): native tab navigation,
	static model, unavailable simulation/training and empty metrics/trajectory.

Both actual X11 clients were visually inspected without forced resizing.
These close two missing desktop capture checks, not all M1 acceptance criteria
or the compact layout checks for these views. No backend result is claimed.
