---
title: "Checklist - refonte modulaire autonome"
lane: todo
created: 2026-10-20T14:35:47.179987+00:00
updated: 2026-10-20T14:35:47.179987+00:00
description: "Audit, frontières, extraction des responsabilités et validation complète."
---

Tâche : [ticket 17](task_20261020_17-scalable-architecture.md).

# Itération 1 - Direction et référence

- [x] Lire les standards, le workflow, la mémoire, l'architecture et le graphe existant.
- [x] Résoudre la direction Rust/Julia ou migration Python.
- [x] Valider la baseline des tests workspace et consigner les tests ignorés (cargo test --workspace --offline, exit 0 ; Julia/CUDA non qualifiés).
- [x] Exécuter une session d'audit Oh My OpenAgent et consigner son identifiant.
- [x] Fixer les frontières et critères d'acceptation dans une ADR.

# Itération 2 - Refactorisation à contrats constants

- [x] Extraire les responsabilités de Project/document sans changer les API publiques.
- [x] Découper l'UI par fonctionnalités et isoler ses tests/supports.
- [x] Séparer les diagnostics, le protocole et le processus TopOpt masse.
- [x] Tester chaque chemin touché et revoir les diffs combinés.

# Itération 3 - Intégration et livraison

- [x] Mettre à jour standards, architecture et TECHNICAL.md avec l'état réel.
- [x] Exécuter fmt, Clippy workspace/all-targets/-D warnings et tests workspace.
- [x] Vérifier le parcours desktop concerné et les limites scientifiques conservées.
- [x] Actualiser l'index Graphify et vérifier les dépendances publiques.
- [ ] Consigner les preuves, limites et sessions ; revoir le diff et créer le commit local.

# Itération 1 bis - Réflexion demandée avant refonte

Les itérations d'implémentation ci-dessus restent en attente de la décision.

- [x] Consigner la demande de réflexion préalable et l'exclusion de Julia dans la cible.
- [x] Choisir le parcours MVP prioritaire et ses critères d'acceptation.
- [x] Confirmer Rust hôte avec un backend scientifique Python éventuel, ou autre direction.
- [ ] Définir la transition hors Julia et qualifier un seul backend sur un petit cas réel.
- [x] Réviser le plan de sessions autonomes après cette décision.

# Itération 4 - Preuves de la tranche structurelle

- [x] Décision ADR-013 : Rust hôte ; Python scientifique à qualifier, Julia exclu de la cible.
- [x] Tenter OMO avec consentement DeepSeek et consigner la session : échec fournisseur Insufficient Balance, aucun audit réalisé.
- [x] Réaliser le fallback natif Project/UI/masse, à fichiers disjoints et API constantes.
- [x] Vérifier les suites ciblées et la conservation des fonctions/tests ; ajout de quatre tests de frontières.
- [x] Contrôle architecture : 21 crates/36 arêtes ; Ruff et actionlint passent.
- [x] Graphify local code/Cargo actualisé : 1490 nœuds/3478 arêtes/123 communautés, sans LLM.

La qualification du backend Python, le retrait fonctionnel de Julia et le MVP
humanoïde DSL/TopOpt/RL restent des travaux suivants, non livrés par cette tranche.
L'audit OMO n'est pas coché comme réussi : le fournisseur a refusé sa première
requête malgré le consentement. Aucun commit d'un ticket partiel n'est effectué.

- [x] Gates combinées : fmt, Clippy, 161 tests réussis/8 ignorés, builds desktop/CLI ; quatre tests Python, Ruff, actionlint.

- [x] Vérifier en natif X11 le mini-bipède chargé via Exemples, puis les trois vues à 1440x831 ; captures inspectées après compilation terminée.
- [x] Consigner preuves et limites dans le ticket, TECHNICAL.md, architecture, mémoire et documentation des captures.
- [ ] Terminer les acceptations globales ouvertes avant commit : audit OMO effectif et qualification du remplacement de Julia.

# Itération 5 - Retrait Julia autorisé et reprise OpenCode

- [x] Relancer l'audit OpenCode/OMO avec DeepSeek et consigner les sessions.
- [x] Documenter ADR-014 : retrait immédiat et minimum de masse indisponible.
- [x] Supprimer sources/projets Julia, lancement natif et dépendances exclusives.
- [x] Préserver DTO neutres, compilation DSL et solveur Rust de rigidité.
- [x] Rendre le refus explicite dans CLI/worker/UI, sans résultat ni export.
- [x] Remplacer les qualifications Julia supprimées par des tests de refus unitaires/intégration/UI.
- [x] Revue OpenCode, gates workspace, preuve native et actualisation Graphify/docs/mémoire.

ADR-014 remplace l'attente d'un moteur qualifié avant retrait : l'utilisateur
autorise la suppression immédiate avec calcul minimum de masse indisponible.
La qualification Python et le MVP intégré demeurent des travaux futurs.

# Itération 5 - Validation finale

- [x] Audit et revue OMO/DeepSeek réussis ; aucun point bloquant en revue finale.
- [x] 163 tests Rust réussis, zéro ignoré ; fmt, Clippy workspace, builds CLI/desktop.
- [x] Quatre tests Python, frontières 21/36, Ruff et actionlint passent.
- [x] CLI E340 indisponible avec span, aucun STL créé.
- [x] Capture native 1440x831 inspectée : préparation complète, minimum de masse et contrôles CPU/CUDA désactivés.
- [x] Graphify local actualisé : 1475 nœuds/3374 arêtes/139 communautés.

La suppression demandée est terminée. La qualification d'un moteur Python
minimum de masse et le MVP humanoïde intégré ne sont pas revendiqués ; le
ticket global conserve ces travaux ouverts, sans commit partiel ni changement
de lane. Les anciens tests supprimés dépendaient exclusivement du moteur retiré,
et ne sont pas comptés comme des qualifications réussies.

# Itération 6 - Checkpoint explicitement demandé

- [x] Recevoir l'autorisation explicite de checkpoint de la tranche validée.
- [x] Revoir le diff et les fichiers ; conserver les objectifs produit ouverts.

Le checkpoint porte sur la refonte et le retrait Julia déjà validés. Il ne
revendique pas l'achèvement du MVP ni la qualification du remplacement Python.

# Itération 7 - Zones rattachées aux instances

- [x] Écrire ADR-015 avant extension DSL/IR.
- [x] Ajouter Object, Instance, identité et projections de régions avec provenance.
- [x] Valider diagnostics/types/poses/IDs et compatibilité des sources Solid.
- [x] Raccorder Project et migrer les servos du mini-bipède sans poses de régions dupliquées.
- [x] Tester une pièce sélectionnant ses seules interfaces, rebuild et historique.
- [x] Exécuter les gates workspace (fmt, Clippy, 173 tests, Python, architecture, Ruff).
- [ ] Inspection native et checkpoint validé (géométrie identique prouvée par test bipède + CLI ; commit sur autorisation).

La tranche DSL/IR/domain est livrée : `Object`/`instance`, projections
`instance_shape`/`instance_void`/`instance_interface`, `ObjectInstanceId` typé
et stable, provenance `instance_references`, paramètres de composant de type
`Instance`. Le mini-bipède migré compile à l'identique (25968 sommets/51660
triangles, 28 parts/17 maillages/11 préparations). Dix tests d'instances ajoutés.
Géométrie inchangée : l'inspection native n'apporterait aucun changement visuel
hors liste de fonctions de la bibliothèque ; le commit reste en attente
d'autorisation explicite.

# Itération 8 - Interfaces vertes communes aux servos

- [x] Confirmer que l’interface verte doit apparaître sur les 14 servos, quelle que soit la préparation sélectionnée.
- [x] Séparer l’affichage visuel de l’interface des régions TopOpt propres à chaque pièce ; ne pas ajouter d’appuis/charges implicites.
- [x] Projeter l’interface `housing` de chaque instance servo vers un overlay placé selon sa pose.
- [x] Ajouter un contrôle et une légende qui distinguent l’interface servo des couches « Conserver » et « Exclure ».
- [x] Tester la présence des 14 interfaces et leur géométrie commune après prise en compte des poses ; vérifier que les TopologySpec restent inchangés.
- [x] Vérifier le parcours natif du mini-bipède et documenter le comportement dans TECHNICAL.md.

# Itération 8 bis - Vert automatique demandé

- [x] Constater les 14 interfaces projetées et la couche cyan non conforme au vert attendu.
- [x] Afficher automatiquement toutes les fixations en vert, avec légende et nombre d'objets.
- [x] Vérifier sans TopOpt, avec les 14 servos et en changeant de support ; aucune modification des contraintes mécaniques.
- [x] Gates workspace, build et capture native du mini-bipède actualisé.

Validation itération8bis : 177 tests Rust réussis/0 ignoré, fmt et Clippy
workspace, build desktop ; quatre tests Python, frontières21/36, Ruff, actionlint.
Captures Design/Optimisation inspectées : 14 objets verts automatiques,
CameraSupport sélectionné. Nouvelle fenêtre laissée ouverte pour inspection.

# Itération 9 - Navigation Design demandée

- [x] Confirmer le format trois onglets : DSL → Graphe cinématique 2D → Vue 3D.
- [x] Ouvrir la page Design sur le DSL et préserver les accès explicites au code/3D.
- [x] Ajouter une vue 2D des instances/interfaces avec état d'articulations indisponibles, sans relations inventées.
- [x] Tester ordre, navigation, deux tailles et provenance du modèle affiché.
- [x] Vérifier les gates, la vue native et documenter/committer la tranche UI validée.

Le cahier de synthèse mécanique multi-corps fourni reste un objectif distinct
non achevé par cette tranche de navigation.

Validation itération9 : 179 tests réussis/0 ignoré, fmt/Clippy workspace, build
desktop ; quatre tests Python, frontières21/36, Ruff/actionlint. Trois captures
natives inspectées ; application laissée sur DSL. Checkpoint de la tranche UI
validée préparé, sans clore la synthèse mécanique multi-corps demandée.

# Itération 10 - Canevas de graphe interactif

- [ ] Projeter les références DSL vers des nœuds/arêtes typés avec provenance.
- [ ] Remplacer les cartes par un moteur de dessin 2D avec zoom, pan, cadrage, déplacement et sélection.
- [ ] Distinguer références géométriques et articulations absentes ; protéger la source obsolète.
- [ ] Tester projection et interactions, vérifier les gates et le rendu natif.
- [ ] Documenter les limites et committer la tranche vérifiée.

# Itération 10 bis - Graphe mécanique et schéma cinématique

- [x] Remplacer la projection de références par le contrat corps/liaisons typés.
- [x] Parser/résoudre body et joint, conserver IDs, unités et provenance.
- [x] Valider références, axes, pas hélicoïdal et appartenances sans interdire les boucles.
- [x] Afficher graphe des liaisons et schéma projeté depuis le même modèle.
- [x] Ajouter l'étau illustratif et la cinématique de conception explicite du bipède.
- [x] Tests ciblés/intégration, gates et documentation (commit en attente d'autorisation, voir itération 11).

# Itération 11 - Lisibilité du graphe et vérification native

L'itération 10 (graphe de références géométriques) est remplacée par l'itération
10 bis (ADR-016) ; ses cases restent non cochées car ce graphe n'est pas livré.

- [x] Reprendre l'état laissé par la session précédente (fmt, Clippy, tests) et corriger le lint de test.
- [x] Remplacer le placement par forces (illisible à 15 corps) par une disposition en arbre par couches, racine = corps fixé au bâti.
- [x] Placer les libellés de liaison sans recouvrement des autres libellés, des noms et des nœuds (graphe et schéma).
- [x] Tests : placement (racine, boucle, 21 corps ramifiés) et non-recouvrement des textes (étau 3 projections, bipède 14 liaisons) ; mutation vérifiée.
- [x] Vérification native étau + bipède à 1440x831, captures docs/screenshots/kinematic-*-20261020.png.
- [x] Aligner TECHNICAL.md, CURRENT.md et README.md sur le modèle mécanique déclaré.
- [x] Gates : fmt, Clippy -D warnings, 202 tests, 4 tests Python, frontières 21 crates/37 arêtes, git diff --check.
- [ ] Revue du diff et commit local sur autorisation explicite.

# Itération 12 - Interface d'arbre des servos

- [x] Diagnostiquer : seules les oreilles du carter étaient des interfaces, rien côté arbre de sortie.
- [x] Ajouter l'interface `shaft` à l'objet servo (pose partagée, 14 instances identiques).
- [x] Faire préserver l'arbre par les pièces enfants (cuisses, tibias, bras, pieds) et le carter par les parents.
- [x] Tests : géométrie du pad sur l'axe des 14 pivots déclarés, interfaces préservées par pièce ; mutation vérifiée.
- [x] Gates (204 tests, fmt, Clippy), contrôle natif, docs (README, ADR-015, TECHNICAL).
