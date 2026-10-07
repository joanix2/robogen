# Bipede - contraintes DSL et supports generes

Tache : [ticket](task_20261007_15-biped-constraints-topopt.md).

# Iteration 1

- [ ] Verifier les dependances M0-M7 et la validite des donnees physiques.
- [ ] Ecrire ADR et contrats types/versionnes pour interfaces, liaisons et OptimizationSpec.
- [ ] Implementer syntaxe/lowering/diagnostics et edition annulable des contraintes.
- [ ] Definir regions preservees/exclues et domaines des supports du bipede.
- [ ] Reutiliser profils parametrables du ticket 13 et solveur M7/M8.
- [ ] Generer et revalider un premier support camera puis un support de jambe multi-cas.
- [ ] Valider export, fabrication, annulation/revisions et gates ; distinguer marche et TopOpt.

# Iteration 2 - Contrat topology composable

- [x] Documenter ADR-012, proprietaires, unites, identites et resultat differe.
- [x] Implementer TopologySpec type/versionne et constructeur topology dans le graphe Solid (tests de composition et IDs passes).
- [x] Verifier interfaces/charges/appuis, objectifs distincts des contraintes et profils de fabrication (validation declarative, aucune garantie numerique).
- [x] Raccorder refus explicite de backend et interdiction de maillage/export factice, avec provenance (tests CAD/projet passes).
- [x] Ajouter exemple support de batterie, inspection CLI et chargement UI annulable (tests CLI/UI passes).
- [x] Valider tests unitaires et verticaux, diagnostics, historique/revisions, documentation et gates (103 tests workspace, fmt, Clippy, builds CLI/desktop, inspection CLI et capture native 1440x831).
