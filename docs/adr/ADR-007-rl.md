# ADR-007 — Backend d'apprentissage par renforcement

- Statut : accepte comme direction, implementation differee au milestone 9
- Date : 2026-09-22

## Contexte

RoboGen doit entrainer plus tard des politiques de locomotion en Rust, d'abord par PPO, sur des environnements Rapier vectorises. Le milestone courant ne doit ni implementer PPO ni simuler de resultats. La pile doit rester remplacable et permettre CPU puis GPU.

## Options considerees

- Burn avec boucle PPO detenue par RoboGen : Rust natif, backends tenseur CPU/WGPU/CUDA, controle du format de run.
- `rl4burn` : composants PPO existants sur Burn, mais projet jeune et surface susceptible d'evoluer.
- bindings Python/PyTorch : ecosysteme mature, mais runtime et packaging distincts de l'application native.
- Candle : pile Rust attractive, sans avantage decisif ici face aux backends et a l'autodiff Burn retenus.

## Decision (cible normative)

Definir maintenant les ports et schemas (`Environment`, espaces d'observation/action, `RewardGraph`, `RlBackend`, configuration, metriques, checkpoints), avec `UnavailableRlBackend` dans l'application.

Au milestone 9, implementer un PPO minimal dont RoboGen possede la boucle d'entrainement et les tests, sur Burn. Commencer par un environnement jouet CPU et des environnements synchrones vectorises ; connecter un robot Rapier seulement apres validation statistique. Le backend Burn (`NdArray`, WGPU ou CUDA futur) est un parametre de l'adaptateur et ne fuit pas dans le domaine.

`rl4burn` sert de reference et peut fournir des composants seulement apres un spike de compatibilite, licence, maintenance, sauvegarde et reproductibilite. Il ne devient pas une dependance architecturale du format de politique.

Chaque run enregistre seed, versions, hash environnement/reward, normalisation, hyperparametres, metriques et checkpoint. Les tests verifient les calculs deterministes et une tendance statistique sur plusieurs seeds, jamais un score exact unique.

## Etat implemente au 2026-09-22

[robogen-rl](../../crates/robogen-rl/src/lib.rs) definit seulement le trait `Environment` (`reset`/`step`, types associes observation/action), `Step` (observation, reward, terminated), `RlBackend::train` sans configuration et `UnavailableRlBackend`, qui retourne `RlError::Unavailable`.

Ces ports sont squelettiques : les schemas d'espaces, `RewardGraph`, rollouts, configuration de run, metriques et checkpoints annonces ci-dessus ne sont pas implementes dans cette crate. Il n'y a ni adaptateur Burn, ni PPO, ni environnements vectorises executables. Le responsable RL doit completer et tester ces contrats au M9 avec le PPO jouet ; aucun entrainement ni resultat fictif ne doit etre expose avant cela. La direction Burn et l'implementation differee restent inchangees.

## Consequences

- aucune complexite ML ne bloque les milestones CAD ;
- Burn offre une trajectoire CPU/GPU sans imposer de types tenseur au domaine ;
- posseder PPO augmente l'effort de validation numerique ;
- le debit Rapier vectorise et les transferts CPU/GPU devront etre mesures avant optimisation ;
- les politiques foundation restent derriere `FoundationPolicyProvider` avec implementation non configuree.

## Strategie de remplacement

Le contrat se situe aux frontieres environnement, politique serialisee, checkpoint et metriques. Un adaptateur rl4burn, Python distant ou autre moteur peut remplacer Burn si les tests CartPole/Pendulum, reprise de checkpoint et schemas de metriques passent. Les checkpoints portent un `format_version` et un identifiant de backend ; aucune compatibilite implicite n'est promise.

Declencheurs de reexamen : PPO continu incorrect ou instable, debit insuffisant sur le robot cible, format Burn non durable, ou besoin d'un simulateur massivement parallele.

## References

- <https://burn.dev/>
- <https://github.com/RPP1011/rl4burn>

