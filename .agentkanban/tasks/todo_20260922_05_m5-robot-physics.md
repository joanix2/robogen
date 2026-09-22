---
title: "Checklist M5 - Robot et physique"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Tranche simple_arm avec physique reelle."
---

Tache : [M5](task_20260922_05_m5-robot-physics.md).

# Iteration 1

- [ ] Definir les contrats RobotModel/SimulationSpec et confirmer ADR-006.
- [ ] Ajouter le DSL minimal deux liens/joint/actuateur et l'exemple simple_arm.
- [ ] Implementer FK, transforms, masse, centre de masse et inertie avec unites.
- [ ] Implementer l'adaptateur Rapier et son mapping prive, collisions, limites et moteur.
- [ ] Raccorder play/pause/step/reset, pas fixe et overlays au desktop sans bloquer l'UI.
- [ ] Tester gravite, limites, invalides et reset ; ajouter le test d'integration vertical.
- [ ] Montrer l'exemple natif, actualiser TECHNICAL.md et executer les trois gates Rust.