---
title: "M7 - Optimiser reellement une poutre par FEM et SIMP"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Elasticite 3D CPU, solveur sparse, densites SIMP et reconstruction."
---

## Conversation

### user

Decliner le premier jalon d'optimisation reelle du cahier des charges.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 14 a 16 et 26.

Responsables : TOPOLOGY et QA, avec UI/RENDER. Dependance : [M6](task_20260922_06_m6-spider.md). Lire ADR-008 avant les choix numeriques.

Plan : examples/topopt_beam, grille hexahedrique 3D, elasticite lineaire, SparseLinearSolver remplacable, interpolation SIMP, sensibilites, filtre de densite, mise a jour optimality criteria, convergence et extraction de surface. CPU d'abord, rayon si pertinent ; pas de solveur sparse GPU maintenant.

Acceptation : cas cantilever calcule avec conditions aux limites, charges et unites explicites ; volume respecte a tolerance documentee et compliance amelioree par rapport a une reference de meme budget matiere. Ne pas exiger une compliance inferieure a la poutre pleine. L'UI affiche uniquement les mesures du run, ses hypotheses et sa convergence, avec progression/annulation et revision source.

Verification : cas analytiques et invariants FEM, sensibilites comparees aux differences finies, residus solveur, budget de volume et integration poutre -> SIMP -> maillage -> comparaison. Distinguer densite, deplacement et Von Mises effectivement calcule. Les trois gates Rust de M0 passent ; benchmark CPU, ADR-008 et TECHNICAL.md actualises.

Checklist : [M7](todo_20260922_07_m7-topopt-beam.md).

### user

### agent

2026-10-08 : dans le ticket actif 15, l'utilisateur a explicitement choisi
« Prioriser M7 autonome », sans attendre M1-M6, pour une poutre a charges
manuelles uniquement. Aucun jalon precedent ni charge robotique n'est valide
implicitement. Implementation et preuves detaillees dans l'iteration 3 du
ticket 15 et examples/topopt_beam/README.md.

Noyau scientifique livre : grille Hex8/elasticite, faer LLT, SIMP/filtre/OC,
progression/annulation et requete versionnee par revision caller. 11 nouveaux
tests ; 114 tests workspace, fmt et Clippy -D warnings passes. Poutre 96 cellules,
volume .35, 65 iterations jusqu'au seuil de variation .005, compliance finale
2.377550262401e-6 J contre 6.271263227758e-6 J pour la reference uniforme.

M7 reste ouvert : pas de reconstruction de surface, export verifie ou resultat
desktop ; pas de chemin complet poutre -> mesh -> comparaison UI. Le backend
TopologySpec reste indisponible et ne transforme pas ces densites en support
robotique. Les champs physiques affiches ne comprennent pas de Von Mises.

### user