# ADR-008 — Solveur d'optimisation topologique

- Statut : backend CPU historique ; orientation vers bibliotheque externe pour la suite, bascule conditionnelle aux contrats ci-dessous
- Date : 2026-09-22 ; orientation revisee le 2026-10-08

## Contexte

### Adaptateur CUDA experimental (iteration 16)

Autorisation utilisateur : accelerer le calcul sur la RTX 3060. Premier candidat :
CUDA.jl/cuSPARSE et Krylov.jl CG preconditionne diagonal en Float64, derriere le
dispatch solve_system! de TopOpt. Assemblage, conditions aux limites, filtre,
MMA et derivees restent ceux du backend existant ; resolution directe et adjointe
passent par le meme adaptateur. Les types CUDA restent dans le processus Julia.
Le CPU direct est conserve comme reference et choix de repli explicite.

Avant activation : solution connue, residu reel, comparaison des deplacements
et gradients avec le CPU, puis mesures sur la grille du tibia.
Un echec CUDA/convergence est un diagnostic, jamais une solution approximative
acceptee ou une bascule CPU silencieuse. Une acceleration n'est annoncee qu'apres
mesure ; aucun seuil mecanique ni maille ne change pour favoriser le GPU.

Qualification executee : CUDA 6.0.0, Krylov 0.8.4, Julia 1.10.10, RTX 3060.
CUDA 6.4.2/GPUCompiler 2.13 echouait a la compilation ; les versions fonctionnelles
sont verrouillees. Aucun type tiers n'est expose par le port Rust.
ComputeBackend (CPU par defaut/CUDA explicite) est transporte et verifie dans
chaque evenement puis conserve dans MassDiagnostic et affiche dans l'UI.

Le residu relatif brut <= 1e-8 s'est revele inaccessible pour certains essais
quasi vides, y compris en CPU direct : 3.49e-7 CPU contre 3.11e-7 GPU sur le
cas de transport. Critere revise explicitement : erreur retrograde composante
par composante <= 1e-12 ET residu relatif <= max(1e-8, plancher d'arrondi).
Le plancher vaut 8*gamma_n*norm(abs(K)*abs(u)+abs(f))/norm(f), avec
gamma_n=n*eps/(1-n*eps), n le nombre maximal de termes par ligne symetrique.
Au plus quatre corrections, second membre normalise, CG borne a 20000 iterations.
Il s'agit d'une stabilite numerique, pas d'une borne d'erreur sur une solution
mal conditionnee. Contraintes mecaniques, KKT et statut non publiable inchanges.

Sur le tibia reel 24192 cellules, deux iterations completes : 300.27 s CPU,
105.18 s CUDA (~2.85x). Masse 56.134595 g, deplacements 1.887961 et 1.243991 mm
contre limite 0.5 mm, sur les deux backends. Candidat non converge/non faisable.
Tests passes : solution/refus/echelles, gradients et differences finies, masques,
deux cas independants, equivalence des densites/masse, transport et UI native.
L'assemblage, les sensibilites et MMA restent CPU ; les petits cas peuvent etre
plus lents sur GPU et le chargement initial Julia/CUDA reste couteux.

### Raffinement et GPU (iteration 14)

Le projecteur de masse ne construit plus le GridModel FEM historique pour
enumerer ses noeuds. Il conserve le meme ordre X lent/Z rapide et les memes
charges distribuees. Le budget Julia est distinct : 32768 cellules, 256 par axe,
150000 DOF et 65536 cellules-cas, contre 2048/64 pour le solveur historique.
La maille cible est exprimee en longueur ; chaque axe utilise ceil(longueur/pas).
Un depassement est refuse, jamais remplace silencieusement par une grille grossiere.
Le filtre garde sa longueur physique : affiner le maillage ne change pas cette
regularisation et ne prouve pas la convergence mecanique.

La RTX 3060 12 Gio est presente et son usage est autorise. TopOpt 0.14.0 n'a pas
de chemin CUDA actif dans ce solveur : l'ancien assembleur CuVector est commente,
les dependances CUDA sont absentes et DirectSolver factorise une matrice CPU.
Cette iteration ne pretend pas activer le GPU. Un adaptateur sparse GPU devra
etre qualifie sur les resolutions/derivees avant remplacement du calcul CPU.

### Integration native experimentale (iteration 13)

Le projecteur commun accepte aussi minimize_mass sous max_displacement seulement.
Il ne change ni l'objectif ni les charges ; les options OC de la projection ne
sont pas utilisees par le moteur de masse. execute_topology historique continue
de refuser cet objectif. Le moteur externe reste experimental : ses densites et
metriques sont des diagnostics de calcul, jamais des meshes exportables. Le
worker natif doit conserver cette distinction, la provenance et les gardes de
revision/annulation. La visualisation n'attend pas la qualification mecanique ;
la publication l'exige toujours. Aucun benchmark n'est attribue a une piece du
bipede et aucune propriete physique manquante n'est completee par defaut.

Transport local schema 1 : dimensions et ordre de grille (X lent, Z rapide),
masques disjoints, elasticite et densite SI, appuis par axe et forces nodales
par cas, limites de deplacement, identite d'operation/revision. Le processus
Julia traduit les indices par coordonnees Ferrite, pas par hypothese d'ordre
commun. Les evenements JSONL portent la meme identite et des champs de densite
non publiables. Le port Rust valide taille, finitude, masques, masse integree
et provenance avant de transmettre les donnees au worker. stdout est reserve
au protocole ; les logs restent sur stderr. Le processus appartient au job et
est arrete/recolte en cas d'annulation ou de protocole invalide. Aucun code du
document n'est execute comme programme Julia.

### Qualification externe du 2026-10-08, iteration 11

La priorite utilisateur minimize_mass place TopOpt.jl avant Scikit-Topt dans
l'ordre de qualification, sans autoriser une bascule non verifiee. TopOpt 0.14.0
(MIT) est maintenant installe dans un depot Julia local avec Manifest verrouille.
Deux correctifs de compatibilite verifies par SHA-256 sont requis : noms de
parametres de types TemperatureFun/TemperatureResult et acces public Ferrite.cellid.
Le candidat teste est donc **0.14.0 + robogen-compat2**, pas la release intacte.

Le noyau experimental compose FEM, filtre, deplacements et MMA de la bibliotheque,
sans nouveau solveur maison. Le test minimum de masse sous compliance passe ;
le test direct deplacement multi-cas/filtrage echoue encore a la faisabilite et
convergence a 500 iterations. **Candidat non qualifie, aucune bascule produit.**
Les resultats et la matrice de capacites sont dans
[le rapport executable](../../crates/robogen-topopt/backends/topopt-jl/README.md).
Les anciennes formulations de choix ci-dessous sont historiques. Le coeur Rust
reste inchange ; stress, fabrication et reanalyse externe restent des gates.

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

## Orientation revisee : reutiliser un moteur existant (2026-10-08)

L'utilisateur demande de concentrer RoboGen sur la declaration et la preparation
du probleme, pas sur le developpement d'un nouveau FEM/SIMP. La suite privilegie
un adaptateur de bibliotheque existante. Scikit-Topt est le premier candidat de
prototype, TopOpt.jl l'alternative si ses capacites correspondent mieux aux
contrats. PyTopo3D reste une piste GPU experimentale, pas un choix valide sur
la seule presence de CUDA. Aucun backend externe n'est installe ou integre par
cette decision ; le backend Rust demeure actif jusqu'a une bascule verifiee.

Le DSL est deja declaratif : TopologySpec et TopologyRequest portent geometrie,
materiau, cas et provenance. Le point a remplacer est execute_topology et son
couplage a GridRequest/FEM/SIMP, pas le parseur. Le port TopologyOptimizer actuel
est encore un squelette : sa simple existence ne signifie pas que le chemin
productif dispatchera un moteur externe. Ne pas prolonger le FEM maison pour
imiter les fonctions d'une bibliotheque candidate. Conserver provisoirement
ses fixtures et mesures comme regressions, sans en faire l'unique oracle.

### Contrat cible, a implementer

- TopologySpec reste la specification semantique. Une IR numerique distincte
	porte schema/version, identite du document/revision et piece, maillage 3D
	(coordonnees, connectivite, type/ordre des elements), reperes et unites SI.
- Regions design/preserve/void disjointes, liens aux interfaces et spans DSL,
	materiau, conditions aux limites et vecteurs de force explicites par cas.
	Les poids d'agregation des cas, la definition du budget de volume et le sens
	physique des densites sont explicites. Aucun nom de face du kernel persiste.
- Objectif et contraintes restent distincts. MinimizeCompliance avec budget de
	volume n'est pas une traduction de MinimizeMass sous limites de resistance.
	Les capacites du backend sont verifiees avant lancement, jamais ignorees.
- Premier transport propose : processus Python local gere par l'adaptateur Rust,
	environnement isole/verrouille, JSON versionne pour requetes/evenements/resultat,
	journaux separes. Maillages/champs volumineux pourront etre des artefacts
	references avec empreinte et convention d'indices, pas du texte arbitraire.
	Aucun type NumPy/scikit-fem ne franchit la frontiere RoboGen.
- Le worker conserve lancement explicite par piece, progression, garde de revision
	et annulation. Prevoir protocole d'annulation, delai et arret du processus enfant
	si une phase tierce est non interruptible ; borner taille/duree des echanges et
	refuser sorties partielles/non finies/incoherentes. Aucun service distant.
- Le resultat conserve backend/version, empreinte d'entree, densites physiques,
	mesures par cas, convergence et diagnostics. Reconstruction et reanalyse sont
	distinctes : la compliance du champ ne valide pas un STL seuille ou lisse.
	RoboGen controle geometrie/provenance et publication ; la reanalyse mecanique
	doit aussi pouvoir etre deleguee au moteur, sans recreer un second FEM produit.

### Preuve requise avant bascule

Epinglez une distribution effectivement verifiee et ses dependances. Executer
une fixture 3D alignee sur les contrats actuels : deux forces opposees en cas
independants, regions obligatoires et vides. Verifier conservation des charges,
appuis, unites et orientations ; densites preservees/vides avant et apres filtres ;
budgets et sensibilite au raffinement. Ajouter des cas d'appuis differents ou les
refuser explicitement si non supportes. Tester annulation/revision, panne du
processus, absence du runtime et resultats malformes. Reanalyser la geometrie
reconstruite ; un exemple visuellement plausible ne suffit pas.

Garder les benchmarks Rust le temps de passer les memes contrats avec le candidat,
en ajoutant des references analytiques/convergence independantes. Ne pas exiger
des densites ou un nombre d'iterations identiques entre optimiseurs differents.
Basculer ensuite l'execution explicite Project/CLI, puis retirer du chemin produit
le coeur numerique historique. Ne pas ajouter cinq adaptateurs simultanement.

### Verification documentaire du candidat

Sources consultees le 2026-10-08, sans execution du paquet :

- [Scikit-Topt installation](https://scikit-topt.readthedocs.io/en/stable/installation.html) :
	import sktopt, Python 3.10-3.13 annonce ; restrictions VTK/PyVista en 3.13.
	Le prototype privilegiera Python 3.12 et le solveur SciPy sans PETSc obligatoire.
- [Scikit-Topt exemples au tag 0.3.10](https://github.com/kevin-tofu/scikit-topt/blob/0.3.10/examples/README.md) :
	API reelle LinearElasticity.from_facets, OC_Config, OC_Optimizer, parameterize
	puis optimize ; pas une fonction generique acceptant directement le pseudo-JSON.
	Le pyproject consulte a ce tag annonce Apache-2.0 et une version interne 0.3.9 :
	verifier l'identite de l'artefact installe avant de figer un lock.
- [Scikit-Topt README](https://github.com/kevin-tofu/scikit-topt) : forces multiples
	annoncees, mais Multiple BC Conditions et arret de boucle figurent au TODO.
	Les appuis par cas, preserve/void apres filtrage et l'annulation doivent donc
	etre verifies par code et essais, pas deduits de la liste marketing.
- [TopOpt.jl documentation](https://juliatopopt.github.io/TopOpt.jl/stable/) :
	documente elements tetra/hexa, MultiLoad, FixedElementProjectorFun, objectifs
	et contraintes differenciables via Nonconvex. Candidat alternatif, pas valide
	localement et pas promesse de toutes contraintes d'impression.
- [PyTopo3D](https://github.com/jihoonkim888/PyTopo3D) : obstacles et CUDA/CuPy
	documentes ; minimisation de masse encore en roadmap dans la version consultee.
	Un force_field unique ne prouve pas des cas de charges independants.

Le GPU local interroge par nvidia-smi est une NVIDIA GeForce RTX 3060, non 3090.
Ce constat n'est ni une validation CUDA du candidat ni un benchmark GPU.
