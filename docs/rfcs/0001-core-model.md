# RFC 0001: kaalang Core Model

- Status: accepted design draft
- Model version: `0.1`
- Implementation target: Rust

## 1. Overview

kaalang is a language for writing flows as valid Rust. It is inspired by DRAKON
but defines its own syntax and semantics.

kaalang's defining property is that visual semantics and execution share one
model. A diagram is not an architectural sketch implemented separately; it is a
view of the same validated flow that kaalang lowers for execution. Reviewers can
therefore reason about program behavior from the diagram without translating it
to another implementation or wondering whether the two have diverged.

A **flow** is a kaalang computation composed of blocks. Named wires make values
available to blocks, while questions and choices divide execution into
alternative branches. Every completed execution reaches the flow's explicit end
block.

In the Rust representation, a flow is written as an ordinary function marked
with `#[kaalang]`. The attribute validates this representation and lowers it to
ordinary Rust control flow. Wire dependencies determine which blocks are ready;
independent ready blocks may execute in any order.

## 2. Terminology

These terms describe the kaalang language independently of its representation
or visualization:

- **flow inputs** are values supplied when a flow begins; named flow inputs
  provide wires;
- **flow outputs** are the ordered values captured by the end block when a flow
  finishes;
- a **block** is one declared unit of a flow; its inputs name the wires it
  captures, and its outputs name the wires it produces;
- an **input** names an available wire captured by a block; a bare input
  consumes the wire, while a `&` input borrows it;
- an **output** declares a wire produced by a block;
- a **producer** is one occurrence of a named flow input or block output that
  provides a wire;
- **alternative producers** provide the same logical wire from mutually
  exclusive branches; at most one is available in any execution;
- a **consumer** is a block whose input captures a wire;
- a **capture** associates a block input with a logical wire; whenever the
  consumer executes, the capture resolves to one available producer occurrence;
- a **capture dependency** exists from a producer occurrence to a consumer when
  at least one possible execution can resolve one of the consumer's captures to
  that occurrence. Such an execution respects branch selection, wire
  availability, and consumption; an equal wire name alone does not establish a
  capture dependency. The dependency participates in every execution where that
  resolution occurs;
- a **wire** is a named value provided by a producer and available for capture
  by block inputs;
- a **branch** is one alternative continuation selected by a question or
  choice;
- a **case** describes one branch of a choice and is not itself a block;
- a **path** is one dependency-ordered chain of blocks through selected
  branches; one execution may contain several paths;
- **independent blocks** can execute in either order without changing the
  availability of each other's inputs;
- the **continuation** of a branch is the set of computational blocks whose
  readiness can transitively depend on that branch's selected output through a
  chain of capture dependencies participating in one possible execution that
  selects the branch; the end block is not part of a continuation;
- relative to one question or choice, the **branch set** of a downstream block
  is the exact set of its branches whose continuations contain that block;
- a **convergence group** is a branch set containing at least two branches;
- the **shared continuation** of a convergence group is the nonempty set of
  computational blocks whose branch set is exactly that group;
- a **continuing branch** for a convergence group is one of its branches. It may
  contain paths that reach the end block without entering the shared
  continuation after a nested question or choice;
- a **terminal branch** belongs to no convergence group of its own question or
  choice;
- a **convergence** occurs where the branches of a convergence group enter its
  shared continuation; a **convergence point** is a block in that continuation
  that no other block of the continuation precedes.

## 3. Block statements

A computational block has this common shape:

```rust
#[action("Description of the block.")]
|input, &borrowed| -> output {
    rust_expression
};
```

`#[action]`, `#[question]`, and `#[choice]` each carry one nonempty Rust string
literal. The string is the block description. A choice also carries two or more
ordered `#[case("description")]` attributes. The end block uses the bare
`#[end]` attribute because its behavior is fully structural.

Source comments remain ordinary Rust comments. Block and case descriptions come
from their attributes.

`#[kaalang]` consumes the closure-shaped syntax and lowers it to ordinary Rust
bindings and expressions.

An authored computational block body must not use a `return` expression or the
`?` operator in its own control-flow scope; it can complete only by normal
evaluation. Occurrences inside a nested closure, async block, or item definition
belong to that nested Rust construct and are permitted. Macro token streams are
opaque to this validation.

## 4. Block kinds

| Block kind | Meaning | Inputs | Outputs |
| --- | --- | --- | --- |
| **action** | performs a computation or effect | zero or more | zero or more |
| **question** | evaluates a logical expression and selects one of two branches | one or more | exactly two unit-valued control wires, one per branch |
| **choice** | selects one of two or more cases and provides a value to the corresponding branch | one or more | one per case, at least two |
| **end** | the unique block that finishes a flow; its inputs form the flow outputs | zero or more | none |

### 4.1 action

An action evaluates its body and may declare zero or more outputs:

```rust
#[action("Split the value.")]
|input| -> (left, right) {
    split(input)
};
```

An action that consumes no wires and performs only an effect may declare neither
inputs nor outputs:

```rust
#[action("Log flow entry.")]
|| -> () {
    println!("start")
};
```

The `-> ()` declaration means zero outputs and requires the body to evaluate to
unit. With one declared output, the complete body value is bound to that wire.
The bare `output` and singleton tuple `(output,)` declarations are equivalent.
With two or more outputs, the body's outer tuple is destructured positionally.
Rust checks that the body value has the required shape.

Independent actions may execute in either order. An action can establish an
order explicitly by producing a named unit-valued wire for a dependent block to
capture:

```rust
#[action("Log flow entry.")]
|| -> entered {
    println!("start")
};

#[action("Continue.")]
|entered| -> () {
    continue_work()
};
```

An action never terminates a flow implicitly.

### 4.2 question

A question evaluates a logical expression and selects one of two positional
branches:

```rust
#[question("Is the request eligible?")]
|valid, &request, &policy| -> (eligible, ineligible) {
    request.age_days <= policy.window_days
};
```

The first output selects the yes/true branch, and the second selects the
no/false branch. The question body is evaluated exactly once, and its result
determines which branch is selected.

Question outputs are distinct unit-valued control wires. Downstream blocks list
the selected control wire and every data wire they need as separate inputs.

### 4.3 choice

A choice selects one of two or more ordered branches and makes the selected
match arm's value available on the corresponding output wire:

```rust
#[choice("Was a value supplied?")]
#[case("A value is available.")]
#[case("No value is available.")]
|input| -> (value, absent) {
    match input {
        Some(value) => value,
        None => (),
    }
};
```

Cases, outputs, and match arms correspond by position and have equal counts.
Each case describes its branch in domain language. The corresponding match arm
contains its Rust pattern, optional guard, bindings, and value.

The match scrutinee is evaluated exactly once. Unlike question outputs, choice
outputs are not restricted to `()`, and different outputs may carry different
Rust types.

An implemented choice body contains exactly one `match` expression. An exact
whole-body `todo!()` is also a valid placeholder. Match bindings and block
locals remain scoped to the selected arm and are unavailable to downstream
blocks; downstream code receives only declared wires.

The selected arm's value leaves the match before the branch continues. A case
value is therefore owned or borrows data that outlives the choice, such as a
borrowed input; it cannot borrow a match binding or another local of its arm.
Each choice branch continuation is entered at most once and cannot be reached
from the authored body.

### 4.4 end

The end block is the unique structural block at which every completed execution
finishes:

```rust
#[end]
|out1, out2, out3| {};
```

The end block is the final authored statement. It has no description, outputs,
or body expressions. Its inputs are bare, consuming identifiers, and its input
list may be empty.

The end block captures logical wires by name. When a captured wire has
alternative producers, the available producer is whichever one ran; the
alternatives do not appear in the `#[end]` syntax. Whenever execution reaches
the end block, exactly one producer for every captured name must be available.

With zero inputs, the end block captures no flow outputs. With one input, the
complete wire value is the sole flow output. With two or more inputs, the
ordered flow outputs form a Rust tuple. Rust checks this value against the
function return type. Capturing an explicitly produced wire whose Rust type is
`()` remains distinct from a zero-input end block: the former is a real named
wire, while the latter captures none.

## 5. Flow inputs and outputs

In the Rust representation, function parameters declare flow inputs. Parameters
written as simple identifiers provide wires. A wildcard parameter (`_`) accepts
and discards a flow input and provides no wire. Other parameter patterns,
including modified bindings and destructuring patterns, are invalid, as is a
method receiver.

Every named flow-input producer must have at least one capture dependency to a
block or the end block unless its spelling begins with `_`;
that prefix permits it to have no consumer. This requirement is existential: one
possible execution establishing the dependency is sufficient, and the wire may
remain uncaptured in other executions. The function return type is the contract
for the ordered flow outputs captured by the end block.

```rust
use kaalang::kaalang;

#[kaalang]
fn decide(request: Request) -> Decision {
    #[question("Is the request valid?")]
    |&request| -> (valid, invalid) { todo!() };

    #[action("Approve the valid request.")]
    |valid, &request| -> result { todo!() };

    #[action("Reject the invalid request.")]
    |invalid, &request| -> result { todo!() };

    #[end]
    |result| {};
}
```

The two actions are alternative producers of the logical `result` wire. The end
block captures whichever producer ran. The example is a complete flow whose
computational bodies are placeholders. `todo!()` retains its Rust behavior and
panics if execution reaches it.

The function body contains closure-shaped Rust expression statements. Each
statement declares one block. Every flow has exactly one end statement, and it
appears last. Like a Rust tail expression, it may omit its semicolon.

### 5.1 Zero-computation flow

A flow may contain no computational blocks, but it still declares its mandatory
end block:

```rust
#[kaalang]
fn nothing() {
    #[end]
    || {};
}
```

This flow has no flow inputs, flow outputs, or wires. Rust represents the
function result as `()`, but kaalang does not create an implicit unit-valued
wire. A completely empty body is invalid because it does not declare an end
block.

An ignored parameter remains explicit:

```rust
#[kaalang]
fn discard(_value: Value) {
    #[end]
    || {};
}

#[kaalang]
fn discard_unnamed_input(_: Value) {
    #[end]
    || {};
}
```

`_value` is a named flow input that provides a wire permitted to remain
uncaptured. `_` declares an unnamed flow input and provides no wire.

## 6. Wires, producers, and captures

Named flow inputs provide the initial wires, and block outputs provide later
wires. The selected branch determines which alternative producer provides a
logical wire.

A logical wire normally has one producer. Several blocks may declare the same
output name only when they are alternative producers. This is not sequential
shadowing: one execution cannot produce the same logical wire twice even if the
earlier value was consumed.

Raw and ordinary spellings of the same Rust identifier name the same wire, so
`value` and `r#value` are interchangeable. Every producer occurrence is authored
before every consumer of its logical wire; a later producer cannot retroactively
join a wire that has already appeared as an input.

Every block-output producer occurrence must have at least one capture dependency
to a later block or the end block unless its name begins with `_`;
that prefix permits the occurrence to have no consumer. This requirement is
existential rather than per-execution and applies separately to every action,
question, and choice output occurrence.

```rust
#[question("Which value should be used?")]
|condition| -> (yes, no) { condition };

#[action("Build the yes value.")]
|yes| -> selected { yes_value() };

#[action("Build the no value.")]
|no| -> selected { no_value() };

#[action("Use the selected value.")]
|selected| -> result { use_value(selected) };

#[end]
|result| {};
```

The shared consumer captures `selected` by name. Under either branch selection
exactly one producer is available, so the consumer is the implicit convergence
point and is executed once. kaalang does not have a merge block.

Each computational block lists every wire available to its body:

```rust
|&request, policy, yes| -> decision { /* body */ };
```

Inputs have two forms:

- `name` consumes the wire in the current execution;
- `&name` borrows the wire and leaves it available to later blocks in that
  execution.

Every input names a wire provided by a flow input or an earlier block output.
Questions and choices have at least one input; an action may have none. Duplicate
inputs are invalid. Whenever a consumer executes, each captured name resolves to
exactly one available producer. No producer is an error; more than one proves
that the supposed alternative producers are not mutually exclusive and is also
an error. Producer resolution is occurrence-level and branch-feasible: a
same-named downstream input does not depend on a particular producer occurrence
unless some possible execution can reach the consumer with that occurrence's
value available.

Consumption is a kaalang rule independent of Rust's `Copy` trait. A block body
receives local bindings only for its listed inputs, so omitted and consumed
wires are out of scope. Rust locals declared inside a block remain local to that
body and may reuse a wire's spelling without changing wire resolution.

Rust checks concrete wire types, moves, borrows, alternative producer type
agreement, and output destructuring. kaalang keeps types out of wire declarations
and relies on Rust inference.

## 7. Execution and implicit convergence

A computational block is ready when all its inputs are available. An action
with no inputs is ready when the flow begins. At each execution step, any ready
computational block may execute. Ready blocks are independent when executing
one cannot change the availability of another's inputs. Independent ready
blocks may execute in any order; kaalang does not thereby guarantee concurrent
execution. If order matters, a wire from one block's output to another block's
input must establish the dependency.

Validation rejects a flow that can make non-independent blocks ready together.
In particular, two consuming blocks, or a consuming and a borrowing block,
cannot be ready for the same wire at the same time.

Each computational block executes at most once. The end block is ready when its
flow outputs are available and no unexecuted computational block is ready. This
ensures that independent active parts of a flow finish before the end block,
including when the end block has no inputs.

Validation computes continuations and convergence groups separately for each
question and choice. One question or choice may have several convergence groups,
but its distinct groups must be disjoint; overlapping groups are invalid.

The branches of each convergence group converge at that group's convergence
points. Its shared continuation is represented once in the semantic model, and
each of its blocks executes at most once.

Read in authored choice-case order, the branches of each convergence group are
adjacent: no case outside the group may separate two of its members. Several
disjoint groups are valid and may be separated by cases outside either group. A
question satisfies this rule automatically because it has only two branches.

At each convergence point, a logical wire is available exactly when every
possible execution reaching that point has exactly one available producer for
it immediately before the point executes. Those producers may be the same
occurrence or alternative occurrences. Rust checks that alternative values have
one type. A wire that is unavailable at a convergence point in some execution
reaching it stays unavailable after the point; branches that have converged
diverge again only through a later question or choice.

Every authored computational block either has no inputs or can become ready from
flow inputs and earlier block outputs. Every branch reaches the same end block.
Branches may reach the end block after different numbers of computational blocks.
Neither an action nor an output permitted to remain uncaptured finishes a flow.

## 8. Grammar

The outer syntax is ordinary Rust:

```text
flow := "#[kaalang]" rust_function

flow_parameter := identifier ":" rust_type | "_" ":" rust_type

action_statement :=
    "#[action(" block_description ")]"
    "|" input_list? "|" "->" action_output_declaration rust_block ";"

question_statement :=
    "#[question(" block_description ")]"
    "|" input_list "|" "->"
        "(" identifier "," identifier ")" rust_block ";"

choice_statement :=
    "#[choice(" block_description ")]"
    case_attribute case_attribute+
    "|" input_list "|" "->"
        "(" identifier "," identifier ("," identifier)* ")"
        choice_body ";"

end_statement :=
    "#[end]"
    "|" end_input_list? "|" "{" "}" ";"?

choice_body := "{" rust_match_expression "}" | "{" "todo!()" "}"
choice_match_arm := rust_pattern rust_guard? "=>" rust_expression

case_attribute := "#[case(" block_description ")]"

block_description := nonempty_rust_string_literal
action_output_declaration := "()" | output_declaration
single_output_declaration := identifier | "(" identifier "," ")"
output_declaration := single_output_declaration | rust_tuple_of_two_or_more_identifiers
input_list := input ("," input)* ","?
input := identifier | "&" identifier
end_input_list := identifier ("," identifier)* ","?
```

Every flow has exactly one end statement, and it is the final authored
statement. The final statement may omit its semicolon. Computational
descriptions are nonempty. Questions and choices require a nonempty input list;
actions and the end block accept an empty one. Choice cases, outputs, and match
arms have equal counts. Outputs within one declaration are distinct. Repeated
output names across blocks are valid only when branch analysis proves their
producers mutually exclusive. Each flow parameter uses `flow_parameter`; a
method receiver is invalid. Validation rejects `return` expressions and the `?`
operator in a computational block body's own control-flow scope. It descends
through ordinary parsed expressions, including choice match scrutinees, guards,
and arms, but not into nested closures, async blocks, item definitions, or macro
token streams.

There is no merge statement.

## 9. Validation and execution

`#[kaalang]` parses block syntax, groups producer occurrences by logical wire
name, validates block-local and branch-dependent invariants, and lowers the flow
to nested Rust `let`, `if`, and `match` expressions.

An action evaluates its body and binds its value to its outputs when it declares
any. Independent computational blocks may be lowered in any order allowed by
their wire dependencies. A question evaluates its body once and executes the
selected branch. A choice preserves the authored match and passes the selected
arm value to the corresponding branch. When producer occurrences in sibling
branches can resolve to the same downstream input, lowering carries the selected
value along each participating capture dependency. A computational target
belongs to the shared continuation of the corresponding convergence group and
binds the logical wire once. The end block instead captures the selected value
as part of the flow result without becoming a shared continuation or convergence
point.

The end block lowers to an empty Rust body, its one captured wire, or the
ordered tuple of its captured wires. Rust checks body types, match
exhaustiveness, ownership, borrows, output patterns, alternative producer types,
and the flow outputs.

kaalang's generated scopes keep omitted wires, consumed wires, match bindings,
and block locals outside downstream block bodies.

An authored `todo!()` remains in the lowered body. A choice placeholder still
type-checks every downstream branch, and execution panics if it reaches the
placeholder.

A placeholder diverges, so Rust reports every block lowered after it as
unreachable code. `#[kaalang]` does not suppress this: an unfinished flow is
meant to be visible, and a workspace that denies warnings rejects one until its
placeholders are written. An author who wants the flow quiet while filling it in
writes `#[allow(unreachable_code)]` on the function, which `#[kaalang]` re-emits
with the function's other attributes.
