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


## Preservation et reanalyse (iteration 5, 2026-10-08)

Le masque cellulaire porte explicitement Design/Preserve/Void. Les regions
preservees ont densite physique 1, les regions interdites 0, et seules les
variables Design sont filtrees entre elles. Les derivees du filtre et le
budget OC excluent les variables constantes, mais le volume preserve compte
dans le budget global. Une fraction incompatible est une erreur ; le coeur
ne genere aucune preservation implicite. L'exemple peut declarer les couches
d'appui/charge comme Preserve.

Un second reconstructeur produit les faces exposees des cellules binaires.
Cette frontiere a marches d'escalier correspond exactement aux Hex8 restants,
preserve les interfaces alignees sur grille et n'est pas une surface lissee.
Surface Nets reste disponible comme projection distincte. La reanalyse de la
frontiere voxel doit supprimer les cellules vides et leurs DOF inutilises,
verifier le chemin de charge vers l'appui et ne pas utiliser le plancher SIMP
pour porter artificiellement des efforts a travers le vide. Les metriques
recalculees concernent cette discretisation, pas une certification de solidite
ni les geometries lissees. L'ancien benchmark sans masque reste reproductible.

## Conditions explicites et cas multiples (iteration 6)

Le coeur FEM devient GridModel sur GridDefinition (dimensions, cellules,
elasticite), avec NodeId local a la grille et BoundaryConditions contenant forces
nodales et blocages de translation par axe. Ni position de l'encastrement ni
face chargee ne sont implicites. BeamModel est un adaptateur qui construit son
ancien probleme et sa moyenne de deplacement en bout ; les benchmarks existants
restent des contrats. Les IDs de grille ne sont pas des references persistantes.

GridRequest conserve cas de charge avec LoadCaseId/source span et revision.
L'objectif de raideur multi-cas est la somme non ponderee des compliances : les
forces ne sont jamais moyennees ou superposees entre cas. Chaque cas produit sa
propre analyse et ses derivees contribuent a OC. Les contraintes futures seront
verifiees par cas, pas contre une moyenne de leurs reponses. Les bornes de
ressources limitent explicitement aussi le nombre de cas.

La reanalyse binaire retire les cellules/DOF absents pour chaque cas. Les charges
ou appuis sur des noeuds absents sont refuses. Une connexion par faces est requise
pour la piece ; le solveur SPD refuse les modes rigides restants. Les appuis
obliques necessitant un changement de base ne sont pas assimiles a des appuis
cartesiens. La projection des interfaces nommees est un etage distinct a venir,
sans nouvelle dependance CAD dans le solveur numerique.

Le test prealable d'appuis porte sur le rang des six modes rigides avec positions
normalisees et tolerance 1e-10. Il complete la factorisation/residu sans pretendre
borner tout conditionnement. Les composantes de force sur DOF bloques sont
explicitement refusees dans cette API (reactions et deplacements imposes non
nuls hors capacites). Les noeuds absents ne sont jamais remplaces par des ressorts.

Reference methodologique consultee : Andreassen et al., 2011,
[Efficient topology optimization in MATLAB using 88 lines of code](https://www.topopt.mek.dtu.dk/-/media/subsites/topopt/apps/dokumenter-og-filer-til-apps/topopt88.pdf),
pour SIMP, filtre de densite, OC et extensions de conditions/cas. Les validations
RoboGen portent sur son propre Hex8 3D et ses sensibilites, pas sur une simple
reproduction du programme MATLAB 2D.
