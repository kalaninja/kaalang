# Contour Agent Guide

## Repository language

Use English for documentation, filenames, identifiers, source code, comments,
tests, commit messages, and pull request content.

## Repository map

- `docs/rfcs/` is the source of truth for Contour syntax, semantics, scope, and
  design decisions. Read the relevant RFCs before conceptual or syntax changes.
- `crates/contour-macros/` implements the procedural macro compilation pipeline:
  parsing, graph construction, analysis, and code generation.
- `crates/contour/` exports the public macro and owns its test suite. Behavior
  tests are executable examples; compile-fail tests document rejected forms.

Do not restate the language contract in this guide or the root README. Link to
the relevant RFC instead.

## Change workflow

When changing syntax or semantics:

1. State the design problem and chosen tradeoff.
2. Update the relevant RFC together with the implementation.
3. Add behavior and compile-fail coverage as appropriate.
4. Keep the change small and reversible.

Preserve unrelated user changes and do not add speculative syntax.

## Validation

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
