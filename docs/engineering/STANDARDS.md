# RoboGen - Engineering Standards

## Objectif

Appliquer une discipline d'ingénierie logicielle rigoureuse à
la refonte de RoboGen.

Ces règles complètent AGENTS.md et les ADR existants.
Elles doivent être intégrées sans écraser les instructions
du Kanban ni les contrats déjà validés.

## 1. Architecture

- Privilégier DDD et architecture hexagonale.
- Organiser le code en `core` et `features`.
- Séparer domaine, application, infrastructure et présentation.
- Le domaine ne dépend jamais des frameworks externes.
- Utiliser des Protocols Python pour les frontières pertinentes.
- Utiliser Adapter, Strategy, Factory et Registry lorsque justifié.
- Centraliser les fonctionnalités transversales.
- Éviter les dépendances circulaires.
- Ne pas créer d'abstractions sans usage concret.
- Réutiliser une bibliothèque éprouvée avant de réimplémenter.

## 2. Qualité du code

- Une classe ou fonction = une responsabilité.
- Privilégier des fichiers de moins de 350 lignes.
- Découper par responsabilité métier, pas arbitrairement.
- Signatures typées obligatoires.
- Docstrings sur les interfaces et comportements non évidents.
- Ne pas utiliser `Any` pour contourner le typage.
- Ne pas coder en dur les paramètres physiques ou numériques.
- Éliminer les duplications.
- Favoriser les fonctions pures pour les transformations.
- Ne pas masquer les erreurs ou les comportements indisponibles.
- Éviter les classes géantes et les services fourre-tout.

## 3. Stack Python

- Lark : parsing du DSL.
- Pydantic : validation et contrats de données.
- Pint : unités physiques.
- Typer : CLI.
- pytest : tests.
- build123d : CAO, à qualifier sur les exemples.
- NumPy/SciPy : calcul scientifique.
- pyMOTO : candidat TopOpt, à qualifier.
- MuJoCo : simulation, lorsque nécessaire.
- NetworkX : graphes de dépendances, si utile.
- PyTorch : uniquement pour les besoins ML validés.

Ne pas introduire FastAPI, un serveur, une base de données
ou des microservices sans besoin démontré.

## 4. Tests et TDD

Appliquer RED -> GREEN -> REFACTOR.

Prévoir :
- Tests unitaires des composants.
- Tests de contrats entre modules.
- Tests d'intégration entre backends.
- Tests de régression du DSL.
- Golden tests sur les exemples existants.
- Tests de propriétés et d'invariants physiques.
- Tests E2E des parcours réellement disponibles.

Toute correction de bug ajoute un test de non-régression.

Ne jamais désactiver un test pour faire passer la CI.

Un mock ne constitue pas une preuve de fonctionnement réel.

## 5. Contrats scientifiques

Toute simulation ou optimisation doit identifier :
- Le problème mathématique réellement résolu.
- Les hypothèses physiques.
- Les unités et repères.
- Les matériaux et leurs propriétés.
- Les cas de charge.
- Les conditions aux limites.
- Les contraintes effectivement appliquées.
- La résolution numérique.
- Les critères de convergence.
- Les limites du backend.
- Les versions et paramètres d'exécution.

Interdictions :

- Ne jamais substituer silencieusement un objectif.
- Ne jamais ignorer une contrainte non supportée.
- Ne jamais inventer une force ou une propriété matérielle.
- Ne jamais déclarer une convergence sans vérification.
- Ne jamais présenter un résultat FEM comme une certification.
- Ne jamais exporter comme pièce validée une géométrie
  qui n'a pas passé les contrôles requis.

Minimiser la compliance n'est pas équivalent à minimiser
la masse sous contrainte de résistance.

## 6. Gestion des changements

Avant toute modification :

1. Identifier le composant responsable.
2. Examiner ses interfaces publiques.
3. Construire le graphe des dépendances impactées.
4. Identifier les tests et exemples affectés.
5. Définir les critères d'acceptation.
6. Modifier la plus petite surface cohérente.

Après modification :

1. Exécuter les tests ciblés.
2. Exécuter les tests de contrats.
3. Tester les intégrations impactées.
4. Mettre à jour la documentation.
5. Vérifier le parcours utilisateur de bout en bout.

Toute couche impactée doit être adaptée ou explicitement
justifiée comme non impactée.

## 7. CI et automatisation

Mettre en place progressivement :

- Ruff : lint et formatage.
- Pyright ou mypy : typage statique.
- pytest : tests automatisés.
- Hooks Git : vérifications rapides.
- GitHub Actions : validation avant fusion.
- Audit des dépendances.
- Contrôles de sécurité adaptés au projet.

Conserver les contrôles Rust tant que Rust reste utilisé.

Les tests scientifiques coûteux doivent être séparés
des tests rapides exécutés à chaque modification.

## 8. Agents et harness

Utiliser un orchestrateur responsable de l'intégration.

Spécialisations proposées :
- Architecture.
- DSL et modèle sémantique.
- CAO et robotique.
- Calcul scientifique : physique et TopOpt.
- QA et validation.

Chaque sous-agent reçoit :
- Un objectif précis.
- Des fichiers autorisés.
- Des interfaces d'entrée et de sortie.
- Les invariants à préserver.
- Les tests exigés.
- Un budget de travail.
- Un format de rapport structuré.

L'orchestrateur est responsable de :
- L'analyse d'impact.
- La gestion du contexte.
- Le cache.
- La coordination.
- La validation finale.
- Les checkpoints Git.

Un agent ne doit pas modifier un module hors de son
périmètre sans coordination.

## 9. Cache et reproductibilité

Les artefacts dérivés doivent être associés à :
- L'empreinte des sources.
- La version du schéma.
- La version du backend.
- La configuration du calcul.
- Les données physiques pertinentes.

Toute modification des dépendances invalide le cache
correspondant.

Un résultat de simulation ou d'optimisation obsolète
ne doit jamais être publié.

## 10. Interface utilisateur

- Feedback immédiat sur les actions.
- Progression visible pour les calculs longs.
- Annulation des opérations.
- Diagnostics compréhensibles.
- Navigation simple, idéalement trois clics maximum.
- Accessibilité et raccourcis clavier.
- Ne jamais bloquer l'interface pendant un calcul.
- Différencier préparation, calcul, échec et résultat validé.

## 11. Exigences contextuelles

Appliquer RGPD et gestion des données personnelles
lorsque le logiciel en traite.

Appliquer DAO/Repository lorsque la persistance le justifie.

Appliquer Infrastructure as Code lors d'un déploiement
qui nécessite une infrastructure reproductible.

SEO uniquement pour une éventuelle interface web publique.

Ne jamais introduire ces couches par principe.

## 12. Définition de terminé

Une tâche est terminée uniquement lorsque :

- Les critères d'acceptation sont satisfaits.
- Les tests pertinents passent.
- Les contrats existants sont préservés ou migrés.
- Les erreurs sont correctement propagées.
- La documentation est cohérente avec le code.
- Aucun résultat expérimental n'est présenté comme validé.
- Les dépendances et caches sont correctement mis à jour.
- Le résultat fonctionne dans le parcours utilisateur prévu.

Une tâche partielle reste explicitement partielle.
