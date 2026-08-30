# RFC 0001: Contour Core Model

- Status: executable discussion draft
- Model version: `0.1`
- Implementation target: Rust

## 1. Overview

Contour is a language for writing flows as valid Rust. It is inspired by DRAKON
but defines its own syntax and semantics.

A Contour flow is an ordinary Rust function marked with `#[contour]`. The
attribute validates the flow and lowers it to ordinary Rust control flow. A flow
contains blocks connected by named wires, and its visible dependencies determine
which block executes next.

Contour uses these terms consistently:

- a **flow** is one function marked with `#[contour]`;
- a **block** is an action, question, choice, or merge statement;
- a **wire** is a named connection between the flow boundary and blocks;
- an **input** names a wire before `->`, and an **output** declares a wire after
  `->`;
- a **branch** is one continuation selected by a question or choice;
- a choice **case** describes one branch, while the corresponding Rust `match`
  **arm** implements it;
- a **path** is the sequence of blocks executed through selected branches;
- a **terminal action** is the last block on a path.

## 2. Flow boundary

Function parameters declare source wires. The function return type is the
contract for every terminal action output.

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

The function body contains closure-shaped Rust expression statements. Each
statement declares one block. The final statement may omit its semicolon as a
Rust tail expression may.

The example is a complete flow whose computational bodies are placeholders.
`todo!()` retains its Rust behavior and panics if execution reaches it.

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
ordered `#[case("description")]` attributes. A merge uses the bare `#[merge]`
attribute because its behavior is fully structural.

Source comments remain ordinary Rust comments. Block and case descriptions come
from their attributes.

`#[contour]` consumes the closure-shaped syntax and executes it through ordinary
Rust bindings and expressions.

## 4. Wires and inputs

A wire is represented by an ordinary Rust binding. Source wires come from
function parameters, and block outputs declare later wires. Every wire name is
unique within a flow.

Each block lists every wire available to its body:

```rust
|&request, policy, yes| -> decision { /* body */ };
```

Inputs have two forms:

- `name` consumes the wire on the current path;
- `&name` borrows the wire and leaves it available to later blocks on that path.

Every input names a source wire or an output declared by an earlier block. Each
block has at least one input, and duplicate inputs are invalid.

Consumption is a Contour rule independent of Rust's `Copy` trait. A block body
receives local bindings only for its listed inputs, so omitted and consumed
wires are out of scope. Rust locals declared inside a block remain local to that
body and may reuse a wire's spelling without changing wire resolution.

Rust checks the concrete wire types, moves, borrows, and output destructuring.
Contour keeps types out of wire declarations and relies on Rust inference.

## 5. Action

An action evaluates a Rust expression and binds its value to one output or a
tuple of outputs:

```rust
#[action("Split the value.")]
|input| -> (left, right) {
    split(input)
};
```

The output declaration becomes a Rust binding pattern, so Rust checks that the
body value has the declared shape. An action with consumed outputs continues to
their downstream blocks.

An action is terminal when its single output has no consumer. That output
becomes the result of the current path and must satisfy the flow's return type.

## 6. Question

A question evaluates a boolean expression and selects one of two positional
branches:

```rust
#[question("Is the request eligible?")]
|valid, &request, &policy| -> (eligible, ineligible) {
    request.age_days <= policy.window_days
};
```

The first output selects the yes/true branch, and the second selects the
no/false branch. The question body is evaluated exactly once and must produce
`bool`.

Question outputs are unit-valued control wires. Downstream blocks list the
selected control wire and every data wire they need as separate inputs.

## 7. Choice

A choice selects one of two or more ordered branches:

```rust
#[choice("What is the sign of the value?")]
#[case("The value is negative.")]
#[case("The value is zero.")]
#[case("The value is positive.")]
|value| -> (negative, zero, positive) {
    match value {
        ..0 => (),
        0 => (),
        _ => (),
    }
};
```

Cases, outputs, and match arms correspond by position and have equal counts.
Each case describes its branch in domain language. Its Rust pattern, optional
guard, bindings, and value remain in the corresponding match arm.

The match scrutinee is evaluated exactly once. The selected arm value becomes
the value of its output wire. Different choice outputs may therefore carry
different Rust types. For example, `Some(value) => value` carries the selected
value, while `None => ()` produces a unit-valued control wire.

An implemented choice body contains exactly one `match` expression. An exact
whole-body `todo!()` is also a valid placeholder. Match bindings and block
locals remain scoped to the selected arm and are unavailable to downstream
blocks; downstream code receives only declared wires.

Each choice branch continuation is entered at most once. Rust ownership rejects
any extra invocation of a generated continuation.

## 8. Merge

A merge combines two or more alternative branch outputs into one wire:

```rust
#[merge]
|negative_value, zero_value, positive_value| -> value {};

#[action("Return the selected value.")]
|value| -> result { value };
```

Merge inputs are bare, consuming identifiers. Exactly one input is available on
each path that reaches the merge. The merge has one output and an empty body.

Continuing branches of one question or choice converge at the same merge.
Sibling branches may instead end at terminal actions. The enclosing Rust `if`
or `match` produces the merge output, so Rust checks that all merged values have
one type.

Only wires available on every continuing branch remain available after the
merge. The merge output is added to that shared set and its continuation is
lowered once.

## 9. Paths and flow result

A valid flow has exactly one ready block at each execution step. A regular
block is ready when all its inputs are available. A merge is ready when one of
its alternative inputs is available.

Every block is reachable from the source wires. At a question or choice,
validation follows each branch independently. A branch either reaches the
common merge selected by its siblings or ends at a terminal action.

Every path ends at a terminal action with one output. Lowered Rust returns that
output from its branch and checks it against the function return type.

## 10. Grammar

The outer syntax is ordinary Rust:

```text
flow := "#[contour]" rust_function

action_statement :=
    "#[action(" block_description ")]"
    "|" input_list "|" "->" output_declaration rust_block ";"

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

choice_body := "{" rust_match_expression "}" | "{" "todo!()" "}"
choice_match_arm := rust_pattern rust_guard? "=>" rust_expression

merge_statement :=
    "#[merge]"
    "|" identifier "," identifier ("," identifier)* "|" "->"
        identifier "{" "}" ";"

case_attribute := "#[case(" block_description ")]"

block_description := nonempty_rust_string_literal
output_declaration := identifier | rust_tuple_of_identifiers
input_list := input ("," input)* ","?
input := identifier | "&" identifier
```

The final block statement may omit the semicolon. Computational descriptions
and input lists are nonempty. Choice cases, outputs, and match arms have equal
counts. A merge has at least two inputs.

## 11. Validation and execution

`#[contour]` parses block syntax, resolves every input to its producing wire,
validates block-local and path-dependent invariants, and lowers the flow to
nested Rust `let`, `if`, and `match` expressions.

An action binds its body value to its outputs. A question evaluates its body
once and executes the selected branch. A choice preserves the authored match
and passes the selected arm value to the corresponding branch. A merge binds
the enclosing branch expression to its output before executing the shared
continuation.

Rust checks body types, match exhaustiveness, ownership, borrows, output
patterns, merged value types, and terminal outputs. Contour's generated scopes
keep omitted wires, consumed wires, match bindings, and block locals outside
downstream block bodies.

An authored `todo!()` remains in the lowered body. A choice placeholder still
type-checks every downstream branch, and execution panics if it reaches the
placeholder.

A placeholder diverges, so Rust reports every block lowered after it as
unreachable code. `#[contour]` does not suppress this: an unfinished flow is
meant to be visible, and a workspace that denies warnings rejects one until its
placeholders are written. An author who wants the flow quiet while filling it in
writes `#[allow(unreachable_code)]` on the function, which `#[contour]` re-emits
with the function's other attributes.
