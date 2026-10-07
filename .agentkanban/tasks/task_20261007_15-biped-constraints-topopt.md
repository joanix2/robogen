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
