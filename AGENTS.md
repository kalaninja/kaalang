# Contour Agent Guide

## Project

Contour is an early-stage DRAKON-inspired graph syntax embedded in valid Rust.
The current milestone validates and directly executes the first textual graph
subset.

Read these files before conceptual or syntax changes:

- `README.md` for the current prototype;
- `docs/rfcs/0001-core-model.md` for semantic decisions;
- `examples/refund-review/src/flow.rs` for the canonical example.

## Repository language

Use English for documentation, filenames, identifiers, source code, comments,
examples, fixtures, tests, commit messages, and pull request content.

## Current status

Version `0.1` is an executable discussion draft. `#[contour]` parses the flat
graph, validates its supported invariants, and lowers it directly to nested Rust
`if` and `let` expressions. Local action and question attributes contain block
descriptions and are consumed by `#[contour]`.

Prefer small, reversible changes that test one language hypothesis at a time.
A `todo!()` block body marks a supported author skeleton and panics only if
execution reaches it.

## Core invariants

- An ordinary `#[contour]` Rust function is the graph boundary.
- Its parameters are source wires and its return type is the terminal contract.
- The body uses closure-shaped Rust expression statements.
- Every block shows inputs before `->` and output identifiers after it.
- Every block has exactly one local `#[action("description")]` or
  `#[question("description")]` attribute with a nonempty Rust string literal.
- Source comments remain non-semantic and never replace the attribute
  description.
- `#[contour]` consumes the closure syntax without generating or calling a
  runtime closure.
- Every input is an individual Rust binding listed between closure pipes as
  `name` or `&name`.
- Bare inputs are consumed by the graph and `&` inputs borrow; Rust enforces
  that consumption only for non-`Copy` values.
- An action may declare one identifier or a tuple of output identifiers; body
  result types remain inferred by Rust.
- A question has exactly two outputs: yes/true first and no/false second; its
  body must evaluate to `bool`.
- `todo!()` means that an agent has not implemented the block yet; it never
  supplies implicit behavior.
- Yes/no are control dependencies and never carry input data implicitly.
- Every reachable path ends at an action with exactly one unconsumed output.
- Terminal action outputs must match the function return type; generated Rust
  performs that concrete type check.
- The function boundary becomes one synthetic End in descriptors and diagrams.
- Inline Rust must not read local values omitted from the closure parameter
  list. Lowering emits ordinary `let` bindings in one scope, so a body can
  still read any earlier wire, including a `Copy` wire another block consumed.
  Body capture validation must close that gap.

## Current scope

The prototype includes required attribute descriptions, local block markers,
closure-shaped block statements, explicit consuming and borrowing inputs and
declared outputs, direct question/action lowering, and Rust-checked return
types. A graph whose bodies are `todo!()` remains a supported author-written
skeleton.

It excludes a runtime scheduler, a `Wire` wrapper, descriptors, stable block
IDs, hidden-capture validation, loops, match/select blocks, joins, merges,
parallel paths, subflows, rendering, layout, serialization, and external graph
files.

## Change workflow

When changing syntax or semantics:

1. State the design problem and chosen tradeoff.
2. Update RFC 0001 and the refund example together.
3. Keep README, tests, and this guide synchronized.
4. Add positive and compile-fail coverage for the changed grammar.
5. Run formatting, checks, Clippy, and all workspace tests.

Preserve unrelated user changes and do not add speculative syntax.

## Validation baseline

The attribute rejects missing, non-string, or empty block descriptions,
imperative body statements, malformed or duplicate block markers, non-closure
block statements, invalid input or output declarations, empty or duplicate
captures, unknown or later inputs, duplicate wires, wrong question output
arity, unconsumed question branches, unreachable blocks, ambiguous next blocks,
non-action terminal paths, and invalid terminal output arity. Rust rejects
non-boolean conditions, ownership errors for non-`Copy` wires, incompatible
action destructuring, and terminal outputs incompatible with the function
return type.

The attribute does not yet detect hidden inline captures and does not support
parallelism, joins, merges, or loops.
