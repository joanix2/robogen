# ADR-008 — Solveur d'optimisation topologique

- Statut : accepte comme direction, implementation differee au milestone 7
- Date : 2026-09-22

## Contexte

La premiere optimisation doit etre reelle, interpretable et executable sur CPU : une poutre simple en elasticite lineaire, objectif compliance/volume, puis integration a une piece robotique. Les solveurs sparse, FEM et reconstruction doivent pouvoir evoluer independamment.

## Options considerees

- SIMP sur grille reguliere hexaedrique : implementation et visualisation progressives, robuste pour une preuve de bout en bout.
- SIMP sur maillage tetraedrique conforme : meilleure fidelite de geometrie, pipeline de maillage et sensibilites plus complexe.
- level-set : frontieres nettes mais implementation initiale plus risquee.
- solveur externe : puissant, mais packaging et traçabilite moins simples pour le MVP.

## Decision

Implementer au milestone 7 un backend CPU SIMP sur grille reguliere, elasticite lineaire petite deformation et cas de charge statique. Le pipeline est explicite : discretisation, conditions aux limites, assemblage sparse, resolution, compliance/sensibilites, filtre de densite, projection optionnelle, mise a jour OC, convergence, isosurface.

`TopologyOptimizer` orchestre des types RoboGen. `SparseLinearSolver` isole l'algebre ; le premier adaptateur utilise les structures et factorisations sparse de `faer` si le spike valide matrice SPD, erreurs et performances. Un conjugate-gradient preconditionne interne reste le repli pour les fixtures simples.

Les regions preservees, enveloppes, charges et appuis sont exprimes dans le repere de la piece avec provenance. Le resultat conserve densites et metriques ; marching cubes produit une geometrie de visualisation/export marquee comme reconstruction approximee, pas comme B-Rep parametrique.

## Consequences

- tranche scientifique reproductible et testable sans GPU ;
- grille reguliere simple mais couteuse en memoire et peu fidele aux surfaces fines ;
- V1 ne couvre ni plasticite, ni grands deplacements, ni fatigue, ni contrainte de stress certifiante ;
- les resultats affichent hypotheses, resolution, residu et convergence ;
- aucune fausse metrique n'est montree quand le backend est absent ou diverge.

## Strategie de remplacement

Les frontieres `FemDiscretizer`, `SparseLinearSolver`, `TopologyAlgorithm` et `SurfaceReconstructor` sont separees. Un backend tetraedrique, level-set, GPU ou distant doit passer les benchmarks de poutre cantilever, patch tests FEM, conservation des regions et tests de tendance compliance/volume.

Changer de solveur invalide les runs caches, pas le `OptimizationSpec`. Les metadonnees enregistrent versions et hypotheses pour comparer les resultats.

## References

- <https://docs.rs/faer/latest/faer/sparse/>

