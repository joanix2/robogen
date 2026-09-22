---
title: "M11 - Finaliser les exports et la qualite desktop"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Formats d'echange, robustesse des projets, rendu et livraison Linux."
---

## Conversation

### user

Decliner le jalon de finition et d'exports du cahier des charges.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 8 a 13, 20 a 27 et 29.

Responsables : UI, CAD/export, RENDER et QA, avec ARCHITECT.
Dependance : [M10](task_20260922_10_m10-robot-learning.md). Ne pas retarder la correction des defauts bloquants des jalons precedents jusqu'a cette tache.

Plan : finaliser STL ASCII/binaire et bundle projet, puis ajouter 3MF/STEP et URDF/MJCF/glTF selon les capacites prouvees des backends. Chaque format constitue une tranche testee distincte. Completer les interactions de rendu V1 restantes (orthographique, edges/vertices, multi-selection et picking faces/aretes/sommets, gizmo, overlays) sans coupler egui au renderer. Revoir accessibilite, raccourcis, diagnostics et performances.

Acceptation : export de selection ou modele complet avec unites, tolerances, bounds et validation de mesh ; reouverture d'un bundle preserve sources/assets et metadonnees. Formats non implementes explicitement indisponibles. Migrations avec fixture et sauvegarde/rollback ; aucun handle backend persiste. Distribution Linux reproductible et parcours Design -> simulation -> TopOpt -> apprentissage documente avec ses limites reelles.

Verification : contrats d'export et reimport quand disponible, projets invalides, sauvegarde interrompue, migrations, tests de picking/camera/commandes et integrations des parcours livres. Captures natives et benchmarks parser, solveur, rebuild, tessellation, rendu, FEM et simulation. Les trois gates Rust de M0 passent ; README, TECHNICAL.md et ADR concernes a jour. Pas de slicer, de LLM ni de features CAD V2/V3 ajoutes implicitement.

Checklist : [M11](todo_20260922_11_m11-polish-export.md).

### user