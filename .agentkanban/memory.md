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
