# RoboGen DSL

`.rgn` files are the human-readable source of truth. The milestone grammar is
documented by the executable example in
[`examples/constrained_bracket/main.rgn`](../../examples/constrained_bracket/main.rgn).

The parser is recovery-oriented: it retains a partial spanned AST and returns
structured diagnostics for editor display. Semantic compilation resolves names
and physical units before CAD evaluation. Renderer, CAD-kernel and physics
types are deliberately absent from the syntax layer.

