---
title: "Checklist M7 - FEM et SIMP"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Solveur CPU valide sur une poutre cantilever."
---

Tache : [M7](task_20260922_07_m7-topopt-beam.md).

# Iteration 1

- [x] Definir le benchmark cantilever, conditions aux limites, tolerances et reference a volume comparable.
- [x] Implementer grille hexahedrique, assemblage sparse et elasticite lineaire avec diagnostics de singularite.
- [x] Valider SparseLinearSolver, residus et deplacements sur references connues.
- [x] Implementer SIMP, sensibilites, filtre, optimality criteria et historique de convergence.
- [x] Verifier sensibilites, bornes des densites, fraction volumique et amelioration de compliance.
- [ ] Reconstruire une surface via marching cubes ou bibliotheque eprouvee et verifier le mesh.
- [ ] Raccorder les vrais resultats au desktop avec progression, annulation et rejet des revisions obsoletes.
- [ ] Ajouter integration et benchmark ; actualiser ADR-008, TECHNICAL.md et executer les trois gates Rust.

Preuves au 2026-10-08 : ticket 15 iteration 3, M7 autonome explicitement
autorise. 11 tests numeriques/integration ajoutes, 114 tests workspace et
fmt/Clippy passes. Exemple beam execute, 65 iterations, volume .35 et ratio
compliance/reference uniforme .379118. Documentation et benchmark CPU presents.
Le dernier item reste ouvert pour l'integration incluant surface et UI ; les
tests actuels s'arretent aux densites/deplacements, pas a un maillage exportable.