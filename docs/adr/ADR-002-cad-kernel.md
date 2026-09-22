# ADR-002 — Kernel CAD

- Statut : accepte provisoirement pour le MVP, validation par spike requise
- Date : 2026-09-22

## Contexte

Le MVP requiert primitives, extrusion, revolution, booleens, tessellation et STL. STEP et les features avancees suivent. Le kernel ne doit contaminer ni l'IR ni le domaine, et RoboGen doit controler ses references geometriques stables.

L'etude documentaire montre que Truck fournit topologie B-Rep, NURBS, modelisation, booleens et tessellation sous Apache-2.0. BrepKit annonce une couverture fonctionnelle plus large, STEP compris, mais est jeune et distribue sous AGPL-3.0. Aucun essai executable n'a encore ete realise dans ce depot : cette ADR ne pretend donc pas valider robustesse ou performances.

## Options considerees

- Truck : Rust natif, architecture modulaire, licence Apache-2.0, historique plus etabli ; couverture et robustesse a mesurer.
- BrepKit : Rust natif, API moderne et couverture annoncee large ; maturite et licence AGPL incompatibles avec certaines distributions futures.
- OpenCascade via FFI : couverture industrielle et STEP mature ; build, ABI, packaging, `unsafe` et portabilite plus lourds.
- modeleur mesh maison : suffisant pour une demo mais pas pour un CAD parametrique B-Rep.

## Decision

Utiliser d'abord `NativeCadKernel`, un adaptateur maillé volontairement limité à
l'extrusion rectangulaire de la preuve verticale. Il établit le contrat
`CadKernel`, les unités et les sorties sans prétendre être un B-Rep de
production. Truck est le premier candidat B-Rep, sous réserve du spike
ci-dessous. Ses types resteront dans `robogen-cad::backends::truck`. Le feature
graph, les sélecteurs stables, les tolérances et les maillages de sortie sont
des types RoboGen.

Le spike doit couvrir et archiver des fixtures pour : cube, cylindre, soustraction booleenne, extrusion d'un contour, revolution, tessellation, STL, STEP si disponible et provenance des faces. Chaque cas verifie volume/bounding box, fermeture du solide, absence de panic et stabilite des references RoboGen apres changement parametrique.

STEP n'est pas un critere bloquant de la premiere tranche STL. Si le support Truck est insuffisant, STEP est marque indisponible jusqu'a un adaptateur dedie ; il n'est pas approxime silencieusement.

## Consequences

- pile pure Rust et licence permissive pour le premier backend ;
- risque reel de geometries degeneres ou de fonctions manquantes, rendu visible par le statut provisoire ;
- obligation de posseder les tests de contrat et de ne pas exposer `truck-*` hors de l'adaptateur ;
- les IDs topologiques du kernel ne sont jamais serialises ;
- BrepKit ne peut etre adopte par defaut sans decision explicite sur l'AGPL.

## Strategie de remplacement

Implementer un nouvel adaptateur `CadKernel`, puis executer les memes fixtures et comparer les invariants. Les documents persistants stockent operations, parametres et selecteurs RoboGen, pas un B-Rep serialise : les projets sont donc reconstruits avec le nouveau backend.

Echec du spike Truck : evaluer dans cet ordre (1) correction ou limitation documentee, (2) OpenCascade isole derriere FFI/service, (3) BrepKit si la politique de distribution accepte l'AGPL ou si une autre licence est obtenue. Un cache genere peut etre invalide lors du changement de backend.

## References

- <https://github.com/ricosjp/truck>
- <https://github.com/andymai/brepkit>
