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

## Tranche autonome autorisee le 2026-10-08

L'utilisateur autorise explicitement le developpement FEM/SIMP sur poutre a
charges manuelles avant la cloture M1-M6. Cette exception ne valide pas ces
jalons et ne debloque ni les charges robotiques de M5/M8 ni les backends RL/IA.
Suivi dans le ticket 15, iteration 3 ; le jalon M7 complet reste ouvert.

Premier adaptateur : faer 0.22.6, factorisation sparse LLT, types tiers prives
a robogen-topopt. Le port accepte une matrice symetrique par triplets du
triangle inferieur, additionne les doublons, et verifie le residu relatif.
Matrices indefinies/singulieres, entrees non finies ou hors budget produisent
une erreur, jamais une solution publiee sans verification. Le spike numerique
precede l'assemblage FEM ; une factorisation directe n'est pas interruptible
en son milieu et doit rester bornee et hors du thread UI.

Spike et tranche scientifique valides : solution connue, singularite, assemblage
Hex8 a huit points de Gauss, modes rigides/energie/traction/flexion, sensibilites
filtrees par differences finies et OC a volume constant. La poutre de reference
conserve la revision du caller et le nom/version de l'adaptateur ; aucune
publication de document, migration ou cache persistant n'est ajoute.
Les parametres et resultats mesures sont dans examples/topopt_beam/README.md.
La reconstruction, les contraintes geometriques/fabrication, les validations
apres reconstruction et le branchement desktop restent a implementer. Le
backend de TopologySpec arbitraire reste explicitement indisponible.

## References

- <https://docs.rs/faer/latest/faer/sparse/>

