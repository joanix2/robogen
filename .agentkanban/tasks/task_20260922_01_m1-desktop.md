---
title: "M1 - Consolider le shell desktop selon la maquette"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Valider les trois vues natives et leur fidelite visuelle sans faux calculs."
---

## Conversation

### user

Decliner le jalon UI du cahier des charges en tache executable.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 9 a 11, 14, 17, 19, 22 et 30 ; maquette jointe a la conversation.

Responsables : UI et RENDER. Dependance : [M0](task_20260922_00_m0-foundations.md).

Le shell sombre Design/Optimisation/Apprentissage est documente comme existant. Auditer et completer cette base native Rust, sans creer une application web ou trois modeles independants.

Plan : barre RoboGen et workflow, taxonomie issue du domaine, viewport dominant, onglet DSL, assistant desactive, panneaux d'optimisation et d'apprentissage. Les actions non branchees sont explicitement indisponibles ; aucune courbe ni metrique ne doit simuler un calcul reel. L'historique assistant de demonstration doit etre identifie comme tel.

Acceptation : navigation fonctionnelle, interface lisible apres redimensionnement, controles raccordes a un etat explicite, captures des trois vues dans docs/screenshots. Assistant IA et foundation model non configures, solver et RL indisponibles tant que leurs jalons ne sont pas livres. Les mutations branchees passent par des commandes annulables.

Verification : tests unitaires des etats, integration de navigation/commandes, lancement desktop et inspection visuelle ; les trois gates Rust de M0 passent. Documenter les limites dans TECHNICAL.md.

Checklist : [M1](todo_20260922_01_m1-desktop.md).

### user

ajoute git git ignore et commit entre chaque ticket

### agent

Initialisation d'un depot Git propre a RoboGen, distinct du depot detecte dans le dossier parent Documents. Git et l'identite utilisateur sont deja configures ; aucune configuration globale ni depot distant modifie.

Exclusions ajoutees pour les toolchains locales, builds, environnements Python, caches Graphify, logs, fichiers .env et export STL de demonstration. Le corpus scientifique PDF de 409 Mo reste local ; son catalogue et ses notices restent versionnables. Cargo.lock, profils d'agents, CI, sources, tests et taches restent suivis.

La regle ajoutee a AGENTS.md impose un commit local apres chaque ticket valide et avant le suivant, avec verification du diff et des tests ; seul l'orchestrateur ecrit dans Git. Aucun push autorise implicitement. Le premier commit est un instantane initial, pas une cloture retroactive de M1.

Etat lors de l'initialisation : M0 valide ; M1 en cours. Les delegues ont ajoute ComponentTaxonomy dans le domaine et SourceDocument avec historique undo/redo dans le projet, ainsi qu'un test vertical 40 -> 70 mm -> undo/redo -> STL. Ces ajouts ne sont pas encore raccordes au shell ; M1 ne doit pas etre coche termine.

### user