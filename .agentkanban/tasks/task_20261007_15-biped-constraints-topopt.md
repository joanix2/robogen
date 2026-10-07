---
title: "Bipede - contraintes DSL et supports generes"
lane: todo
created: 2026-10-07
updated: 2026-10-07
description: "Interfaces, regions admissibles/exclues et supports derives par FEM/SIMP."
---

## Conversation

### user

Disposer les pieces disponibles pour un mini humanoide bipede. Les supports
mecaniques doivent etre generes par optimisation topologique ; le DSL definit
les contraintes. « vas y fait le creer des ticket si besoin ».

### agent

## Dependances et ordre

Suite demandee du ticket [14](task_20261007_14-biped-layout.md).
Le travail de conception est autorise. L'implementation du solveur reste dans
[M7](task_20260922_07_m7-topopt-beam.md), apres ses dependances M0-M6 ;
les charges robotiques et revalidations dans [M8](task_20260922_08_m8-robot-topopt.md).
Reutiliser le ticket [13](task_20261007_13-topopt-print-profile.md) pour les
profils parametrables. La demande de bipede n'atteste aucun jalon valide et
ne debloque pas implicitement le developpement FEM avant M7.

## Contrat cible

Le .rgn exprime le cahier des charges de chaque support : composants references
par IDs, liaisons/axes, reperes, interfaces preservees, domaine ou la matiere
est autorisee, volumes interdits (composants, debattements, cables, outils),
charges/appuis, materiau et objectif. Il ne prescrit pas la forme finale.
Les sources de bibliotheque conservent evidemment la geometrie des composants
achetes. Une contrainte inconnue/non supportee doit produire un diagnostic ;
aucune declaration TopOpt ignoree ni bloc fictif presente comme executable.

Les artefacts generes restent hors DSL, indexes par revision/hash/version du
solveur et accompagnés d'hypotheses, convergence et unites. L'application au
robot est explicite et annulable. Unknown reste distinct de zero pour masses,
couples, rigidite, materiaux et profils de fabrication.

## Plan

1. ADR de contrats persistants DSL -> SemanticModel -> Assembly/RobotModel ->
   OptimizationSpec ; IDs/types/provenance/diagnostics et migration documentes.
2. Interfaces fonctionnelles des composants verifies : points de fixation du
   boitier servo et interface de sortie sont distincts ; palonniers/vis/roulements
   non fournis restent inconnus. Ne pas deduire une interface de la seule image.
3. Regions et exclusions parametrables pour bassin, cuisses, tibias, pieds,
   torse et bras ; separation aux articulations. Gerer collisions de poses et
   enveloppes de mouvement, symetrie, entraxes, debattements et courses.
4. Premiere piece de reference : support camera, charges manuelles documentees,
   puis support de jambe multi-cas et ensemble. Pas de masses DisplayOnly pour FEM.
5. Brancher M7/M8 et profils du ticket 13 ; calcul hors UI avec progression,
   annulation et rejet des revisions obsoletes. Optimiser/reconstruire/reverifier.
6. Validation geometrique, numerique et fabrication ; temps d'impression issu
   du trancheur, aucun temps invente. Validation marche dans M5/robotique a part.

## Acceptation

Edition d'une contrainte -> diagnostic ou calcul reel -> support genere ->
verification des interfaces et exclusions -> affichage/export -> undo.
Tests des unites, references ambigues/manquantes, conflits, bornes, preservation
des interfaces, contraintes supportees, revisions, annulation et persistance.
Benchmarks FEM/SIMP passes avant toute affirmation de rigidite ; aucune promesse
de resistance/couple moteur ou de marche tant que les donnees restent inconnues.

## Donnees encore a obtenir avant calcul mecanique

Masse et centre de masse des composants, reference/couple/courses des servos,
interfaces des palonniers, charges de reference, proprietes du PLA imprime,
hauteur de couche/parois/orientation et volume utile de la Finder. Les valeurs
connues du profil utilisateur restent celles du ticket 13. Ces inconnues ne
bloquent pas la disposition initiale du ticket 14.

Checklist : [TODO](todo_20261007_15-biped-constraints-topopt.md).

### user

### user

Il faudrait implementer le texte joint : une operation topology(domain,
preserve, void, load_cases, material, objective, constraints, manufacturing)
composable avec les autres operations geometriques du DSL.

### agent

## Iteration 2 - Operateur declaratif composable autorise

Cette demande autorise l'implementation du contrat DSL/IR et de son parcours
jusqu'au diagnostic de backend. Le ticket 15 devient actif ; pas de changement
de lane. Le calcul FEM/SIMP reste dans M7, dont les prerequisites ne sont pas
valides par cette demande. Le ticket global ne sera pas marque termine sans
supports effectivement generes et revalides.

Hypothese : l'AST d'appels et listes existant permet topology() sans ajouter
un second langage. Les composants Solid existants fournissent domain/preserve/
void ; les wrappers habituels peuvent consommer son resultat differe.
Premier discriminant : compilation semantique d'un support de batterie puis
refus explicite de meshing/export avec provenance, sans retourner son domaine
comme faux support optimise.

Plan : ADR-012 avant les contrats ; quantites Force/Torque/Mass/Frequency,
OptimizationSpec versionne et IDs nommes ; lowering contextuel de topology,
interfaces preservees, cas de charge, objectifs, limites et profils FDM/CNC ;
validation structurelle/unites/references/bornes et contraintes inconnues ;
raccordement du port TopOpt indisponible, exemple executable au niveau
semantique, inspection CLI et edition UI annulable ; tests de composition,
diagnostics, historique et revisions, documentation et gates Rust.

Les interfaces sont des regions nommees explicites : aucune epaisseur de
preservation implicite de 3 mm, ni selection de face par indice kernel.
Les inclusions/intersections geometriques et verifications mecaniques seront
requises du backend avant publication ; la validation du DSL ne les certifie
pas. Les exemples du texte utilisant offset/sweep/fillet/hull/drill ou |>
ne deviennent pas disponibles par cette tranche et seront documentes comme
non pris en charge. Aucun objectif ou contrainte reconnu ne sera presente
comme applique par le backend absent.

### user

continue

### agent

## Iteration 2 - Contrat livre et verifie

Reprise des modifications existantes. Le premier controle cargo check a
detecte les branches CAD manquantes pour SolidOperation::Topology ; elles
refusent maintenant explicitement le noeud differe sous les wrappers CAD.

TopologySpec schema 1, IDs d'optimisation/interfaces/cas de charge, unites
Force/Torque/Mass/Frequency et lowering contextuel sont raccordes. Validation
des regions nommees, charges non nulles, supports, objectifs, six limites,
profils FDM/CNC, references, doublons, unites et bornes. Les contraintes
reconnues ne sont pas declarees appliquees : aucun backend numerique n'existe.

Project appelle le port TopOpt avec spec, materiau et revision de compilation
(revision du worker ; 0 pour compilation synchrone autonome). Diagnostic E330
sur l'appel topology, aucun domaine renvoye comme faux support. Historique,
annulation et rejet des revisions obsoletes conservent le dernier maillage
valide pour affichage seulement ; export bloque. Le compteur de revision
local n'est pas une identite globale de cache.

Exemple examples/topology_battery_support : domaine et interfaces explicites,
exclusion batterie, charge et PLA illustratifs. CLI inspect-topology inspecte
sans mailler : 1 operation, 2 regions preservees, 1 exclusion, 1 cas de charge,
6 contraintes et 1 profil ; message solver unavailable. Check/export refusent.
Menu Exemples > Support batterie (contraintes TopOpt) ouvre le DSL ; chargement
et undo/redo testes. README, TECHNICAL, docs DSL, architecture et ADR-012 alignes.

Validation : 103 tests workspace passes, cargo fmt --check et cargo clippy
--workspace --all-targets -- -D warnings passes, builds CLI/desktop passes,
inspection CLI reelle et git diff --check passes. Tests cibles : 8 compositions,
27 declarations invalides, cas de charge multiples/bornes, IDs qualifies,
refus CAD, historique/asynchronisme/annulation, CLI et UI. Deux attentes de
test initiales ont ete corrigees : rotate prend x/y/z/shape, division par zero
renvoie E235. Aucun contournement du comportement existant.

Verification native X11 par clics sur le menu : capture
docs/screenshots/topology-declaration-native.png, client reel 1440x831 inspecte.
DSL ouvert, E330 a 23:12 visible, ancienne taxonomie marquee resultat precedent,
export et ajouts desactives. Cette capture ne montre aucun resultat TopOpt.

Iteration 2 terminee, ticket global encore ouvert : FEM/SIMP, generation et
revalidation des supports, validation geometrique des regions, profils calibres
et donnees mecaniques restent M7/M8/ticket 13 et iteration 1. Aucun commit ni
changement de lane effectue dans cette reprise.

### user

2026-10-08 : commit et continue c'est ton objectif.

### agent

Checkpoint local explicitement demande pour le contrat de l'iteration 2,
sans cloture du ticket global ni push. Racine Git verifiee : RobotGEN.
Revalidation au 2026-10-08 : 103 tests workspace passes, fmt --check,
Clippy all-targets -D warnings et diff --check propres. Le perimetre du commit
est le contrat topology, ses tests, exemples, documentation et suivi.

Suite dans l'iteration 1 : verifier les dependances M0-M7 sur leurs criteres
et preuves, sans assimiler les 103 tests a une validation de tous les jalons.
Le solveur reel reste conditionne par M7 et ses prerequisites ; les donnees
mecaniques du robot ne seront pas inventees pour contourner ce verrou.

### user
