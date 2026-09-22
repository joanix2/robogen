---
title: "M8 - Integrer la TopOpt aux pieces et charges robotiques"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Charges manuelles ou extraites de simulation, interfaces preservees et re-simulation."
---

## Conversation

### user

Decliner le jalon TopOpt d'une piece robotique du cahier des charges.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 14 a 16, 21 et 23.

Responsables : TOPOLOGY et ROBOTICS, avec PHYSICS, CAD et UI. Dependance : [M7](task_20260922_07_m7-topopt-beam.md).

Plan : relier une region CAD a LoadCase/LoadHistory et SimulationLoadExtractor ; tracer charges, repere, temps et revision vers leur origine. Integrer preserve regions/interfaces, enveloppe et options de fabrication reellement supportees. Exposer comme indisponibles les contraintes non calculees plutot que promettre leur respect.

Acceptation : optimiser une piece de l'araignee avec charge manuelle puis une charge issue d'une trajectoire reelle ; comparer masse, compliance et autres metriques disponibles. Conserver le resultat comme artefact versionne hors DSL ; son application est une commande explicite annulable, suivie d'un rebuild et d'une re-simulation. Aucun remplacement silencieux de la piece source.

Verification : transformations de forces/moments, preservation des interfaces, revision obsolete, persistance du run et integration simulation -> charges -> optimisation -> application -> re-simulation -> undo. Les trois gates Rust de M0 passent ; hypotheses physiques, limites et TECHNICAL.md actualises.

Checklist : [M8](todo_20260922_08_m8-robot-topopt.md).

### user