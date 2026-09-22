# RoboGen

RoboGen is a native, declarative robotics design environment written in Rust. A
`.rgn` document is the source of truth; CAD, rendering, robot models and future
simulation/optimisation pipelines are projections of the same semantic model.

## Current vertical slice

The current milestone provides:

- a native dark desktop shell for Design, Optimisation and Training;
- a typed, diagnostic-producing parser for the constrained-bracket DSL subset;
- parameter-aware sketch evaluation and deterministic extrusion to a triangle mesh;
- live rebuild after editing `WIDTH` or `HEIGHT` in the DSL editor;
- binary STL export through the CLI or desktop export action;
- explicit unavailable states for AI, topology optimisation and RL.

Run the UI with `cargo run -p robogen-desktop`, or compile/export the example:

```sh
cargo run -p robogen-cli -- check examples/constrained_bracket/main.rgn
cargo run -p robogen-cli -- export-stl examples/constrained_bracket/main.rgn bracket.stl
```

Quality gates are `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace`.

