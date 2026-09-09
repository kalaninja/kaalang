# RFC 0004: kaalang Rust Lowering

- Status: accepted design draft
- Language: [RFC 0001: kaalang Language](0001-language.md)

## 1. Overview

The `#[kaalang]` attribute translates a validated kaalang flow into ordinary
Rust bindings and control-flow expressions. [RFC 0001](0001-language.md) defines
which programs are valid and what they mean. This RFC describes how lowering
implements that contract; it does not add language rules.

Before lowering, the source is parsed, producer occurrences are grouped by
logical wire name, and block-local and branch-dependent invariants are
validated. Lowering consumes the resulting validated model and its verified
execution plan. The [visual language](0002-visual-language.md) draws the same
source order.

The examples below pair complete kaalang functions with illustrative Rust. They
show the relevant bindings, scopes, and control flow, not a promised
token-for-token expansion. Names beginning with `wire_` stand for hygienic
generated bindings; their plain Rust spellings do not demonstrate macro hygiene.
The examples omit the signature-preserving outer function that forwards its
named parameters to the generated implementation. Unit control-wire bindings and
generated lint attributes are omitted where the branch structure already shows
their role. Tail result expressions in simplified examples stand for generated
Rust `return` statements. Internal names and the generator's own data structures
are implementation details.

## 2. Bindings, scopes, and actions

The outer function preserves every authored parameter. It forwards named
parameters to hygienic internal bindings in a nested implementation; wildcard
parameters remain only in the outer signature because they provide no wire. The
internal binding is mutable only when the authored parameter permits and a block
requests a mutable capture, so a mutable capture of an immutable parameter is
rejected before lowering. Each authored block body receives local aliases only
for its listed inputs:

| Capture     | Rust alias                   |
| ----------- | ---------------------------- |
| `name`      | `let name = wire_name;`      |
| `mut name`  | `let mut name = wire_name;`  |
| `&name`     | `let name = &wire_name;`     |
| `&mut name` | `let name = &mut wire_name;` |

Hygienic internal names keep omitted wires unavailable under their authored
names. Block-local Rust names cannot change another block's wire resolution. The
model validates each `&mut` capture of a block output against its authored `mut`
declaration, and validates matching mutability across alternative producers.
Permitted mutable captures get mutable internal output bindings and bindings
after merges. These internal modifiers have expansion spans so unused internal
mutability does not produce author-facing warnings. An output's authored `mut`
is a permission and need not be exercised. Authored parameter bindings, mutable
input aliases, and body locals retain Rust's normal lint behavior.

Input aliases belong to their block's scope. Either kind of borrowed capture can
produce a reference that outlives the alias when the underlying owner remains
valid. A value capture moves a non-`Copy` value into its alias; a reference to
that alias-owned value cannot escape the block, including when the alias is
mutable. Rust checks these lifetimes and conflicts between live shared and
mutable borrows. Mutation does not add a new producer or change execution order.

An action becomes a `let` initializer containing its input aliases and authored
body. An expression body is first wrapped in a plain block, so braces do not
change its scope or lowering. Its declared pattern is preserved with hygienic
wire names: a single identifier binds the whole body value, while a tuple
pattern destructures it, including a singleton tuple. Only internal storage
mutability is inferred from the already permitted captures. An action declaring
none binds the unit pattern, as in `let () = { body };`, so Rust rejects a body
of any other type. These Rust bindings implement
[RFC 0001 §4.1](0001-language.md#41-action) and
[§6](0001-language.md#6-wires-producers-and-captures).

For example, the first action borrows a string and produces two outputs. The
second action consumes the original string together with those outputs:

```rust
use kaalang::kaalang;

#[kaalang]
fn measure(text: String) -> (String, usize, bool) {
    #[action("Measure the text.")]
    let (length, empty) = |&text| { (text.len(), text.is_empty()) };

    #[action("Return the text and its measurements.")]
    let result = |text, length, empty| { (text, length, empty) };
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
inferred wire type when no common Rust binding would otherwise unify their
types.

## 3. Questions, merges, and execution order

A question becomes an `if` whose condition evaluates the input aliases and
authored body once. Its true arm binds the output position carrying `#[yes]` and
executes that continuation; its false arm does the same for `#[no]`. With no
answer attributes, these are the first and second positions respectively.

A merge uses a labeled block as an ordinary `let` initializer. Each incoming
branch transfers its selected producer values with `break 'join values`; several
merged wires travel together as a tuple. The shared continuation is emitted once
after this binding. This implements the merge and ordering rules of
[RFC 0001 §7](0001-language.md#7-execution-and-implicit-convergence).

Mutability belongs to binding patterns only. A wire captured through `&mut`
after the merge is bound as `let mut wire_name = ...`; tuple bindings mark each
mutable wire separately. Branches still transfer ordinary value expressions with
`break 'join wire_name`, without capture modifiers.

The initializer is wrapped in `match () { () => 'join: { ... } }` to give Rust a
coercion context. Without it, the first `break` can fix the inferred type too
early, for example to an array reference before a later branch provides a slice.
This one-arm unit match performs no runtime selection and is omitted from the
illustrative Rust below.

```rust
use kaalang::kaalang;

#[kaalang]
fn choose(condition: bool) -> u32 {
    #[question("Use the first value?")]
    let (yes, no) = |condition| { condition };

    #[action("Build the first value.")]
    let selected = |yes| { 1 };

    #[action("Build the second value.")]
    let selected = |no| { 2 };

    #[action("Add ten to the selected value.")]
    let result = |selected| { selected + 10 };
}
```

Illustrative Rust:

```rust
fn choose(wire_condition: bool) -> u32 {
    let wire_selected = 'join: {
        if {
            let condition = wire_condition;
            condition
        } {
            break 'join 1;
        } else {
            break 'join 2;
        }
    };
    let wire_result = {
        let selected = wire_selected;
        selected + 10
    };
    wire_result
}
```

Lowering follows the source order the plan verified, including all branch work a
merge waits for. It does not reorder blocks or introduce concurrent execution.

A partial merge's labeled block nests inside the labeled block of its wider
merge. A branch can break to the inner label to run the partial merge's
continuation, or directly to an enclosing label to skip continuations it does
not enter. The partial continuation can then pass its outputs to the wider join.
Each label identifies a join destination statically; no generated `Option` or
`Result` carries a routing tag or stores a wire. Authored `Option` and `Result`
values remain ordinary data.

Joins transfer the values of same-named alternative outputs into their shared
lexical scope, including names with no later captures. A leading underscore
waives only the capture requirement. Values already bound in the enclosing scope
are not transferred again. Producer records remain available for structural
checks after a Rust move; they do not require another transfer of the value.

Each branch has an ordinary Rust lexical scope. A value bound there remains
local unless the current join transfers it. Remaining owned values drop when
that scope ends. Lowering does not move an owner's binding outside its branch to
extend the lifetime of a derived reference. Rust rejects a reference that
escapes the scope of its owner. Action outputs keep ordinary `let` initializers,
including Rust's temporary lifetime extension in their original scope.

Owners bound before the branching expression remain in their enclosing scope. A
merged owner is bound in the join's shared continuation and may be borrowed
there. Values from a completed partial merge stay in that continuation's scope;
only same-named alternatives reaching a wider join are carried into its shared
scope. Other partial values drop when their scope ends, so a reference to such
an owner cannot cross the wider join.

Ending a borrowed input alias does not itself drop the owner. Branch scope exit
releases a local guard without an explicit action. For earlier release, an
authored value capture and `drop` call remain available under
[RFC 0001 §6](0001-language.md#6-wires-producers-and-captures); Rust rejects the
move if a live output still borrows the owner.

Every validated flow lowers to nested expressions like these. The branch rule of
[RFC 0001 §7](0001-language.md#7-execution-and-implicit-convergence) keeps each
block inside one branch until a merge joins it, so ordinary Rust bindings
express every accepted flow and each authored block body is emitted once.
Lowering never gives a wire an optional slot or decides at run time whether a
block executes.

## 4. Choices and case values

Lowering preserves the authored choice `match`, including its scrutinee,
patterns, guards, arm order, and arm expressions. It evaluates the scrutinee
once and carries the selected arm's value out of that arm before executing the
case continuation. This scope boundary implements
[RFC 0001 §4.3](0001-language.md#43-choice).

Nested labeled blocks let each case pass its value directly into an ordinary
`let` binding. Each arm breaks out of its input and match-arm scopes before its
continuation runs. The continuation then breaks to its join label or returns the
flow's `result`. The example evaluates one authored `match`.

```rust
use kaalang::kaalang;

#[kaalang]
fn length_or_zero(input: Option<String>) -> usize {
    #[choice("Was text supplied?")]
    #[case("Text is available.")]
    #[case("No text is available.")]
    let (text, absent) = |input| {
        match input {
            Some(value) => value,
            None => (),
        }
    };

    #[action("Measure the supplied text.")]
    let result = |text| { text.len() };

    #[action("Return zero for absent text.")]
    let result = |absent| { 0 };
}
```

Illustrative Rust:

```rust
fn length_or_zero(wire_input: Option<String>) -> usize {
    let () = 'case_absent: {
        let wire_text = 'case_text: {
            let input = wire_input;
            match input {
                Some(value) => break 'case_text value,
                None => break 'case_absent (),
            }
        };
        let wire_result = {
            let text = wire_text;
            text.len()
        };
        return wire_result;
    };
    return 0;
}
```

The authored binding `value` ends with its arm. Returning `&value` instead would
attempt to carry a reference to an arm-owned local across this boundary and Rust
would reject it. A value borrowing a borrowed input can cross the boundary when
that input outlives the choice. Placing the continuation inside the authored arm
would incorrectly extend the local's availability.

Case continuations that reach partial or wider merges break directly to the
corresponding join labels from §3. A continuation returns from the function only
when it reaches end.

## 5. End and terminal branches

Every arrival at the implicit end block becomes a direct Rust `return` of the
selected `result` value. A merge of alternative `result` producers happens
before that return, under [RFC 0001 §4.4](0001-language.md#44-end); end does not
become a separate computational continuation or merge operation.

When one branch finishes the flow while its siblings yield a value to an
enclosing merge, its `return` avoids the continuation it does not enter. This
generated control transfer implements the end block and does not permit an
authored `return` in a computational body.

```rust
use kaalang::kaalang;

#[kaalang]
fn finish_or_join(refine: bool, finish_early: bool) -> u32 {
    #[question("Refine the value?")]
    let (nested, direct) = |refine| { refine };

    #[question("Finish early?")]
    let (skip, join) = |nested, finish_early| { finish_early };

    #[action("Finish before the shared step.")]
    let result = |skip| { 100 };

    #[action("Build the refined value.")]
    let shared = |join| { 1 };

    #[action("Build the direct value.")]
    let shared = |direct| { 2 };

    #[action("Add ten to the shared value.")]
    let result = |shared| { shared + 10 };
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

A zero-computation flow from
[RFC 0001 §5.1](0001-language.md#51-zero-computation-flow) needs only its input
binding and a return:

```rust
use kaalang::kaalang;

#[kaalang]
fn identity<T>(result: T) -> T {}
```

Illustrative Rust:

```rust
fn identity<T>(wire_result: T) -> T {
    return wire_result;
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
other attributes, implementing the opt-out described by
[RFC 0001 §5](0001-language.md#5-flow-inputs-and-outputs).

Expansion replaces the kaalang body but preserves the authored function
signature, including parameter names and patterns. Hygienic wire names remain
confined to a nested implementation. It also preserves the function's name,
visibility, generics, parameter and return types, supported qualifiers, where
clause, and other function attributes. kaalang v0.1 supports synchronous flows,
including `const fn`; an `async fn` is rejected before lowering under
[RFC 0001 §5](0001-language.md#5-flow-inputs-and-outputs). The `#[kaalang]`
attribute and block and case descriptions are consumed during translation; the
closure-shaped declarations do not become callable Rust closures.
