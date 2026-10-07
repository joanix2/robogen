# Optimisation topologique et profils d'impression parametrables

Ticket differe : aucune implementation autorisee par cette checklist.

# Iteration 1 - A reprendre apres les prerequisites M7

- [ ] Confirmer la reprise et les capacites disponibles de M7/ADR-008.
- [ ] Definir les contrats types, unites, provenance, schema et priorites des profils/substitutions ; ADR si necessaire.
- [ ] Ajouter les profils machine/materiau/fabrication editables et le prereglage Finder sans inventer les champs inconnus.
- [ ] Exposer enveloppes, zones preservees/exclues, appuis et cas de charge.
- [ ] Exposer les reglages du solveur et les contraintes de fabrication avec validation et limites explicites.
- [ ] Relier source, UI, persistance et commandes annulables ; verifier les allers-retours et diagnostics.
- [ ] Integrer le backend M7 hors UI avec progression, annulation et garde de revision.
- [ ] Revalider la geometrie reconstruite et comparer reference/optimise avec provenance des mesures.
- [ ] Integrer une estimation de trancheur configure ou afficher explicitement temps/supports indisponibles.
- [ ] Verifier les risques FDM, documenter les hypotheses et definir un protocole d'essais physiques.
- [ ] Valider tests unitaires/integration, demonstration native, documentation, fmt, Clippy et tests workspace.