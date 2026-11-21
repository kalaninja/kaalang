# kaalang Agent Guide

## Repository map

- `CONTRIBUTING.md` maps the crates and defines the workflow, code conventions,
  terminology policy, validation, and commit convention. Read it before making
  or committing changes.
- `docs/rfcs/` is the source of truth for kaalang syntax, semantics, scope, and
  design decisions. Read the relevant RFCs before conceptual or syntax changes.
- `docs/architecture.md` maps the compiler and renderer steps to their modules
  and to the RFC paragraphs in force. Read it before changing how the crates
  divide the work.
- `GLOSSARY.md` names the terms the RFCs, crates, diagnostics, and tests share.
  Read it before naming a new concept.

## Documentation

Minimum text, maximum meaning. Every sentence states a fact, a decision, or a
rule the reader cannot get elsewhere; cut the rest. No introductions,
restatements, or summaries.

The README is an overview of the language: it links the RFC directory once and
never a specific RFC.
