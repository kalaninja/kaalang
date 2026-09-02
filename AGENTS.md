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
- `crates/kaalang-model/` parses, resolves, and analyzes flows into the shared
  validated semantic model.
- `crates/kaalang-macros/` owns the procedural macro entry point and Rust code
  generation.
- `crates/kaalang-svg/` lays out validated models and renders standalone SVG,
  including the `cargo-kaalang` CLI.
- `crates/kaalang/` exports the public macro and owns its test suite. Behavior
  tests are executable examples; compile-fail tests document rejected forms.
  The `.svg` beside each `behavior/<flow>.rs` is drawn by `cargo test -p kaalang`
  and reviewed in the diff; never edit one by hand.

Do not restate the language contract in this guide or the root README. Link to
the relevant RFC instead.
