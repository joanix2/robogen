# Contexte RoboGen

- Cahier des charges : `docs/init-prompt.md` ; maquette fournie dans la conversation.
- Architecture normative : `docs/architecture/OVERVIEW.md` et `docs/adr/ADR-001` a `ADR-009`.
- Au 2026-09-22, README.md et TECHNICAL.md decrivent un workspace, un shell desktop, un DSL minimal, une extrusion et un export STL. Ce constat documentaire ne vaut pas validation des jalons M0-M3.
- Le test `crates/robogen-testkit/tests/vertical_slice.rs` couvre une projection de triangles et un STL binaire ; il ne prouve pas a lui seul un rendu GPU natif ni la reconstruction incrementale.
- Douze taches M0-M11 sont creees dans `tasks/`, initialement dans `todo`. Ne pas modifier leur lane : elle appartient a l'utilisateur et a l'extension.
- Ordre conseille : valider M0, puis M1, M2 et M3 avant de commencer M4-M11. Les dependances precises figurent dans chaque tache.
- Chaque tache possede deja son fichier TODO associe. Avant implementation : selectionner la tache et relire sa conversation et sa checklist selon INSTRUCTION.md. La creation du backlog n'autorise pas l'implementation.
- Chaque tranche conserve le DSL et SemanticModel comme source de verite, les IDs et unites types, la provenance et les frontieres de crates. Toute mutation est une commande annulable.
- Tout calcul long est hors thread UI, expose progression/annulation et rejette les resultats de revisions obsoletes. Aucun faux resultat physique, TopOpt, RL ou IA.
- Definition de fini commune : tests unitaires du comportement change, integration verticale, demonstration native du parcours concerne, documentation TECHNICAL.md et ADR si necessaire, puis `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Ne pas commencer FEM/SIMP avant M7, PPO avant M9, ni un LLM ou foundation model reel dans ce backlog. Conserver les providers desactives.
- Agents VS Code dans `.github/agents/` : RoboGen orchestre onze sous-agents ; Architect et QA sont aussi selectionnables directement. Explore est en lecture seule. Protocole, noms exacts et perimetres dans AGENTS.md ; seul le parent met a jour les fichiers Kanban partages pour le travail delegue. Les profils ne changent pas les regles d'autorisation ni les lanes.
- M0 valide localement le 2026-09-22 : trois gates Rust passes, 21 tests, build desktop et capture X11/RTX3060 `docs/screenshots/m0-native.png`. CI existante complete mais non executee sur GitHub. Toutes les crates heritent des lints. Architecture/ADR distinguent desormais cible et implementation. L'utilisateur autorise la suite des tickets dans l'ordre, sans changer leurs lanes.
- Git initialise dans RoboGen apres M0 et pendant la preparation M1, independamment du depot parent Documents. Premier commit = instantane initial, non cloture M1. L'utilisateur autorise un commit local apres chaque ticket valide, avant le suivant ; seul le parent commit, aucun push. PDFs scientifiques locaux ignores ; catalogue/notices suivis.

- 2026-10-07 : demande autorisee de creation des tickets et debut du mini bipede.
  Ticket actif 14 (biped-layout) : 17 composants disposes, rotate type/ADR-011,
  exemple mini_biped et menu, 94 tests workspace passes ; correction cadrage
  verifiee ensuite par test UI cible/fmt/Clippy/build et capture native.
  Ticket 15 (biped-constraints-topopt) specifie supports generes depuis contraintes,
  interfaces et domaines ; reutilise M7/M8 et ticket 13. Aucun solveur TopOpt,
  support optimise, marche ou propriete mecanique validee ajoute par le ticket 14.
  Les changements servo 12 preexistants restent non committes et ouverts ; pas
  de commit global attribuant leur completion au ticket bipede.

- 2026-10-07 : l'utilisateur demande explicitement « commit tout » apres la
  livraison du bipede. Autorisation d'un checkpoint local global des travaux
  courants (y compris servo/bibliotheque preexistants), sans cloturer les
  acceptations encore ouvertes, sans changement de lane et sans push.

- 2026-10-07 : ticket 15 iteration 2 livre le contrat topology differe (ADR-012),
  types physiques/IDs, validation semantique, refus CAD/projet E330, inspection
  CLI et exemple topology_battery_support accessible en UI. 103 tests, fmt,
  Clippy, builds et capture native passes. Aucun FEM/SIMP ni support genere ;
  ticket global ouvert. Revision TopOpt worker-scoped, 0 en synchrone autonome.

- 2026-10-08 : commit ee470fd pour topology declaratif, puis choix explicite
  utilisateur « Prioriser M7 autonome » : exception a l'ordre M1-M6 limitee a
  la poutre a charges manuelles. Ticket 15 iteration 3 : faer sparse LLT, Hex8,
  SIMP/filtre/OC, exemple beam et 11 nouveaux tests ; 114 tests workspace et
  fmt/Clippy passes. Benchmark 65 iterations, volume .35, compliance/reference
  .379118 ; aucune surface reconstruite, publication desktop ou support robot.
  E330 reste actif. M5/M8 et les donnees physiques du bipede restent requis.

- 2026-10-08 : ticket 15 iteration 4 livre reconstruction de visualisation
  Surface Nets + STL via exemple, 118 tests/fmt/Clippy passes. Deux couches
  exterieures et subdivision trilineaire x2 requises pour le benchmark. Mesh
  ferme de 3 composantes, 208 sommets/404 triangles, volume 13463 mm3 vs densites
  33600 mm3 ; appui/face chargee perdus. Ne pas attribuer compliance du champ
  au mesh. Suite : preserver interfaces/chemins de charge et reanalyser avant
  publication mecanique. Desktop/TopologySpec restent indisponibles (E330).

- 2026-10-08 : ticket 15 iteration 5 ajoute masques Design/Preserve/Void,
  frontiere voxel exacte et reanalyse sans rigidite du vide. 124 tests/fmt/Clippy
  passent. Nouveau benchmark explicite volume .85/seuil .65 : 80/96 cellules,
  interfaces conservees, compliance binaire 2.964559539151e-7 J, STL ferme.
  Seuil .5 depasse budget, .75 deconnecte : refus testes. Aucun ajustement cache,
  aucune garantie generale ni certification. Objectif global actif, DSL/desktop
  E330 toujours en attente d'adaptateur TopologySpec, contraintes et publication.

- 2026-10-08 : ticket 15 iteration 6 generalise le coeur FEM GridModel et SIMP
  a 1..16 cas identifies (appuis/forces distincts, somme compliance/derivees,
  analyses individuelles). BeamModel/optimize_beam sont des adaptateurs, pas un
  second solveur. Reanalyse binaire tous cas/progression/annulation ; controle
  des modes rigides. 130 tests/fmt/Clippy passes, integration charges opposees
  jusqu'au STL. Projecteur geometries/interfaces DSL et publication encore
  absents : objectif global actif, E330 conserve, aucun support robot revendique.

- 2026-10-08 : ticket 15 iteration 7 execute un TopologySpec depuis .rgn via CLI
  optimize-topology : projection CSG conservative, interfaces volumiques,
  forces/moments et fixed/frictionless cartesien, SIMP et reanalyse multi-cas.
  Budget volume/masse et deplacement controles avant STL ; autres capacites
  refusees. Exemple topology_manual : 22 updates, 216 g, 336 triangles, 2 cas.
  139 tests/fmt/Clippy passes ; regression translation corrigee en repere local.
  DSL garde seulement contraintes/geometrie, reglages numeriques externes.
  Objectif global non acheve : composition CAD/desktop, minimisation masse,
  stress/securite/frequence/fabrication et supports robot. E330 reste dans le
  chemin document ; CLI exporte uniquement le noeud choisi dans son repere local.

- 2026-10-08 : correctif VS Code, composant rust-analyzer 1.88.0 installe et declare
  avec rust-src dans rust-toolchain.toml. server.path pointe le proxy local
  .cargo-home/bin/rust-analyzer : la decouverte de l'extension utilise process.env
  avant server.extraEnv. Handshake LSP version 1.88.0/exit 0 verifie ; recharger
  la fenetre VS Code pour appliquer a la session ouverte. Compilateur inchange.

- 2026-10-08 : ticket 15 iteration 8 raccorde TopOpt supporte au worker Project,
  compose maillages sous transforms/couleurs/compound et publie rapports natifs.
  Booleens aval et capacites absentes refuses. Prereglage document fixe [12,4,2],
  filtre12mm/seuil.65/100iterations ; reglages differents CLI/API uniquement.
  144 tests/fmt/Clippy/build/capture1080x680 passes. Utilisateur exige fixations
  exactes et renvoie au DSL servo : il cote boitier/trous/arbre, pas le palonnier,
  cannelures ou vis de retenue. Bipede toujours17composants sans supports generes.
  Objectif global ouvert, changements non committes, aucune certification.

- 2026-10-08 : iteration 9 remplace le calcul automatique par preparation seule
  dans Conception et commande explicite par PartId dans Optimisation. Couches
  domaine/preserve/keepout separees des meshes exportables ; reglages numeriques
  par piece dans UI, charges/materiaux/interfaces via DSL. Worker serialise les
  jobs, conserve les autres resultats, etats distincts et diagnostics par piece.
  Tout changement DSL invalide conservativement tous les resultats sans recalcul.
  Export selectionne en Optimisation ; export global requiert toutes les pieces
  calculees. 149 tests/fmt/Clippy/build/native passes. Contours, pas volumes
  transparents remplis. Pas de nouvelles interfaces/supports robot ni commit.

- 2026-10-08 : priorite utilisateur du ticket 15 : minimiser la masse sous les
  contraintes declarees, pas maximiser la rigidite a volume fixe. Choisir une
  bibliotheque existante selon cette capacite ; Scikit-Topt n'est pas impose.
  Premiere preuve attendue : minimize_mass, masse avant/apres et reanalyse de
  toutes les contraintes sur le solide reconstruit. Aucun moteur ajoute ici.

- 2026-10-08 : iteration 10 du ticket 15 prepare le bipede reel : 17 composants
  inchanges + 11 supports TopOpt incomplets, trois couches, selection CameraSupport.
  TopologySpec schema 2 : Option<MaterialId>, material: unknown, listes vides
  explicites ; E347 avant solveur, aucun fallback DisplayOnly. Launch/export
  bloques, historique teste. Fmt/Clippy/tests workspace/build passent ; capture
  native 1080x680, clics XTest non concluants. Masse minimale/charges/materiaux/
  palonniers et debattements encore ouverts, aucun support calcule ni commit.

- 2026-10-08 : ticket 15 iteration 15, l'utilisateur autorise explicitement les
  estimations pour le PLA. LeftShinSupport seul : PLA_Estimated (1240 kg/m3,
  2 GPa, nu .35, resistance nominale 25 MPa non imposee), forces [10,0,-40] N
  et [0,10,-40] N, fleche .5 mm. Zones de transfert rigide ideales deportees,
  pas de fixations reelles validees ; servos/perçages/exclusions inchanges.
  Projection 24192 cellules a 2.5 mm et resultantes testees ; gates Rust passent.
  UI native actualisee, aucun calcul lance. Les dix autres supports restent
  incomplets ; montage reel, qualification et export toujours non resolus.

- 2026-10-08 : ticket 15 iteration 16 ajoute GPU CUDA explicite aux diagnostics
  natifs minimum de masse (CPU par defaut). CUDA 6.0.0/Krylov 0.8.4, Float64 CG,
  residu reel + erreur retrograde/plancher d'arrondi documentes ADR-008.
  Tibia 24192 cellules, deux iterations : 300.27 s CPU /105.18 s CUDA (~2.85x),
  equivalence masse/deplacements/gradients verifiee, candidat non converge.
  Tests GPU/transport/UI et gates Rust passent ; aucun export mecanique valide.
  Utilisateur demande commit checkpoint ; futur ticket RL exclu, pas de cloture.

- 2026-10-20 : ticket 17 de cadrage de refonte et standards utilisateur ajoutés.
  L'utilisateur demande de réfléchir avant toute session d'édition autonome,
  exclut Julia de la cible, et exige un MVP humanoïde imprimable avec les trois
  fonctions intégrées : génération DSL, TopOpt et locomotion par RL. Rust hôte
  + adaptateurs Python est une proposition, pas encore une décision de stack.
  Matériel de référence à préciser ; marche simulée ne prouve pas marche réelle.
  Baseline cargo test --workspace --offline passée, qualifications ignorées
  Julia/CUDA non relancées. Aucun code métier refactorisé ni session OMO lancée.

- 2026-10-20 : pour le MVP intégré DSL/TopOpt/RL, l'utilisateur choisit explicitement
  de reprendre le mini-bipède actuel et de préciser ses composants : 14 servos,
  carte, batterie, caméra, 11 supports à préparer. Le cadrage technique continue.

- 2026-10-20 : « lance la » autorise la tranche structurelle du ticket 17.
  ADR-013 acceptée : Rust hôte, Python scientifique à qualifier ; Julia exclu
  de la cible, adaptateur historique isolé mais non retiré. Consentement
  DeepSeek explicite reçu ; OMO 5.1.38 session
  ses_ea63790c1ffeXxEUB3v1zZ9NjP échoue Insufficient Balance dès la première
  requête. Aucun audit ni code produit par OMO ; fallback natif disjoint
  Project/UI/masse terminé. Façades publiques et paramètres scientifiques
  inchangés ; sources de ces périmètres sous 350 lignes. Contrôle Cargo Python
  standard en CI : 21 crates, 36 arêtes de production, cycles/frontières gardés.
  Fmt/Clippy workspace, 161 tests réussis et huit qualifications ignorées,
  builds desktop/CLI et quatre tests de frontières passent. Graphify local
  code/Cargo actualisé : 1490 nœuds/3478 arêtes/123 communautés, sans LLM.
  Ticket global reste ouvert pour qualification/remplacement et objectif MVP ;
  aucun commit de ticket partiel, aucun push.

- Ticket 17, preuve native finale : trois captures refactor-*-20261020.png dans
  docs/screenshots, 1440x831, mini-bipède chargé via Exemples et compilation finie.
  Design montre 28 instances ; Optimisation bloque CameraSupport incomplet ;
  Apprentissage conserve simulation/training indisponibles. Captures inspectées,
  processus de test fermé ; aucun nouveau résultat scientifique revendiqué.

- 2026-10-20, ticket 17 iteration 5 : utilisateur autorise le retrait immédiat
  de Julia sans remplaçant et confirme les crédits DeepSeek. ADR-014 remplace
  le maintien transitoire ADR-013 : sources/manifests/viewer Julia et transport/
  superviseur Rust supprimés, tempfile retiré de TopOpt et serde_json dev-only.
  Port diagnose_mass conservé : E348 indisponible, E345 annulation, zéro événement.
  Project conserve E330 contenant E348 avec span ; CLI execute_topology E340
  avant projection ; aucune substitution par la rigidité. Capability false
  désactive lancement/réglages CPU/CUDA en UI, DTO et préparations conservés.
  Solveur Rust de rigidité conservé. Qualifications Julia supprimées avec le
  moteur, remplacées par tests actifs API/worker/UI. Validation finale : fmt,
  Clippy workspace, 163 tests réussis/0 ignoré, builds desktop/CLI, quatre tests
  Python, architecture21/36, Ruff et actionlint passent. CLI refuse sans STL ;
  capture native mass-backend-unavailable-20261020.png inspectée, processus fermé.
  OMO/DeepSeek audit ses_ea6243118ffeG4Fc9WGSP2qvxN et revue
  ses_ea61f65aaffe8pFEKwzEkb8STJ réussis ; revue sans point bloquant, sessions
  lecture seule, implémentation parent/spécialiste natif. Graphe actualisé
  1475 nœuds/3374 arêtes/139 communautés sans LLM. Aucun backend Python ni RL
  qualifié ; ces objectifs restent futurs, aucun résultat scientifique inventé.

- Ticket 17 : l'utilisateur demande explicitement un commit de checkpoint après
  livraison de la refonte/retrait Julia. Autorisation de consigner la tranche
  validée sans clore les objectifs Python/MVP encore ouverts ; aucun push.

- Ticket 17, itération 7 : l'utilisateur demande de corriger l'association des
  zones plein/vide à un objet. ADR-015 écrite : valeurs `Object` réutilisables et
  déclarations `instance` nommées, projections `instance_shape`/`instance_void`/
  `instance_interface`, `ObjectInstanceId` typé et stable (dérivé du nom), type de
  paramètre de composant `Instance`, provenance `instance_references` dans
  SemanticModel (champs serde-default). Les sources Solid existantes restent valides.
  Mini-bipède migré : 14 instances de servo portent une pose unique partagée par
  boîtier/exclusion/contacts, supprimant `pitch_servo`/`roll_servo`/`pitch_contacts`/
  `pitch_exclusion`/`roll_contacts`/`roll_exclusion`. Géométrie compilée identique :
  25968 sommets/51660 triangles, 28 parts/17 maillages/11 préparations (test bipède).
  Dix tests d'instances ajoutés (types, diagnostics, poses distinctes, IDs/provenance,
  seules interfaces choisies, compatibilité Solid, refus bibliothèque). Gates : fmt,
  Clippy workspace, 173 tests réussis/0 ignoré, quatre tests Python, architecture
  21/36, Ruff passent. Changements non committés, en attente d'autorisation.

- Ticket 17, itération 7 commitée : `1a92a8a` « feat(dsl): add object instances
  with shared regions » (19 fichiers, 1133 insertions / 81 suppressions), à la
  demande de l'utilisateur. Aucun push. Itération 8 ouverte : l'utilisateur veut
  voir l'interface `housing` commune sur les 14 servos en permanence (overlay vert
  dédié, distinct des couches TopOpt Conserver/Exclure). Plan validé, exploration
  du chemin de rendu en cours, aucune implémentation encore ; aucune modification
  des TopologySpec, appuis, charges ou résultats d'optimisation prévue.

- Ticket17 itération8bis : après HEAD3a1c90b, correction de l'overlay d'interfaces
  cyan en vert, légende Fixations auto avec nombre d'objets, visible par défaut
  indépendamment du support sélectionné et même sans TopOpt. Les 14 servos
  possèdent chacun leurs deux bagues housing automatiquement projetées ; aucun
  TopologySpec/appui/charge modifié. Deux régressions UI ajoutées (vert peint,
  masquage et présence des 14 overlays pour les 11 sélections). 177 tests workspace
  passent/0 ignoré, fmt/Clippy/builddesktop et contrôles Python/architecture/Ruff/
  actionlint passent. Captures natives servo-auto-fixations-*-20261020.png
  inspectées, nouvelle application laissée ouverte en Optimisation pour revue.

- Ticket17 itération9 : utilisateur confirme trois onglets Design dans l'ordre
  DSL → Graphe cinématique 2D → Vue 3D ; DSL au démarrage. Actions explicites
  d'exemples conservent leur sélection 3D. Vue2D des seuls objets/interfaces
  déclarés, clic vers source valide, dernier modèle valide signalé et lien
  bloqué si DSL invalide. Articulations explicitement indisponibles, aucune
  liaison inférée/inventée. Cahier complet fourni conservé dans
  docs/engineering/MECHANICAL_SYNTHESIS.md, objectif mécanique toujours ouvert.
  179 tests workspace passent/0 ignoré, fmt/Clippy/builddesktop, quatre tests
  Python, frontières21/36, Ruff/actionlint passent. Captures design-tabs-*-
  20261020.png inspectées ; application reconstruite laissée ouverte sur DSL.
