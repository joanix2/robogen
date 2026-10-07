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

### agent

Commit cree : ee470fd, feat(topology): add validated deferred DSL operation.
31 fichiers ; aucune publication distante. Worktree propre apres ce commit.

Poursuite du prealable de l'iteration 1 : M0 possede des preuves de validation,
M1-M6 ne sont pas clos. Les avancees DSL/CAD des tickets supplementaires ne
valent pas validation retroactive. Manques confirmes dans les implementations :
SketchEntity ne contient que Rectangle ; PhysicsBackend utilise exclusivement
UnavailablePhysics ; RobotModel reste un squelette sans projection dynamique.
M7 depend formellement de M6 et la simulation de M5 n'est pas implementee.

Preuves natives M1 completees pendant cet audit : navigation vers Optimisation
et Apprentissage par clics XTest, captures reelles 1440x831 inspectees dans
docs/screenshots/m1-optimisation-20261008.png et
docs/screenshots/m1-apprentissage-20261008.png. Solveur/entrainement indisponibles,
metriques et trajectoires vides ; parametres indicatifs desactives. Les controles
Optimisation ne sont pas une projection du TopologySpec. Pas de revendication
de comparaison complete a la maquette ni de controle compact de ces deux vues.

Decision de routage necessaire avant changement de perimetre : respecter la
chaine M1-M6, ou autoriser explicitement une premiere tranche M7 autonome sur
poutre a charges manuelles. Cette derniere ne requiert techniquement pas une
simulation robotique, mais derogerait a l'ordre documentaire actuel. Le robot
reste soumis aux validations M5/M8 et a ses donnees physiques inconnues.

### user

2026-10-08 : Prioriser M7 autonome. Autorisation explicite du changement
d'ordre : developper et tester le solveur sur une poutre ; le robot et ses
charges restent pour M5/M8.

### agent

Iteration 3 autorisee, plan/todo/implementation dans ce ticket : premiere
tranche scientifique M7 a charges manuelles, sans cloturer M1-M6. Exception
limitee a la poutre ; PPO/LLM et optimisation du robot restent hors perimetre.

Plan : adaptateur sparse faer isole avec solution connue et singularite ;
elements hexaedriques lineaires a integration Gauss 2x2x2, encastrement et
force repartie sur la face libre ; validation patch/energie/residu/flexion ;
SIMP avec filtre de densite et mise a jour OC, sensibilites par differences
finies et comparaison a densite uniforme de meme volume ; exemple executable
avec historique reel, progression/annulation et provenance. La publication
de geometrie topology() arbitraire reste bloquee tant que toutes ses contraintes
ne sont pas supportees et revalidees. Aucun support robot certifie implicitement.

### user

### agent

## Iteration 3 - Tranche scientifique M7 verifiee

Implementation reelle dans robogen-topopt : port SparseLinearSolver, adaptateur
faer 0.22.6 LLT prive, Hex8 a huit points de Gauss et encastrement/charge de face,
SIMP penalise, filtre de densite avec derivees transposees, OC et historique
d'etats resolus. Entrees Length/Pressure/Force typees, compliance en joules ;
la requete complete, sa revision et l'identifiant du solveur accompagnent le
resultat. Limites de ressources et diagnostics, progression et annulation
cooperative. La factorisation directe reste non interruptible en son milieu.

L'exemple cargo run -p robogen-topopt --example beam calcule effectivement
120x40x20 mm sur 12x4x2 cellules, E=70 GPa, nu=.3, force totale -1 N en Z,
volume .35. Arret DesignChange a l'iteration 65 : variation .004620056,
residu 6.616003e-13, compliance 2.377550262401e-6 J contre 6.271263227758e-6 J
pour une densite uniforme au meme volume (ratio .379118). Mesure locale debug
20.932 s hors compilation, pas une garantie de performance. Le run initial
limite a 60 iterations indiquait correctement IterationLimit. La convergence
du critere de variation ne prouve pas un optimum global.

Verification : 10 tests unitaires et 1 integration ajoutes (solution sparse,
singularite, modes rigides, energie multiaxiale, traction analytique, raffinement
en flexion, sensibilites en differences finies, volume, annulation, reanalyse
finale et mise a l'echelle de charge). 114 tests workspace passes ; fmt --check,
Clippy workspace all-targets -D warnings et diagnostics editeur propres.
Documentation benchmark, architecture, ADR-008, TECHNICAL et suivi alignes.

Checkpoint local de cette tranche validee sous le message
feat(topopt): add standalone cantilever FEM and SIMP ; aucun push, aucun
changement de lane ni cloture du ticket 15 ou du jalon M7 entier.

Suite ouverte : reconstruction de surface verifiee, publication desktop hors
thread UI/revisions, raccordement des capacites TopologySpec et revalidation
des contraintes. Le chemin topology() reste E330 ; aucun champ de densites
n'est presente comme piece exportable. M5/M8, interfaces/charges/materiau et
profils de fabrication restent requis avant les supports robotiques.

### user
