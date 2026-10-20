# Screenshots

CUDA minimum-mass integration, 2026-10-08:

- [Biped CUDA, 1440x831](biped-cuda-native.png): native X11 client,
  LeftShinSupport selected, GPU CUDA active for the next run, target 2.5 mm
  and 24192 cells. Inspected without resizing/compositing. This capture shows
  the actual control and preparation, not an optimized or printable tibia.
  CPU/CUDA timing and numerical comparisons are in the backend README.

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

M7 standalone reconstruction, 2026-10-08:

- [Actual beam STL diagnostic](m7-beam-surface-20261008.png): offline matplotlib
	figure generated from the actual STL reimported through trimesh, not a native
	application capture. Three components are colored separately; 208 vertices,
	404 triangles. Watertightness, consistent winding and volume were checked on
	reimport. Neither the X=0 support nor the X=120 loaded face is preserved.
	This figure demonstrates a visualization artifact, not a usable mechanical
	support, native UI integration or post-reconstruction strength validation.

Topology document publication, 2026-10-08:

- [Computed topology, 1080x680](topology-publication-native.png): actual X11
	client at the native minimum size, after removing WM maximization. The manual
	DSL fixture is computed by the worker; Optimisation shows revision/settings,
	22 iterations, 216 g and both reanalysed cases alongside its 336-triangle mesh.
	Inspected at 1080x831 and 1080x680; no compositing or image rescaling. The audit
	window was selected by process ID because a separate biped window was open.
	This is a manual-load fixture, not a generated biped support or manufacturing
	certification. Automated tests cover undo/redo, cancellation and export guards.

Preparation and explicit per-part calculation, 2026-10-08 (supersedes automatic
publication in the previous capture):

- [Conception preparation, 1080x680](topology-preparation-native.png): battery
	fixture domain/preserve/keepout contours; no computed mesh and export disabled.
- [Selection, 1080x680](topology-selection-native.png): Optimisation remains
	non-calculated after tab selection. Exclusion hidden by an actual XTest click;
	red viewport pixels decrease from 8040 to 44 (remaining coordinate axes).
- [Explicit result, 1080x680](topology-explicit-result-native.png): manual fixture
	after clicking Lancer cette piece; calculated status, mesh and real metrics.

Native 1440x831 and compact layouts inspected. The WM ignored ordinary resizing;
override-redirect was temporarily enabled only on the audit client to obtain
1080x680, not to resize an image. A battery calculation also verified a localized
unsupported-capability failure without losing preparation. The manual fixture
was observed non-calculated before launch. Two-part independence, cancellation,
history and export guards are covered by automated tests, not claimed as a full
manual two-part walkthrough. These images do not show generated robot supports.

Biped support preparation, 2026-10-08 (iteration 10):

- [Biped regions, 1080x680](biped-support-regions-native.png): actual X11 client,
  17 purchased components and 11 incomplete support preparations. Blue domains,
  green contact regions and red exclusions; no computed support or mass result.
  Dense exclusion contours can obscure component surfaces when all are visible.
  The client was identified by PID, captured without image rescaling/compositing
  and left open. XTest menu/tab clicks produced no visible navigation; automated
  UI tests, not this capture, verify selection, undo/redo and export/launch guards.

Modular refactor, 2026-10-20 (ticket 17):

- [Design](refactor-design-20261020.png): mini-biped loaded through Exemples,
  compilation finished, 28 taxonomy instances (17 components and 11 support
  preparations). Domain/preserve/keepout overlays remain distinct.
- [Optimization](refactor-optimization-20261020.png): CameraSupport selected,
  incomplete physics reported, launch disabled and no result published.
- [Learning](refactor-learning-20261020.png): same mini-biped preview, simulation
  and training unavailable, no metrics or recorded trajectory.

All three captures are actual 1440x831 X11 clients of the rebuilt binary,
visually inspected after native menu/tab clicks. No resizing or compositing.
Eframe used the NVIDIA RTX 3060 Vulkan driver; this verifies presentation,
not CUDA computation, physical simulation, printability or RL locomotion.
The temporary verification process was closed after capture.

Julia removal, 2026-10-20 (ADR-014):

- [Unavailable minimum-mass engine](mass-backend-unavailable-20261020.png):
  rebuilt native X11 client, 1440x831, complete manual-support preparation
  changed to minimum mass. The explicit unavailable label, disabled CPU/CUDA
  selection/settings and disabled launch are visible. Only preparation outlines
  are shown; no optimized mesh or scientific metrics are claimed. Captured after
  native tab navigation and visually inspected, without resizing/compositing.
  The verification process was closed afterward.

Older Julia/CUDA screenshots above are historical and do not represent the
current backend capabilities.

Automatic servo mounts, 2026-10-20:

- [Design](servo-auto-fixations-design-20261020.png): all 14 servo instances
  automatically display their green housing interfaces (two contact rings each).
  `Fixations auto · 14 objets` is checked by default.
- [Optimization](servo-auto-fixations-optimization-20261020.png): CameraSupport
  is selected, yet the green mounts of all 14 servos remain visible. No servo
  preserve regions were manually added to this camera support.

Both are inspected native 1440x831 clients of the rebuilt application, captured
after compilation and native menu/tab clicks without resizing/compositing.
The new window was left open in Optimization for inspection. These automatic
interface guides do not add mechanical loads/supports or qualify manufacturing.
