# ADR-003 — Solveur de contraintes de sketch

- Statut : accepte pour le MVP
- Date : 2026-09-22

## Contexte

Le sketcher doit resoudre incrementellement des contraintes 2D, indiquer les degres de liberte et diagnostiquer conflits et redondances. SolveSpace/libslvs est eprouve mais GPL-3.0-or-later et son solver est couple a du C++. Cette contrainte de licence et de packaging est importante pour une application desktop portable.

## Options considerees

- libslvs : riche et eprouve, mais FFI et GPL pour le binaire lie.
- backend Rust natif numerique : controle des types, diagnostics et licence ; travail algorithmique supplementaire.
- solveur symbolique generaliste : exact sur certains cas mais mal adapte a l'edition interactive et aux contraintes geometriques non lineaires.
- backend de sketch fourni par un kernel CAD : couplage indesirable entre sketch et B-Rep.

## Decision (cible normative)

Construire un backend natif limite aux contraintes V1 necessaires aux tranches verticales. `robogen-sketch` produit un `ConstraintProblem` sans type de backend. `robogen-constraints` :

1. decompose le graphe en composantes ;
2. elimine les variables fixees et applique une normalisation d'echelle ;
3. minimise les residus par Gauss-Newton amorti/Levenberg-Marquardt avec Jacobienne analytique quand disponible ;
4. estime rang et DOF a partir du Jacobien ;
5. retourne solution, residus par contrainte, statut (`Solved`, `Underconstrained`, `Overconstrained`, `Diverged`) et un ensemble conflictuel approximatif.

Le MVP implemente d'abord point, segment, cercle, coïncidence, horizontal, vertical, distance, distance X/Y, rayon/diametre et fix. Les autres contraintes sont ajoutees seulement avec tests. Une solution precedente sert d'initialisation lors d'une edition.

Un adaptateur libslvs peut exister comme feature non activee par defaut pour comparaison, uniquement si ses obligations GPL sont clairement propagees au packaging.

## Etat implemente au 2026-09-22

[NativeConstraintSolver](../../crates/robogen-constraints/src/lib.rs) recoit un `Sketch`, pas un `ConstraintProblem` normalise. Il traite des rectangles : controle d'existence des entites et de dimensions positives, application de distances X/Y et detection de dimensions contradictoires pour un meme axe/rectangle. Horizontal, vertical et fix participent au comptage des contraintes.

`remaining_degrees_of_freedom` vaut `4 * nombre_de_rectangles` moins un compteur de contraintes, sature a zero ; `fully_constrained` signifie seulement que ce compteur atteint zero. Ce n'est pas une mesure du rang : des contraintes redondantes peuvent reduire ce nombre sans enlever une liberte geometrique. Il n'y a ni Jacobienne, ni minimisation iterative, ni residus par contrainte, ni diagnostic general de sur-contrainte. La decision numerique et la couverture V1 ci-dessus restent la cible M4 (responsables CAD/contraintes), a valider par le corpus de contrat avant de presenter ces DOF comme un diagnostic general.

## Consequences

- aucun FFI ni obligation GPL dans la distribution par defaut ;
- controle precis des unites, tolerances et diagnostics ;
- la couverture V1 est volontairement plus petite et la detection d'un conflit minimal n'est pas garantie ;
- les tests golden doivent couvrir sous/sur-contrainte, degenerescences, invariance d'echelle et continuite lors du drag ;
- l'UI ne deduit jamais les DOF elle-meme.

## Strategie de remplacement

`ConstraintSolver` est teste par un corpus independant du backend. Un remplacant doit respecter les conventions d'unites, l'ordre stable des variables, les tolerances et le schema de diagnostics. Les sketches persistent entites et contraintes, pas l'etat interne du solveur ; aucun format projet ne change.

Reexaminer le backend natif si les sketches realistes ne convergent pas dans le budget interactif, si les contraintes V1 ne suffisent plus ou si une solution permissive mature apparait.

## References

- <https://solvespace.com/download.pl>
- <https://solvespace.github.io/solvespace-web/library.html>

