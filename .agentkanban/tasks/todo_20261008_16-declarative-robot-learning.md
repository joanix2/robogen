---
title: "Checklist - Apprentissage declaratif et backends existants"
lane: todo
created: 2026-10-08T11:41:25Z
updated: 2026-10-08T11:41:25Z
description: "Cadrage, contrats, MVP Isaac Lab/RSL-RL, sim2sim et recherche ulterieure sous autorisation distincte."
---

Tache : [ticket 16](task_20261008_16-declarative-robot-learning.md).

# Iteration 1 - Cadrage et prerequis

- [ ] Verifier M5/M9/M10 et les donnees physiques disponibles ; lister les blocages du bipede sans inventer masses, inerties ou moteurs.
- [ ] Auditer sources, versions, licences distinctes code/poids/datasets et conditions commerciales des candidats.
- [ ] Mesurer compatibilite OS/GPU/VRAM, stockage et cout ; dimensionner les environnements au lieu d'imposer 4096.
- [ ] Reviser ADR-007 et, si necessaire, ADR-006/architecture ; reconcilier les plans M9/M10 Burn/PPO/Rapier avec les adaptateurs externes, sans supprimer leurs gates.
- [ ] Confirmer explicitement l'autorisation de la tranche d'implementation ; maintenir foundation policies et LLM desactives.

# Iteration 2 - Contrats et preparation declarative

- [ ] Definir la syntaxe policy/train/learn et les diagnostics ; aucune execution a l'ouverture, compilation ou modification du document.
- [ ] Definir Robot Learning IR versionnee : morphologie, simulation, tache, observations/actions, rewards, contraintes, terminaisons et randomisations.
- [ ] Fixer unites, reperes, ordre des articulations, normalisation, gains, pas temporels et provenance/revisions ; distinguer penalites et contraintes dures.
- [ ] Definir PolicyArtifact, empreintes de compatibilite, checkpoints et invalidation apres changement du robot.
- [ ] Definir les ports backend et protocole d'execution externe : capacites, progression, annulation, reprise, budgets et diagnostics, sans types tiers publics.

# Iteration 3 - MVP externe, apres prerequis M9

- [ ] Verrouiller un environnement Isaac Lab/RSL-RL et son installation reproductible sans reimplementation PPO/FEM/physique.
- [ ] Valider M9 jouet avec moteur externe, baseline, seeds et budget fixes ; conserver les tests rapides separes des benchmarks longs.
- [ ] Exporter un RobotModel en URDF et ses configurations annexes ; verifier import, axes, articulations, inerties, collisions, moteurs et capteurs.
- [ ] Construire une tache de locomotion avec observations/actions/rewards/reset/terminations et randomisations declares.
- [ ] Fixer avant entrainement les criteres quantifies de vitesse, chutes, energie, violations, robustesse et comparaison a la baseline.
- [ ] Entrainer/evaluer sur plusieurs seeds et seeds reservees ; partir d'un robot simple puis du bipede lorsque ses prerequis sont satisfaits.
- [ ] Verifier checkpoint/reprise et compatibilite ; tester annulation, absence du runtime, crash, sorties invalides et revision obsolete.

# Iteration 4 - Publication et validation sim2sim

- [ ] Implementer le lancement explicite et les metriques reelles dans Apprentissage avec UI reactive et commandes annulables lorsque pertinentes.
- [ ] Evaluer MuJoCo Playground/mjlab ou un adaptateur MuJoCo cible ; comparer les memes conventions et documenter les ecarts sim2sim.
- [ ] Choisir un export d'inference supporte (ONNX/TorchScript selon compatibilite), tester sa parite et le replay RoboGen.
- [ ] Ajouter tests unitaires des conversions/contrats et integration DSL -> robot -> run -> policy -> evaluation ; aucun faux resultat.
- [ ] Actualiser TECHNICAL.md, ADR, exemples et protocole reproductible avec versions, couts et mesures.
- [ ] Passer cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings et cargo test --workspace ; verifier le parcours natif.

# Recherche future - Hors MVP, autorisation distincte obligatoire

- [ ] Evaluer BeyondMimic pour motion tracking et retargeting sur le robot cible.
- [ ] Auditer GEAR-SONIC/BONES-SEED : checkpoints, embodiment supporte, adaptation, licences et ressources.
- [ ] Etudier BFM-Zero et X-WBC, sans activer de backend ni reutiliser des poids avant validation juridique et technique.
- [ ] Evaluer GenBot1K et MorphologyEncoder : donnees physiques, encodage variable, experts/distillation, morphologies reservees et budget.
- [ ] Comparer prior/adaptation courte/residual RL avec apprentissage sans prior a budget comparable.
- [ ] Etudier Light-O1 et GR00T uniquement au niveau objectifs/mouvements, avec contrat explicite vers le controleur bas niveau.
- [ ] Definir un protocole sim-to-real distinct avant tout deploiement physique ; aucune commande moteur autorisee par ce ticket seul.