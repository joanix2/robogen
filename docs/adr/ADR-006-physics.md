# ADR-006 — Backend physique

- Statut : accepte pour le MVP robotique
- Date : 2026-09-22

## Contexte

RoboGen a besoin de corps rigides 3D, collisions, joints, moteurs, contacts, capteurs et terrains. Le meme modele robotique doit pouvoir etre simule par un autre moteur plus tard, notamment pour la precision ou l'entrainement massif.

## Options considerees

- Rapier3D : Rust natif, Apache-2.0, corps/joints/contacts/capteurs et determinisme optionnel.
- PhysX/Bullet via FFI : couverture mature, mais build natif, `unsafe`, ABI et packaging multiplateforme.
- moteur maison : controle total, cout et risque incompatibles avec le MVP.
- simulateur externe : utile pour validation haute fidelite, trop lourd pour l'interaction desktop initiale.

## Decision (cible normative)

Rapier3D est le backend initial derriere `PhysicsBackend`. `RobotModel` et `SimulationSpec` ne stockent aucun handle Rapier. Un `RapierWorldMap` ephemere traduit les IDs RoboGen vers corps, colliders et joints Rapier.

Le monde avance avec un pas fixe configure, un ordre d'insertion stable et une seed explicite pour les composants stochastiques. Les commandes moteur sont appliquees avant le step ; un `PhysicsSnapshot` restitue poses, vitesses, contacts, forces et mesures de capteurs dans les unites RoboGen.

Les geometries de collision sont distinctes des maillages visuels. Les maillages dynamiques concaves sont refuses ou decomposes explicitement. Les overlays de debug consomment le snapshot, pas l'API Rapier.

## Etat implemente au 2026-09-22

[robogen-physics](../../crates/robogen-physics/src/lib.rs) expose un port squelettique : `PhysicsBackend::build_world` recoit un `RobotModel`, retourne un type associe `World`, et `step` recoit un `dt_seconds: f32` sans snapshot de sortie. `UnavailablePhysics` retourne une erreur d'indisponibilite pour les deux operations.

Il n'y a pas d'adaptateur Rapier, de `RapierWorldMap` ou de chaine `SimulationSpec` -> `PhysicsSnapshot` implementee dans ce port. Le pas fixe, les commandes moteur, contacts, capteurs et garanties de reproductibilite ci-dessus restent des exigences M5 (responsables ROBOTICS et PHYSICS), a prouver par les scenarios de contrat. Le choix de Rapier n'atteste donc aucune simulation fonctionnelle actuelle.

## Consequences

- integration pure Rust et deploiement simple ;
- simulation interactive et reproductible localement sous conditions controlees ;
- le determinisme inter-machines n'est pas suppose : il est active et teste pour les scenarios qui l'exigent ;
- la fidelite de contact, d'actionneur et de friction doit etre calibree avant toute conclusion d'ingenierie ;
- reconstruire le monde est necessaire apres certains changements structurels.

## Strategie de remplacement

Les scenarios de contrat decrivent pendule, chute, contact, moteur limite et chaine articulee avec tolerances physiques. Un nouveau backend traduit `SimulationSpec` et rend `PhysicsSnapshot`. Les logs de runs enregistrent backend/version afin de ne pas comparer silencieusement deux moteurs.

Ajouter un backend externe ou haute fidelite si les besoins de contact, soft-body, differentiabilite ou debit RL depassent Rapier. Aucun projet ne migre : seuls les caches et runs sont recalcules.

## References

- <https://rapier.rs/docs/>
- <https://rapier.rs/docs/user_guides/rust/determinism/>

