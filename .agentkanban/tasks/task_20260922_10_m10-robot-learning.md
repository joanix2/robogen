---
title: "M10 - Entrainer et evaluer un robot articule"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "RobotEnvironment Rapier, RewardGraph et vue Apprentissage avec metriques reelles."
---

## Conversation

### user

Decliner le jalon RL robotique du cahier des charges.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 12, 13, 17 a 19 et 26.

Responsables : RL et ROBOTICS, avec PHYSICS, UI/RENDER et QA. Dependance : [M9](task_20260922_09_m9-ppo-toy.md).

Plan : RobotEnvironment consomme les specs robot/simulation/task versionnees, construit observations et actions bornees, compile le reward DSL et gere reset/termination. Commencer avec un robot articule simple avant examples/rl_quadruped et l'araignee. Etendre Jacobian/IK et capteurs uniquement selon les besoins de controle, avec tests propres au domaine.

Acceptation : entrainement reel pilote par l'onglet Apprentissage ; terrain, seed, algorithme et budget explicites ; courbes reward/vitesse/chutes/energie alimentees par le run. Replay et gait preview utilisent des snapshots enregistres ; modifier la vitesse de lecture n'altere pas le pas physique. Terrains ou capteurs non supportes restent indisponibles.

Verification : tests observations/actions/reward/reset, integration DSL -> RobotModel -> Rapier -> PPO -> politique -> replay ; evaluation statistique multi-seeds avec baseline. L'UI reste reactive, peut annuler et rejette les publications obsoletes. Les trois gates Rust de M0 passent ; exemples et TECHNICAL.md a jour. IA et foundation model restent desactives.

Checklist : [M10](todo_20260922_10_m10-robot-learning.md).

### user