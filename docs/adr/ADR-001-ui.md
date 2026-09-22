# ADR-001 — Framework d'interface desktop

- Statut : accepte pour le MVP
- Date : 2026-09-22

## Contexte

RoboGen demande une UI native Rust, dense et interactive, un viewport wgpu dominant, des panneaux dockes, du picking et des mises a jour frequentes. La cible initiale est Linux, avec Windows et macOS ensuite. La logique metier et le renderer doivent rester independants du toolkit.

## Options considerees

- `egui` + `eframe` avec backend wgpu : integration directe, portable, immediate mode adapte aux outils et iteration rapide.
- `iced` : architecture applicative structuree mais integration d'un renderer CAD sur mesure moins directe pour ce MVP.
- `Slint` : bon outillage declaratif, mais introduit un second langage UI et une frontiere moins naturelle avec les interactions CAD immediates.
- `winit` + widgets maison : controle maximal, cout et risque disproportionnes.

## Decision (cible normative)

Utiliser `egui` via `eframe` avec son backend wgpu. `eframe` possede la boucle `winit`, la surface et les entrees de fenetre. `robogen-ui` construit les panneaux et emet des commandes applicatives.

`robogen-render` demeure independant d'egui. Il rend une `RenderScene` vers une cible wgpu et expose la texture, les resultats de picking et les overlays de domaine. La composition egui est realisee dans l'application desktop. L'etat de widget reste distinct du `SemanticModel`.

Au MVP, le docking peut etre implemente avec les primitives egui ou un composant leger isole dans `robogen-ui`; le format projet ne persiste aucune structure propre a ce composant.

## Etat implemente au 2026-09-22

[Le desktop](../../apps/robogen-desktop/src/main.rs) configure bien `eframe::Renderer::Wgpu`. Cependant, [robogen-render](../../crates/robogen-render/src/lib.rs) ne depend pas de wgpu : `RobotPreviewRenderer::render_mesh` projette un `PreviewMesh` sur CPU en `ViewportFrame` contenant des primitives ecran. [robogen-ui](../../crates/robogen-ui/src/lib.rs) les dessine avec le painter egui ; eframe compose ensuite l'interface via wgpu.

Ce chemin ne fournit ni `RenderScene`, ni cible/texture CAD wgpu dediee, ni buffer de picking. L'independance d'egui du module de projection est effective, mais ne prouve pas le contrat GPU cible. Les commandes annulables partagees et `TaskManager` ne sont pas implementes non plus. Le raccordement au viewport natif avec picking, commandes et traitements hors thread UI reste un travail M3 (responsables RENDER, UI et orchestration), sans changement du choix egui/eframe.

## Consequences

- iteration rapide et pile graphique unique wgpu ;
- meme base pour Linux, Windows et macOS ;
- les formulaires complexes exigent une discipline stricte pour ne pas placer les regles metier dans les widgets ;
- l'immediate mode reconstruit l'UI chaque frame, donc les travaux lourds passent par `TaskManager` ;
- les versions `egui`, `eframe`, `egui-wgpu`, `winit` et `wgpu` doivent etre montees ensemble et testees sur les trois OS.

## Strategie de remplacement

Les presenters et commandes de `robogen-ui` ne retournent pas de types egui. Pour remplacer le toolkit :

1. conserver `AppCommand`, les view-models et le contrat du renderer ;
2. reimplementer uniquement la couche de widgets et la composition de texture ;
3. valider navigation, raccourcis, DPI, IME et accessibilite avec les tests d'interaction ;
4. migrer les preferences d'affichage, jamais le modele projet.

Reexaminer ce choix si l'accessibilite, l'IME, le docking multi-fenetre ou les performances d'un document realiste ne satisfont pas les criteres de milestone.

## References

- <https://github.com/emilk/egui>
- <https://docs.rs/egui-wgpu/>

