# ADR-012 - Declarative topology as a deferred geometry operation

- Status: accepted for declaration, validation and unavailable-backend handling
- Date: 2026-10-07

## Contract and ownership

`topology(...)` is a Solid-valued, deferred geometry operation in robogen-ir.
It is not a Boolean. `TopologySpec<G>` holds domain, preserved regions, voids,
material identity, load cases, objective, constraints and manufacturing data.
The syntax uses existing calls, lists and named `:` arguments. An explicit
name qualifies OptimizationId by feature; interface/load-case names qualify
IDs by optimization. Physical values use domain-owned units, SI internally.
Source spans remain on operations, interfaces, loads, constraints and profiles.
Domain, interfaces, load vectors and manufacturing directions are in the
topology node's local frame. Outer transforms place the eventual result;
pose-dependent loads must be declared/resolved separately, not inferred.
No kernel faces, tensors, physics handles or widget state enter this model.

The spec schema version is 1. It is derived from .rgn, not a new project file
format. No previous TopologySpec was persisted, so no migration is needed.
A compiler without this feature diagnoses an unknown constructor. Old derived
IR must be discarded/recompiled; future backend cache keys must include source
revision/hash and producer/schema version. Original .rgn files remain the
rollback path. No computed mesh/density is written into source declarations.

CAD only builds ordinary geometry. It must reject an unresolved Topology node,
including one nested inside transforms, compounds or Booleans. Project currently
owns build orchestration (documented in OVERVIEW); it now also calls the TopOpt
port with the versioned spec, material and document revision. TopOpt depends on
IR/domain, never CAD/UI. IR never imports a backend. This deliberate dependency
addition adds no external library; a future extracted orchestration crate must
move both CAD and TopOpt dispatch together.

## Syntax and semantics

All topology arguments are required, including explicit empty lists where
appropriate: name, domain, preserve, void, material, load_cases, objective,
constraints, manufacturing. Preserve accepts ordinary Solid expressions and
`interface(name: "mount", shape: solid)` for regions addressable by loads.
Force/moment/fixed/pin/frictionless targets reference a named preserved region,
not a transient triangulated face. Surface selection and load distribution on
the region remain backend responsibilities and are not claimed solved here.
No automatic clearance or preservation thickness is invented.

Geometry expressions compose using existing constructors. Components can bind
these solids locally and reuse them in domain/preserve/void. A resulting
Topology node can be nested under difference, union, transforms and compounds.
Future fillet/hull/offset/sweep/drill are not silently emulated. Geometry
invariants P subset G subset domain, and G disjoint void, are mandatory backend
publication checks, including after downstream operations and reconstruction.
The semantic check does not prove those geometric or mechanical invariants.

Each named load case has at least one nonzero force or moment and an explicit
support. Every case must be evaluated; none is averaged away. Valid material
elastic constants are required, but input values are not experimental evidence.
Objectives (minimize_mass / maximize_stiffness) are separate from positive
limits (displacement, stress, safety factor, mass, first frequency) and a
volume fraction in (0,1]. A manufacturing list contains at most one selected
process profile: FDM or CNC_3AXIS, with typed dimensions and a unit direction.
Unknown options, duplicate names/limits, invalid units/ranges and unresolved
interfaces produce localized errors. Lists/geometry expansion are bounded.

## Honest execution boundary

M7 is not implemented by this decision. The current TopOpt port returns
Unavailable for every valid request and never returns the domain as a result.
Project reports that failure with the topology call span; stale preview data
may remain visible under the existing diagnostic UI, but export stays disabled.
Background compilation, cancellation and document revision guards still apply.
`inspect-topology` validates declarations without meshing and reports counts
and backend unavailability; regular check/export still require real geometry.

Solver implementation, voxel masks, geometric conflict checks, load mapping,
material provenance, capability checks per objective/constraint/process,
convergence, reconstruction, post-operation revalidation and generated-result
persistence remain explicit M7/M8 and ticket 13/15 work. A numerical run must
reject unsupported constraints rather than merely accepting these declarations.
No Fusion service integration, remote upload or numerical result is implied.

The current compilation revision is worker-scoped; synchronous/standalone
builds use 0. This is not a globally unique cache key. TopologyRequest replaces
the unused target-name/volume-fraction placeholder; its only existing adapter
remains unavailable. The old result DTO is not a publication contract for M7.
