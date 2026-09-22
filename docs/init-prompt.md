# PROJET : RoboGen
# Mission : reconstruire en Rust un environnement intégré de conception robotique
# Type : application desktop native, modulaire, paramétrique, orientée CAD + robotique
# Priorité : architecture saine + vertical slices réellement fonctionnelles
# Langage principal : Rust
# UI : Rust
# Rendu 3D : wgpu / WebGPU
# IA : PAS à implémenter maintenant, uniquement prévoir les interfaces
# Cible initiale : Linux desktop, architecture portable Windows/macOS

Tu es l’agent principal chargé de concevoir et implémenter RoboGen.

RoboGen est un environnement de conception robotique qui réunit dans un même logiciel :

1. conception déclarative d’un robot ;
2. sketch paramétrique et CAD 3D ;
3. définition de la cinématique ;
4. définition des matériaux, actuateurs, capteurs et contraintes ;
5. visualisation 3D temps réel ;
6. optimisation topologique des pièces ;
7. simulation physique ;
8. apprentissage par renforcement ;
9. export pour fabrication, notamment STL/3MF/STEP ;
10. à terme, génération et modification du modèle via LLM.

Le logiciel doit être construit autour d’un DSL déclaratif modulaire qui constitue la SOURCE DE VÉRITÉ.

Le modèle 3D, l’arbre CAD, le modèle robotique, la simulation, l’optimisation et l’interface utilisateur sont des projections de ce modèle déclaratif.

Ne construis PAS trois sous-applications indépendantes Design / Optimisation / RL.
Construis un seul graphe de modèle partagé.

==================================================
1. PRINCIPES D’ARCHITECTURE
==================================================

Le projet doit être organisé comme un Cargo workspace.

Architecture cible :

robogen/
    Cargo.toml
    rust-toolchain.toml
    README.md
    AGENTS.md
    docs/
        architecture/
        adr/
        dsl/
        screenshots/
    apps/
        robogen-desktop/
        robogen-cli/
        robogen-server/        # squelette seulement pour futur calcul distant
    crates/
        robogen-domain/
        robogen-dsl/
        robogen-ir/
        robogen-project/
        robogen-ui/
        robogen-render/
        robogen-sketch/
        robogen-constraints/
        robogen-cad/
        robogen-assembly/
        robogen-robotics/
        robogen-physics/
        robogen-topopt/
        robogen-rl/
        robogen-export/
        robogen-agent-api/
        robogen-telemetry/
        robogen-testkit/
    examples/
        spider_robot/
        simple_arm/
        constrained_bracket/
        topopt_beam/
        rl_quadruped/
    assets/
        icons/
        materials/
        environments/

Ne mets pas toute la logique dans l’application desktop.

L’application desktop doit principalement :
- composer les services ;
- gérer l’état UI ;
- transmettre les commandes ;
- afficher les résultats.

Les règles métier doivent rester dans les crates dédiées.

Utiliser :
- serde pour les données sérialisables ;
- UUID ou IDs typés pour toutes les entités ;
- thiserror pour les erreurs ;
- tracing pour les logs ;
- rayon pour le parallélisme CPU quand pertinent ;
- wgpu pour le moteur graphique ;
- egui + winit + wgpu pour l’interface native, sauf justification technique meilleure.

Éviter :
- dépendances cycliques ;
- singletons globaux ;
- logique métier dans les widgets ;
- dépendance directe du domaine à wgpu ;
- dépendance directe du DSL au CAD kernel ;
- références fragiles du type face[13].

Créer des traits pour tous les backends importants.

Exemples :

trait CadKernel
trait ConstraintSolver
trait PhysicsBackend
trait TopologyOptimizer
trait RlBackend
trait MeshExporter
trait FoundationPolicyProvider
trait AiAssistantProvider

L’objectif est que les implémentations puissent évoluer sans modifier le modèle métier.

==================================================
2. ARCHITECTURE DES AGENTS DE DÉVELOPPEMENT
==================================================

Organiser le développement comme si plusieurs spécialistes travaillaient ensemble.

Créer AGENTS.md décrivant les responsabilités suivantes :

ARCHITECT
- maintient les limites de crates ;
- valide les ADR ;
- empêche les dépendances transversales sauvages.

DSL AGENT
- parser ;
- AST ;
- typage ;
- unités ;
- diagnostics ;
- IR.

CAD AGENT
- sketch ;
- contraintes ;
- feature tree ;
- B-Rep ;
- opérations solides.

RENDER AGENT
- wgpu ;
- caméra ;
- picking ;
- overlays ;
- gizmos ;
- performances.

ROBOTICS AGENT
- assemblages ;
- joints ;
- actuateurs ;
- cinématique ;
- génération physique.

TOPOLOGY AGENT
- FEM ;
- SIMP ;
- contraintes ;
- reconstruction du résultat.

RL AGENT
- environnement ;
- observations ;
- actions ;
- récompenses ;
- PPO ;
- vectorisation.

UI AGENT
- reproduction des écrans ;
- interactions ;
- navigation ;
- panels.

QA AGENT
- tests ;
- snapshots ;
- invariants ;
- benchmarks ;
- fichiers exemples.

Si l’environnement OpenCode permet des sous-agents, les utiliser.
Sinon suivre ces responsabilités séquentiellement.

==================================================
3. ARCHITECTURE FONCTIONNELLE GLOBALE
==================================================

Le pipeline logique est :

Natural Language [future]
          |
          v
      RoboGen DSL
          |
          v
     Typed AST
          |
          v
    Semantic IR
          |
    +-----+----------------------+------------------+
    |                            |                  |
    v                            v                  v
 CAD Feature Graph       Robot/Kinematic Graph   Simulation Spec
    |                            |                  |
    v                            v                  v
 B-Rep / Mesh           Links/Joints/Actuators  Physics World
    |                            |                  |
    +--------------+-------------+                  |
                   |                                |
                   v                                v
               Robot Model --------------------> Physics
                   |                                |
                   |                                v
                   |                            RL Training
                   |
                   v
          Topology Optimisation
                   |
                   v
             New Geometry
                   |
                   v
              Re-simulation

Toutes les données doivent pouvoir être retracées jusqu’à une déclaration du DSL.

==================================================
4. DSL ROBOGEN
==================================================

Créer un langage déclaratif textuel modulaire.

Extension proposée :
.rgn

Le langage doit être :
- déclaratif ;
- fortement typé ;
- déterministe ;
- modulaire ;
- sérialisable ;
- lisible humainement ;
- adapté à la génération future par LLM ;
- indépendant du renderer ;
- indépendant du solveur physique.

Supporter les unités physiques.

Exemples :
12 mm
45 deg
2.5 Nm
500 g
9.81 m/s2
200 MPa

Ne jamais représenter ces valeurs comme de simples f64 anonymes dans le domaine.

Créer des types explicites ou un système d’unités.

Le DSL doit supporter :

module
import
parameter
material
sketch
part
feature
assembly
robot
body
link
joint
actuator
sensor
constraint
environment
optimization
training
export

Exemple de syntaxe cible :

module spider;

parameter LEG_COUNT = 8;
parameter FEMUR_LENGTH = 120 mm;
parameter TIBIA_LENGTH = 150 mm;

material Aluminium6061 {
    density: 2700 kg/m3;
    young: 69 GPa;
    poisson: 0.33;
    yield_strength: 276 MPa;
}

sketch FemurProfile on XY {
    point p0 = (0 mm, 0 mm);
    point p1 = (120 mm, 0 mm);

    circle pivot_a {
        center: p0;
        radius: 8 mm;
    }

    circle pivot_b {
        center: p1;
        radius: 8 mm;
    }

    line top = ...;
    line bottom = ...;

    constrain horizontal(p0, p1);
    constrain distance(p0, p1, FEMUR_LENGTH);
}

part Femur {
    material: Aluminium6061;

    base = extrude(FemurProfile, 8 mm);

    hole_a = hole(
        face: select_face(base, normal = +Z),
        center: FemurProfile.p0,
        diameter: 6 mm
    );

    hole_b = hole(...);

    final = subtract(base, hole_a, hole_b);
}

actuator HipServo {
    type: rotary;
    max_torque: 2.5 Nm;
    max_velocity: 8 rad/s;
}

joint Hip {
    type: revolute;
    axis: Z;
    limits: [-45 deg, 45 deg];
    actuator: HipServo;
}

robot Spider {
    body chassis;

    repeat LEG_COUNT around Z {
        link femur: Femur;
        link tibia: Tibia;

        joint hip: Hip;
        joint knee: Knee;
    }
}

environment RockyTerrain {
    gravity: (0, 0, -9.81 m/s2);
    terrain: rocks;
    friction: 0.8;
}

optimization FemurLightweight {
    target: Femur;

    objective {
        minimize mass;
    }

    constraints {
        max_displacement: 0.3 mm;
        safety_factor: 2.0;
        volume_fraction: 0.35;
    }

    preserve {
        interface pivot_a;
        interface pivot_b;
    }

    manufacturing {
        process: resin_printing;
        min_thickness: 1.2 mm;
    }
}

training SpiderLocomotion {
    robot: Spider;
    environment: RockyTerrain;

    algorithm: PPO;

    observations {
        joint_positions;
        joint_velocities;
        body_orientation;
        body_velocity;
        contacts;
    }

    actions {
        joint_targets;
    }

    reward {
        forward_velocity: +1.0;
        stability: +0.5;
        energy: -0.05;
        falling: -10.0;
    }
}

Le DSL devra plus tard pouvoir recevoir des modules personnalisés :

import robotics.standard;
import materials.aluminium;
import actuators.servos;
import optimization.topology;
import environments.rough_terrain;

Créer un système de namespace.

==================================================
5. PIPELINE DU DSL
==================================================

Construire :

source
 -> lexer
 -> parser
 -> AST avec source spans
 -> resolver
 -> symbol table
 -> type checker
 -> unit checker
 -> semantic IR
 -> dependency graph
 -> evaluator / compiler

Les erreurs doivent être de qualité IDE :

error[E104]:
FemurProfile.rgn:17:9

Expected Length
Found Angle

radius: 25 deg
        ^^^^^^

Le parser ne doit jamais panic sur une entrée utilisateur.

Préparer :
- syntax highlighting ;
- autocomplétion future ;
- go-to-definition futur ;
- diagnostics inline.

Créer des tests parser golden-file.

==================================================
6. MODÈLE DE DONNÉES ET GRAPHE DE FEATURES
==================================================

S’inspirer conceptuellement de FreeCAD :

Document
 -> Objects
 -> Parameters
 -> Sketches
 -> Features
 -> Bodies
 -> Parts
 -> Assemblies

Mais éviter un gros Object dynamique non typé.

Créer des structures Rust fortement typées.

Une pièce paramétrique est un graphe de features.

Exemple :

Sketch001
   |
Extrude001
   |
Fillet001
   |
Hole001
   |
Pattern001
   |
FinalBody

Quand un paramètre change :
- invalider uniquement les descendants ;
- recalculer le graphe ;
- mettre à jour la géométrie ;
- conserver les IDs autant que possible.

Implémenter dirty propagation.

CRITIQUE :
ne jamais utiliser durablement un index de face généré par le kernel.

Créer un système de références géométriques stables.

Par exemple :

FaceRef::Semantic(...)
FaceRef::FeatureOutput(...)
FaceSelector::Normal(...)
FaceSelector::LargestPlanar(...)
EdgeSelector::Circular(...)
VertexSelector::Intersection(...)

Le problème de topological naming doit être traité explicitement.

==================================================
7. SKETCHER 2D PARAMÉTRIQUE
==================================================

Le sketcher doit fonctionner comme un CAD classique.

Entités minimales V1 :
- Point
- LineSegment
- Circle
- Arc
- ConstructionLine

Puis :
- Ellipse
- Bezier
- B-Spline

Contraintes V1 :
- coincident
- horizontal
- vertical
- parallel
- perpendicular
- tangent
- equal_length
- equal_radius
- distance
- distance_x
- distance_y
- angle
- radius
- diameter
- midpoint
- symmetry
- fix

L’interface doit afficher :
- degrés de liberté restants ;
- géométrie sous-contrainte ;
- géométrie totalement contrainte ;
- contraintes conflictuelles ;
- dimensions éditables.

Pour le solveur :

Créer trait ConstraintSolver.

Étudier une implémentation SolveSpace/slvs pour le MVP si sa licence est acceptable.

NE PAS faire dépendre directement robogen-sketch de slvs.

Créer :

robogen-constraints/
    src/
        lib.rs
        solver.rs
        model.rs
        backends/
            slvs.rs
            native.rs

Si l’usage GPL pose problème pour la distribution future, le backend doit pouvoir être remplacé.

Préparer un backend native Rust futur.

==================================================
8. CAD 3D
==================================================

Le système CAD doit être paramétrique.

Opérations V1 obligatoires :

Primitives :
- box
- cylinder
- sphere

Sketch features :
- extrude
- revolve

Operations :
- union
- subtract
- intersection

Features :
- hole
- mirror
- linear_pattern
- circular_pattern

V2 :
- fillet
- chamfer
- sweep
- loft
- shell
- offset
- draft

V3 :
- surfaces
- sheet metal

Créer trait CadKernel.

Faire une courte expérimentation technique documentée dans ADR :

Option A : Truck
Option B : BrepKit

Tester au minimum :
- cube ;
- cylinder ;
- boolean subtract ;
- extrude sketch ;
- revolve ;
- tessellation ;
- STL export ;
- STEP export si disponible ;
- conservation des références de features.

Choisir le backend après les tests.

Toute utilisation du kernel doit passer par robogen-cad.

L’IR ne doit connaître aucun type Truck/BrepKit.

==================================================
9. RENDERER WGPU
==================================================

Construire robogen-render indépendamment du CAD.

Utiliser wgpu.

Le renderer doit prendre une RenderScene générique.

RenderScene contient :
- meshes ;
- instances ;
- edges ;
- vertices ;
- materials ;
- lights ;
- overlays ;
- selection IDs.

Fonctionnalités V1 :

- perspective camera ;
- orthographic camera ;
- orbit ;
- pan ;
- zoom ;
- grid ;
- axes XYZ ;
- view cube ;
- shaded mode ;
- wireframe overlay ;
- edges ;
- vertices ;
- hover ;
- selection ;
- multi-selection ;
- face picking ;
- edge picking ;
- vertex picking ;
- object picking ;
- transform gizmo ;
- bounding boxes ;
- center of mass marker ;
- joint axes ;
- collision shapes toggle.

Le picking doit utiliser un ID buffer GPU ou une méthode déterministe équivalente.

Ne pas faire du picking uniquement par bounding box.

Prévoir plusieurs passes :

geometry pass
edge pass
selection pass
overlay pass
UI composition

Préparer plus tard :
- SSAO ;
- shadows ;
- outlines ;
- PBR ;
- transparency ;
- exploded view.

Le renderer ne doit pas dépendre d’egui.

egui affiche simplement la texture/render target produite par robogen-render.

==================================================
10. ÉCRAN PRINCIPAL : DESIGN
==================================================

Reproduire le langage visuel de la maquette fournie.

Top bar :

ROBOGEN
From idea to real robots

Workflow :
1 Design
2 Optimisation
3 Apprentissage

À droite :
- Nouveau projet
- Enregistrer
- Exporter
- Profil

Sous la top bar, écran en 3 colonnes.

COLONNE GAUCHE :
Bibliothèque / Taxonomie

Recherche :
"Rechercher un composant..."

Arbre :

Robot
  Corps
    Châssis
    Coque
    Support

  Membre
    Patte
    Bras
    Roue
    Aile
    Queue

  Articulation
    Rotule
    Pivot
    Prismatique

  Actionneur
    Servo
    Moteur BLDC
    Vérin

  Capteur
    IMU
    Caméra
    Capteur de force

  Matériau
    PLA
    Résine
    Aluminium
    Titane
    Carbone

Cette taxonomie doit provenir du domain model et être extensible via modules.

ZONE CENTRALE :

Tabs :
- Vue 3D
- Code DSL

Vue 3D :
- toolbar verticale gauche ;
- select ;
- sketch ;
- transform ;
- joint ;
- measure ;
- camera controls.

Bas :
- vue éclatée ;
- centre de masse ;
- caméra perspective/orthographique ;
- settings ;
- fullscreen.

Code DSL :
- éditeur texte ;
- syntax highlighting ;
- diagnostics ;
- sélection synchronisée.

Quand l’utilisateur sélectionne :

part Femur

dans le DSL,
la pièce correspondante doit être sélectionnée dans la vue 3D.

Et réciproquement.

COLONNE DROITE :

Assistant IA.

Pour l’instant :
- UI complète ;
- historique de chat mock ;
- input ;
- suggestions ;
- boutons.

Mais aucun LLM réel.

Créer trait :

AiAssistantProvider

avec :

DisabledAiAssistant

L’UI affiche :
"Assistant IA non configuré"

Le domain ne doit jamais dépendre de l’IA.

==================================================
11. MODES DESIGN
==================================================

Le mode Design possède trois sous-contextes :

A. ROBOT
gestion de :
- corps ;
- liens ;
- joints ;
- actuateurs ;
- capteurs.

B. SKETCH
éditeur 2D paramétrique.

C. PART
feature tree d’une pièce.

La vue gauche change selon le contexte.

Pour une pièce :

Femur
  Sketch001
  Pad001
  Hole001
  Hole002
  Fillet001

Prévoir undo/redo global avec Command Pattern.

Toute commande utilisateur doit pouvoir être annulée.

==================================================
12. ROBOTIQUE / CINÉMATIQUE
==================================================

Créer une représentation indépendante de la simulation :

RobotModel
Link
Joint
Actuator
Sensor
CollisionGeometry
VisualGeometry
InertialProperties

JointType :
- Fixed
- Revolute
- Continuous
- Prismatic
- Ball
- Planar

Une Link possède :
- transform ;
- geometry ;
- mass ;
- center_of_mass ;
- inertia_tensor.

Calculer automatiquement depuis la géométrie quand possible.

Actuator :
- torque limit ;
- velocity limit ;
- gear ratio ;
- damping ;
- control mode.

Prévoir :
- FK ;
- Jacobian ;
- IK ;
- center of mass ;
- support polygon.

Exporter à terme :
- URDF ;
- MJCF ;
- USD.

==================================================
13. PHYSICS
==================================================

Créer PhysicsBackend.

Backend Rust initial :
Rapier3D.

Le domain robotique est converti vers Rapier.

Ne jamais stocker les handles Rapier dans RobotModel.

Créer un mapping séparé.

Fonctionnalités :
- rigid bodies ;
- multibody joints ;
- collisions ;
- motors ;
- friction ;
- contacts ;
- gravity ;
- terrains ;
- sensors ;
- simulation step.

Créer DebugPhysicsOverlay pour afficher :
- collision shapes ;
- contact points ;
- forces ;
- center of mass ;
- joint axes.

==================================================
14. ONGLET OPTIMISATION TOPOLOGIQUE
==================================================

Reproduire l’écran montré dans la maquette.

Layout :

GAUCHE :
Paramètres d’optimisation

Objectif :
- minimiser masse ;
- minimiser compliance ;
- compromis masse/rigidité.

Contraintes mécaniques :
- matériau ;
- charge maximale ;
- facteur de sécurité.

Contraintes géométriques :
- conserver interfaces ;
- respecter enveloppe ;
- zones interdites ;
- preserve regions.

Contraintes fabrication :
- procédé ;
- épaisseur minimale ;
- overhang maximum ;
- symétrie éventuelle.

Bouton :
Lancer l’optimisation.

CENTRE :
tabs
- Résultat
- Comparaison
- Analyse
- Historique

Afficher :
Avant optimisation
Après optimisation

Puis cartes :
- masse ;
- réduction ;
- compliance ;
- déplacement max ;
- stress max ;
- facteur de sécurité.

Sous les modèles :
visualisation de Von Mises avec color map.

Ne pas simuler de faux résultats.

Tant que le solveur n’existe pas, afficher explicitement :
"Solver unavailable".

==================================================
15. MOTEUR D’OPTIMISATION TOPOLOGIQUE
==================================================

Implémenter une V1 réelle.

Méthode initiale :
SIMP.

Pipeline :

CAD Design Region
 -> discretisation FEM
 -> boundary conditions
 -> loads
 -> solve linear elasticity
 -> compliance
 -> sensitivities
 -> density filtering
 -> optimisation update
 -> convergence
 -> iso-surface
 -> output mesh

Variables :

rho_i in [rho_min, 1]

Young modulus :

E_i = E_min + rho_i^p (E_0 - E_min)

Objectif V1 :

min compliance

sous contrainte :

volume <= volume_fraction

Implémenter :
- 3D hexahedral grid simple ;
- linear elasticity ;
- sparse matrix ;
- density filter ;
- SIMP penalisation ;
- optimality criteria update ;
- convergence history.

Architecture :

robogen-topopt/
    mesh/
    fem/
    material/
    loads/
    solver/
    simp/
    filters/
    reconstruction/
    metrics/

Utiliser une abstraction SparseLinearSolver.

Commencer CPU.

Utiliser rayon.

N’essaye PAS immédiatement d’implémenter un solveur sparse GPU complexe.

wgpu pourra ensuite accélérer :
- filters ;
- voxel operations ;
- marching cubes ;
- visualisation ;
- éventuellement iterative solvers.

V2 :
- stress constraints ;
- buckling ;
- multiple load cases ;
- dynamic loads ;
- fatigue ;
- level-set ;
- lattice / TPMS.

Prévoir l’architecture dès maintenant mais ne pas implémenter toutes ces méthodes.

==================================================
16. LIEN ROBOT -> TOPOLOGY OPTIMISATION
==================================================

Ne considère pas les charges comme uniquement manuelles.

Le modèle doit permettre deux origines :

ManualLoadCase
SimulationLoadCase

SimulationLoadCase peut être calculé depuis une trajectoire de simulation.

Exemple :

robot marche
 -> collect joint torques
 -> collect contact forces
 -> collect accelerations
 -> construire load cases
 -> optimiser la pièce

Créer :

LoadCase
LoadHistory
SimulationLoadExtractor

Cette abstraction sera essentielle plus tard pour la co-optimisation structure + contrôle.

==================================================
17. ONGLET APPRENTISSAGE
==================================================

Reproduire l’esprit de l’écran montré.

Layout :

GAUCHE :

Environment

Terrain :
- Plat
- Rochers
- Escaliers
- Sable

Paramètres entraînement :
- algorithme ;
- nombre environnements ;
- durée ;
- seed.

Reward editor :
- vitesse avant ;
- stabilité ;
- économie énergie ;
- éviter chute ;
- hauteur corps.

Bouton :
Lancer l’entraînement.

CENTRE :

viewport simulation 3D.

Overlay :
Episode
Vitesse
Récompense
Chutes
Temps réel

DROITE :

courbes :
- récompense moyenne ;
- vitesse ;
- taux de chute ;
- énergie.

BAS :

"Comportement actuel"

Afficher une séquence temporelle/gait preview.

Contrôle vitesse de lecture.

==================================================
18. RL EN RUST
==================================================

Créer trait RlBackend.

Utiliser Burn pour réseaux neuronaux.

Évaluer rl4burn pour PPO.

Algorithme V1 :
PPO.

Créer une API interne indépendante de la bibliothèque :

trait Environment {
    type Observation;
    type Action;

    fn reset(...);
    fn step(...);
}

RobotEnvironment :
- Rapier physics ;
- observation builder ;
- action decoder ;
- reward evaluator ;
- termination evaluator.

Reward DSL doit être compilé vers une RewardGraph.

Exemple :

RewardGraph
    ForwardVelocity(weight=1.0)
    Stability(weight=0.5)
    EnergyPenalty(weight=-0.05)
    FallPenalty(weight=-10)

Supporter vectorisation de plusieurs environnements.

V1 :
CPU environments parallèles avec rayon.

Policy network :
Burn.

Backend de calcul :
WGPU quand pertinent.

Conserver abstraction pour :
- CUDA ;
- autres simulateurs ;
- futurs foundation models.

==================================================
19. FOUNDATION MODEL : UNIQUEMENT INTERFACE
==================================================

Ne pas intégrer de foundation model maintenant.

Mais prévoir :

trait FoundationPolicyProvider {
    fn initialise_policy(
        robot: &RobotModel,
        task: &TaskSpec
    ) -> Result<InitialPolicy>;
}

Implémentation initiale :

NoFoundationPolicy

Dans l’UI :

Foundation model
[ Non configuré ]

Future pipeline :

Task
 + Robot morphology encoding
 + Foundation policy
 -> initial controller
 -> residual PPO fine-tuning

Ne rien hardcoder aujourd’hui.

==================================================
20. EXPORT
==================================================

Créer robogen-export.

V1 :
- STL ;
- project .rgn ;
- project bundle.

Puis :
- 3MF ;
- STEP ;
- URDF ;
- MJCF ;
- glTF/GLB.

Export STL :

File > Export > STL

Options :
- binary/ascii ;
- chord tolerance ;
- angular tolerance ;
- units ;
- selected parts / whole robot.

Avant export :
- vérifier manifold si possible ;
- avertir si mesh invalide ;
- afficher bounding box ;
- afficher taille estimée.

Créer le chemin futur :

RoboGen
 -> STL / 3MF
 -> slicer externe
 -> impression résine/FDM.

Ne pas implémenter un slicer maintenant.

==================================================
21. GESTION DE PROJET
==================================================

Format projet :

project.rgnproj

Il doit référencer :
- fichiers DSL ;
- assets ;
- meshes importés ;
- materials ;
- training runs ;
- optimisation runs.

Les résultats volumineux ne doivent pas être directement stockés dans le DSL.

Structure possible :

my_robot/
    project.rgnproj
    src/
        robot.rgn
        chassis.rgn
        leg.rgn
        materials.rgn
        training.rgn
    assets/
    generated/
        meshes/
        simulation/
        optimization/
        training/

Le DSL reste lisible par Git.

==================================================
22. UI / UX
==================================================

Style inspiré de la maquette fournie :

- dark ;
- professionnel ;
- dense mais lisible ;
- panneaux dockés ;
- accents bleus ;
- viewport central dominant ;
- peu de fenêtres modales.

Ne jamais bloquer tout l’UI pendant :
- rebuild CAD ;
- topopt ;
- RL.

Créer TaskManager.

Les calculs longs retournent :
- progression ;
- statut ;
- métriques ;
- cancel handle.

Exemple :

TaskStatus {
    Pending,
    Running { progress },
    Completed,
    Failed,
    Cancelled
}

Prévoir cancellation propre.

==================================================
23. SYNCHRONISATION DSL / UI
==================================================

Exigence très importante.

Le DSL et l’interface graphique éditent le même modèle.

Éviter :

UI model
<- conversion approximative ->
DSL model

Créer un seul SemanticModel.

Workflow :

DSL edit
 -> parse
 -> semantic diff
 -> command
 -> domain update
 -> CAD rebuild
 -> renderer update

GUI edit
 -> command
 -> semantic update
 -> pretty printer
 -> DSL patch

Préserver autant que possible :
- commentaires ;
- ordre ;
- formatting.

Pour le début, le pretty printer peut réécrire proprement un bloc ciblé.

Documenter cette limitation.

==================================================
24. EVENT BUS
==================================================

Créer un event system typé.

Exemples :

ProjectLoaded
DslChanged
SemanticModelChanged
SketchSolved
CadFeatureRebuilt
GeometryUpdated
SelectionChanged
RobotModelChanged
SimulationStarted
SimulationFrame
OptimizationProgress
OptimizationCompleted
TrainingProgress
TrainingCompleted

Éviter les chaînes de callbacks implicites.

==================================================
25. UNDO / REDO
==================================================

Toute action utilisateur mutante doit passer par une Command.

Exemples :

AddSketchEntity
DeleteSketchEntity
SetConstraint
ChangeParameter
AddFeature
DeleteFeature
MoveComponent
AddJoint
ChangeMaterial

Command :
- execute ;
- undo ;
- description.

Créer UndoStack.

==================================================
26. TESTS
==================================================

Chaque crate doit avoir tests unitaires.

Créer tests d’intégration pour les pipelines importants.

Premier test vertical :

DSL
 -> parse
 -> sketch rectangle
 -> constraints
 -> extrude
 -> tessellate
 -> render mesh
 -> export STL

Deuxième :

DSL spider
 -> RobotModel
 -> links/joints
 -> Rapier
 -> simulation gravity
 -> motors.

Troisième :

cantilever beam
 -> FEM
 -> SIMP
 -> lower compliance
 -> lower material volume.

Quatrième :

simple articulated robot
 -> environment
 -> PPO
 -> reward improves statistically sur petit benchmark.

Ne pas exiger qu’un test RL stochastic soit parfaitement déterministe.
Tester tendances ou composants déterministes séparément.

==================================================
27. QUALITÉ
==================================================

À chaque milestone :

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

Pas de unwrap dans les bibliothèques sauf invariant prouvé.

Pas de panic sur input utilisateur.

unsafe interdit sauf module isolé + commentaire SAFETY.

Benchmarks pour :
- parser ;
- sketch solver ;
- CAD rebuild ;
- tessellation ;
- render ;
- FEM ;
- simulation.

==================================================
28. ADR OBLIGATOIRES
==================================================

Créer Architecture Decision Records pour :

ADR-001 UI framework
ADR-002 CAD kernel
ADR-003 constraint solver
ADR-004 DSL parser
ADR-005 project format
ADR-006 physics backend
ADR-007 RL backend
ADR-008 topology optimisation solver
ADR-009 stable geometry naming

Chaque ADR :
- contexte ;
- options ;
- choix ;
- conséquences ;
- stratégie de remplacement.

==================================================
29. ORDRE D’IMPLÉMENTATION
==================================================

Ne tente surtout pas tout simultanément.

MILESTONE 0
Architecture + workspace + CI.

Résultat :
application vide mais compilable.

MILESTONE 1
UI proche de la maquette avec données mock.

Résultat :
navigation entre :
Design
Optimisation
Apprentissage.

MILESTONE 2
DSL minimal.

Support :
parameter
material
sketch
part
extrude.

MILESTONE 3
Vertical slice CAD.

Créer dans le DSL :
rectangle contraint
 -> extrude
 -> afficher
 -> modifier dimension
 -> reconstruction live
 -> STL.

CE MILESTONE EST LA PREMIÈRE PREUVE DU PROJET.

MILESTONE 4
Sketcher GUI.

Créer points/lignes/cercle avec souris.
Ajouter contraintes.
Voir DOF.

MILESTONE 5
Robot model.

Deux liens + joint revolute.
Simulation physique.
Afficher axe articulation.

MILESTONE 6
Araignée RoboGen exemple.

Châssis.
Patte paramétrique.
Pattern x8.
Joints.
Actuateurs.

MILESTONE 7
TopOpt minimale réelle.

Optimiser une poutre simple.
Afficher densité.
Marching cubes.
Comparer avant/après.

MILESTONE 8
TopOpt intégrée à une pièce robotique.

MILESTONE 9
RL minimal.

PPO sur environnement jouet.

MILESTONE 10
RL robot articulé.

MILESTONE 11
Polish + exports.

==================================================
30. PREMIÈRE TÂCHE À EXÉCUTER MAINTENANT
==================================================

Commence par inspecter le dépôt existant.

NE SUPPRIME RIEN sans raison documentée.

Produis d’abord :

docs/architecture/OVERVIEW.md
docs/adr/ADR-001-ui.md
docs/adr/ADR-002-cad-kernel.md
docs/adr/ADR-003-constraint-solver.md
docs/adr/ADR-006-physics.md
docs/adr/ADR-007-rl.md
AGENTS.md

Ensuite crée le Cargo workspace.

Puis implémente MILESTONE 1.

Le premier écran doit déjà ressembler fortement à la maquette RoboGen :

top bar
workflow tabs
taxonomy tree
3D viewport
DSL tab
AI placeholder
optimization screen
training screen

Utiliser des modèles 3D temporaires simples.

NE commence PAS encore PPO.
NE commence PAS encore le solveur FEM.
NE commence PAS encore à implémenter un LLM.

Une fois le shell fonctionnel :

implémente le vertical slice du MILESTONE 2 + 3 :

un fichier :

examples/constrained_bracket/main.rgn

décrit un sketch paramétrique,
avec dimensions et contraintes,
qui est extrudé,
rendu avec wgpu,
modifiable,
et exportable en STL.

Le changement :

parameter WIDTH = 40 mm;

vers :

parameter WIDTH = 70 mm;

doit reconstruire automatiquement le sketch et la pièce affichée.

==================================================
31. RÈGLE D’OR
==================================================

Toujours privilégier une petite fonctionnalité traversant réellement :

DSL
 -> Domain
 -> Solver
 -> CAD
 -> Renderer
 -> UI

plutôt que cinq sous-systèmes gigantesques composés uniquement de stubs.

Le produit doit évoluer par vertical slices.

À chaque étape :
1. implémenter ;
2. compiler ;
3. tester ;
4. afficher ;
5. documenter ;
6. seulement ensuite passer à la suite.