# Contour Agent Guide

## Project

Contour is an early-stage DRAKON-inspired graph syntax embedded in valid Rust.
The current milestone validates and directly executes the first textual graph
subset.

Read these files before conceptual or syntax changes:

- `README.md` for the current prototype;
- `docs/rfcs/0001-core-model.md` for semantic decisions;
- `examples/playground/src/refund_review.rs` for the canonical example.

## Repository language

Use English for documentation, filenames, identifiers, source code, comments,
examples, fixtures, tests, commit messages, and pull request content.

## Current status

Version `0.1` is an executable discussion draft. `#[contour]` parses the flat
graph, validates its supported invariants, and lowers it directly to nested Rust
`if`, `match`, and `let` expressions. Local action, question, choice, and case
attributes contain block descriptions and are consumed by `#[contour]`.

Prefer small, reversible changes that test one language hypothesis at a time.
A `todo!()` block body marks a supported author skeleton and panics only if
execution reaches it.

## Core invariants

- An ordinary `#[contour]` Rust function is the graph boundary.
- Its parameters are source wires and its return type is the terminal contract.
- The body uses closure-shaped Rust expression statements.
- Every block shows inputs before `->` and output identifiers after it.
- Every block has exactly one local `#[action("description")]`,
  `#[question("description")]`, or `#[choice("description")]` attribute with a
  nonempty Rust string literal.
- Source comments remain non-semantic and never replace the attribute
  description.
- `#[contour]` consumes the closure syntax without generating or calling a
  runtime closure.
- Every input is an individual Rust binding listed between closure pipes as
  `name` or `&name`.
- Bare inputs are consumed by the graph and `&` inputs borrow. Internal wire
  bindings keep omitted or consumed names out of block scope even for `Copy`
  values.
- An action may declare one identifier or a tuple of output identifiers; body
  result types remain inferred by Rust.
- A question has exactly two outputs: yes/true first and no/false second; its
  body must evaluate to `bool`.
- A choice has at least two ordered `#[case("description")]` attributes and the
  same number of tuple outputs. Its implemented body is exactly one Rust
  `match` with the same number of arms. Cases, outputs, and arms correspond
  positionally; each arm value becomes the value of its output wire, and payload
  types may differ between outputs. Case text is semantic description, not a
  Rust pattern.
- `todo!()` means that an agent has not implemented the block yet; it never
  supplies implicit behavior.
- Question outputs are unit-valued control dependencies. Choice outputs carry
  only the value explicitly returned by their corresponding match arm.
- Every reachable path ends at an action with exactly one unconsumed output.
- Terminal action outputs must match the function return type; generated Rust
  performs that concrete type check.
- The function boundary becomes one synthetic End in descriptors and diagrams.
- Inline Rust cannot read graph wires omitted from the closure parameter list.
  Lowering stores wires under internal names and creates source-spelled aliases
  only inside the block that captured them. Hygienic choice continuations keep
  match bindings and block locals out of downstream scopes.

## Current scope

The prototype includes required attribute descriptions, local block markers,
closure-shaped block statements, explicit consuming and borrowing inputs and
declared outputs, hidden-capture isolation, direct question/action/choice
lowering, and Rust-checked return types. A graph whose bodies are `todo!()`
remains a supported author-written skeleton.

It excludes a runtime scheduler, a `Wire` wrapper, descriptors, stable block
IDs, loops, joins, merges, parallel paths, subflows, rendering, layout,
serialization, and external graph files.

## Change workflow

When changing syntax or semantics:

1. State the design problem and chosen tradeoff.
2. Update RFC 0001 and the relevant playground example together.
3. Keep README, tests, and this guide synchronized.
4. Add positive and compile-fail coverage for the changed grammar.
5. Run formatting, checks, Clippy, and all workspace tests.

Preserve unrelated user changes and do not add speculative syntax.

## Validation baseline

The attribute rejects missing, non-string, or empty block descriptions,
imperative body statements, malformed or duplicate block markers, non-closure
block statements, invalid input or output declarations, empty or duplicate
captures, unknown or later inputs, duplicate wires, wrong question output
arity, malformed or mismatched choice cases and outputs, choice bodies other
than one `match` or exact `todo!()`, mismatched choice arm and output counts,
unconsumed question or choice branches, unreachable blocks, ambiguous next
blocks, non-action terminal paths, and invalid terminal output arity. Rust
rejects non-boolean conditions, non-exhaustive choice matches, incompatible
scrutinee patterns, invalid downstream uses of choice payloads, hidden wire
captures, ownership errors, incompatible action destructuring, and terminal
outputs incompatible with the function return type.

The attribute does not support parallelism, joins, merges, or loops.
