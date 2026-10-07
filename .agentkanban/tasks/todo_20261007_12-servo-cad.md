---
title: "Checklist - Servomoteur CAD"
lane: todo
created: 2026-10-07
updated: 2026-10-07
description: "Geometrie, apparences et parcours natif du servomoteur."
---

Tache : [Servomoteur CAD](task_20261007_12-servo-cad.md).

# Iteration 1

- [x] Reparer les booleens et verifier volumes, orientation et fermeture (6 tests CAD passes).
- [x] Creer le modele servo_main(jeu)/servo avec les dimensions de reference (2870 triangles, bounds et fermeture verifies).
- [x] Tester la tranche source, composants, jeu, historique, mesh et STL (tests verticaux passes, jeu 0 -> 0.2 mm).
- [x] Documenter et implementer les apparences source vers apercu (ADR-010, 3 couleurs testees).
- [x] Rendre l'exemple accessible dans le desktop et mesurer le rebuild (environ 0.8 s debug ; worker annulation/revisions teste).
- [ ] Confirmer manuellement le parcours natif Exemples -> edition JEU -> camera -> export (clics synthetiques X11 non acceptes ; contrats automatises passes).
- [x] Actualiser la documentation et executer fmt, Clippy et tests workspace (69 tests passes, Clippy -D warnings et fmt propres).

# Verification visuelle

- [x] Capturer et inspecter le rendu natif 1440x831 et le client compact 1080x680 ; corriger les traits parasites des triangles fins.
- [x] Documenter la taille compacte forcee au niveau du client et les limites de l'automatisation native.

# Iteration 2 - Carte embarquee

- [x] Modeliser la carte 104 x 90 x 37 mm et ses composants visuels (compilation et fermeture validees).
- [x] Ajouter l'exemple au menu et tester dimensions, couleurs et STL (hauteurs 37 et 42 mm, fermeture, export et undo verifies).
- [x] Verifier le rendu natif et documenter les approximations (capture 1440x831 inspectee).
- [x] Executer les gates Rust et consigner le resultat (72 tests passes, fmt et Clippy propres).

# Iteration 3 - Occultations de l'apercu

- [x] Remplacer le tri moyen par un tampon de profondeur par pixel (triangles croises testes dans les deux ordres).
- [x] Raccorder tous les apercus UI et tester le cache camera/mesh/taille.
- [x] Tester clipping proche, entrees invalides et ventilateur devant PCB aux resolutions 600x400 et 320x240.
- [x] Inspecter la capture native 1440x831 apres rotation XTest (compute-board-depth.png, occultations corrigees).
- [x] Documenter les limites CPU et verifier fmt, Clippy -D warnings, 76 tests workspace et build desktop.

La capture de l'iteration 2 ne prouvait pas les occultations correctes ; la
preuve corrigee est celle de l'iteration 3. Le parcours servo manuel de
l'iteration 1 reste ouvert.

# Iteration 4 - Camera Raspberry Pi

- [x] Creer le generateur camera et son instance d'apres le plan de 2013 ; compiler via CLI (1773 sommets, 3558 triangles).
- [x] Verifier cotes, percages ouverts, fermeture, couleurs, parametrage et STL (diametres 2 et 3 mm).
- [x] Ajouter l'exemple au menu natif et tester chargement, export et undo (test asynchrone et objectif visible a deux resolutions).
- [x] Capturer le rendu natif et documenter les approximations (camera-native.png, 1440x831 inspectee).
- [x] Executer fmt, Clippy et tests workspace ; consigner les preuves (78 tests, build desktop et export CLI passes).

# Iteration 5 - Taxonomie et bibliotheque DSL

- [x] Creer les trois generateurs DSL reutilisables et leur insertion annulable sans remplacer le document (tests projet et recompilation autonome).
- [x] Conserver la carte multicorps via compound explicite, borne et teste, sans modifier union (positions et nombre de triangles identiques).
- [x] Afficher a gauche les instances generees en haut et les fonctions en bas ; assistant a droite (correction utilisateur, tests 1440x831 et 1080x680).
- [x] Relier selections, recherche, etats en cours/erreur et navigation source aux donnees reelles (17 tests UI passes).
- [x] Tester plusieurs instances, undo/redo, erreurs et integration ; verifier le rendu natif et les gates Rust (85 tests workspace, 17 UI apres correction gauche/droite, trois ajouts natifs et capture 1440x680).

# Iteration 6 - Batterie Li-ion OEM

- [x] Creer le pack jaune nominal 18 x 68 mm avec fils et connecteur parametriques, sans confondre cellule nue et pack (test de cotes et variations passe).
- [x] Tester dimensions, couleurs, geometrie fermee, export et variations/diagnostics (tests cibles et STL passes).
- [x] Ajouter la fonction a la bibliotheque et l'exemple au menu ; verifier insertion, historique et chargement (tests UI et insertion des quatre generateurs passes).
- [x] Documenter les sources fournies et estimations, inspecter l'apercu natif (battery-native.png, 1440x831).
- [x] Executer les gates Rust et consigner les resultats sans cloturer le parcours manuel servo restant (88 tests, fmt, Clippy et builds passes).
