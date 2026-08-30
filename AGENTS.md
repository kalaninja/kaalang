# Contour Agent Guide

## Repository language

Use English for documentation, filenames, identifiers, source code, comments,
tests, commit messages, and pull request content.

## Repository map

- `CONTRIBUTING.md` defines the development workflow, terminology policy,
  validation baseline, and commit convention. Read it before making or
  committing changes.
- `docs/rfcs/` is the source of truth for Contour syntax, semantics, scope, and
  design decisions. Read the relevant RFCs before conceptual or syntax changes.
- `crates/contour-macros/` implements the procedural macro compilation pipeline:
  parsing, flow resolution, analysis, and code generation.
- `crates/contour/` exports the public macro and owns its test suite. Behavior
  tests are executable examples; compile-fail tests document rejected forms.

Do not restate the language contract in this guide or the root README. Link to
the relevant RFC instead.
