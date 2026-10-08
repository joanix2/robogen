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

## Projection CPU et execution explicite (iteration 7, 2026-10-08)

Un projecteur TopOpt lit SolidGeometry sans creer de nouveau modele persistant.
Il classe les cellules par intervalles/bornes analytiques sur CSG : inside,
outside, boundary. Le domaine retient uniquement des cellules entierement dedans ;
les keepouts excluent toute cellule d'interieur intersectant potentiellement le
vide ; preserve impose toute cellule intersectant potentiellement P. Si ces
regles entrent en conflit, ou P sort du domaine couvert, la resolution/requete
est refusee. Une classification conservative peut refuser une forme faisable ;
elle ne peut pas autoriser un empietement inconnu. Les frontieres sont traitees
comme celles de solides regularises (contacts de surface sans volume commun).
La fraction de volume se refere au domaine discret interieur, avant keepouts ;
les volumes physiques effectifs et la resolution restent exposes. Les transformations
et CSG sont analyses recursivement avec budgets ; topology imbrique non resolu
reste indisponible. Aucun simple echantillonnage au centre ne certifie les regions.

Les interfaces existantes sont des volumes, pas des faces. Pour ce backend, fixed
bloque tous les noeuds des cellules d'interface preservees. Une force totale est
repartie uniformement dans ce volume voxelise, avec poids nodaux Hex8 constants
(1/8 par coin par cellule). Le centre d'application est donc le centroide de ce
volume discret ; aucune face de montage n'est devinee. Le contrat expose ces
hypotheses et conserve l'ID de l'interface. Un moment peut se projeter en couple
de forces a resultant nul via le tenseur geometrique des noeuds ponderes.
Frictionless cartesien bloque sa seule direction normale ; pin et normale
oblique sont refuses tant qu'un adaptateur approprie n'existe pas.

Execution explicite : TopologyRequest + reglages numeriques hors DSL -> requete
Grid -> optimisation -> reconstruction binaire -> reanalyse tous cas -> controles
budget et deplacement. MaximizeStiffness minimise la somme des compliances ;
minimize_mass, stress/securite/frequence et fabrication non implementes sont
refuses avec provenance, non assimiles a des capacites presentes. Ils restent
au backlog de l'objectif global. Le resultat conserve source/revision/material,
origine de grille, analyses et historique. La masse utilise la densite declaree.

La CLI compose compilation semantique, TopOpt et exporter existant : export de
l'operation choisie dans son repere local, avec origine de grille restituee.
Elle ne publie pas encore un document aux Booleens/transforms aval et ne retire
pas le garde E330 du chemin Project/desktop. Les options cellules/filtre/seuil
sont des reglages d'execution explicites, pas une description manuelle du support.
Les deps TopOpt/domain sont ajoutees a l'application CLI, pas entre adaptateurs.

Les translations communes sont combinees puis soustraites symboliquement avant
classification : comparaisons dans le repere local de grille, origine restituee
a la sortie. Une regression a detecte qu'operer directement sur des coordonnees
globales [1,2,3] m creait un faux conflit sur une geometrie de 120 mm ; cette
normalisation le corrige sans elargir les regions avec une tolerance arbitraire.
Le test conserve les masques, poids/centres d'interface et dimensions lors du
deplacement commun. Les predicates restent en virgule flottante, non certifies.

## Publication dans le document (iteration 8, 2026-10-08)

Cette section remplace le constat d'indisponibilite globale de Project ci-dessus.
Project compose l'execution TopOpt et CAD sans modifier le modele semantique.
ResolvedTopologyKernel consomme une table revision-locale de maillages RoboGen
indexes par OptimizationId. Aucun type de solveur ne franchit la frontiere CAD.
Translation, rotation, couleur et compound sont autorises autour des resultats ;
les booleens aval restent refuses faute de revalidation mecanique. Les charges
restent locales : un placement ne transforme pas celles-ci en gravite globale.

ProjectSnapshot publie maillages et TopologyReport ensemble : nom/ID, span,
materiau, revision, origine, reglages, backend, iterations, masse et reanalyse de
chaque cas (compliance, deplacement maximal, residu). Le worker conserve garde
de revision et annulation ; aucune mutation du source ni cache persistant.
Les rapports precedents sont marques obsoletes lorsque le document est invalide
ou en calcul, export desactive. CAD valide indices/coordonnees, mais ne certifie
pas la mecanique d'un maillage fourni par un autre appelant.

Le chemin document utilise le prereglage de la fixture manuelle : cellules
[12,4,2], filtre 12 mm, seuil 0.65, limite 100 iterations. Ces valeurs sont
exposees dans le rapport natif, pas adaptees silencieusement pour obtenir une
forme. Seule la CLI/API permet actuellement d'autres reglages. Ce prereglage ne
convient pas aux fixations millimetriques sans etude de resolution. Les capacites
absentes restent refusees : E330 encapsule le diagnostic TopOpt localise.

## Preparation puis calcul explicite (iteration 9)

Decision utilisateur du 2026-10-08 : la compilation ne lance plus TopOpt.
CAD projette separement les volumes domaine/preserve/keepout, sans attribution
de proprietes mecaniques. Ces apercus ne rejoignent jamais les meshes exportables.
Project orchestre une commande explicite par PartId dans le worker existant.
Les resultats des autres pieces sont conserves. Une edition de source invalide
conservativement tous les resultats TopOpt sans relancer le solveur ; le worker
rejette les travaux obsoletes. Les reglages numeriques sont propres au lancement.
La preparation geometrique n'implique pas que le backend accepte les contraintes.

## Preparation incomplete du bipede (iteration 10)

`topology(..., material: unknown, load_cases: [], constraints: [], ...)` declare
explicitement une preparation sans donnees mecaniques inventees. Dans ce mode,
preserve peut aussi etre vide lorsque les interfaces restent a mesurer. Les
geometries sont affichees par le chemin de preparation existant ; aucune n'est
un resultat calculable ou exportable. Les autres arguments restent requis et
les declarations effectivement fournies restent validees. Une faute de nom de
materiau reste une erreur, seul le mot reserve `unknown` designe cette absence.

TopologySpec schema 2 porte un Option<MaterialId>. Les sources existantes restent
compatibles ; recompiler les anciens IR/caches derives depuis .rgn. Un materiau
de presentation sur Part ne remplit jamais ce champ implicitement. Toute requete
incomplete est refusee avant le solveur avec provenance et donnees manquantes.
Le backend disponible ne gagne aucune capacite minimize_mass par cette extension.
