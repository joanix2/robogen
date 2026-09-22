# ADR-004 — Parser du DSL

- Statut : accepte pour le MVP
- Date : 2026-09-22

## Contexte

Le DSL est la source de verite. Il doit etre deterministe, modulaire, fortement type et fournir des diagnostics localises sans panic. Le futur editeur exige spans, analyse partielle d'un fichier invalide et patchs cibles. Le parser ne doit connaitre ni CAD, ni physique.

## Options considerees

- lexer `logos` et grammaire LR(1) `LALRPOP` : grammaire explicite, generation compile-time, types Rust.
- `pest`/PEG : grammaire accessible mais priorites et alternatives ordonnees peuvent masquer des ambiguïtés.
- parser combinator : controle fin de la recuperation, au prix d'une grammaire dispersee dans le code.
- parser manuscrit : controle maximal, cout de maintenance et risque de divergence documentaire.

## Decision (cible normative)

Pour la tranche verticale, utiliser un lexer et un parser de descente manuels,
orientés récupération, limités au sous-ensemble exécutable. Les tokens
contiennent un `Span` en offsets d'octets. La grammaire produit un AST purement
syntaxique ; résolution des symboles, types et unités a lieu dans
`robogen-ir`. `logos` + LALRPOP reste la cible lorsque la grammaire modulaire
sera stabilisée, sans changer l'AST public ni le corpus de diagnostics.

La recuperation se fait aux frontieres stables (`;`, `}`, debut de declaration) et produit des noeuds d'erreur afin de poursuivre l'analyse. Les commentaires et trivia sont conserves dans une couche CST/token stream utilisee par le patcher, sans polluer l'AST semantique.

Le parser est contraint par des golden tests : entree valide, multiples diagnostics, EOF tronque, tokens inconnus, profondeur limitee et fuzzing sans panic.

## Etat implemente au 2026-09-22

[Le lexer/parser actuel](../../crates/robogen-dsl/src/lib.rs) est manuel et conserve des spans d'octets sur les tokens/AST. En revanche, `lex` saute les espaces et les commentaires `//` sans les emettre dans le flux de tokens. Il n'existe pas de CST conservant les trivia ni de patcher assurant la preservation des commentaires et de l'ordre hors du bloc modifie.

La chaine source conservee par `Project` n'est pas une preuve de round-trip : ses overrides de parametres modifient un AST clone lors du rebuild, sans patcher cette source. La preservation et les tests d'edition ciblee relevent de M2 (responsable DSL) ; leur raccordement aux commandes texte/GUI releve de M3. Les exigences CST, recuperation et corpus ci-dessus ne constituent pas un bilan de couverture deja validee et ne changent pas le choix de parser.

## Consequences

- syntaxe centralisee, reproductible et inspectable ;
- pas d'étape de génération au MVP, mais davantage de code manuel à maintenir ;
- la recuperation d'erreur doit etre concue explicitement et testee ;
- CST/trivia et AST impliquent deux representations aux responsabilites distinctes ;
- l'analyse semantique reste testable sans parser via des fixtures AST.

## Strategie de remplacement

Figer d'abord le flux de tokens, les types AST, les spans et le corpus golden.
Un nouveau parser doit produire le même AST et des diagnostics équivalents en
localisation/catégorie. Le parser reste privé à `robogen-dsl`; le remplacer par
LALRPOP/Logos ne modifie ni l'IR ni le format projet.

Reexaminer ce choix si la recuperation multi-erreurs ne satisfait pas l'edition interactive ou si le temps d'analyse incremental depasse le budget mesure.

## References

- <https://docs.rs/lalrpop/>
- <https://github.com/maciejhirsz/logos>
