# kaalang Agent Guide

## Repository language

Use English for documentation, filenames, identifiers, source code, comments,
tests, commit messages, and pull request content.

## Repository map

- `CONTRIBUTING.md` defines the development workflow, code organization
  conventions, terminology policy, validation baseline, and commit convention.
  Read it before making or committing changes.
- `docs/rfcs/` is the source of truth for kaalang syntax, semantics, scope, and
  design decisions. Read the relevant RFCs before conceptual or syntax changes.
- `crates/kaalang-compiler/` parses and resolves flows into the shared validated
  semantic model, decides the diagram, and lowers the verified plan to Rust.
- `crates/kaalang-macros/` owns the procedural macro entry point and nothing
  else. A `proc-macro` crate may export only its macros, so anything kept here
  could not be called, tested, or measured from another crate.
- `crates/kaalang-svg/` lays out validated models and renders standalone SVG.
- `crates/kaalang-testing/` owns the shared test material: the fixture corpus,
  generated performance probes and cycle shapes, and the harness the budgets
  use. Nothing outside a test depends on it.
- `crates/kaalang-cli/` owns the `cargo kaalang` subcommand, shipped as the
  `cargo-kaalang` binary.
- `crates/kaalang/` exports the public macro and owns its test suite. Behavior
  tests are executable examples; compile-fail tests document rejected forms. The
  `.svg` beside each `behavior/<flow>.rs` is drawn by `cargo test -p kaalang`
  and reviewed in the diff; never edit one by hand.

Do not restate the language contract in this guide or the root README. Link to
the relevant RFC instead.
