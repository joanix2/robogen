# Bipede - disposition parametrique des composants

Tache : [ticket](task_20261007_14-biped-layout.md).

# Iteration 1

- [x] Verifier le toolchain et la suite existante ; consigner les limites M0-M3.
- [x] Documenter ADR et implementer rotate avec angles types, couleurs, booleens et compounds.
- [x] Creer la disposition autonome et parametrique des 17 composants, sans supports factices.
- [x] Ajouter le chargement UI annulable et des tests de la tranche complete.
- [x] Verifier la vue native, documentation et gates Rust ; enregistrer les limites et le statut Git.

## Preuves

- Base : 88 tests, Rust/Cargo 1.88.0 ; M0-M3 non declares complets.
- IR/CAD : diagnostics d'angles, rotations composees, plans booleens,
  volumes/fermeture/couleurs et transforms de compounds verifies.
- Projet : 17 enveloppes separees, dimensions et definitions commerciales
  preservees, deplacement parametrique et IDs stables verifies.
- Integration : source -> maillages fermes colores -> STL millimetrique.
- UI : chargement asynchrone, export, undo/redo, refus d'export obsolete,
  visibilite et cadrage complet a 600x400 et 320x240.
- 94 tests workspace, fmt, Clippy -D warnings et builds passes. Apres la
  correction native de cadrage : test UI cible, fmt, Clippy et build repasses.
- Capture finale mini-biped-native.png inspectee apres clics dans le menu natif.
- Pas de commit : base preexistante du ticket 12 ouverte/non committée ; aucun
  staging ni changement de lane. Suite contraintes/generation dans le ticket 15.

# Iteration 2 - Checkpoint global autorise

- [x] Revoir .gitignore avant le commit global (demande utilisateur) ; 28 exclusions et 18 chemins conserves verifies.
- [x] Verifier les fichiers indexes et les captures ; aucun fichier indexe ne correspond aux exclusions.
- [x] Preparer le checkpoint de tous les travaux courants sans declarer les tickets ouverts termines ; autorisation « commit tout ».
