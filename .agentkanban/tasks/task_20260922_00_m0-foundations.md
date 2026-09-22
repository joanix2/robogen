---
title: "M0 - Valider les fondations et la CI"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Auditer le workspace existant, les frontieres et les gates avant de poursuivre."
---

## Conversation

### user

Creer les taches pour poursuivre RoboGen selon docs/init-prompt.md et la maquette fournie.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 1, 2, 21, 24, 27 a 30.

Responsables : ARCHITECT et QA. Dependances : aucune. Premiere tache a reprendre.

Le workspace, les ADR et une tranche rectangle/extrusion existent deja d'apres README.md et TECHNICAL.md. Leur presence ne vaut pas validation : ne pas les recreer ni supprimer l'existant. Le test vertical actuel verifie 12 triangles et la taille du STL, pas une execution GPU.

Plan : etablir un bilan reproductible, confirmer les limites de crates et rendre les controles executables en CI. Documenter les ecarts reels avant toute modification de frontiere ou de backend.

Acceptation : desktop compilable et lancable sous Linux ; CI executant `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` ; ADR-001 a ADR-009 coherents avec les implementations et leurs limites ; TECHNICAL.md actualise. Ne pas declarer un controle passe sans l'avoir execute.

Checklist : [M0](todo_20260922_00_m0-foundations.md).

Les 12 taches M0 a M11 et leurs checklists constituent la feuille de route demandee. Elles commencent dans `todo`, sans attester que les jalons sont termines. Aucun code applicatif n'est modifie par cette planification. Selectionner ensuite une tache avec `@kanban /task` et utiliser `implement` pour commencer son execution.

### user

### agent

Creation du backlog terminee : 12 taches et 12 checklists associees, avec dependances, responsables, perimetres et criteres d'acceptation. Verification automatique reussie des comptes, metadonnees, colonnes `todo`, liens locaux et associations tache/checklist. Les doublons crees pendant la preparation ont ete retires ; aucun fichier preexistant n'a ete supprime. Le contexte est conserve dans ../memory.md. Aucun test Cargo execute pour ce changement documentaire ; aucune implementation de jalon declaree terminee. Prochaine action : selectionner M0 avec `@kanban /task`, puis envoyer `implement`.

### user

intall graphify et des linter

### agent

Installation d'outillage demandee explicitement, sans implementation applicative : lien Graphify casse vers une ancienne revision Snap remplace via uv ; package graphifyy 0.9.15 aligne sur la skill existante. Commande `graphify --version` validee, aucun graphe genere. Clippy 0.1.88 et rustfmt 1.8.0 etaient deja installes dans la toolchain Rust 1.88.0 locale.

Extension rust-lang.rust-analyzer 0.3.3057 installee. Configuration VS Code ajoutee pour fournir CARGO_HOME, RUSTUP_HOME et PATH aux nouveaux terminaux Linux et au serveur rust-analyzer, avec Clippy comme commande de controle. Les deux environnements ont ete testes avec les commandes de version. Documentation ajoutee dans TECHNICAL.md.

Controles globaux executes : `cargo fmt --check` signale un ecart de formatage dans robogen-ui ; `cargo clippy --workspace --all-targets -- -D warnings` est bloque par 12 erreurs E0425 dans cette meme crate (fonctions UI manquantes). Aucun code Rust modifie pour masquer ou corriger ces problemes hors du perimetre de l'installation. M0 n'est pas declare valide.

### user

creer les agents et sous agents et modifie le AGENTS.md

### agent

Personnalisation du projet executee selon la demande : 12 profils VS Code crees dans `.github/agents/`. Portee workspace retenue pour partager les roles RoboGen avec le depot ; aucun profil utilisateur modifie.

- Agents selectionnables : RoboGen (orchestrateur), RoboGen Architect et RoboGen QA.
- Sous-agents : les deux specialistes precedents, plus Explore, DSL, CAD, Render, Robotics, Physics, Topology, RL et UI.
- Seul l'orchestrateur peut deleguer, avec une liste explicite de onze noms. Les specialistes ne peuvent pas deleguer a nouveau ; Explore est limite aux outils de lecture/recherche.
- AGENTS.md enrichi avec liens vers les profils, protocole de delegation, propriete des fichiers, coordination Kanban, limites de milestones et role PHYSICS. Bloc Kanban protege conserve exactement. Les regles de propriete ne constituent pas un sandbox systeme.
- TECHNICAL.md documente l'utilisation et les limites. Aucun code applicatif, modele IA produit, backend, lane ou choix de modele Copilot modifie.

Validation automatique reussie : parsing YAML des 12 profils, noms uniques, resolution de la liste de delegation, restrictions d'outils, visibilite, liens locaux et bloc protege. Un titre de role duplique introduit pendant l'edition a ete retire avant le controle final. La decouverte effective dans le selecteur VS Code et l'execution d'une delegation ne sont pas attestees par ces controles statiques. Recharger la fenetre si les nouveaux profils n'apparaissent pas. Aucun jalon applicatif declare termine et aucun test Cargo requis pour ces seuls fichiers de personnalisation/documentation.

### user

prend les tache dans l'odre et implemente les tickets 1 par 1

### agent

Autorisation de traiter les tickets sequentiellement recue. M0 traite en premier, avec delegation effective a RoboGen Architect (audit puis documentation) et RoboGen UI (reparation de compilation), sans ecritures concurrentes sur les memes fichiers.

Bilan M0 : termine localement. Le depot ne contient pas de repertoire .git ; aucun commit ni reset effectue. La CI Linux existante contient deja les trois gates. Onze manifestes de crates ont ete raccordes aux lints du workspace ; les unwrap des tests concernes ont ete remplaces par propagation d'erreurs ou assertions explicites. Les neuf helpers de presentation UI manquants ont ete restaures, avec couverture headless des vues et etats indisponibles.

Preuves : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` reussis, 21 tests passes. `cargo build -p robogen-desktop` reussi. Fenetre native X11 1440x831 dessinee via eframe/wgpu sur NVIDIA RTX 3060 ; capture inspectee dans docs/screenshots/m0-native.png. Les avertissements du pilote Vulkan n'ont pas empeche ce rendu. Aucun resultat heberge GitHub Actions n'est revendique.

Architecture et ADR corriges pour separer garanties cibles et implementations actuelles : projection CPU/egui, solveur rectangle limite, absence de CST/trivia, commandes et tasks asynchrones, provenance partielle, ports physiques/RL encore indisponibles. Les dependances inter-crates actuelles sont documentees et restent a clarifier en M3. Aucun backend futur implemente dans M0. Checklist cochee avec ces preuves ; lane inchangee selon les regles Kanban. Passage au ticket M1.

### user