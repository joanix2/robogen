---
title: "Checklist M7 - FEM et SIMP"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Solveur CPU valide sur une poutre cantilever."
---

Tache : [M7](task_20260922_07_m7-topopt-beam.md).

# Iteration 1

- [ ] Definir le benchmark cantilever, conditions aux limites, tolerances et reference a volume comparable.
- [ ] Implementer grille hexahedrique, assemblage sparse et elasticite lineaire avec diagnostics de singularite.
- [ ] Valider SparseLinearSolver, residus et deplacements sur references connues.
- [ ] Implementer SIMP, sensibilites, filtre, optimality criteria et historique de convergence.
- [ ] Verifier sensibilites, bornes des densites, fraction volumique et amelioration de compliance.
- [ ] Reconstruire une surface via marching cubes ou bibliotheque eprouvee et verifier le mesh.
- [ ] Raccorder les vrais resultats au desktop avec progression, annulation et rejet des revisions obsoletes.
- [ ] Ajouter integration et benchmark ; actualiser ADR-008, TECHNICAL.md et executer les trois gates Rust.