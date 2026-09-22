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
