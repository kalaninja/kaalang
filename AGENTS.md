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
- `docs/architecture.md` maps the compiler and renderer steps to their modules.
  Read it before changing how the crates divide the work.
- `GLOSSARY.md` names the terms the RFCs, crates, diagnostics, and tests share.
  Read it before naming a new concept.
- `crates/kaalang-compiler/` parses and resolves flows into the shared validated
  semantic model, decides the diagram, exposes its common arrangement checks,
  and lowers the verified plan to Rust. It also parses type declarations and
  expands datatypes.
- `crates/kaalang-macros/` owns the procedural macro entry point and nothing
  else. A `proc-macro` crate may export only its macros, so anything kept here
  could not be called, tested, or measured from another crate.
- `crates/kaalang-render/` owns cycle-boundary checks and compacts a verified
  arrangement into the shared rows, columns, and routes every renderer starts
  from.
- `crates/kaalang-svg/` measures the shared arrangement and renders standalone
  SVG without changing its structural layout, and lays out and renders type
  diagrams.
- `crates/kaalang-testing/` owns the shared test material: the fixture corpus,
  generated performance probes and cycle shapes, and the harness the budgets
  use. Nothing outside a test depends on it.
- `crates/kaalang-cli/` owns the `cargo kaalang` subcommand, shipped as the
  `cargo-kaalang` binary.
- `crates/kaalang/` exports the public macro and owns its test suite. Behavior
  tests are executable examples; compile-fail tests document rejected forms. The
  `.svg` beside each `flow/<feature>/<flow>.rs` is drawn by `just rust svg` and
  reviewed in the diff. Never edit one by hand.
- `crates/kaalang/examples/gallery/` holds the gallery modules and generated
  SVGs. Run the gallery with `cargo run -p kaalang --example gallery` or
  `cargo test -p kaalang --example gallery`. Draw its diagrams through the CLI
  with `just rust gallery`. The fixture corpus is collected from
  `crates/kaalang/tests/`.

Do not restate the language contract in this guide or the root README. This
guide links the relevant RFC instead. The README is an overview of the language:
it links the RFC directory once and never a specific RFC.
