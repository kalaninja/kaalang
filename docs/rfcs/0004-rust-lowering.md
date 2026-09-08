# RFC 0004: kaalang Rust Lowering

- Status: accepted design draft
- Language: [RFC 0001: kaalang Language](0001-language.md)

## 1. Overview

The `#[kaalang]` attribute translates a validated kaalang flow into ordinary
Rust bindings and control-flow expressions. [RFC 0001](0001-language.md)
defines which programs are valid and what they mean. This RFC describes how
lowering implements that contract; it does not add language rules.

Before lowering, the source is parsed, producer occurrences are grouped by
logical wire name, and block-local and branch-dependent invariants are
validated. Lowering consumes the resulting validated model and its verified
execution plan. The [visual language](0002-visual-language.md) uses the same
plan to show the chosen execution order.

The examples below pair complete kaalang functions with illustrative Rust.
They show the relevant bindings, scopes, and control flow, not a promised
token-for-token expansion. Names beginning with `wire_` stand for hygienic
generated bindings; their plain Rust spellings do not demonstrate macro hygiene.
Unit control-wire bindings and generated lint attributes are omitted where the
branch structure already shows their role. Internal names, routing encodings,
and the generator's own data structures are implementation details.

## 2. Bindings, scopes, and actions

Named function parameters become internal Rust bindings for flow-input wires.
A wildcard parameter remains a wildcard. Each authored block body receives
local aliases only for its listed inputs: a consuming input binds the wire's
value, and a borrowed input binds a shared reference to it. Hygienic internal
names keep omitted and consumed wires unavailable under their authored names.
Block-local Rust names cannot change another block's wire resolution.

An action becomes a `let` initializer containing its input aliases and authored
body. Its declared outputs become the binding pattern: one output binds the
whole body value, including for a singleton output declaration; multiple outputs
use a tuple pattern. These Rust bindings implement [RFC 0001
§4.1](0001-language.md#41-action) and
[§6](0001-language.md#6-wires-producers-and-captures).

For example, the first action borrows a string and produces two outputs. The
second action consumes the original string together with those outputs:

```rust
use kaalang::kaalang;

#[kaalang]
fn measure(text: String) -> (String, usize, bool) {
    #[action("Measure the text.")]
    |&text| -> (length, empty) { (text.len(), text.is_empty()) };

    #[action("Return the text and its measurements.")]
    |text, length, empty| -> result { (text, length, empty) };
}
```

Illustrative Rust:

```rust
fn measure(wire_text: String) -> (String, usize, bool) {
    let (wire_length, wire_empty) = {
        let text = &wire_text;
        (text.len(), text.is_empty())
    };
    let wire_result = {
        let text = wire_text;
        let length = wire_length;
        let empty = wire_empty;
        (text, length, empty)
    };
    wire_result
}
```

Generated aliases use ordinary Rust moves and borrows to implement the captures
specified in [RFC 0001 §6](0001-language.md#6-wires-producers-and-captures).
Output patterns and body expressions leave the type and ownership checks
specified there to Rust. Alternative producers must also be checked against one
inferred wire type when no common Rust binding would otherwise unify their types.

## 3. Questions, merges, and execution order

A question becomes an `if` whose condition evaluates the input aliases and
authored body once. Its two arms bind the selected unit-valued control wire and
execute the corresponding continuation.

A merge is implemented by passing a selected producer value out of the branch
expression into the binding used by subsequent captures. Several merged wires
can be passed together as a tuple. The shared continuation is emitted once
after this binding. This implements the merge and ordering rules of [RFC 0001
§7](0001-language.md#7-execution-and-implicit-convergence).

```rust
use kaalang::kaalang;

#[kaalang]
fn choose(condition: bool) -> u32 {
    #[question("Use the first value?")]
    |condition| -> (yes, no) { condition };

    #[action("Build the first value.")]
    |yes| -> selected { 1 };

    #[action("Build the second value.")]
    |no| -> selected { 2 };

    #[action("Add ten to the selected value.")]
    |selected| -> result { selected + 10 };
}
```

Illustrative Rust:

```rust
fn choose(wire_condition: bool) -> u32 {
    let wire_selected = if {
        let condition = wire_condition;
        condition
    } {
        1
    } else {
        2
    };
    let wire_result = {
        let selected = wire_selected;
        selected + 10
    };
    wire_result
}
```

Lowering follows the verified plan's permitted serial order, including all
branch work required before a merge and wire production required before a
consumer-selecting question or choice. An inferred ordering adds no input alias
to that selection. Independent ready blocks can be placed in any order the
language permits; lowering does not introduce concurrent execution.

The nested expressions above illustrate a structured flow. A validated plan
that needs availability checks can instead lower to a sequence of guarded
blocks with optional wire values. Borrowing reads an available value;
consumption takes it out of its slot. Both forms implement the same validated
dependencies, merges, and execution order, and emit each authored block body
once. The choice of form adds no source-language restriction.

## 4. Choices and case values

Lowering preserves the authored choice `match`, including its scrutinee,
patterns, guards, arm order, and arm expressions. It evaluates the scrutinee
once and carries the selected arm's value out of that arm before executing the
case continuation. This scope boundary implements [RFC 0001
§4.3](0001-language.md#43-choice).

The example uses a two-case `Result` to carry values of different types to a
second dispatch. Here `Ok` and `Err` distinguish cases; they do not represent
success and failure of the flow. The representation is illustrative.

```rust
use kaalang::kaalang;

#[kaalang]
fn length_or_zero(input: Option<String>) -> usize {
    #[choice("Was text supplied?")]
    #[case("Text is available.")]
    #[case("No text is available.")]
    |input| -> (text, absent) {
        match input {
            Some(value) => value,
            None => (),
        }
    };

    #[action("Measure the supplied text.")]
    |text| -> result { text.len() };

    #[action("Return zero for absent text.")]
    |absent| -> result { 0 };
}
```

Illustrative Rust:

```rust
fn length_or_zero(wire_input: Option<String>) -> usize {
    let wire_case = {
        let input = wire_input;
        match input {
            Some(value) => ::core::result::Result::Ok(value),
            None => ::core::result::Result::Err(()),
        }
    };
    let wire_result = match wire_case {
        ::core::result::Result::Ok(wire_text) => {
            let text = wire_text;
            text.len()
        }
        ::core::result::Result::Err(()) => 0,
    };
    wire_result
}
```

The authored binding `value` ends with its arm. Returning `&value` instead would
attempt to carry a reference to an arm-owned local across this boundary and
Rust would reject it. A value borrowing a borrowed input can cross the boundary
when that input outlives the choice. Placing the continuation inside the
authored arm would incorrectly extend the local's availability.

## 5. End and terminal branches

The implicit end block becomes the selected `result` value in the function's
return position. Normally this is a tail expression. A merge of alternative
`result` producers happens before that return, under [RFC 0001
§4.4](0001-language.md#44-end); end does not become a separate computational
continuation or merge operation.

When one branch finishes the flow while its siblings yield a value to an
enclosing merge, lowering can use a Rust `return` for the finished branch. It
returns that branch's `result`, avoiding the continuation it does not enter.
This generated control transfer implements the end block and does not permit
an authored `return` in a computational body.

```rust
use kaalang::kaalang;

#[kaalang]
fn finish_or_join(refine: bool, finish_early: bool) -> u32 {
    #[question("Refine the value?")]
    |refine| -> (nested, direct) { refine };

    #[question("Finish early?")]
    |nested, finish_early| -> (skip, join) { finish_early };

    #[action("Finish before the shared step.")]
    |skip| -> result { 100 };

    #[action("Build the refined value.")]
    |join| -> shared { 1 };

    #[action("Build the direct value.")]
    |direct| -> shared { 2 };

    #[action("Add ten to the shared value.")]
    |shared| -> result { shared + 10 };
}
```

Illustrative Rust:

```rust
fn finish_or_join(wire_refine: bool, wire_finish_early: bool) -> u32 {
    let wire_shared = if {
        let refine = wire_refine;
        refine
    } {
        if {
            let finish_early = wire_finish_early;
            finish_early
        } {
            return 100;
        }
        1
    } else {
        2
    };
    let wire_result = {
        let shared = wire_shared;
        shared + 10
    };
    wire_result
}
```

A zero-computation flow from [RFC 0001
§5.1](0001-language.md#51-zero-computation-flow) needs only its input binding
and the tail expression:

```rust
use kaalang::kaalang;

#[kaalang]
fn identity<T>(result: T) -> T {}
```

Illustrative Rust:

```rust
fn identity<T>(wire_result: T) -> T {
    wire_result
}
```

Rust checks each return value against the authored function return type,
including `()` when the return type is omitted.

## 6. Placeholders and function attributes

An authored `todo!()` remains in the lowered body. For a choice placeholder,
lowering also emits the downstream case continuations so Rust checks every
branch even though reaching the placeholder panics. This preserves the
placeholder behavior specified by [RFC 0001 §4.3](0001-language.md#43-choice).

A placeholder diverges, so Rust reports code lowered after it as unreachable.
Lowering does not suppress `unreachable_code`; a workspace that denies warnings
rejects such an unfinished flow unless the author opts out. An authored
`#[allow(unreachable_code)]` on the function is re-emitted with the function's
other attributes, implementing the opt-out described by [RFC 0001
§5](0001-language.md#5-flow-inputs-and-outputs).

Expansion replaces the kaalang body and renames named parameters for hygiene.
It preserves the function's name, visibility, generics, parameter and return
types, qualifiers, where clause, and other function attributes. The
`#[kaalang]` attribute and block and case descriptions are consumed during
translation; the closure-shaped declarations do not become callable Rust
closures.
