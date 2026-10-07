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

# Verification des prerequisites - 2026-10-08

- [x] Relire les criteres M0-M7 et distinguer validations et avancements partiels.
- [x] Verifier les blocages dans le code sketch/robotique/physique, sans cocher les jalons absents.
- [x] Completer deux preuves natives M1 : Optimisation et Apprentissage en 1440x831, sans resultats fictifs.
- [x] Fixer l'ordre autorise avant d'entamer FEM/SIMP : M7 autonome explicitement autorise le 2026-10-08.

Le premier item de l'iteration 1 reste ouvert : dependances et donnees physiques
ne sont pas validees par cet audit.

# Iteration 3 - M7 autonome autorise, poutre a charges manuelles

- [x] Valider le port sparse faer : solution connue, residu, entrees invalides et singularite.
- [x] Implementer FEM hexaedrique, poutre de reference et validations analytiques/invariants.
- [x] Implementer SIMP, filtre de densite, OC et historique ; verifier sensibilites et volume.
- [x] Exposer un calcul de poutre reproductible avec provenance, progression et annulation (API synchrone et exemple autonome, aucun appel UI).
- [x] Documenter mesures et limitations et verifier les gates (114 tests workspace, fmt, Clippy -D warnings ; benchmark reel a 65 iterations).

L'autorisation du 2026-10-08 leve l'ordre M1-M6 uniquement pour cette tranche
scientifique M7. Reconstruction, publication desktop et supports robot seront
suivis explicitement ; pas de cloture du jalon complet sans leurs validations.

Checkpoint local autorise pour cette tranche scientifique validee. La suite
reconstruction/publication reste ouverte dans le TODO M7 et l'iteration 1.
