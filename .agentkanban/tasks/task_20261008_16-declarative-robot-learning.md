---
title: "Apprentissage declaratif - Isaac Lab, RSL-RL et politiques multi-morphologies"
lane: todo
created: 2026-10-08T11:41:25Z
updated: 2026-10-08T11:41:25Z
description: "Compiler le robot et ses comportements vers des moteurs existants ; MVP RL externe, validation sim2sim et recherche foundation policies separee."
---

## Conversation

### user

Creer un ticket a partir de la proposition fournie : appliquer a l'apprentissage
la meme philosophie qu'a l'optimisation topologique. RoboGen ne doit pas
reimplementer PPO, un simulateur physique ou un controleur humanoide depuis zero.
Il decrit le robot, les taches et les objectifs, puis compile vers des briques
existantes. Design, optimisation et apprentissage sont des projections d'un meme
modele declaratif, et non des applications independantes.

MVP souhaite : DSL RoboGen -> URDF -> Isaac Lab -> RSL-RL PPO -> politique de
locomotion. MuJoCo sert a la validation sim2sim. Ensuite, evaluer BeyondMimic pour
le suivi de mouvement, SONIC comme prior, puis MorphologyEncoder, GenBot1K et la
distillation cross-embodiment. Les pistes BFM-Zero, X-WBC, Light-O1 et GR00T sont
conservees comme recherche, avec leurs contraintes de licence et de ressources.

### agent

## Objectif et statut

Ticket de cadrage cree a partir du texte utilisateur, sans implementation ni
verification externe des annonces citees. La cible est un compilateur robotique :
intention -> geometrie et morphologie -> probleme de simulation/apprentissage ->
controleur versionne. Reutiliser les moteurs scientifiques existants, conserver
les contrats RoboGen et rendre les capacites indisponibles explicites.

Responsables : RL et ROBOTICS ; ARCHITECT pour contrats/backends, PHYSICS pour la
conformite des simulations, UI pour Apprentissage, QA pour les preuves mesurees.

Checklist : [ticket 16](todo_20261008_16-declarative-robot-learning.md).

## Dependances et autorisations

- [M5 robot/physique](task_20260922_05_m5-robot-physics.md) : RobotModel articule,
  inerties, actionneurs, capteurs et SimulationSpec suffisamment complets.
- [M9 RL jouet](task_20260922_09_m9-ppo-toy.md) puis
  [M10 apprentissage robot](task_20260922_10_m10-robot-learning.md) : conserver leurs
  gates de validation et l'ordre des prerequis, sans dupliquer ces jalons.
- [Ticket 15](task_20261007_15-biped-constraints-topopt.md) : articulation avec la
  preparation declarative, les revisions et l'optimisation mecanique. Le placement
  des composants du bipede ne prouve pas un robot dynamique complet ; les masses,
  COM, inerties, liaisons et caracteristiques moteurs manquantes sont des blocages.
- Les anciens plans Burn/PPO maison et RobotEnvironment Rapier doivent etre
  reconcilies explicitement avec cette direction dans ADR-007, ADR-006 si utile,
  les TODO M9/M10 et l'architecture avant implementation. Ne pas les modifier ou
  les considerer remplaces par la seule creation de ce ticket.
- Aucun PPO avant M9. Les foundation policies et LLM restent desactives dans le
  backlog actuel ; leur etude ci-dessous n'autorise aucune activation. Une phase
  ulterieure exige une autorisation distincte et des prerequis verifies.
- Creation du ticket uniquement : pas de dependance installee, telechargement de
  poids/dataset, entrainement, deploiement moteur ou modification de lane existante.

## Contrat declaratif cible

Syntaxe a concevoir, non disponible aujourd'hui : une declaration `policy`, ou
une operation `train()` / `learn()`, produit une specification puis un artefact
`Policy`, jamais un Solid. Design produit un robot ; optimisation produit un
robot revise ; apprentissage produit un controleur associe a cette revision.

Le DSL exprime :

- Robot cible et comportement, par exemple marche avec consigne de vitesse.
- Observations : positions/vitesses articulaires, IMU, contacts des pieds,
  consigne et eventuelles informations morphologiques.
- Actions : cibles articulaires ou autres modes explicitement supportes ; bornes,
  ordre des moteurs, gains, frequence de controle et conventions declares.
- Objectifs : suivre la vitesse, rester droit, limiter energie et mouvements
  brusques ; rewards, reset et terminaisons definis sans ambiguite.
- Contraintes : limites articulaires, couples et auto-collision. Une penalite de
  reward n'est pas une garantie dure de respect ; distinguer les deux contrats.
- Randomisation : masses, friction, force moteur, bruit capteur, avec distributions
  et bornes physiques explicites. Les pourcentages de la proposition sont des
  exemples, pas des valeurs calibrees ni une autorisation de fabriquer des donnees.
- Configuration de run : backend, algorithme existant, seeds, budget et nombre
  d'environnements. 4096 environnements est un exemple a dimensionner, pas une
  exigence garantie sur le GPU local.

Compilation et ouverture du document preparent/valident uniquement. Un lancement
explicite depuis Apprentissage cree le run hors thread UI. Progression, annulation,
reprise, erreurs et rejet des publications obsoletes sont obligatoires.

## Architecture cible

```text
DSL + SemanticModel
  -> RobotModel / morphologie + TaskSpec
  -> Robot Learning IR versionnee
  -> export URDF (MVP), MJCF/USD selon capacites verifiees
  -> Isaac Lab + RSL-RL (entrainement)
  -> PolicyArtifact + evaluation
  -> MuJoCo (validation sim2sim)
  -> export d'inference supporte + replay RoboGen
```

L'IR separe robot, tache, simulation et entrainement : IDs types, unites/reperes,
graphe cinematique, inerties, collisions, actionneurs, capteurs, observations,
actions, rewards/terminaisons, randomisations et budgets. Inclure document/revision,
empreintes d'assets et de configuration, versions de backend et provenance DSL.
URDF seul ne transporte pas toutes les conventions d'actionnement et de tache :
generer les configurations complementaires et valider les imports.

Les adaptateurs encapsulent les types PhysX, MuJoCo, PyTorch et les configurations
Isaac Lab/RSL-RL. Ne pas imposer un moteur unique au modele persistant, ni remplacer
implicitement Rapier pour toutes les fonctions de RoboGen. Privilegier une
execution externe isolee avec protocole borne, supervision des processus et
diagnostics clairs si dependances, GPU ou capacites manquent.

PolicyArtifact contient poids, schema/normalisation des observations, ordre et
echelle des actions, etat recurrent si present, pas temporels, robot compatible,
versions, seeds et metriques d'evaluation. ONNX/TorchScript sont des options a
evaluer selon le moteur ; tester la parite de l'export avec l'inference d'origine.
Une modification de geometrie, masse ou actionneur rend la compatibilite a
revalider, pas automatiquement transferable.

## MVP et preuves attendues

1. Auditer versions, licences, distribution, import robot et budgets materiels ;
   choisir une combinaison Isaac Lab/RSL-RL verrouillee et un runtime isole.
2. Passer M9 sur un environnement jouet et une baseline avec le moteur externe,
   sans reecrire PPO, les returns/advantages ou un simulateur dans RoboGen.
3. Exporter un RobotModel physiquement complet, verifier articulations, axes,
   masses/inerties, collisions, limites et correspondance commandes/capteurs.
4. Executer un entrainement explicite de locomotion, du robot articule simple
   vers le bipede lorsque ses prerequis mecaniques sont disponibles. Evaluer sur
   plusieurs seeds avec seeds d'evaluation reservees, budget declare et baseline.
5. Mesurer suivi de vitesse, chutes, energie, violations et robustesse sur des
   episodes reproductibles ; fixer les seuils avant le run, pas apres les resultats.
6. Tester sauvegarde/reprise, annulation, runtime absent, echec du processus,
   revision obsolete et refus d'une policy incompatible. Aucune courbe simulee.
7. Faire une validation sim2sim MuJoCo avec les memes conventions et tolerances
   explicites. Un succes Isaac Lab ne prouve ni la parite MuJoCo ni le sim-to-real.
8. Livrer inference/replay et metriques dans Apprentissage, avec tests unitaires
   des conversions et integration DSL -> run -> policy -> evaluation. Documenter
   commandes/couts/resultats ; executer fmt, Clippy et tests workspace, puis une
   demonstration native. Les benchmarks longs restent separes des tests rapides.

## Recherche ulterieure, hors acceptation MVP

- BeyondMimic : reutiliser motion tracking et organisation commands/rewards/events/
  observations/terminations ; verifier les besoins en donnees et retargeting.
- GEAR-SONIC : evaluer un prior whole-body et BONES-SEED, sans supposer qu'un
  checkpoint calibre pour Unitree G1 controle directement le mini bipede RoboGen.
- BFM-Zero : etudier latent comportemental et adaptation ; la restriction non
  commerciale signalee par la proposition doit etre verifiee avant tout usage.
- Embodiment Scaling Laws / GenBot1K : evaluer conditioning morphologique,
  experts/distillation et generalisation sur morphologies reservees. Distinguer
  reutilisation de checkpoints, adaptation courte/residual RL et reentrainement
  complet multi-morphologies ; budget de donnees/calcul explicite.
- MorphologyEncoder : masses, longueurs, graphe, axes, limites et couples sont
  des entrees candidates a pi(etat, morphologie, objectif). Verifier disponibilite
  des donnees, encodage, normalisation, masques et dimension variable ; ne pas
  affirmer que le DSL actuel contient deja toutes ces proprietes.
- X-WBC : suivre representation commune et adaptateurs morphologiques comme
  recherche. Publication, depot, checkpoints et performances annonces a verifier.
- Light-O1 et GR00T/VLA : eventuels planificateurs de mouvements/objectifs de haut
  niveau, pas commandes directes des servos. Le raccord a un controleur bas niveau
  exige un contrat explicite de retargeting, frequence et limites de securite.
- Les priors sont une entree optionnelle d'initialisation/adaptation ou un
  controleur reutilise, pas une etape obligatoire apres tout entrainement PPO.
  Evaluer le gain face a un apprentissage sans prior a budget comparable.

Aucun deploiement physique automatique. Le transfert reel necessite calibration,
limites independantes de la politique, arret d'urgence et protocole autorise.
Auditer separement licences du code, poids et datasets, ainsi que leurs conditions
commerciales ; la publicite d'un depot n'est pas une autorisation universelle.

## Sources de la proposition, a verifier

- [Isaac Lab et algorithmes RL](https://github.com/isaac-sim/IsaacLab/blob/develop/docs/source/concepts/reinforcement_learning.rst)
- [Isaac Lab architecture/imports](https://isaac-sim.github.io/IsaacLab/main/source/refs/reference_architecture/index.html)
- [Isaac Lab](https://github.com/isaac-sim/IsaacLab) ; alternatives a auditer : MuJoCo Playground et mjlab.
- [Embodiment Scaling Laws, papier](https://arxiv.org/abs/2505.05753) et [code/GenBot1K](https://github.com/BoAi01/embodiment-scaling-laws)
- [GEAR-SONIC documentation](https://github.com/NVlabs/GR00T-WholeBodyControl/blob/main/docs/source/index.rst) et [README/licences](https://github.com/NVlabs/GR00T-WholeBodyControl/blob/main/README.md)
- [GRAIL/SONIC et BONES-SEED](https://github.com/NVlabs/GRAIL/blob/main/imports/SONIC/docs/source/index.rst)
- [Light-O1](https://github.com/lightorigins/Light-O1)
- [BFM-Zero, papier](https://arxiv.org/abs/2511.04131) et [code](https://github.com/LeCAR-Lab/BFM-Zero)
- [X-WBC, reference fournie](https://arxiv.org/abs/2609.15213) et [organisation fournie](https://github.com/LogosRoboticsGroup)
- [Isaac-GR00T](https://github.com/NVIDIA/Isaac-GR00T)
- [Unitree RL Lab](https://github.com/unitreerobotics/unitree_rl_lab)
- [BeyondMimic / whole body tracking](https://github.com/HybridRobotics/whole_body_tracking)

Les versions, nombres de morphologies/mouvements, performances zero-shot et
licences du texte initial sont des assertions a auditer, pas des resultats RoboGen.

### user