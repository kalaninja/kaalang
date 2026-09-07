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
alternative branches. Every completed execution produces the flow's `result`
wire, which the implicit end block captures.

In the Rust representation, a flow is written as an ordinary function marked
with `#[kaalang]`. The attribute validates this representation and lowers it to
ordinary Rust control flow. Wire dependencies determine which blocks are ready;
independent ready blocks may execute in any order.

## 2. Terminology

These terms describe the kaalang language independently of its representation
or visualization:

- **flow inputs** are values supplied when a flow begins; named flow inputs
  provide wires;
- the **flow output** is the value of the `result` wire, which the implicit end
  block captures when a flow finishes;
- a **block** is one unit of a flow; its inputs name the wires it captures, and
  its outputs name the wires it produces. Every block but the end block is
  authored;
- an **input** names an available wire captured by a block; a bare input
  consumes the wire, while a `&` input borrows it;
- an **output** declares a wire produced by a block;
- a **producer** is one occurrence of a named flow input or block output that
  provides a wire;
- **alternative producers** provide the same logical wire from mutually
  exclusive branches; their repeated output name always declares one implicit
  merge before every capture of that name;
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
- a **branch output** is a question or choice output; it selects one branch
  and is consumed by its implicit merge, or, for a name with no alternative
  producers, by at most one block whenever that output is selected;
- a **case** describes one branch of a choice and is not itself a block;
- a **path** is one dependency-ordered chain of blocks through selected
  branches; one execution may contain several paths;
- **independent blocks** can execute in either order without changing the
  availability of each other's inputs;
- a question or choice **decides** a block when two executions that select
  different branches of it, and the same branch of every other question or
  choice they both run, differ in whether the block executes;
- the **continuation** of a branch is the set of computational blocks whose
  readiness can transitively depend on that branch's selected output through a
  chain of capture dependencies and implicit wire order participating in one
  possible execution that selects the branch; the end block is not part of a
  continuation;
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
- a **merge**, or **convergence point**, is the implicit junction of all
  alternative producers of one logical wire, before any consumer captures it.
  It is not an authored block or a new producer occurrence;
- an **entry block** of a shared continuation has no predecessor in that
  continuation. Entry blocks consume values after their merges; they are not
  the convergence points themselves.

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
ordered `#[case("description")]` attributes. The end block is implicit and
therefore carries no attribute at all.

Source comments remain ordinary Rust comments. Block and case descriptions come
from their attributes.

Every authored block declares at least one output. A block that produced none
could never be ordered before the flow finishes, so section 7 would reject it;
the grammar rejects it first.

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
| **action** | performs a computation or effect | zero or more | one or more |
| **question** | evaluates a logical expression and selects one of two branches | one or more | exactly two unit-valued control wires, one per branch |
| **choice** | selects one of two or more cases and provides a value to the corresponding branch | one or more | one per case, at least two |
| **end** | the implicit block that finishes a flow | the `result` wire | none |

### 4.1 action

An action evaluates its body and declares one or more outputs:

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

An action that consumes no wires and performs only an effect declares a named
unit-valued wire, which is also how an action establishes an order explicitly:

```rust
#[action("Log flow entry.")]
|| -> entered {
    println!("start")
};

#[action("Continue.")]
|entered| -> result {
    continue_work()
};
```

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

Question outputs are distinct unit-valued control wires. The downstream block
lists the selected control wire and every data wire it needs as separate inputs.
A control wire is a branch output: its continuation consumes it whenever the
output is selected. The continuation is its implicit merge when the name has
alternative producers, or otherwise at most one capturing block.

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
Rust types. Like question outputs, choice outputs are branch outputs: the
selected case hands its value to its implicit merge, or otherwise at most one
capturing block. That continuation must execute whenever that case is selected.

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

Every flow has one implicit end block. It is not authored, and there is no end
statement. It consumes exactly one wire, `result`, whose value is the flow
output. Rust checks that value against the function return type, which is `()`
when the function declares none.

`result` completes the flow: no computational block captures it. When `result`
has alternative producers they merge before the end block exactly as any
repeated output name does (section 6), so a question or choice output may itself
be named `result`. Whenever execution reaches the end block, exactly one
producer of `result` must be available.

A flow returning unit produces a unit-valued `result` like any other wire;
kaalang has no zero-input end block and no implicit unit wire.

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
for the `result` wire the end block captures.

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
}
```

The two actions are alternative producers of the logical `result` wire. The end
block captures whichever producer ran. The example is a complete flow whose
computational bodies are placeholders. `todo!()` retains its Rust behavior and
panics if execution reaches it.

The function body contains closure-shaped Rust expression statements. Each
statement declares one computational block. Like any Rust tail expression, the
final statement may omit its semicolon.

### 5.1 Zero-computation flow

A flow contains no computational blocks only when a flow input already provides
`result`:

```rust
#[kaalang]
fn identity<T>(result: T) -> T {}
```

Any other empty body is invalid, because nothing produces `result`. A flow that
computes nothing but must still finish declares one action:

```rust
#[kaalang]
fn nothing() {
    #[action("Finish without doing anything.")]
    || -> result {};
}
```

An ignored parameter remains explicit:

```rust
#[kaalang]
fn discard(_value: Value) {
    #[action("Finish without the flow input.")]
    || -> result {};
}

#[kaalang]
fn discard_unnamed_input(_: Value) {
    #[action("Finish without the flow input.")]
    || -> result {};
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

Repeating an output name always declares a merge, including when different
blocks capture that name or its leading underscore permits it to remain unused.
Every capture names the merged wire. A consumer's other inputs cannot select a
particular producer occurrence before the merge. Branch-local values must use
different names; a local borrow of a repeated name does not bypass its merge.

Raw and ordinary spellings of the same Rust identifier name the same wire, so
`value` and `r#value` are interchangeable. Every producer occurrence is authored
before every consumer of its logical wire; a later producer cannot retroactively
join a wire that has already appeared as an input.

No computational block captures `result`; the end block consumes it and the
flow finishes.

Every block-output producer occurrence must have at least one capture dependency
to a later block or the end block unless its name begins with `_`;
that prefix permits the occurrence to have no consumer. A block all of whose
outputs may remain uncaptured is still invalid, because nothing would order it
before the flow finishes (section 7). For action outputs and
merged wires, this requirement is existential rather than per-execution. A
question or choice output without alternative producers must be captured by its
consumer in every execution selecting the output. For such an output, the `_`
prefix permits no consumer, but does not make an existing consumer optional.

```rust
#[question("Which value should be used?")]
|condition| -> (yes, no) { condition };

#[action("Build the yes value.")]
|yes| -> selected { yes_value() };

#[action("Build the no value.")]
|no| -> selected { no_value() };

#[action("Use the selected value.")]
|selected| -> result { use_value(selected) };
```

The alternative `selected` outputs merge before the shared consumer. Under
either branch selection exactly one producer supplies the merged wire. The
consumer captures that wire by name and executes once. kaalang has no authored
merge block.

Each computational block lists every wire available to its body:

```rust
|&request, policy, yes| -> decision { /* body */ };
```

Inputs have two forms:

- `name` consumes the wire in the current execution;
- `&name` borrows the wire and leaves it available to later blocks in that
  execution.

The outputs of one action appear together: an execution that runs the action
produces all of them, so they are ordinary data wires that later blocks borrow
or consume under these rules. The outputs of a question or choice are
alternatives: an execution produces exactly one of them. A branch output without
alternative producers is captured by at most one block, which consumes it; a
borrow or a second consumer would give the selected branch a second continuation.

That consumer must execute whenever the branch output is selected. Its other
inputs must therefore be available in every such execution. For example, an
action capturing the yes outputs of two independent questions is invalid:
either question can select yes while the other selects no, leaving the selected
output without its consumer. The flow must express a nested question or converge
alternative producers before a consumer that needs both results. Silently
skipping the consumer is invalid: that branch then produces no `result`.
Forwarding a branch output through an action does not lift this: the questions
and choices that decide the action's consumers stay the same (section 7).

When a branch output shares its name with an alternative producer, its one
continuation is the implicit merge. Captures after that merge use ordinary
merged data: they may borrow it or have different consumers in different
executions. They do not capture the raw branch output.

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

A computational block is ready when all its inputs are available and the
implicit wire order described below is satisfied. An action with no inputs is
ready when the flow begins. At each execution step, any ready computational
block may execute. Ready blocks are independent when executing
one cannot change the availability of another's inputs. Independent ready
blocks may execute in any order; kaalang does not thereby guarantee concurrent
execution. If order matters, a wire from one block's output to another block's
input must establish the dependency.

Validation rejects a flow that can make non-independent blocks ready together.
In particular, two consuming blocks, or a consuming and a borrowing block,
cannot be ready for the same wire at the same time. This capture-conflict check
uses ordinary wire availability; implicit wire ordering does not replace an
explicit dependency between conflicting captures.

The questions and choices that decide a block form a chain through capture
dependencies: each lies downstream of a branch of the previous one. Implicit
wire order does not make otherwise independent deciders a valid chain. Two
independent questions or choices cannot both decide whether one block executes,
whichever way a selection withholds an input: a branch output that was not selected, a wire
that only some branches produce, or a wire that a branch consumed. Independent
selections meet in one block only through wires that every branch of the
earlier question or choice provides.

Each computational block executes at most once. The end block is ready when
`result` is available, like any other consumer. A flow in which a computational
block can be ready at the same time as the end block is invalid: nothing would
order that block's work before the flow finishes. Together with the
capture-conflict rule above, this makes every participating block either the
producer of `result` or a transitive predecessor of it, through capture
dependencies. Both rules are needed: without the conflict rule a block could
avoid being ready beside the end block only by taking a wire away from the
producer's own chain.

Validation computes continuations and convergence groups separately for each
question and choice. One question or choice may have several convergence groups.
Groups may be disjoint or one may contain the other; partially overlapping
groups are invalid. This also applies to the producer branches of implicit wire
merges, even when the merged name is unused. Nesting allows an earlier partial
merge followed by a merge with the remaining alternatives before end.

The shared continuation of each convergence group is represented once in the
semantic model, and each of its blocks executes at most once. Its entry blocks
are consumers after implicit wire merges. One merge can precede several
consumers, and a consumer can capture several merged wires.

Read in authored choice-case order, the branches of each convergence group are
adjacent: no case outside the group may separate two of its members. Several
disjoint groups are valid and may be separated by cases outside either group. A
question's own two branches satisfy this local rule automatically; nested
selections still obey the rule below.

Adjacency also applies across nested questions and choices: branches producing
a merged wire must form one uninterrupted interval in authored branch order.
A branch that does not produce the wire cannot separate two that do, even if
it finishes with `result`. This rule also applies to unused merged names.
Selections unrelated to which producer supplies the wire, or whether it is
supplied at all, do not split the interval.

The interval is defined over the selections that affect the wire's production.
For each execution, record its producer occurrence, or absence. A question or
choice affects this record when two executions differ only at that selection
under section 2's agreement rule and have different records. Retain only these
selections in each execution's branch trace, in authored block order. Order the
traces lexicographically by authored branch order, which places each deciding
ancestor before its descendants. The traces with a producer occurrence must
occupy consecutive positions in this order. Unrelated selections before or
after a completed merge therefore do not affect adjacency.

For example, nested exits ordered `skip, join`, followed by the outer `direct`
branch, allow `join` and `direct` to merge into `shared` while `skip` finishes
with `result`. Ordering them `join, skip, direct` is invalid: `skip` separates
the two producers of `shared`.

Nested convergence must not bypass an enclosing convergence and rejoin its
ordinary continuation farther downstream. More precisely, reject a flow when:

- two executions differ only at a question or choice `Q` (using the agreement
  rule of section 2), and only one produces an occurrence of a merged wire `a`;
- some producing execution of `a` does not run `Q`, so this merge also receives
  a branch outside `Q`;
- the execution bypassing `a` produces an occurrence of another merged wire
  `b`, ordered after the merge of `a` by the combined wire order below, and `b`
  is not `result`.

Thus nested branches returning to the enclosing shared continuation must
converge before it or together with it. A branch may instead finish with
`result`, after its own local work, without returning to that continuation,
provided it lies outside the earlier merge's branch interval.
Partial merges inside one question or choice and a new selection after a
completed merge remain valid. This is a language restriction, independent of
whether a renderer can route a particular diagram.

An implicit merge closes its producer branches before a consumer may capture
the merged wire. Consider all executions in which one of that wire's producers
runs, including executions with no capture. A question or choice selects the
producer when two executions in this context differ only in its selection
(using the agreement rule of section 2) and produce different occurrences.
Every computational block whose participation such a selection decides within
this context must finish before the merge whenever that block participates.
This includes work using a value provided by only one producer branch. Such a
value remains local; it cannot be carried past the merge as a hidden optional
input. The context excludes executions producing none of the wire's
occurrences, such as a terminal case outside a partial convergence.

An implicit merge also precedes a question or choice that decides whether one
of its consumers executes, when both of these conditions hold:

- every execution that runs the question or choice produces one of the merged
  wire's occurrences;
- capture dependencies and existing merge-to-consumer and branch-completion
  order do not require that question or choice to precede the merge.

The second condition keeps a selection needed to produce or complete the merge
before it. The first keeps a partial merge from suppressing a selection in an
execution where the merged wire does not exist. Derive all such orderings from
the original dependencies and merges together, before adding any of them;
authored statement order must not decide which ordering wins.

For example, if one question's branches produce `counted` and `seen`, and a
second question selects between actions that capture those merged values, both
merges finish before the second question evaluates its body. Its authored
inputs need not mention either value. This is an execution dependency, not an
implicit capture: it neither exposes another wire in the question's body nor
borrows or consumes it. The second question belongs to the first question's
shared continuation.

The same rule applies to an ordinary, unmerged action output: its producer
precedes a question or choice deciding whether one of its consumers executes,
provided the producer runs in every execution running that selection and the
original capture and merge order does not place the selection before the
producer. Derive these orderings together with the merge orderings, from the
same original dependencies. They add no capture and do not depend on authored
statement order. For example, an action preparing `setup` precedes the question
whose branch actions borrow `setup`, even when the question captures only
`condition`. Two actions merely sharing consumers remain independent.

Validation combines capture dependencies with producer-to-merge,
branch-local-work-to-merge, merge-to-consumer, and the inferred
merge-to-question-or-choice and ordinary-producer-to-question-or-choice order.
This order must be acyclic. A cycle means
that completing a producer branch requires a value from
a merge that already waits for that branch. For example, independently merging
`left_value` and `right_value` is invalid when left-only work needs the merged
`right_value` and right-only work needs the merged `left_value`. A one-way use
of a fully merged value in another question's branch remains valid.

Rust checks that all alternative producers have one type. A branch-local value
absent on another converging path cannot reappear after convergence: it must
finish its own branch before the merge, and the merged wire carries nothing of
it. Past the merge the wire is ordinary data. A later block may leave it
uncaptured, and a question or choice anywhere in the flow may decide which
block captures it, exactly as for any other action output.

Every authored computational block either has no inputs or can become ready from
flow inputs and earlier block outputs. Every branch provides `result`, directly
or through its continuation, and branches may do so after different numbers of
computational blocks. Only `result` finishes a flow.

## 8. Grammar

The outer syntax is ordinary Rust:

```text
flow := "#[kaalang]" rust_function

flow_parameter := identifier ":" rust_type | "_" ":" rust_type

action_statement :=
    "#[action(" block_description ")]"
    "|" input_list? "|" "->" output_declaration rust_block ";"

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

case_attribute := "#[case(" block_description ")]"

block_description := nonempty_rust_string_literal
single_output_declaration := identifier | "(" identifier "," ")"
output_declaration := single_output_declaration | rust_tuple_of_two_or_more_identifiers
input_list := input ("," input)* ","?
input := identifier | "&" identifier
```

There is no end statement: the end block is implicit and `result` names the
flow output wire. The final statement may omit its semicolon. Computational
descriptions are nonempty. Questions and choices require a nonempty input list;
an action accepts an empty one. Every authored block declares at least one
output, so `-> ()` is invalid. Choice cases, outputs, and match arms have equal
counts. Outputs within one declaration are distinct. Repeated output names
across blocks are valid only when branch analysis proves their producers
mutually exclusive. Each flow parameter uses `flow_parameter`; a method receiver
is invalid. Validation rejects `return` expressions and the `?` operator in a
computational block body's own control-flow scope. It descends
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
arm value to the corresponding branch. Equally named outputs merge before
every downstream capture, independently of the consumer's other inputs.
Lowering preserves branch completion before a merge and the implicit order of
wire production before a consumer-selecting question or choice, and binds each
selected value once. A merge also precedes end when `result` has alternative producers; end
itself is neither a merge nor a computational
shared-continuation block.

The end block lowers to the `result` binding as the function's tail expression.
Rust checks body types, match exhaustiveness, ownership, borrows, output
patterns, alternative producer types, and the flow output.

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
