---
title: "Checklist M8 - TopOpt robotique"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Boucle charges, optimisation et re-simulation tracee."
---

Tache : [M8](task_20260922_08_m8-robot-topopt.md).

# Iteration 1

- [ ] Definir les contrats LoadCase, LoadHistory, SimulationLoadExtractor et leur provenance.
- [ ] Extraire des charges reelles avec reperes, unites et hypotheses explicites.
- [ ] Projeter la piece CAD et les interfaces preservees vers le domaine FEM.
- [ ] Raccorder les contraintes supportees et signaler les autres comme indisponibles.
- [ ] Stocker runs et maillages sous generated avec revisions, options et versions de backend.
- [ ] Ajouter comparaison avant/apres et application du resultat par commande annulable.
- [ ] Tester toute la boucle jusqu'a la re-simulation, y compris annulation et resultats obsoletes.
- [ ] Actualiser TECHNICAL.md et executer les trois gates Rust.