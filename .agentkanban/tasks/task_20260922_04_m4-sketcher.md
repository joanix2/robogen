---
title: "M4 - Construire le sketcher graphique contraint"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Edition souris des entites 2D avec contraintes, DOF et synchronisation DSL."
---

## Conversation

### user

Decliner le jalon sketcher du cahier des charges en tache executable.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 7, 11, 23 et 25.

Responsables : UI et CAD/contraintes, avec DSL et QA. Dependance : [M3](task_20260922_03_m3-cad-slice.md). Consulter ADR-003 avant de modifier le solveur.

Plan : commencer par point/segment/cercle a la souris et dimensions editables, puis arcs et lignes de construction. Les widgets emettent des commandes ; robogen-sketch porte les entites et robogen-constraints le solveur. Aucun type slvs dans les contrats. Etendre progressivement les contraintes V1, avec diagnostic explicite pour toute contrainte encore non supportee.

Acceptation : creer et dimensionner un profil depuis la GUI, voir DOF et conflits, retrouver le DSL correspondant, extruder et annuler/refaire l'edition. Etablir la couverture de coincident, horizontal/vertical, parallel/perpendicular, tangent, equal_length/radius, distance/x/y, angle, radius/diameter, midpoint, symmetry et fix sans annoncer un support inexistant.

Verification : contrats solveur (sous-contraint, determine, contradictoire, degenere), tests unitaires des commandes et integration GUI -> DSL -> contraintes -> extrusion. Les trois gates Rust de M0 passent ; demonstration native et limites documentees dans TECHNICAL.md.

Checklist : [M4](todo_20260922_04_m4-sketcher.md).

### user