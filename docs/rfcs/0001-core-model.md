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

A kaalang flow is an ordinary Rust function marked with `#[kaalang]`. The
attribute validates the flow and lowers it to ordinary Rust control flow. A flow
contains blocks connected by named wires, and its visible dependencies determine
which block executes next.

kaalang uses these terms consistently:

- a **flow** is one function marked with `#[kaalang]`;
- a **block** is an action, question, choice, or End statement;
- a **wire** is a named logical connection between one or more path-exclusive
  producers and its consumers;
- an **input** captures a wire before `->`, and an **output** produces a wire
  after `->`;
- a **branch** is one continuation selected by a question or choice;
- a choice **case** describes one branch, while the corresponding Rust `match`
  **arm** implements it;
- a **path** is the sequence of blocks executed through selected branches;
- **End** is the unique structural block at which every path finishes.

## 2. Flow boundary

Function parameters written as simple identifiers produce source wires. A
wildcard parameter (`_`) accepts and discards its argument at the boundary and
produces no wire. A named wire whose spelling begins with `_` may be left
unconsumed intentionally. The function return type is the contract for the
ordered wires captured by End.

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

The two actions produce the same logical `result` wire on mutually exclusive
paths. End captures whichever producer ran. The example is a complete flow
whose computational bodies are placeholders. `todo!()` retains its Rust
behavior and panics if execution reaches it.

The function body contains closure-shaped Rust expression statements. Each
statement declares one block. Every flow contains exactly one End statement,
and End is the final authored statement. The final statement may omit its
semicolon as a Rust tail expression may.

### 2.1 Zero-computation flow

A flow may contain no computational blocks, but it still declares its mandatory
End:

```rust
#[kaalang]
fn nothing() {
    #[end]
    || {};
}
```

This flow has zero source wires, zero result wires, and no wire connecting its
boundaries. Rust represents the function result as `()`, but kaalang does not
create an implicit unit-valued wire. A completely empty body is invalid because
it does not declare End.

An ignored parameter remains explicit:

```rust
#[kaalang]
fn discard(_value: Value) {
    #[end]
    || {};
}

#[kaalang]
fn discard_at_the_boundary(_: Value) {
    #[end]
    || {};
}
```

`_value` is a named source wire permitted to remain unconsumed. `_` declares no
source wire at all.

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
ordered `#[case("description")]` attributes. End uses the bare `#[end]`
attribute because its behavior is fully structural.

Source comments remain ordinary Rust comments. Block and case descriptions come
from their attributes.

`#[kaalang]` consumes the closure-shaped syntax and executes it through ordinary
Rust bindings and expressions.

## 4. Wires, producers, and inputs

A wire is represented by an ordinary Rust binding after all alternative
producers have converged. Source wires come from named function parameters, and
block outputs declare later producers.

A logical wire normally has one producer. Several blocks may declare the same
output name only when their producer occurrences are path-exclusive: no
execution path may run more than one of them. This is not sequential shadowing;
declaring the same output name twice on one path is invalid even if the earlier
value was consumed.

Raw and ordinary spellings of the same Rust identifier name the same wire, so
`value` and `r#value` are interchangeable. Every producer occurrence is authored
before every consumer of its logical wire; a later producer cannot retroactively
join a wire that has already appeared as an input.

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

The shared consumer captures `selected` by name. On either path exactly one
producer is available, so the consumer is the implicit convergence point and is
executed once. kaalang does not have a merge block.

Each computational block lists every wire available to its body:

```rust
|&request, policy, yes| -> decision { /* body */ };
```

Inputs have two forms:

- `name` consumes the wire on the current path;
- `&name` borrows the wire and leaves it available to later blocks on that path.

Every input names a source wire or an output declared by an earlier block. Each
computational block has at least one input, and duplicate inputs are invalid.
On every path reaching a consumer, each captured name resolves to exactly one
available producer. No producer is an error; more than one proves that the
supposed alternatives are not path-exclusive and is also an error.

Consumption is a kaalang rule independent of Rust's `Copy` trait. A block body
receives local bindings only for its listed inputs, so omitted and consumed
wires are out of scope. Rust locals declared inside a block remain local to that
body and may reuse a wire's spelling without changing wire resolution.

Rust checks concrete wire types, moves, borrows, alternative producer type
agreement, and output destructuring. kaalang keeps types out of wire declarations
and relies on Rust inference.

## 5. Action

An action evaluates a Rust expression and binds its value to one or more
outputs:

```rust
#[action("Split the value.")]
|input| -> (left, right) {
    split(input)
};
```

With one declared output, the complete body value is bound to that wire. The
bare `output` and singleton tuple `(output,)` declarations are equivalent. With
two or more outputs, the body's outer tuple is destructured positionally. Rust
checks that the body value has the required shape.

An action never terminates a flow implicitly. Its non-ignored outputs must be
captured by later blocks or by End on every path where they are produced.

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

Question outputs are distinct unit-valued control wires. Downstream blocks list
the selected control wire and every data wire they need as separate inputs.

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

## 8. End

End is the unique structural block at which every path finishes:

```rust
#[end]
|out1, out2, out3| {};
```

End is the final authored statement. It has no description, outputs, or body
expressions. Its inputs are bare, consuming identifiers. Unlike computational
blocks, End may have zero inputs.

End captures logical wires by name. When a captured wire has path-exclusive
producers, each path supplies the producer that ran; the alternatives do not
appear in End syntax. Every path reaching End must provide exactly one producer
for every captured name.

With zero inputs, End produces no result wires. With one input, the complete
wire value is the function result. With two or more inputs, their values form an
ordered Rust tuple. Rust checks this value against the function return type.
Capturing an explicitly produced wire whose Rust type is `()` remains distinct
from a zero-input End: the former is a real named wire, while the latter creates
none.

## 9. Paths and implicit convergence

A valid flow has exactly one next block at each execution step on a path. A
computational block is ready when all its inputs are available. End is ready
when all its captured result wires are available.

Validation follows each question and choice branch independently. Alternative
producers with the same output name converge at their first shared consumer.
Only one such producer may be available on any path. The shared consumer and
its continuation are represented once in the semantic plan and execute once.

Read in authored choice-case order, branches that enter one shared continuation
are adjacent. A case that reaches End cannot separate two continuing cases,
because preserving their skewer order would force one connection to cross
another. A question cannot express this arrangement because it has only two
branches.

Wires available on every continuing branch remain available after convergence.
An alternative logical wire is added to that shared set when every continuing
path supplies exactly one of its producers. Rust checks that the branch values
have one type.

Every authored block is reachable from the source wires, and every path reaches
the same End block. Branches may reach End after different numbers of
computational blocks, but no action or other unconsumed output terminates a path
implicitly.

## 10. Grammar

The outer syntax is ordinary Rust:

```text
flow := "#[kaalang]" rust_function

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

end_statement :=
    "#[end]"
    "|" end_input_list? "|" "{" "}" ";"

choice_body := "{" rust_match_expression "}" | "{" "todo!()" "}"
choice_match_arm := rust_pattern rust_guard? "=>" rust_expression

case_attribute := "#[case(" block_description ")]"

block_description := nonempty_rust_string_literal
single_output_declaration := identifier | "(" identifier "," ")"
output_declaration := single_output_declaration | rust_tuple_of_two_or_more_identifiers
input_list := input ("," input)* ","?
input := identifier | "&" identifier
end_input_list := identifier ("," identifier)* ","?
```

Every flow has exactly one End statement, and it is the final authored
statement. The final statement may omit its semicolon. Computational
descriptions and input lists are nonempty; only End accepts an empty input list.
Choice cases, outputs, and match arms have equal counts. Outputs within one
declaration are distinct. Repeated output names across blocks are valid only
when path analysis proves their producers mutually exclusive.

There is no merge statement.

## 11. Validation and execution

`#[kaalang]` parses block syntax, groups producer occurrences by logical wire
name, validates block-local and path-dependent invariants, and lowers the flow
to nested Rust `let`, `if`, and `match` expressions.

An action binds its body value to its outputs. A question evaluates its body
once and executes the selected branch. A choice preserves the authored match
and passes the selected arm value to the corresponding branch. When sibling
paths produce the same logical wire, the enclosing Rust branch expression
yields their alternative values and the shared continuation binds the logical
wire once.

End lowers to an empty Rust body, its one captured wire, or the ordered tuple of
its captured wires. Rust checks body types, match exhaustiveness, ownership,
borrows, output patterns, alternative producer types, and the End result.
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
