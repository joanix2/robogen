# Bipede - contraintes DSL et supports generes

Tache : [ticket](task_20261007_15-biped-constraints-topopt.md).

# Iteration 1

- [ ] Verifier les dependances M0-M7 et la validite des donnees physiques.
- [ ] Ecrire ADR et contrats types/versionnes pour interfaces, liaisons et OptimizationSpec.
- [ ] Implementer syntaxe/lowering/diagnostics et edition annulable des contraintes.
- [ ] Definir regions preservees/exclues et domaines des supports du bipede.
- [ ] Reutiliser profils parametrables du ticket 13 et solveur M7/M8.
- [ ] Generer et revalider un premier support camera puis un support de jambe multi-cas.
- [ ] Valider export, fabrication, annulation/revisions et gates ; distinguer marche et TopOpt.

# Iteration 2 - Contrat topology composable

- [x] Documenter ADR-012, proprietaires, unites, identites et resultat differe.
- [x] Implementer TopologySpec type/versionne et constructeur topology dans le graphe Solid (tests de composition et IDs passes).
- [x] Verifier interfaces/charges/appuis, objectifs distincts des contraintes et profils de fabrication (validation declarative, aucune garantie numerique).
- [x] Raccorder refus explicite de backend et interdiction de maillage/export factice, avec provenance (tests CAD/projet passes).
- [x] Ajouter exemple support de batterie, inspection CLI et chargement UI annulable (tests CLI/UI passes).
- [x] Valider tests unitaires et verticaux, diagnostics, historique/revisions, documentation et gates (103 tests workspace, fmt, Clippy, builds CLI/desktop, inspection CLI et capture native 1440x831).

# Verification des prerequisites - 2026-10-08

- [x] Relire les criteres M0-M7 et distinguer validations et avancements partiels.
- [x] Verifier les blocages dans le code sketch/robotique/physique, sans cocher les jalons absents.
- [x] Completer deux preuves natives M1 : Optimisation et Apprentissage en 1440x831, sans resultats fictifs.
- [x] Fixer l'ordre autorise avant d'entamer FEM/SIMP : M7 autonome explicitement autorise le 2026-10-08.

Le premier item de l'iteration 1 reste ouvert : dependances et donnees physiques
ne sont pas validees par cet audit.

# Iteration 3 - M7 autonome autorise, poutre a charges manuelles

- [x] Valider le port sparse faer : solution connue, residu, entrees invalides et singularite.
- [x] Implementer FEM hexaedrique, poutre de reference et validations analytiques/invariants.
- [x] Implementer SIMP, filtre de densite, OC et historique ; verifier sensibilites et volume.
- [x] Exposer un calcul de poutre reproductible avec provenance, progression et annulation (API synchrone et exemple autonome, aucun appel UI).
- [x] Documenter mesures et limitations et verifier les gates (114 tests workspace, fmt, Clippy -D warnings ; benchmark reel a 65 iterations).

L'autorisation du 2026-10-08 leve l'ordre M1-M6 uniquement pour cette tranche
scientifique M7. Reconstruction, publication desktop et supports robot seront
suivis explicitement ; pas de cloture du jalon complet sans leurs validations.

Checkpoint local autorise pour cette tranche scientifique validee. La suite
reconstruction/publication reste ouverte dans le TODO M7 et l'iteration 1.

# Iteration 4 - Reconstruction M7 autonome

- [x] Extraire une surface avec bibliotheque existante et DTO sans type tiers.
- [x] Valider fermeture, orientation, triangles et composantes ; tester champs et annulation.
- [x] Raccorder les densites SIMP et produire un artefact de visualisation identifie comme approximation.
- [x] Documenter hypotheses/mesures et executer les gates (118 tests, fmt, Clippy ; suivi et checkpoint de la visualisation uniquement).

118 tests, fmt et Clippy passent. Artefact reel : 208 sommets, 404 triangles,
3 composantes. Les faces d'appui/charge sont perdues ; la surface n'est pas
un support valide. Preservation et reanalyse mecanique restent a implementer.

# Iteration 5 - Regions imposees et reanalyse binaire

- [x] Ajouter Design/Preserve/Void au calcul SIMP, filtre et derivees coherents, budget incluant preserve.
- [x] Verifier infeasibilite, sensibilites, interfaces et exclusions sous iterations/annulation.
- [x] Extraire une frontiere voxel exacte et verifier fermeture/volume/interfaces.
- [x] Reanalyser la geometrie binaire avec cellules vides absentes ; refuser charges/appuis perdus et fragments.
- [x] Executer benchmark/regressions/gates et documenter les mesures sans cloturer l'objectif global.

# Iteration 6 - FEM et SIMP a conditions explicites et cas multiples

- [x] Extraire le modele Hex8 general, IDs nodaux et appuis/charges explicites ; conserver adaptateur poutre.
- [x] Generaliser SIMP aux cas identifies, somme des compliances/derivees et analyses individuelles.
- [x] Reanalyser chaque cas sur le solide binaire ; verifier absence de vide porteur, appuis/charges et cancellation.
- [x] Tester configurations non-X, forces opposees, sensibilites multi-cas et regressions poutre/STL.
- [x] Documenter contrats/hypotheses, verifier gates et committer la tranche sans clore le systeme complet.

# Iteration 7 - Projecteur et execution TopologySpec via CLI

- [x] Classifier CSG de facon conservative et construire les masques, bornes et conflits avec provenance.
- [x] Projeter interfaces volumiques, forces/appuis et cas identifies ; documenter la distribution choisie.
- [x] Executer TopologyRequest avec capacites explicites et verification volume/masse/deplacement sur solide reanalyse.
- [x] Ajouter commande CLI, exemple .rgn et tests DSL -> calcul -> STL ainsi que refus/annulation/invariants.
- [x] Verifier gates, documenter limites et committer sans clore l'objectif global.

Validation : 139 tests workspace, fmt --check et Clippy -D warnings passes.
Les objectifs/contraintes absents et la publication desktop restent ouverts.

# Correctif outillage VS Code

- [x] Declarer et installer rust-analyzer compatible avec Rust 1.88.0.
- [x] Verifier la selection, les commandes locales et documenter le rechargement.

# Iteration 8 - Raccordement des resultats au document

- [x] Composer les maillages reanalyses sous transformations/couleurs/compound, refuser les booleens aval.
- [x] Executer les declarations supportees dans Project/worker avec revision, annulation et rapports.
- [x] Verifier publication, historique, diagnostics et parcours natif.
- [ ] Raccorder les supports du bipede uniquement avec interfaces et charges explicites ; signaler les inconnues.

Validation : 144 tests workspace, fmt, Clippy -D warnings, build desktop,
capture native 1080x680 et controle 1080x831 passes. Choix utilisateur : fixations
exactes. DSL servo relu : pattes/trous/arbre cotes, palonnier/cannelures/vis absents.
La derniere acceptance reste bloquee ; ne pas marquer le bipede assemble.

# Iteration 9 - Preparation et calcul explicite par piece

- [x] Projeter domaine/preserve/keepout sans solveur, hors maillages exportables.
- [x] Lancer une piece dans le worker, conserver les autres resultats et gerer les etats/revisions.
- [x] Afficher couches et selection/reglages/lancement/resultats dans les vues natives.
- [x] Verifier absence de calcul implicite, independance, annulation, invalidation et export.
- [x] Actualiser documentation, gates et verification native.

Validation iteration 9 : 149 tests workspace, fmt, Clippy -D warnings, build
desktop et diagnostics editeur passes. Captures natives de preparation,
selection et resultat explicite ; clics de masquage/lancement verifies.
Edition conservative : tout changement DSL invalide tous les resultats TopOpt.
Interfaces/charges/materiaux modifies via DSL, reglages numeriques dans le panneau.
Objectif supports du bipede toujours ouvert, aucune nouvelle capacite du solveur.

# Priorite utilisateur - Minimiser la masse (2026-10-08)

Cette priorite remplace l'ordre de prototype compliance/volume propose dans la
conversation : le choix de bibliotheque est subordonne a l'objectif minimize_mass.

- [ ] Selectionner un backend existant capable de minimiser la masse sous les contraintes requises ; Scikit-Topt est candidat, non impose.
- [ ] Definir une reference faisable, les cas de charge, limites mecaniques/fabrication et tolerances ; signaler les donnees physiques manquantes.
- [ ] Valider un premier probleme minimize_mass, avec preserve/keepout et controle de chaque contrainte pour chaque cas, sans substitution par maximisation de rigidite a volume fixe.
- [ ] Comparer masses avant/apres et reanalyser le solide reconstruit avant publication ; rapporter convergence et limites, sans garantie de minimum global.
- [ ] Tester refus de capacites absentes et diagnostic d'infaisabilite ; ne pas relacher les contraintes pour obtenir un resultat.

# Cible de travail confirmee - Mini bipede (2026-10-08)

La livraison et la demonstration doivent porter sur examples/mini_biped/main.rgn,
pas sur une piece de test autonome. Garder les benchmarks comme tests internes.

- [ ] Preparer les domaines/preserve/keepout des supports dans les reperes du bipede existant, sans modifier les composants achetes ni inventer les fixations.
- [ ] Afficher la preparation dans Conception sur ce robot, sans calcul automatique ni faux support exportable.
- [ ] Traiter le premier support de reference (camera) dans cette scene ; diagnostiquer les donnees mecaniques encore manquantes avant calcul.
- [ ] Demonstrer selection, minimisation de masse, reanalyse et publication par support dans Optimisation sur le bipede, puis etendre aux autres supports.

# Iteration 10 - Preparation incomplete dans la scene du bipede

- [x] Documenter schema 2 et materiau inconnu explicite, sans donnees physiques inventees.
- [x] Ajouter onze preparations dans mini_biped en conservant les 17 composants et leurs placements.
- [x] Construire domaines parametrables, contacts aux trous connus et exclusions locales ; verifier les bornes des contacts.
- [x] Refuser le calcul incomplet E347 dans Project/CLI/TopOpt ; afficher les manques et bloquer lancement/export dans l'UI.
- [x] Tester trois couches, selection camera, historique, revision et absence de resultats automatiques ; passer fmt/Clippy/tests workspace/build.
- [x] Capturer le bipede prepare en natif 1080x680 ; documenter que les clics XTest n'ont pas valide le parcours manuel.
- [ ] Completer interfaces articulaires, volumes de mouvement/outillage/cables et separation des domaines : la preparation ne suffit pas a valider l'assemblage.
- [ ] Renseigner materiau, cas de charges et limites verifies puis integrer le moteur minimize_mass ; calculer et reanalyser le premier support reel.

# Iteration 11 - Qualification minimum de masse et donnees sourcées

Autorisation : « Implémentez et va chercher les informations sur Internet nécessaires. »
Le plan en six lots est applique ; LeftShinSupport remplace la camera comme
premiere livraison mecanique. Les benchmarks restent des tests internes.

- [x] Installer un environnement local verrouille TopOpt.jl et executer un test 3D de minimisation de masse avec regions imposees.
- [ ] Qualifier deplacements, contraintes, cas independants, filtrage, convergence et refus ; documenter les capacites non prouvees.
- [x] Sourcer les donnees materiel, distinguer reference candidate et identification confirmee ; ne pas inventer les interfaces.
- [ ] Completer montage et donnees physiques de LeftShinSupport (lots 1 et 2).
- [ ] Raccorder le moteur qualifie au port numerique et worker existants (lot 4).
- [ ] Reanalyser puis publier le tibia gauche dans le bipede (lot 5), avant extension aux jambes (lot 6).

Validation partielle : smoke test compliance/masse 7/7 ; deplacements filtres
multi-cas 13/16, trois echecs de faisabilite/convergence encore presents a 500
iterations. Candidat non qualifie. Gates Rust passes. Details, reproductions et
deux correctifs amont limites dans backends/topopt-jl/README.md. Aucun commit de
cloture, aucun support du robot calcule ; lots 1/2 et 4/5/6 restent ouverts.

# Iteration 12 - Resultat graphique demande

- [ ] Exporter les donnees du calcul reel et ses coordonnees, sans changer les limites.
- [ ] Afficher densites 3D, masses, deplacements et convergence dans un rapport interactif explicitement non valide.
- [ ] Verifier le rendu, fournir une capture et ouvrir le resultat graphique.

# Iteration 13 - Correction de l'integration native

- [x] Decoupler projection et objectif OC, verifier conservation des masques et charges.
- [x] Raccorder les problemes declares au processus Julia local ; borner protocole, erreurs, revisions et annulation.
- [x] Conserver densites/metriques de diagnostic hors des resultats exportables et les afficher dans Optimisation native.
- [x] Verifier tests du transport, parcours worker, invalidation et absence de publication d'un candidat non qualifie.
- [ ] Lancer le bipede dans RoboGen, verifier la vue native et documenter exactement les calculs possibles/manquants.
- [ ] Passer les gates Rust et actualiser les documents sans cloturer la qualification mecanique ou les supports incomplets.

# Iteration 14 - Precision physique du maillage

- [x] Separer budget de projection masse et FEM historique, tester maille de 2.5 mm et conservation des charges.
- [x] Executer le moteur Julia a 6144 cellules et mesurer temps/memoire, sans pretendre a la convergence.
- [x] Verifier le GPU autorise et la disponibilite reelle du chemin CUDA de TopOpt.
- [x] Exposer maille cible/XYZ, taille reelle et budget dans l'interface native.
- [x] Verifier l'affichage de grilles fines et les regressions, documenter les limites restantes.
- [ ] Qualifier un adaptateur GPU avant d'annoncer une acceleration du calcul.

# Iteration 15 - Hypotheses PLA autorisees pour le tibia gauche

- [x] Renseigner PLA et parametres mecaniques explicitement estimes dans le DSL reel.
- [x] Declarer une liaison proximale ideale et deux cas de charge, sans modifier les servos achetes.
- [x] Tester la projection a la resolution native et documenter les limites des fixations.
- [x] Verifier le projet et afficher les hypotheses dans RoboGen natif.

# Iteration 16 - Acceleration GPU mesuree

- [x] Tester un solveur sparse CUDA Float64 et son refus des resolutions invalides.
- [x] Comparer deplacements et gradients CPU/GPU sur les memes problemes FEM.
- [x] Mesurer le cout sur la grille du tibia sans modifier les hypotheses.
- [x] Raccorder le choix de backend et sa provenance au diagnostic natif si qualifie.

Mesure : deux iterations du tibia, 300.27 s CPU contre 105.18 s CUDA (~2.85x).
Qualification numerique limitee aux tests executes ; convergence mecanique et
publication du tibia restent ouvertes. Commit checkpoint demande par l'utilisateur.

# Iteration 17 - Defaut d'optimisation signale

- [ ] Reproduire l'echec et verifier les gradients composes et contraintes.
- [ ] Corriger la cause avec la bibliotheque existante, limites/tolerances inchangees.
- [ ] Verifier la qualification et le calcul sur le tibia PLA declare.
- [ ] Verifier integration, diagnostics, documentation et checkpoint autorise.
