---
title: "M3 - Prouver la tranche CAD live jusqu'au STL"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Rectangle contraint, extrusion, vrai rendu wgpu et rebuild WIDTH 40 vers 70 mm."
---

## Conversation

### user

Decliner la premiere preuve verticale du cahier des charges en tache executable.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 6, 8, 9, 20 a 26 et 30.

Responsables : CAD et RENDER, avec DSL, UI et QA. Dependance : [M2](task_20260922_02_m2-dsl.md).

Le pipeline rectangle/extrusion/STL et un test vertical existent. Ce test construit une preview et compte les triangles ; il ne prouve pas a lui seul le rendu GPU ni le rebuild asynchrone. Verifier ces points explicitement sans refaire les composants deja fonctionnels.

Plan : charger constrained_bracket, resoudre le sketch, extruder via CadKernel, produire une RenderScene generique et l'afficher par wgpu sans dependance egui dans le renderer. Raccorder edition DSL et inspecteur a des commandes undo/redo, rebuild incremental et selection synchronisee. Le TaskManager gere progression, annulation et rejet des revisions obsoletes ; evenements types pour la publication.

Acceptation : WIDTH passe de 40 a 70 mm depuis texte et GUI ; dimensions, mesh affiche et STL changent de facon coherente ; undo/redo restaure chaque etat. Une erreur DSL ne remplace pas silencieusement le dernier modele valide. Un resultat ancien ne remplace jamais le document courant. Ouverture/enregistrement du projet preserves ; export invalide diagnostique.

Verification : integration DSL -> solveur -> CAD -> RenderScene -> STL, tests de revisions/annulation/undo, validation des bounds et unites, preuve visuelle du rendu wgpu et controles orbit/pan/zoom/picking deterministe. Les trois gates Rust de M0 passent ; TECHNICAL.md decrit les limites du backend natif.

Checklist : [M3](todo_20260922_03_m3-cad-slice.md).

### user