---
title: "M5 - Simuler deux liens et un joint revolute"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Premier RobotModel independant du backend, cinematique et simulation Rapier."
---

## Conversation

### user

Decliner le jalon robotique et physique du cahier des charges en tache executable.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 12, 13, 24 et 26.

Responsables : ROBOTICS et PHYSICS, avec DSL, RENDER et QA. Dependance : [M4](task_20260922_04_m4-sketcher.md). Lire ADR-006 avant l'adaptateur Rapier.

Plan : creer la tranche examples/simple_arm avec declarations DSL, deux liens, joint revolute, inertie et actuateur types. Projeter RobotModel vers SimulationSpec puis PhysicsBackend ; garder les handles Rapier dans un mapping prive. Ajouter FK et rendu de l'axe, commandes de configuration et controles de simulation a pas fixe.

Acceptation : gravite, collisions, limites et moteur produisent une simulation reelle ; reset reproductible pour configuration/seed fixes ; l'etat physique ne reecrit pas silencieusement le DSL. Afficher centre de masse, axe, collisions et contacts disponibles. Entrees physiques invalides retournees comme diagnostics avec provenance.

Verification : tests unitaires FK, masse/inertie et mapping ; integration DSL -> RobotModel -> Rapier -> snapshot/rendu. Tests de limites, pas fixe et reset ; demonstration desktop. Les trois gates Rust de M0 passent et TECHNICAL.md est a jour. PPO reste indisponible.

Checklist : [M5](todo_20260922_05_m5-robot-physics.md).

### user