---
title: "Bipede - disposition parametrique des composants"
lane: todo
created: 2026-10-07
updated: 2026-10-07
description: "Premiere disposition des composants existants, sans supports dessines ni faux resultat TopOpt."
---

## Conversation

### user

Disposer les pieces disponibles pour un mini humanoide bipede. Les supports
mecaniques doivent etre generes par optimisation topologique ; le DSL definit
les contraintes. « vas y fait le creer des ticket si besoin ».

### agent

## Plan et autorisation

La demande actuelle autorise creation des tickets et implementation de cette
premiere tranche. Ticket actif pour cette conversation. Les roles architecture,
DSL, CAD, UI et QA sont couverts sequentiellement dans cette session ; les profils
VS Code ne sont pas invoques comme des agents externes. Aucune lane existante changee.

1. Verifier la base existante par tests, sans declarer M0-M3 completes. Preserver
   les nombreuses modifications preexistantes du ticket servo encore non cloture.
2. Documenter puis ajouter une rotation rigide typee au DSL/IR/CAD. Hypothese :
   les composants existants peuvent etre disposes sans changer leur geometrie.
   Contrat : angles types, origine locale, ordre X puis Y puis Z, provenance,
   volumes/couleurs conserves et composition correcte avec translation/compound.
3. Creer examples/mini_biped, source autonome avec generateurs existants et
   instances nommees : 5 servos par jambe, 2 par bras, carte, batterie et camera.
   Les dimensions et positions sont des hypotheses editables ; les equations
   partagent ecartement, longueurs et hauteurs. Aucun support solide pre-dessine.
4. Charger depuis Exemples avec historique, compilation en arriere-plan et
   diagnostics existants ; verifier la vue native et les transformations.
5. Documenter explicitement la difference entre disposition geometrique et
   contraintes semantiques d'assemblage/TopOpt encore a implementer au ticket 15.
   Aucune marche, masse ou tenue des actionneurs validee par DisplayOnly.

## Acceptation

- Components sources reutilises sans redimensionner les objets du commerce.
- Axes des servos orientes, camera regardant devant, carte verticale et batterie
  proche du bassin ; instances gauche/droite et positions parametriques.
- Les tests couvrent rotation, ordre des transformations, fermeture/volume,
  diagnostics, positions, changement de parametre, undo/redo et affichage.
- Documentation, capture native si accessible et gates Rust consignees.
- Le ticket 15 porte contraintes, domaines et generation ; M7/M8 et le ticket
  13 sont reutilises, sans duplication de solveur ni levee de leurs dependances.

## Git

Le ticket servo 12 et ses dependances sont deja modifies/non suivis avant ce
travail ; ils restent ouverts. Ne pas les inclure dans un commit de completion
bipede. Un commit autonome ne sera possible qu'apres resolution de cette base
ou staging verifie des seuls changements coherents de ce ticket.

Checklist : [TODO](todo_20261007_14-biped-layout.md).

### user

### agent

## Verification intermediaire

Toolchain local verifie : Rust/Cargo 1.88.0. La base avant changement passe
88 tests workspace ; cela ne ferme pas les parcours natifs encore ouverts M0-M3.
ADR-011 redigee avant l'extension Rotate. Les tests cibles IR/CAD passent.

Le premier test d'enveloppes a detecte un conflit coude/hanche en roulis avec
un ecartement d'epaules de 172 mm ; passage a 190 mm. Les 17 enveloppes de la
pose par defaut sont maintenant separees. Compilation : 25968 sommets,
51660 triangles. Le test de longueur de cuisse conserve les IDs et deplace
les hanches et composants superieurs de la valeur attendue, sans deplacer
les genoux ni chevilles. Les definitions de bibliotheque sont inchangees.

Les 94 tests workspace, fmt et Clippy -D warnings passent. La premiere capture
native du chargement par le menu a revele un cadrage coupant la tete : cible
camera ajustee au milieu de la hauteur normalisee (1.5). Un test exige
maintenant toutes les faces dans le viewport avec marge a 600x400 et 320x240.
Ce test UI, fmt, Clippy et le build desktop repassent apres correction.
La capture native finale reste a inspecter avant le bilan.

### user

### agent

## Livraison de la disposition initiale

Premiere tranche realisee : [source mini_biped](../../examples/mini_biped/main.rgn),
[guide et parametres](../../examples/mini_biped/README.md),
[ADR-011](../../docs/adr/ADR-011-component-placement.md) et entree
**Exemples > Mini bipède — disposition**. Le projet contient 14 servos, carte,
batterie et camera, sans structure porteuse factice. Gabarit initial environ
190 x 69 x 362.1 mm ; les equations de position sont modifiables dans Code DSL.
La disposition ne constitue pas un solveur de contraintes d'assemblage.

La [capture native finale](../../docs/screenshots/mini-biped-native.png)
1440x831 a ete inspectee apres clics XTest sur le menu : toutes les parties,
y compris tete et chevilles, sont visibles. Aucun redimensionnement force.
L'ancienne instance de test a ete fermee ; la version corrigee reste affichee.
La fenetre RoboGen preexistante de l'utilisateur n'a pas ete modifiee.

Validation : 94 tests workspace passes, fmt --check et Clippy --workspace
--all-targets -- -D warnings passes, builds CLI/desktop passes. Apres correction
finale du cadrage, le test UI asynchrone/undo/redo/cadrage a repasse, ainsi que
fmt, Clippy et build desktop. Export CLI vers /tmp/mini-biped-layout.stl :
2583084 octets, 51660 triangles. Compilation debug de l'ordre de 13 secondes,
hors thread UI. Pas de preuve de marche ni d'efforts physiques.

Suite creee : [ticket 15](task_20261007_15-biped-constraints-topopt.md),
avec checklist pour interfaces, contraintes DSL, domaines/exclusions et
supports generes. Les tickets M7/M8 et profils 13 restent les dependances ;
aucun solveur duplique ni jalon declare termine. Les inconnues mecaniques sont
listees et ne sont pas remplacees par les valeurs DisplayOnly.

Statut Git : differences de cette tranche relues par rapport a la copie
pre-travail ; git diff --check passe. Aucun staging/commit global : la base
servo/bibliotheque du ticket 12 contient des modifications preexistantes non
committees et une acceptation encore ouverte. Les inclure violerait l'ordre
des checkpoints et attribuerait leur completion a ce ticket. Aucun push ni
changement de lane. La checklist d'implementation 14 est verifiee ; le checkpoint
Git attend la resolution de cette base. La generation des supports reste au 15.

### user

### user

commit tout

### agent

Checkpoint global explicitement autorise par cette demande : inclure toutes
les modifications courantes du depot RobotGEN, y compris les travaux
preexistants servo/bibliotheque, le bipede, les tests, la documentation,
les captures et les tickets 12 a 15. Cette autorisation leve la restriction
precedente au commit global ; elle ne declare pas les tickets incomplets
termines et ne change aucune lane.

Racine Git verifiee : /home/joanix/Documents/RobotGEN. Les toolchains,
artefacts de build, caches et logs restent exclus par .gitignore. Les huit
nouvelles captures PNG ont ete controlees pour format, dimensions et taille.
Les validations de la livraison precedente restent applicables : 94 tests
workspace, fmt, Clippy ; apres cadrage, test UI cible et gates repasses.
Aucun changement de code dans cette operation de checkpoint ; pas de
reexecution inutile des tests. Le diff est controle avant commit.

Message du checkpoint : `chore: checkpoint component library and biped layout`.
Commit local uniquement, sans push, sans changement de branche ni reecriture
d'historique. Les supports TopOpt et les acceptations ouvertes restent suivis
dans leurs tickets.

### user

### user

gère le git ignore avant

### agent

.gitignore revise avant le checkpoint global deja autorise : target/ couvre
les builds imbriques ; caches Python, exports/ et generated/, logs tournes,
profils/coverage, fichiers temporaires editeur et metadonnees OS sont exclus.
Les regles de toolchains locales, PDF telecharges, etat projet et .env sont
conservees. Cargo.lock, configuration Cargo/VS Code partagee, CI, agents,
sources, exemples, tickets et captures documentaires restent versionnables.
Les exceptions .env.example et .env.template sont preservees.

Verification executee avec git check-ignore --no-index : 28 chemins locaux
ou generes ignores et 18 chemins source/reference conserves, tous conformes.
git ls-files -ci --exclude-standard ne retourne aucun fichier : aucun retrait
de l'index necessaire. Les captures ajoutees totalisent moins de 2 Mo et chaque
PNG est inferieur a 300 Ko. Aucun fichier de build, cache ou secret local
identifie dans les chemins prepares.

La relecture du diff indexe signale seulement un espace sur une ligne vide
historique du ticket 12 (ligne 387), conserve pour respecter son historique
append-only. Aucun changement de code ni de tests requis pour cette correction
du gitignore. Le checkpoint global inclut cette correction ; sans push ni
cloture implicite des tickets ouverts.

### user
