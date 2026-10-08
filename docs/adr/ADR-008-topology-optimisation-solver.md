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

## Reconstruction autonome (iteration 4)

Premier adaptateur SurfaceReconstructor : fast-surface-nets 0.2.1 (MIT/Apache),
alternative eprouvee a marching cubes sur grille reguliere. Aucun type tiers
n'est expose. Echantillons aux centres des cellules, deux couches exterieures de
densite nulle pour fermer les faces de bord du bloc, interpolation lineaire du champ seuil-densite ; ce champ n'est
pas une distance signee metrique. Seuil borne a [0.5,1) afin de ne pas etendre
le domaine. L'algorithme arrondit les bords et peut supprimer des liaisons fines.
Echantillonnage trilineaire subdivise par deux avant extraction : la grille
initiale du benchmark produisait une arete non-manifold, refusee par le
validateur. Cette subdivision ne change ni les densites FEM ni le seuil.
Le volume de la surface n'est ni le volume filtre ni un budget garanti.

Avant restitution : controles de finitude, indices, triangles non degeneres,
bornes, fermeture et orientation des aretes, liens des sommets et composantes.
Les artefacts conservent le seuil et l'identite du reconstructeur. Les resultats
SIMP portent deja requete, revision et solveur ; une surface seule ne certifie
ni rigidite ni contraintes de fabrication. Les intersections geometriques et
la reanalyse mecanique de la surface ne sont pas garanties par ces controles.
Un resultat ambigu/degenerate est refuse, pas repare silencieusement.

L'exemple/test compose le DTO surface avec le Mesh RoboGen deja consomme par
robogen-export, via des dev-dependencies seulement. Pas d'arete de production
TopOpt -> CAD/export ni de second writer STL. STL est exprime en millimetres ;
l'artefact est une visualisation non revalidee, pas une piece certifiee. La
publication des declarations topology() demeure bloquee par E330.

Mesure : la surface du benchmark a trois composantes et ne conserve ni la face
fixe ni la face chargee. La topologie fermee ne vaut pas validation mecanique.
La prochaine tranche doit preserver les interfaces et reanalyser la geometrie ;
elle ne doit pas attribuer la compliance du champ de densites au STL seuille.

## References

- <https://docs.rs/faer/latest/faer/sparse/>

