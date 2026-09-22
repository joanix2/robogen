# Architecture de RoboGen

Statut : architecture cible du MVP, avec bilan d'implementation M0  
Derniere mise a jour : 2026-09-22

## Intention

RoboGen est une application desktop native de conception robotique parametrique. Elle ne juxtapose pas un modeleur, un simulateur et un outil d'apprentissage : ces fonctions projettent toutes un meme modele semantique issu du DSL `.rgn`.

Le MVP doit prouver une tranche verticale courte et reelle : charger un projet, analyser un sketch contraint, l'extruder, le tesseller, l'afficher, modifier un parametre puis exporter le resultat en STL. Les interfaces de topologie, physique, RL et IA sont prevues sans faire entrer leurs details dans le domaine.

## Etat implemente au 2026-09-22

Les sections suivantes decrivent la cible normative, sauf les constats explicitement marques comme actuels. Un contrat nomme dans ce document n'atteste ni son implementation ni la validation d'un milestone. Le bilan M0 constate les limites suivantes :

| Surface | Implementation actuelle et ecart a la cible | Jalon de suivi |
|---|---|---|
| DSL | Lexer/parser manuels avec spans ; espaces et commentaires elimines par le lexer, sans CST conservant les trivia ni patcher preservant les blocs non modifies. Voir [ADR-004](../adr/ADR-004-dsl-parser.md). | M2 |
| Contraintes | Solveur de rectangles avec application de dimensions et controles limites ; DOF par comptage, pas par rang du Jacobien. Voir [ADR-003](../adr/ADR-003-constraint-solver.md). | M4 |
| Rendu | Projection CPU en primitives ecran, dessinees par egui sur le backend wgpu d'eframe ; ni `RenderScene`, ni passes CAD wgpu, ni buffer de picking. Voir [ADR-001](../adr/ADR-001-ui.md). | M3 |
| Projet | Manifest `schema_version`/`name`/une seule `source`, chargement avec controle lexical de chemin ; pas de rejet des versions inconnues, de confinement des symlinks, de writer `ProjectStore` ou de migrations. Voir [ADR-005](../adr/ADR-005-project-format.md). | M3 pour chargement/enregistrement et chemins ; migrations robustes au M11 |
| Orchestration | `Project::set_parameter` pose des overrides en memoire ; `rebuild` est synchrone et reconstruit les meshes des parts dependantes, mais refait le lowering et la resolution des contraintes. Pas de commandes annulables partagees, de patch source, de `TaskManager`, de progression/annulation ou de garde de revision asynchrone. | M2 pour patch source ; M3 pour commandes et coherence du rebuild |
| Geometrie stable | `FaceRef`/`FaceRole` sont des enums, sans resolver ni table d'evolution topologique. Voir [ADR-009](../adr/ADR-009-stable-geometry-naming.md). | M3 pour provenance et selection de la tranche CAD |
| Physique/RL | Ports squelettiques et backends explicitement indisponibles ; ni Rapier ni PPO executables. Voir [ADR-006](../adr/ADR-006-physics.md) et [ADR-007](../adr/ADR-007-rl.md). | M5 / M9 |

Le chemin actuel est visible dans [robogen-project](../../crates/robogen-project/src/lib.rs) : source -> AST -> modele type -> contraintes de rectangles -> extrusion triangulee -> meshes. Cette tranche ne prouve pas a elle seule la cible B-Rep, le rendu GPU CAD, la selection stable ou le rebuild asynchrone ci-dessous.

## Regles structurantes (cible)

1. Le DSL et son `SemanticModel` sont la source de verite persistante.
2. Toute mutation passe par une commande annulable ; les widgets ne modifient pas directement le domaine.
3. Les projections CAD, robotique, physique, rendu, optimisation et RL sont reconstructibles.
4. Les entites portent des identifiants types et stables. Un index de face emis par un kernel n'est jamais une reference persistante.
5. Les crates de domaine ne dependent ni de `egui`, ni de `wgpu`, ni de Rapier, ni d'un kernel CAD.
6. Les backends externes sont caches derriere des traits et echangent des DTO appartenant a RoboGen.
7. Les calculs longs sont asynchrones, annulables et associes a une revision du document ; un resultat obsolete n'est jamais publie.
8. Une petite tranche integree et testee prime sur un sous-systeme etendu compose de stubs.

## Flux de donnees (cible)

```text
Sources .rgn ─> CST/AST + spans ─> resolution/types/unites ─> SemanticModel
                                                          │
                     ┌────────────────────────────────────┼────────────────────┐
                     v                                    v                    v
              FeatureGraph                         RobotModel          SimulationSpec
                     │                                    │                    │
                     v                                    └──────────┬─────────┘
              B-Rep ─> RenderMesh                         PhysicsWorld
                     │                                           │
                     ├────────────> export                        └──> rollouts RL
                     └────────────> domaine FEM ─> resultat TopOpt
```

Chaque noeud derive conserve une `SourceOrigin` (fichier, span, declaration et feature d'origine). Les diagnostics peuvent donc revenir au DSL et les resultats peuvent etre invalides precisement.

L'edition suit deux chemins qui convergent :

```text
edition texte -> parse/diff semantique -> Command -> SemanticModel -> rebuild cible
edition GUI   -> Command -> SemanticModel -> patch du bloc DSL -> rebuild cible
```

Au MVP, le patcher peut reformater le bloc modifie, mais ne doit pas reecrire les fichiers sans rapport. Les commentaires et l'ordre hors du bloc sont preserves.

## Couches et dependances

Les fleches indiquent le sens autorise dans la cible, pas le graphe Cargo actuel.

```text
apps (desktop, CLI, serveur futur)
  -> orchestration/UI, projet, export, backends

adaptateurs (render, CAD, constraints, physics, topopt, RL, export)
  -> robotics/assembly/sketch/IR

modele applicatif (project, IR, robotics, assembly, sketch)
  -> domain

frontiere texte (DSL)
  -> domain ; produit l'IR, ne connait aucun backend

fondation (domain)
  -> serde/types d'unites uniquement
```

Une crate peut dependre d'une couche inferieure, jamais de l'application ou d'un backend frere. Les echanges transversaux utilisent des types de domaine ou des evenements types. Les crates `robogen-agent-api` et les providers IA restent des ports sans implementation distante au MVP.

**Ecart actuel :** [robogen-project/Cargo.toml](../../crates/robogen-project/Cargo.toml) depend directement de `robogen-cad` et `robogen-constraints` pour orchestrer compilation et rebuild. [robogen-export/Cargo.toml](../../crates/robogen-export/Cargo.toml) depend de `robogen-cad`, dont il consomme le DTO RoboGen `Mesh`. Le graphe actuel est acyclique, mais ces aretes ne respectent pas la separation de couches cible ci-dessus. Leur presence n'autorise pas tous les couplages entre backends ; M3 doit clarifier ces contrats avec les responsables Project/CAD/Export et ARCHITECT. Ce bilan ne deplace aucun type ni ne decide une nouvelle frontiere.

## Responsabilite des composants (cible)

| Composant | Responsabilite | Ne doit pas contenir |
|---|---|---|
| `robogen-domain` | IDs types, unites, erreurs communes, provenance | UI, GPU, handles de backend |
| `robogen-dsl` | lexer, parser, CST/AST avec spans, diagnostics, patch ciblé | appels CAD ou physique |
| `robogen-ir` | resolution, typage, controle d'unites, `SemanticModel`, dependances | syntaxe d'UI, types de kernels |
| `robogen-project` | manifest, chemins, chargement atomique, migrations | blobs de calcul dans le DSL |
| `robogen-sketch` | entites et contraintes 2D, DOF et diagnostic abstraits | API native du solveur |
| `robogen-constraints` | port `ConstraintSolver` et backend natif | widgets de sketch |
| `robogen-cad` | feature graph, `CadKernel`, invalidation, tessellation | dependance DSL, rendu wgpu |
| `robogen-assembly` | instances et contraintes d'assemblage | handles physiques |
| `robogen-robotics` | liens, joints, actuateurs, capteurs, cinematique | handles Rapier |
| `robogen-render` | `RenderScene`, passes wgpu, picking, camera | `egui`, B-Rep du kernel |
| `robogen-physics` | `PhysicsBackend`, mapping separe domaine/backend | donnees persistantes du projet |
| `robogen-topopt` | FEM/SIMP, charges, progression, resultat | logique d'ecran |
| `robogen-rl` | environnements, rewards, rollouts, port d'entrainement | dependance directe aux widgets |
| `robogen-export` | STL puis autres formats, validations d'export | mutation du modele |
| `robogen-ui` | vues, presentation, commandes et navigation | regles metier |
| `robogen-telemetry` | evenements de diagnostic et metriques locales | donnees metier proprietaires |
| `robogen-testkit` | fixtures, doubles de ports, assertions de pipeline | code de production essentiel |

Les applications ne font que composer ces services. `robogen-desktop` possede la boucle de fenetre et l'etat de presentation ; `robogen-cli` reutilise exactement les memes commandes ; `robogen-server` reste un squelette jusqu'au calcul distant.

## Contrats de backend (cible)

Les ports principaux vivent du cote consommateur et ne revelent aucun type tiers :

- `ConstraintSolver` recoit un probleme normalise et rend solution, rang/DOF, residus et conflits ;
- `CadKernel` recoit des operations de feature et rend des handles opaques ephemeres, une tessellation et une provenance topologique ;
- `PhysicsBackend` construit un monde a partir d'un `SimulationSpec`, avance d'un pas fixe et rend un snapshot RoboGen ;
- `TopologyOptimizer` consomme domaine, maillage, charges et options, puis emet progression et resultat ;
- `RlBackend` consomme un environnement, une politique et une configuration reproductible ;
- `MeshExporter` consomme un maillage valide et des options explicites ;
- `FoundationPolicyProvider` et `AiAssistantProvider` ont uniquement des implementations `No*` au MVP.

Chaque adaptateur traduit les erreurs tierces en erreurs structurees RoboGen, conserve la cause pour les logs et ne panique pas sur une entree utilisateur.

## Rebuild incremental et concurrence (cible)

Le `DependencyGraph` associe les declarations et features a des `ArtifactKey`. Une commande augmente la revision du document, marque les descendants sales, puis planifie seulement les artefacts necessaires a la vue ou a l'export. Le cache est indexe par `(ArtifactKey, input_hash, backend_version, options_hash)`.

`TaskManager` execute CAD, FEM, simulation longue et RL hors du thread UI. Chaque tache expose statut, progression, annulation cooperative et revision source. La publication est acceptee uniquement si la revision correspond toujours. Les evenements transportent des IDs et petits snapshots ; les gros buffers sont partages par handles possedes et immuables.

Le pas physique est fixe. Le rendu interpole pour l'affichage mais ne pilote pas le temps de simulation. Les seeds, versions et configurations sont enregistrees pour les experiences ; la reproductibilite numerique inter-plateformes n'est pas promise sans test explicite.

## Identite, selection et geometrie stable (cible)

Les declarations, features, liens et contraintes utilisent des newtypes autour d'UUID. Les entites temporaires du kernel restent dans l'adaptateur CAD. Une selection persistante combine :

- l'ID de la feature productrice ;
- un role semantique (`CapStart`, `CapEnd`, `Lateral`, etc.) ;
- la chaine de provenance a travers les operations ;
- un selecteur geometrique et une signature quantifiee comme solution de repli.

Une resolution ambigue produit un diagnostic et bloque la feature dependante ; elle ne choisit jamais silencieusement `face[n]`. Le detail est fixe par [ADR-009](../adr/ADR-009-stable-geometry-naming.md).

## Format projet et artefacts (cible)

`project.rgnproj` est un manifest TOML versionne. Il reference des sources et assets par chemins relatifs normalises. Les sources `.rgn` restent textuelles et fusionnables par Git. Les resultats lourds vivent sous `generated/` avec metadonnees, hash des entrees et version du producteur ; ils sont supprimables et reconstructibles. Voir [ADR-005](../adr/ADR-005-project-format.md).

## Premiere tranche verticale (criteres cibles)

1. Ouvrir `examples/constrained_bracket/project.rgnproj` et `main.rgn`.
2. Parser `parameter`, `material`, `sketch`, `part` et `extrude` avec spans.
3. Resoudre les symboles et verifier les dimensions physiques.
4. Resoudre un rectangle contraint avec le backend natif.
5. Construire puis tesseller l'extrusion via `CadKernel`.
6. Convertir la tessellation en `RenderScene`, afficher et permettre la selection.
7. Modifier `WIDTH` par commande depuis le texte ou l'inspecteur.
8. Invalider seulement le sketch et ses descendants, ignorer tout resultat d'une ancienne revision.
9. Exporter un STL apres validation du maillage.

Ce flux constitue le critere de sortie des milestones DSL/CAD. TopOpt, robotique et RL ne doivent pas le retarder.

## Verification et observabilite

- tests unitaires par crate ;
- golden tests du parser et des diagnostics ;
- tests de contrat executes contre chaque backend ;
- fixtures geometriques avec invariants plutot que comparaison brute de floats ;
- tests d'integration des tranches verticales ;
- seeds fixes et seuils statistiques pour le RL ;
- `tracing` avec revision, task ID, entity ID et duree ;
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` a chaque milestone.

Les benchmarks ciblent le parser, le solveur de sketch, le rebuild CAD, la tessellation, le picking, le pas physique et le solveur FEM. Aucun resultat d'optimisation ou d'entrainement n'est simule dans l'UI : un backend absent est affiche comme indisponible.

## Decisions et evolution

Les choix remplacables et leurs seuils de reexamen sont documentes dans `docs/adr/`. Un remplacement doit d'abord passer les tests de contrat et les fixtures de reference. Les formats persistants et DTO du domaine evoluent par migration ; un changement d'API tierce reste confine a son adaptateur.

