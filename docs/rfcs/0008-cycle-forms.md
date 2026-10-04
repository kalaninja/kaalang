# RFC 0008: Cycle forms

- Status: proposed
- Language: [RFC 0001: kaalang Language](0001-language.md)
- Visual language: [RFC 0002: kaalang Visual Language](0002-visual-language.md)
- Renderer: [RFC 0003: kaalang SVG Renderer](0003-svg-renderer.md)
- Lowering: [RFC 0004: kaalang Rust Lowering](0004-rust-lowering.md)
- Stages: [RFC 0006: Stages](0006-stages.md)
- Refinements: [RFC 0007: Language refinements](0007-language-refinements.md)

## 1. Motivation and scope

Visiting every item of a collection is already expressible: an action builds a
cursor, a choice asks it for the next item, and `continue` repeats. The idiom
draws two nodes that only manage the iteration, and its reader has to recognize
the pattern before seeing the work done for each item.

This RFC names the two forms a cycle can take. A **loop cycle** is the cycle of
RFC 0007, now spelled with Rust's `loop` keyword. A **for cycle** runs its body
once per item of a Rust iterator, with no structural transfers inside it. It is
drawn between two caps instead of inside a boundary.

The for cycle is deliberately compact. It cannot stop early, export a value, or
repeat through `continue`; a search or an early exit stays a loop cycle. In
return its body is a plain sequence that every item passes through, and its
diagram needs no back edge.

This RFC defines syntax, execution, validation, Rust lowering, and the visual
representation of both forms. Section 8 identifies the earlier provisions it
supersedes. Earlier accepted RFC texts remain unchanged.

### 1.1 Terms

- A **loop cycle** is a cycle whose body is a Rust `loop` block. RFC 0007 §2
  defines its gate, scope, outputs, and `continue`.
- A **for cycle** is a cycle whose body is a Rust `for` loop.
- The **iteration header** of a for cycle is its capture list together with the
  item pattern and the expression after `in`.
- **Item wires** are the wires the item pattern binds for one iteration.
- The **for-entry** and the **for-end** are the two caps that enclose an
  expanded for cycle's body in the diagram.

## 2. Syntax

The `kaalang` attribute is assumed to be in scope in all examples.

### 2.1 Loop cycles

A loop cycle writes `loop` before its body. Every other rule of RFC 0007 is
unchanged:

```rust
#[cycle("Count to the limit.")]
let finish = |start| loop {
    #[question("Has the limit been reached?")]
    let (finish, again) = |counter, limit| counter >= limit;

    #[action("Increment the counter.")]
    |again, &mut counter| *counter += 1;

    |again| continue;
};
```

The shorthands of RFC 0007 §3 carry over. `let finish = loop { ... };` is
`let finish = || loop { ... };`, and a cycle with neither gate nor outputs may
write `#[cycle("...")] loop { ... };`. Braces may enclose the `loop`, as they
may enclose a `match`, `return`, or `continue`: `|gate| { loop { ... } }` is
`|gate| loop { ... }`. A brace block that holds the body's statements directly,
the form before this RFC, is invalid.

### 2.2 For cycles

A for cycle writes a Rust `for` loop as its body:

```rust
#[cycle("Add every value to the total.")]
|values| for value in values {
    #[action("Add the value.")]
    |value, &mut total| *total += *value;
};
```

The capture list uses the four ordinary capture forms of RFC 0001 §6. The
expression after `in` is any Rust expression that reads only those aliases, the
way an action body reads its captures. Ranges and iterator adapters are written
in Rust: `0..n`, `(1..=n).rev()`, `(0..n).step_by(2)`, or
`values.iter().enumerate()`. An iterator that needs more work is built by an
earlier action and captured as a wire.

The item pattern is one binding with optional `mut`, which binds one item wire.
A for cycle that does not use its items writes `_`, which binds none. An item
that needs destructuring, such as a pair from `enumerate()`, is captured whole
and taken apart by an action.

A for cycle may declare one output, written `let done = ...` or
`let (done,) = ...`. Without `let`, or with `let ()`, it declares none. An empty
capture list may be omitted: `for index in 0..3 { ... }` is
`|| for index in 0..3 { ... }`. Braces may enclose the `for` as they enclose a
loop cycle's `loop`; rustfmt writes a `for` closure body that way, as in
`|values| { for value in values { ... } }`.

### 2.3 Grammar

Using RFC 0001's block notation, replace RFC 0007 §2.2's cycle alternative with:

```text
cycle_statement :=
    "#[cycle(" block_description ")]"
    ( ("let" output_pattern "=")? ("|" (identifier ","?)? "|")? loop_body
    | ("let" for_output "=")? ("|" input_list? "|")? for_body
    ) ";"
loop_body := "loop" cycle_body | "{" loop_body ";"? "}"
for_body :=
    "for" item_pattern "in" rust_expression cycle_body | "{" for_body ";"? "}"
cycle_body := "{" block_statement* "}"
for_output := output_binding | "(" ")" | "(" output_binding "," ")"
item_pattern := output_binding | "_"
```

`block_statement` keeps RFC 0007's alternatives; section 4 states which of them
a for cycle's body excludes. Neither form takes a label or body-level
attributes. A `loop` or `for` statement without `#[cycle]`, and a `while` in any
position outside a computational body, remain invalid.

## 3. Execution of a for cycle

### 3.1 Header and items

The header captures participate like an action's: they gate the cycle's entry
under the ordinary branch rules, and they move, copy, or borrow their wires
once, before the first iteration. Unlike a loop cycle's gate, they are real
captures with Rust value operations. The `in` expression is evaluated once over
those aliases and iterated through `IntoIterator`. The aliases end once the
iterator is built. A value capture moves or copies its wire into the `in`
expression, so iterating a collection by reference needs a borrowing capture, as
in `|&values| for value in values.iter()`. A borrowing capture keeps its wire
borrowed for as long as the iterator holds the borrow.

Each iteration binds the item pattern to the next item. Item wires are
iteration-local: they are fresh on every iteration, inner blocks capture them
like any body-local wire, and they cannot be captured outside the body. An item
wire, like any producer, needs a consumer unless its name begins with `_`.

### 3.2 Body

The body is a kaalang sequence executed once per item, in iteration order. It
inherits outer data under RFC 0007 §2.1, so inner blocks capture outer wires
directly and their dependencies are recorded against the original producers.
Every outer wire used inside must be available whenever the cycle is entered.
The header aliases are not visible to inner blocks; an inner capture of the same
outer wire reaches its original storage, and Rust checks it against the borrow
or move the header made.

Every route through the body ends at the end of the body, where the iteration
ends and the next item begins. A route may instead diverge in a nested loop
cycle. Branches inside the body converge before its end, as the repeating routes
of a loop cycle converge before its `continue`: a branch with no work of its own
merges with the others through a same-named wire. Branch-local and body-local
owners drop when the iteration ends.

Questions, choices, actions, calls, and nested cycles of either form follow
their ordinary rules inside the body. A nested loop cycle's `continue` and
outputs belong to that loop; a nested for cycle completes before the enclosing
body continues.

### 3.3 Completion

A for cycle completes when its iterator yields no further item, including before
the first iteration. Completion is therefore always reachable: a for cycle has a
normal continuation even when every body route diverges, because its iterator
may be empty.

When the cycle declares an output, that output is a unit wire produced by the
cycle at completion. It behaves like a completed loop cycle's single output: it
becomes available to the containing sequence and may gate later blocks, merge
with same-named wires on other branches, or become a stage transition when the
enclosing stage declares that output. A for cycle with no declared output is
placed by source order, like an action without outputs, and later blocks that
capture the outer state its body changed run after it.

A value computed inside the body reaches the containing sequence only through
outer state: create an owner before the cycle, mutate it through an inner `&mut`
capture, and capture it after the cycle.

## 4. Validation

A for cycle is rejected when:

- its body contains a structural `continue` or `return` of its own;
- a block in its body produces a wire named after the cycle's declared output;
- it declares more than one output;
- its `in` expression reads a wire it does not capture, under the ordinary
  computational-body rules of RFC 0001 §3;
- its item pattern is anything other than one binding or `_`;
- its body ends an iteration while the branches of a selection inside it are
  still separate;
- it carries a label;
- its body moves an outer non-`Copy` value on a route that reaches the next
  iteration, which Rust rejects in the generated loop as it does for a loop
  cycle.

A structural `continue` in a for body is rejected even when an enclosing loop
cycle exists: continue targets only its directly containing cycle. A loop
cycle's body is rejected when its braces hold anything other than its `loop`.

## 5. Rust lowering

A loop cycle lowers as RFC 0007 §5.2 describes; the keyword changes nothing.

A for cycle lowers to a native Rust loop over an iterator, the way Rust lowers
its own `for`. Its header captures bind as an action's capture aliases do, in
authored order, inside a scope that evaluates the `in` expression and ends
before the loop, so the body never sees the aliases. `IntoIterator` turns the
value of that scope into the iterator. Each iteration first takes the next item.
With one, it binds the item wire and runs the lowered body, which ends by
repeating the loop. Without one, it leaves the loop with the cycle's `()`
output.

For example:

```rust
#[kaalang]
fn sum(values: &[i64]) -> i64 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add every value to the total.")]
    let added = |values| for value in values {
        #[action("Add the value.")]
        |value, &mut total| *total += *value;
    };

    |added, total| return total;
}
```

Illustrative Rust, omitting unit control-wire bindings and lint allowances:

```rust
fn sum(wire_values: &[i64]) -> i64 {
    let mut wire_total = 0;
    let wire_added = {
        let mut items = IntoIterator::into_iter({
            let values = wire_values;
            values
        });
        'cycle: loop {
            match items.next() {
                Some(wire_value) => {
                    let value = wire_value;
                    let total = &mut wire_total;
                    *total += *value;
                    continue 'cycle;
                }
                None => break 'cycle,
            }
        }
    };
    {
        let total = wire_total;
        return total;
    }
}
```

Analysis records one representative iteration, as for a loop cycle. An iteration
either takes an item, runs the body and repeats, or finds none and completes.
Every block of the body runs only on the first of those routes, so the cycle
completes whenever it is entered. Rust checks the iterator type, moves, borrows,
and drop order. Whether a `for` loop is allowed in a `const fn` is decided by
Rust.

## 6. Visual representation

### 6.1 Expanded for cycle

An expanded for cycle has no boundary, caption, loop marker, or back edge. Two
caps enclose its body:

- The **for-entry** is a rectangle whose two upper corners are cut at 45°. It
  holds the cycle's description, rendered and wrapped like an action's. Its
  receiving label lists the header captures, as an action's does, and its
  hand-over lists the item wires. An `_` item pattern has no hand-over label.
- The **for-end** is a rectangle whose two lower corners are cut at 45°. It
  repeats the cycle's description, rendered like the for-entry's, so nested
  bottom caps each name the for-entry they close. Every body route that ends the
  iteration arrives at it. Several such routes first meet on one unmarked rail,
  as repeating routes meet at a loop cycle's iteration tail, and enter the cap
  together.

Both caps attach their connections like an action: arrivals at the upper edge,
the continuation at the lower edge. The cycle's continuation leaves the bottom
cap and carries the declared output's hand-over label when there is one. An
empty body connects the for-entry directly to the for-end.

The for-end sits in the for-entry's column, so the cycle opens and closes on one
vertical line; a route that ends the iteration in another column turns into it.
The body is laid out as an ordinary sequence between the caps. No outer node or
route passes through the area the body occupies between them, and no body route
leaves it except through the for-end. A body route that never ends its
iteration, such as one inside an endless loop cycle, still stays above the
for-end. Outer data captured inside the body follows the existing control routes
through the for-entry, as it follows a loop cycle's entry. The repetition is
carried by the two caps; no line returns from the for-end to the for-entry.

A nested loop cycle inside a for body keeps its boundary within the body. A for
cycle nested in a loop body keeps its caps within that loop's boundary.

### 6.2 Collapsed for cycle

A collapsed for cycle becomes the same cycle node as a collapsed loop cycle,
with the loop marker and the authored description. Its derived input list
follows RFC 0007 §6.1, with the header captures in place of the gate: the header
captures in authored order, then each outer wire captured in the body on its
first occurrence. Item wires and body locals are excluded. The node has one
normal exit, labeled with the declared output when there is one and unlabeled
otherwise.

## 7. Examples

### 7.1 A signal on each branch

```rust
#[kaalang]
fn total_of(values: &[i64], twice: bool) -> i64 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[question("Should every value count twice?")]
    let (double, single) = |twice| twice;

    #[cycle("Add every value twice.")]
    let counted = |double, values| for value in values {
        #[action("Add the value twice.")]
        |value, &mut total| *total += 2 * *value;
    };

    #[cycle("Add every value once.")]
    let counted = |single, values| for value in values {
        #[action("Add the value.")]
        |value, &mut total| *total += *value;
    };

    |counted, total| return total;
}
```

Each branch runs its own for cycle. Their `counted` outputs merge by the
ordinary rules, so the return follows whichever cycle ran.

### 7.2 Ranges and destructured items

```rust
#[kaalang]
fn countdown(from: u32) -> Vec<(usize, u32)> {
    #[action("Start with an empty log.")]
    let mut log = || Vec::new();

    #[cycle("Count down to one, numbering each step.")]
    |from| for numbered in (1..=from).rev().enumerate() {
        #[action("Take the step number and the value apart.")]
        let (step, value) = |numbered| numbered;

        #[action("Record the step.")]
        |step, value, &mut log| log.push((step, value));
    };

    |log| return log;
}
```

### 7.3 A question in the body

```rust
#[kaalang]
fn sum_of_even(values: &[u32]) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add the even values.")]
    |values| for value in values {
        #[question("Is the value even?")]
        let (even, _counted) = |value| *value % 2 == 0;

        #[action("Add the value.")]
        let _counted = |even, value, &mut total| *total += *value;
    };

    |total| return total;
}
```

The odd route does no work of its own; its `_counted` output merges with the
action's, so both routes end the iteration together. `_counted` needs no
consumer.

## 8. Changes to earlier RFCs

This RFC supersedes the provisions below. Earlier accepted RFC texts remain
unchanged.

- **RFC 0001 §§3, 4.5 and 8; RFC 0007 §§2.2 and 3:** a cycle body is a Rust
  `loop` or `for` loop rather than a bare brace block, with the grammar in §2.3
  here. The bare-body and `let`-without-captures shorthands apply to both forms.
- **RFC 0001 §3:** output patterns still exclude wildcards; only a for cycle's
  item pattern admits `_`.
- **RFC 0001 §4:** a cycle repeats a nested sequence either until it exports an
  output (loop cycle) or once per item of an iterator (for cycle).
- **RFC 0007 §2.1:** the single gate is a loop cycle's header. A for cycle's
  header is an ordinary capture list, as in §3.1 here.
- **RFC 0007 §§2.2–2.3:** declared alternative outputs, the rule that a cycle
  without outputs can only repeat or diverge, the structural `continue`, and the
  rejection of a body ending without an output or transfer apply to loop cycles.
  A for cycle declares at most one unit output, completes when its iterator is
  exhausted, and ends every iteration at the end of its body.
- **RFC 0007 §5.1:** a for cycle's frame has only repeating and diverging
  routes, and the for cycle completes unconditionally in its containing frame.
- **RFC 0002 §4:** add the for-entry and for-end of §6.1 here to the node kinds.
  They appear only in an expanded for cycle.
- **RFC 0002 §4.8, RFC 0003 §§2–3, and RFC 0007 §6.1:** the boundary, caption,
  entry junction, iteration tail, and back edge describe expanded loop cycles.
  An expanded for cycle uses §6.1 here instead; its collapsed node uses §6.2.
- **RFC 0004 §7 and RFC 0007 §5.2:** a for cycle lowers as §5 here.
- **RFC 0006:** for cycles may appear in preparation and stage bodies under the
  same rules; a for cycle's declared output can be a stage transition like any
  completed cycle's output.

Concrete compiler and renderer types remain implementation choices.

## 9. Acceptance scenarios

| Scenario                                                              | Required result                                                                                          |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Loop cycle written with `loop`, with or without gate or output        | Accept with RFC 0007's semantics and unchanged diagrams.                                                 |
| Cycle body written as a brace block of statements                     | Reject and name the `loop` and `for` forms.                                                              |
| Braces around a cycle's `loop` or `for`                               | Accept as the same cycle.                                                                                |
| For cycle over a borrowed collection with no declared output          | Run the body once per item; later blocks capturing the changed state run after it.                       |
| For cycle with one declared output                                    | Provide the unit output after the iterator is exhausted.                                                 |
| For cycle over an empty iterator                                      | Run no iteration and complete.                                                                           |
| `in` expression with ranges and adapters over header captures         | Accept any Rust expression reading only those aliases.                                                   |
| `in` expression reading an uncaptured wire                            | Reject under the ordinary capture rules.                                                                 |
| Tuple item pattern                                                    | Reject; an action takes the captured item apart.                                                         |
| `_` item pattern                                                      | Bind no item wire.                                                                                       |
| Nested or reference item pattern                                      | Reject.                                                                                                  |
| Item wire without a consumer                                          | Reject unless its name begins with `_`.                                                                  |
| Question in the body whose branches merge before its end              | Accept; both routes end the iteration together.                                                          |
| Branch of a question in the body that diverges in a nested loop cycle | Accept; only the routes that end the iteration need to merge.                                            |
| Branches still separate at the end of the body                        | Reject.                                                                                                  |
| Structural `continue` or `return` in a for body                       | Reject.                                                                                                  |
| Body produces the for cycle's declared output                         | Reject.                                                                                                  |
| Two outputs on a for cycle                                            | Reject.                                                                                                  |
| Labeled `for` cycle                                                   | Reject.                                                                                                  |
| Outer non-`Copy` value moved in the body                              | Let Rust reject the repeated move.                                                                       |
| Owned items moved into an inner block                                 | Accept; each item is a fresh iteration-local wire.                                                       |
| Two branches each with a for cycle and a same-named output            | Merge the outputs by the ordinary rules.                                                                 |
| For cycle inside a loop cycle, and loop cycle inside a for            | Keep each cycle's own rules; inner transfers target only their own loop.                                 |
| Expanded for cycle                                                    | Draw the for-entry with the description and labels, the body, and the for-end; no boundary or back edge. |
| Collapsed for cycle                                                   | Draw the loop-marked cycle node with the derived inputs and one normal exit.                             |
