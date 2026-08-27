# RFC 0001: Contour Syntax Model

- Status: executable discussion draft
- Model version: `0.1`
- Implementation target: Rust

## 1. Problem

Contour needs a readable textual graph whose execution agrees with its visible
topology. This revision keeps the graph body valid Rust and implements the
smallest useful executor: compile-time validation followed by direct lowering
to ordinary Rust control flow.

## 2. Canonical source

The graph lives in an ordinary Rust function marked with `#[contour]`:

```rust
use contour::contour;

#[contour]
fn decide(request: Request) -> Decision {
    #[question("Is the request valid?")]
    |&request| -> (valid, invalid) { todo!() };

    #[action("Approve the valid request.")]
    |valid, &request| -> approved { todo!() };

    #[action("Reject the invalid request.")]
    |invalid, &request| -> rejected { todo!() };
}
```

Function parameters are initial wires, the names after each block arrow declare
outputs, and the function return type is the contract for terminal action
outputs. Lowering turns those output declarations into ordinary Rust `let`
patterns. There is no outer graph macro, explicit end block, custom `<-`
operator, external compiler, `build.rs`, generated source file, YAML
representation, or external graph file.

The example is intentionally complete as a graph but incomplete as an
implementation. The author supplies intent and topology first. An agent later
replaces `todo!()` bodies without changing the graph contract.

## 3. Attribute boundary

`#[contour]` is a procedural attribute because it inspects the whole function
and validates graph-wide invariants. It parses the current flat
subset into one graph AST, validates it, and replaces the source statements with
nested Rust `if` and `let` expressions. Executable behavior and validation thus
come from the same graph.

The attribute lives in a separate `proc-macro` crate, uses `syn` and `quote`, and
is re-exported by `contour`.

Local `#[action("description")]` and `#[question("description")]` attributes
mark closure-shaped statements for the surrounding procedural attribute. Their
nonempty Rust string literal is the exact natural-language description. A
nearby source comment remains non-semantic and is never substituted for that
description.

Parameters before `->` list separate inputs. Identifiers after `->` declare
outputs, and the block expression is the implementation body. Rust parses the
output position as a closure return type; `#[contour]` deliberately reinterprets
its simple identifiers as output names before type checking.

The local markers are consumed by `#[contour]`; they are not independently
exported attribute macros. The closure syntax is parsed but reinterpreted by
`#[contour]`, so lowering creates no runtime closure. This keeps one parser and
makes block bodies ordinary Rust expressions that `rustfmt` formats normally. A
marker used outside a `#[contour]` function is rejected by Rust.

Rust requires every closure expression to have a body. The canonical author
skeleton therefore uses `todo!()` as an explicit implementation placeholder;
an incomplete `|inputs|;` expression would not be valid Rust.

## 4. Wires and inputs

A wire is the graph role of an ordinary Rust binding, not a runtime wrapper.
Function parameters introduce source wires and block arrows introduce later
wires. Lowering represents them as ordinary Rust bindings.

Each block lists every incoming connection separately:

```rust
|&request, policy, yes| -> decision { /* body */ };
```

The closure parameters have graph-capture semantics. They identify existing
wires; they are not runtime closure parameters.

- `&request` borrows the existing binding and exposes the same name inside the
  block;
- `policy` consumes the existing binding and exposes the same name inside the
  block;
- aliases and arbitrary input expressions are not accepted;
- every block currently requires at least one input.

`#[contour]` checks graph name resolution and declaration order. Generated Rust
checks concrete types, moves, borrows, and output destructuring. A value can feed
several later blocks by borrowing it each time. There is no `Bundle` or implicit
grouping of inputs.

Consumption is a graph rule before it is a Rust move. `#[contour]` drops a bare
capture from its compile-time path state, but lowering keeps every wire as an
ordinary `let` in one scope, so Rust rejects a second use only for non-`Copy`
values. A `Copy` wire stays readable after the block that consumed it.

Inline Rust is normatively forbidden from reading function locals omitted from
the parameter list. Body capture analysis is not implemented yet, so a block
body can read any earlier wire and nothing reports it. A future validation pass
must enforce this rule and close the `Copy` gap above.

## 5. Action block

An action is an attributed closure-shaped statement. An author can declare it
without implementation code:

```rust
#[action("Build the decision.")]
|yes, &request| -> decision { todo!() };
```

An implementation agent replaces the placeholder body:

```rust
#[action("Build the decision.")]
|yes, &request| -> decision {
    Decision::approved(request)
};
```

An identifier or a tuple of identifiers may appear after the arrow:

```rust
#[action("Split the value.")]
|input| -> (left, right) {
    split(input)
};
```

The body owns leaf computation but not graph routing. It may call ordinary Rust
functions or be a Rust block expression and must not invoke subsequent graph
blocks. The arrow declares output names rather than duplicating their Rust
types; concrete types are inferred from the body and checked by Rust. `todo!()`
means explicitly unimplemented; it never means a no-op, unit result, or default
value.

An action is terminal when its output has no consumer. Every reachable graph
path ends at a terminal action, and each terminal action exposes exactly one
output. During lowering, that output becomes the result of its control-flow
branch. Rust checks it against the function return type without Contour
duplicating the concrete type in graph syntax.

## 6. Question block

A question is a DRAKON question with two positional control results. Like an
action, it may first be declared using only intent and connections:

```rust
#[question("Is the request eligible?")]
|valid, &request, &policy| -> (eligible, ineligible) { todo!() };
```

The implementation agent replaces the placeholder with a boolean expression:

```rust
#[question("Is the request eligible?")]
|valid, &request, &policy| -> (eligible, ineligible) {
    request.age_days <= policy.window_days
};
```

The first result means yes/true and the second means no/false. Their names are
arbitrary. The body must evaluate to `bool` and is evaluated exactly once after
all inputs are available. A `todo!()` body has no default result.

Yes/no are control wires only. They never contain copies of `request`, `policy`,
or other inputs. Downstream blocks list the selected control and every required
data wire explicitly. Successor blocks remain outside the question.

`#[contour]` requires exactly two output names and lowers the condition to a
Rust `if`. It binds only the selected unit-valued control output in each branch.

## 7. Flow termination

There is no public `end!` block. The closing function boundary and its return
type already express the single logical end of the algorithm:

```rust
#[contour]
fn decide(request: Request) -> Decision {
    // Questions and actions omitted.
    #[action("Approve the request.")]
    |yes, &request| -> approved { todo!() };

    #[action("Reject the request.")]
    |no, &request| -> rejected { todo!() };
}
```

"Terminal" is a graph property, not source position: `approved` and `rejected`
are terminal because they are reachable and have no downstream consumer. The
attribute requires every path to finish at such an action with exactly one
output. Lowered Rust branches return their respective output values, so Rust
enforces compatibility with `Decision`.

Descriptors and renderers should derive exactly one synthetic DRAKON End from
the function boundary and connect every mutually exclusive terminal result to
it. This structural node needs no user-authored description or extra merge
syntax.

## 8. Grammar

The outer syntax is ordinary Rust:

```text
contour_function := "#[contour]" rust_function

action_statement :=
    "#[action(" block_description ")]"
    "|" capture_list "|" "->" output_declaration rust_block ";"

question_statement :=
    "#[question(" block_description ")]"
    "|" capture_list "|" "->"
        "(" identifier "," identifier ")" rust_block ";"

block_description := nonempty_rust_string_literal
output_declaration := identifier | rust_tuple_of_identifiers
capture_list := input ("," input)* ","?
input := identifier | "&" identifier
```

The attribute description and capture list are nonempty. Comments may surround
a block but have no graph meaning. The closure body is required by Rust grammar.
`todo!()` is the canonical body for an unimplemented block; no separate status
flag exists.

## 9. Validation and lowering

The attribute resolves every closure parameter to a function parameter or an
earlier block output, then follows the available wires from the unique starting
block. It converts the arrow's identifiers into a Rust `let` pattern and
extracts the closure body rather than constructing or calling a closure. At a
question it generates one Rust `if`, evaluates the condition once, and
continues separately with the yes or no control output. At an action it binds
the body result to the declared output pattern. Each terminal action output is
the result of its generated branch.

Bare captures are consumed and disappear from that compile-time path state;
borrowed captures remain available. Exactly one block must be ready at each
step. This deliberately rejects parallel execution, joins, and implicit routing
instead of adding a runtime scheduler.

An author-written `todo!()` remains in the lowered body. The skeleton therefore
compiles, but execution fails if it reaches an unimplemented block.

## 10. Deferred work

Version `0.1` does not define graph descriptors, IDs, hidden-capture validation,
implementation-status descriptors, runtime scheduling, match/select, joins,
merges, parallel paths, loops, subflows, async behavior, visualization, layout,
or serialization.

`rust-analyzer` provides diagnostics, completion, and control-wire rename through
the expanded macro. A single Rename does not currently span both graph-level
data-wire captures and their block-local body bindings, and Go to Definition may
offer both the capture and its producer. Graph-aware navigation and refactoring
remain deferred editor tooling.

The surface Rust syntax should remain unchanged unless a separate design
decision revises it.

## 11. Success criteria

The draft succeeds when:

1. a realistic flat graph is valid and readable Rust;
2. each block shows its inputs and outputs around one arrow;
3. an author-only refund graph with `todo!()` bodies compiles;
4. missing block intent, name-resolution, ownership, destructuring, and
   condition errors have compile-fail coverage;
5. implemented question branches execute exactly one terminal action;
6. flow termination requires no redundant public block syntax.
