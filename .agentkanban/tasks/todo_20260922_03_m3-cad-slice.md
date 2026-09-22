---
title: "Checklist M3 - Tranche CAD"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Preuve de bout en bout et coherence entre revisions."
---

Tache : [M3](task_20260922_03_m3-cad-slice.md).

# Iteration 1

- [ ] Executer le test vertical existant et identifier les preuves encore absentes.
- [ ] Verifier chargement/enregistrement de project.rgnproj et validation des chemins sources.
- [ ] Raccorder le mesh a un vrai viewport wgpu, avec camera et picking autre que bounding box seule.
- [ ] Raccorder les editions DSL/GUI aux commandes, undo/redo et selection synchronisee par provenance.
- [ ] Verifier dirty propagation, progression, annulation et rejet des resultats obsoletes hors thread UI.
- [ ] Tester WIDTH 40 -> 70 mm, erreur DSL et undo/redo sur modele, bounds, rendu et STL.
- [ ] Verifier le STL exporte, ses unites et les diagnostics de mesh invalide.
- [ ] Capturer la preuve visuelle native ; actualiser TECHNICAL.md et executer les trois gates Rust.