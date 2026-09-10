# RFC 0001: kaalang Language

- Status: accepted design draft

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
alternative branches, and while loops repeat a nested sequence. Every completed
execution produces the flow's `result` wire, which the implicit end block
captures.

A flow is written as an ordinary Rust function marked with `#[kaalang]`. Source
order is execution order: each block runs where it is written, in every
execution that reaches it.

This RFC defines kaalang's syntax and semantics.
[RFC 0004: kaalang Rust Lowering](0004-rust-lowering.md) describes how the
language is translated to Rust, and
[RFC 0002: kaalang Visual Language](0002-visual-language.md) defines its visual
representation.

## 2. Terminology

These terms describe the kaalang language independently of its representation or
visualization:

- **flow inputs** are values supplied when a flow begins; named flow inputs
  provide wires;
- the **flow output** is the value of the `result` wire, which the implicit end
  block captures when a flow finishes;
- a **block** is one unit of a flow; its inputs name the wires it captures, and
  its outputs name the wires it produces. Every block but the end block is
  authored;
- an **input** names an available wire captured by a block; `name` and
  `mut name` bind its value, while `&name` and `&mut name` bind shared and
  mutable references to it;
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
  that occurrence. Such an execution respects branch selection and wire
  availability; an equal wire name alone does not establish a capture
  dependency. The dependency participates in every execution where that
  resolution occurs;
- a **wire** is a named value provided by a producer and available for capture
  by block inputs;
- a **branch** is one alternative continuation selected by a question or choice;
- a **branch output** is a question or choice output; it selects one branch and
  is consumed by its implicit merge, or, for a name with no alternative
  producers, by at most one block whenever that output is selected;
- an **iteration** is one execution of a while body after its condition selects
  yes;
- a **case** describes one branch of a choice and is not itself a block;
- the **branch ancestry** of a block is the set of branches it belongs to: a
  while body inherits its header's ancestry and its yes branch; otherwise a
  block belongs to a branch when it captures that branch's output, or a wire
  produced by a block that belongs to the branch. A wire with alternative
  producers carries only the ancestry all of them share;
- a question or choice **decides** a block when two executions that select
  different branches of it, and the same branch of every other question or
  choice they both run, differ in whether the block executes;
- the **continuation** of a branch is the set of computational blocks whose
  participation can transitively depend on that branch's selected output through
  a chain of capture dependencies and wire merges participating in one possible
  execution that selects the branch; the end block is not part of a
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
  alternative producers of one logical wire, before any consumer captures it. It
  is not an authored block or a new producer occurrence;
- an **entry block** of a shared continuation has no predecessor in that
  continuation. Entry blocks consume values after their merges; they are not the
  convergence points themselves.

## 3. Block statements

A computational block has this common shape:

```rust
#[action("Description of the block.")]
let output = |input, &borrowed| {
    rust_expression
};
```

`#[action]`, `#[question]`, and `#[choice]` each carry one nonempty Rust string
literal. The string is the block description. A question may carry an ordered
`#[yes]` and `#[no]` pair, each optionally containing a nonempty branch
description. A choice carries two or more ordered `#[case("description")]`
attributes. The end block is implicit and therefore carries no attribute at all.

Source comments remain ordinary Rust comments. Block, question-branch, and case
descriptions come from their attributes.

An action may declare no outputs; an ordinary question and a choice must declare
theirs. The while form of a question has a nested body and no outputs (§4.5). An
action written without `let`, or with the pattern `let ()`, declares none; its
body must evaluate to `()`, which Rust checks. A body may be a Rust expression
or a braced block. For example, `let output = |input| input + 1;` and
`let output = |input| { input + 1 };` are equivalent. This lets ordinary Rust
formatting remove unnecessary braces without changing the flow.

Block attributes precede the statement. Output patterns contain only simple
identifiers with optional `mut`, a flat tuple of those bindings, or `()`. Type
annotations, wildcard outputs, nested patterns, `ref` bindings, and `let else`
are not supported. Block closures have no return type annotation or `move`,
`async`, `const`, or lifetime modifier. A braced body has no label or body-level
attributes.

An authored computational block body must not use a `return` expression or the
`?` operator in its own control-flow scope; it can complete only by normal
evaluation. Occurrences inside a nested Rust construct with its own control-flow
scope, such as a closure or item definition, belong to that construct and are
permitted. A `break` or `continue` must target a Rust loop or labeled block
entirely inside that computational body; it cannot transfer control to a kaalang
loop. Macro token streams are opaque to this validation.

## 4. Block kinds

| Block kind   | Meaning                                                                           | Inputs            | Outputs                                               |
| ------------ | --------------------------------------------------------------------------------- | ----------------- | ----------------------------------------------------- |
| **action**   | performs a computation or effect                                                  | zero or more      | zero or more                                          |
| **question** | evaluates a logical expression and selects one of two branches                    | one or more       | exactly two unit-valued control wires, one per branch |
| **choice**   | selects one of two or more cases and provides a value to the corresponding branch | one or more       | one per case, at least two                            |
| **while**    | repeats a nested sequence while its condition is true                             | one or more       | none                                                  |
| **end**      | the implicit block that finishes a flow                                           | the `result` wire | none                                                  |

### 4.1 action

An action evaluates its body and declares zero or more outputs:

```rust
#[action("Split the value.")]
let (left, right) = |input| {
    split(input)
};
```

A single identifier binds the complete body value to that wire. A tuple pattern
destructures the body's outer tuple positionally, including a singleton pattern:
`let (output,) = |input| { (input,) };` binds the tuple's element, while
`let output = |input| { (input,) };` binds the tuple itself. Rust checks that
the body value has the required shape. Output bindings may each carry `mut`, as
in `let (header, mut body) = |packet| { split(packet) };`.

An action that only performs an effect declares no outputs. Its body evaluates
to `()`, and the block hands nothing over:

```rust
#[action("Log the value.")]
|&value| {
    println!("{value}")
};
```

A named unit-valued output remains useful when a later block in the same branch
has to capture something the effect provides:

```rust
#[action("Log flow entry.")]
let entered = || {
    println!("start")
};

#[action("Continue.")]
let result = |entered| {
    continue_work()
};
```

### 4.2 question

A question evaluates a logical expression and selects one of two positional
branches:

```rust
#[question("Is the request eligible?")]
#[no("Reject the request.")]
#[yes("Continue processing.")]
let (ineligible, eligible) = |valid, &request, &policy| {
    request.age_days <= policy.window_days
};
```

Answer attributes and outputs correspond by position. The `#[yes]` output is
selected when the question body evaluates to true, and the `#[no]` output when
it evaluates to false. The attributes may appear in either order. Omitting both
is equivalent to writing `#[yes]` followed by `#[no]`, preserving the concise
form:

```rust
#[question("Is the request eligible?")]
let (eligible, ineligible) = |valid, &request, &policy| {
    request.age_days <= policy.window_days
};
```

If either answer attribute is written, both must be present exactly once. A
branch description is optional, but when present it is one nonempty Rust string
literal. The question body is evaluated exactly once per visit to the question.

Question outputs are distinct unit-valued control wires and branch outputs
(section 6). The downstream block lists the selected control wire and every data
wire it needs as separate inputs.

### 4.3 choice

A choice selects one of two or more ordered branches and makes the selected
match arm's value available on the corresponding output wire:

```rust
#[choice("Was a value supplied?")]
#[case("A value is available.")]
#[case("No value is available.")]
let (value, absent) = |input| {
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
Rust types. Like question outputs, they are branch outputs (section 6).

An implemented choice body contains exactly one `match` expression. An exact
whole-body `todo!()` is also a valid placeholder; every downstream branch still
undergoes type checking. Match bindings and block locals remain scoped to the
selected arm and are unavailable to downstream blocks; downstream code receives
only declared wires.

The selected arm's value leaves the match before the branch continues. A case
value is therefore owned or borrows data that outlives the choice, such as a
borrowed input; it cannot borrow a match binding or another local of its arm.
Each choice branch continuation is entered at most once per visit to the choice
and cannot be reached from the authored body.

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

### 4.5 while

A while uses `#[question]` on an ordinary Rust `while` statement. Its condition
has the same explicit capture syntax as other blocks; those captures belong to
the condition, not to the body:

```rust
#[action("Initialize the counter.")]
let mut count = || 0usize;

#[question("Is the counter below the limit?")]
while (|&count, &limit| *count < *limit) {
    #[action("Increment the counter.")]
    |&mut count| *count += 1;
}

#[action("Return the counter.")]
let result = |count| count;
```

The condition has at least one input and evaluates to `bool`, checked by Rust.
Its input aliases are created afresh before every check and end before the body
runs. Yes enters the body; normal completion of the body returns to the
condition. No leaves the loop and continues with the statements below it. The
body may be empty. The loop performs zero or more iterations, and kaalang does
not prove termination.

The ordered `#[yes]` and `#[no]` attributes follow §4.2 unchanged, including
optional descriptions and the default yes-first order. They determine branch
positions, never which answer enters the body. The while declares no control
wires or data outputs; there is no separate loop attribute.

The body contains ordinary kaalang statements, including nested while loops.
Each body block explicitly captures its own inputs. A block with no inputs runs
at its position in each iteration. If the while's inputs are unavailable in a
branch, neither its condition nor any part of its body participates there.

Wires declared outside the loop remain in their enclosing scope. State carried
between iterations is updated through an explicit mutable capture of an outer
mutable wire. The body cannot redeclare an enclosing wire. Its local outputs
exist only for the current iteration; they cannot be captured after the loop or
by its condition. Alternative producers merge within that iteration under §6.
Sibling loop bodies may reuse local names without declaring one shared wire.
Uncaptured owned wires drop when their iteration or nested branch scope ends.
Rust locals retain their ordinary computational-body scopes.

Normal branches of a question or choice inside the body may finish the iteration
without producing `result`; their routes join the return to the condition. They
must still obey the usual ancestry and merge rules for any work before that
return. Producing `result` completes the entire flow, including from a nested
loop, and skips every remaining iteration and the after-loop continuation.

This version has no kaalang `break`, `continue`, loop labels, or `while let`.
The local Rust control transfers permitted by §3 keep their ordinary behavior.
Moving a non-`Copy` outer wire on repeated checks or iterations is subject to
Rust's normal move checking; kaalang neither clones it nor stores an optional
replacement.

## 5. Flow inputs and outputs

kaalang v0.1 supports synchronous Rust functions, including `const fn`. An
`async fn` cannot declare a flow.

Function parameters declare flow inputs. Identifier parameters provide wires;
`name: T` declares an immutable binding and `mut name: T` explicitly permits
mutable borrowing of that wire. A wildcard parameter (`_`) accepts and discards
a flow input and provides no wire. Other parameter patterns, including `ref`
bindings and destructuring patterns, are invalid, as is a method receiver.

A named flow input is a producer occurrence and follows section 6's capture
requirement. The function return type is the contract for the `result` wire the
end block captures.

```rust
use kaalang::kaalang;

#[kaalang]
fn decide(request: Request) -> Decision {
    #[question("Is the request valid?")]
    let (valid, invalid) = |&request| { todo!() };

    #[action("Approve the valid request.")]
    let result = |valid, &request| { todo!() };

    #[action("Reject the invalid request.")]
    let result = |invalid, &request| { todo!() };
}
```

The two actions are alternative producers of the logical `result` wire. The end
block captures whichever producer ran. The example is a complete flow whose
computational bodies are placeholders. `todo!()` retains its Rust behavior and
panics if execution reaches it.

Unreachable-code warnings remain visible for unfinished flows. An author who
wants to silence them while filling in the bodies writes
`#[allow(unreachable_code)]` on the function.
[RFC 0004 §6](0004-rust-lowering.md#6-placeholders-and-function-attributes)
describes how lowering preserves placeholders and function attributes.

The function body contains attributed `let` declarations and closure-shaped
expression statements. Each statement declares one computational block. Every
`let` requires its semicolon, including the last declaration. A final action
without an output declaration may omit its semicolon like any Rust tail
expression.

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
    let result = || {};
}
```

An ignored parameter remains explicit:

```rust
#[kaalang]
fn discard(_value: Value) {
    #[action("Finish without the flow input.")]
    let result = || {};
}

#[kaalang]
fn discard_unnamed_input(_: Value) {
    #[action("Finish without the flow input.")]
    let result = || {};
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
shadowing: one execution cannot produce the same logical wire twice within its
declaring scope, whatever a block did with the earlier value. Each while
iteration creates fresh instances of its local wires (§4.5).

All alternative producers of a logical wire must declare the same mutability,
even when the wire is unused or never mutably borrowed. A merge preserves that
declaration; one mutable alternative cannot make an immutable alternative
mutable.

Repeating an output name always declares a merge, including when different
blocks capture that name or its leading underscore permits it to remain unused.
Every capture names the merged wire. A consumer's other inputs cannot select a
particular producer occurrence before the merge. Branch-local values must use
different names; a local borrow of a repeated name does not bypass its merge.

The selected value passes into the merge's shared lexical scope even when no
later block captures it. A leading underscore only permits a missing capture; it
does not suppress the merge or shorten the value's scope. Outputs with different
names remain distinct branch-local values.

Raw and ordinary spellings of the same Rust identifier name the same wire, so
`value` and `r#value` are interchangeable. Every producer occurrence is authored
before every consumer of its logical wire; a later producer cannot retroactively
join a wire that has already appeared as an input.

No computational block captures `result`; the end block consumes it and the flow
finishes.

Every producer occurrence, a named flow input or a block output, must have at
least one capture dependency to a later block or the end block unless its name
begins with `_`; that prefix permits the occurrence to have no consumer. A block
may leave every output uncaptured, and an action may declare none at all: source
order places it whether or not anything reads what it provides. For flow inputs,
action outputs, and merged wires, this requirement is existential rather than
per-execution: one possible execution establishing the dependency is sufficient.
A question or choice output without alternative producers must be captured by
its consumer in every execution selecting the output. For such an output, the
`_` prefix permits no consumer, but does not make an existing consumer optional.

```rust
#[question("Which value should be used?")]
let (yes, no) = |condition| { condition };

#[action("Build the yes value.")]
let selected = |yes| { yes_value() };

#[action("Build the no value.")]
let selected = |no| { no_value() };

#[action("Use the selected value.")]
let result = |selected| { use_value(selected) };
```

The alternative `selected` outputs merge before the shared consumer. Under
either branch selection exactly one producer supplies the merged wire. The
consumer captures that wire by name and executes once. kaalang has no authored
merge block.

Each computational block lists every wire available to its body:

```rust
let decision = |&request, policy, yes| { /* body */ };
```

Inputs have four forms:

- `name` binds the wire's value by ordinary Rust assignment, so a `Copy` type
  copies and any other type moves;
- `mut name` performs the same move or copy into a mutable block-local binding;
- `&name` binds a shared reference to the wire;
- `&mut name` binds a mutable reference to the wire.

These forms apply to actions, questions, while conditions, and choices alike. A
flow-input wire can be captured through `&mut name` only when its parameter
explicitly declares `mut name: T`; the capture never makes a parameter mutable.
A block-output wire can be captured through `&mut name` only when its output
binding declares `mut name`. An output without `mut` does not permit mutable
borrowing. The modifier grants permission; an unused permission is accepted. A
mutation through `&mut name` changes the original wire's stored value, which
later captures observe in source order. It does not declare another producer or
change the wire's capture dependencies. A `mut name` value capture needs no
mutable producer: it creates a mutable local binding after the move or copy.
Changing a copied value changes only that local copy. References inside a
captured value retain their ordinary Rust behavior.

```rust
// The flow declares `mut text: String`.
#[action("Append to the original text.")]
|&mut text| {
    text.push('!');
};

#[action("Move and extend the changed text.")]
let result = |mut text| {
    text.push('?');
    text
};
```

The end of a borrowing block ends its input alias. A produced reference can keep
the shared or mutable borrow live beyond the block. The underlying owner follows
the scope of its wire binding. A value created only inside one branch and not
carried out through the merge remains local to that branch; any remaining owned
value is dropped when the branch scope ends. Its scope is not extended to keep a
derived reference alive. Rust rejects a reference crossing the merge when its
owner remains inside the finished branch.

An owner bound before a selection remains in its enclosing scope and is not
transferred again by that selection's merge. An owner carried through a merge is
available in its shared continuation, whether or not it is captured there. An
owner from a partial merge stays in that partial continuation's scope unless the
same name has alternatives reaching the wider merge. Values without those
alternatives end with the partial scope.

For release earlier than scope exit, capture the resource by value and
explicitly drop it before the operation that needs it released. Rust rejects the
move if a live output still borrows the resource. Capturing a branch-specific
resource in a common action still requires it to be provided wherever that
action executes.

The outputs of one action appear together: an execution that runs the action
produces all of them, so they are ordinary data wires that later blocks borrow
or consume under these rules. The outputs of a question or choice are
alternatives: an execution produces exactly one of them. A branch output without
alternative producers is captured by at most one block, which consumes it using
`name` or `mut name`; either kind of borrow or a second consumer would give the
selected branch a second continuation.

That consumer must execute whenever the branch output is selected. Its other
inputs must therefore be available in every such execution. For example, an
action capturing the yes outputs of two independent questions is invalid: either
question can select yes while the other selects no, leaving the selected output
without its consumer. The flow must express a nested question or converge
alternative producers before a consumer that needs both results. Silently
skipping the consumer is invalid: that branch then produces no `result`.
Forwarding a branch output through an action does not lift this: the questions
and choices that decide the action's consumers stay the same (section 7).

When a branch output shares its name with an alternative producer, its one
continuation is the implicit merge. Captures after that merge use ordinary
merged data: they may borrow it with `&name` or `&mut name`, or have different
consumers in different executions. They do not capture the raw branch output.

Every input names a wire provided by a flow input or an earlier block output.
Questions and choices have at least one input; an action may have none.
Duplicate inputs are invalid regardless of their capture modifiers. Other
patterns, including `ref name`, `&mut mut name`, and destructuring, are invalid.
Whenever a consumer executes, each captured name resolves to exactly one
available producer. No producer is an error; more than one proves that the
supposed alternative producers are not mutually exclusive and is also an error.
Producer resolution is occurrence-level and branch-feasible: a same-named
downstream input does not depend on a particular producer occurrence unless some
possible execution can reach the consumer with that occurrence's value
available.

A block body receives local bindings only for its listed inputs, so omitted
wires are out of scope. Rust locals declared inside a block remain local to that
body and may reuse a wire's spelling without changing wire resolution.

Rust checks block body types, concrete wire types, repeated uses, moves,
borrows, match exhaustiveness, alternative producer type agreement, and output
destructuring. kaalang keeps types out of wire declarations and relies on Rust
inference. It does not detect `Copy`, insert clones, or reorder captures to
satisfy ownership or borrowing rules.

## 7. Execution and implicit convergence

Source order is the order of block declarations in each lexical sequence. For
each branch selection, every participating block executes once in that order
within its enclosing iteration. A while repeats its body according to §4.5. The
compiler neither rearranges blocks nor stops an execution early to hide later
participating work.

Named flow-input producers participate in every execution. A block participates
when its enclosing loop bodies are entered and every one of its inputs has a
participating earlier producer, so a block with no inputs participates wherever
it is written. All outputs of a participating action participate together, while
a participating question or choice provides only its selected output. Two
participating producers of one logical wire are an error, even when the wire is
unused.

Every authored computational block participates in at least one execution, and
each participating block executes once per visit to its enclosing sequence.

After a question or choice selects a branch, every participating block declared
below it must belong to that branch, by the branch ancestry of section 2, until
a merge joins the selected branch with the others or `result` finishes the
execution, or the enclosing iteration ends. Shared inputs do not attach a block
to a selected branch, and source position alone does not supply missing
ancestry. Lexical nesting supplies only the enclosing while's ancestry (§4.5).
The rule applies to actions, questions, and choices alike: a second selection
cannot start while the first selection's branches remain open. Branch membership
and completed merges are checked per execution, so interleaved declarations from
mutually exclusive branches do not conflict merely because of their source
positions.

The author writes separate blocks inside the branches, arranges a wire merge
before the common work, or moves that work above the selection when its
dependencies and Rust ownership permit. The compiler performs none of those
rewrites, and it neither attaches common work to all open branches nor resumes
those branches after a shared block.

Consequently, the questions and choices that decide a block form a chain through
capture dependencies or normal loop exits: each lies downstream of a branch of
the previous one, or after the loop containing it. A selection after a while
requires the loop to finish normally instead of producing `result`, including
when a nested selection in its body can finish the flow. This control order adds
no capture dependency or wire merge. Two independent questions or choices cannot
both decide whether one block executes, whichever way a selection withholds an
input. A partial merge admits only work belonging to the branches it joins. Two
disjoint partial merges do not permit a common action or independent selection
shared by both groups; a merge must join the groups before their common work can
run.

Validation computes continuations and convergence groups separately for each
question and choice. One question or choice may have several convergence groups.
Groups may be disjoint or one may contain the other; partially overlapping
groups are invalid. This also applies to the producer branches of implicit wire
merges, even when the merged name is unused. Nesting allows an earlier partial
merge followed by a merge with the remaining alternatives before end.

The shared continuation of each convergence group is represented once in the
semantic model, and each of its blocks executes at most once per visit to its
enclosing sequence. Its entry blocks are consumers after implicit wire merges.
One merge can precede several consumers, and a consumer can capture several
merged wires.

The branches of a convergence group are adjacent in authored branch order,
within one choice and across nested questions and choices alike: a branch that
does not produce a merged wire cannot separate two that do, even if it finishes
with `result`, and this holds for unused merged names too. Several disjoint
groups may be separated by branches outside either group. Precisely, the
interval is defined over the selections that affect the wire's production. For
each execution, record its producer occurrence, or absence. A question or choice
affects this record when two executions differ only at that selection under
section 2's agreement rule and have different records. Retain only these
selections in each execution's branch trace, in authored block order. Order the
traces lexicographically by authored branch order, which places each deciding
ancestor before its descendants. The traces with a producer occurrence must
occupy consecutive positions in this order. Unrelated selections before or after
a completed merge therefore do not affect adjacency.

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
- the execution bypassing `a` produces an occurrence of another merged wire `b`,
  ordered after the merge of `a` by the combined wire order below, and `b` is
  not `result`.

Thus nested branches returning to the enclosing shared continuation must
converge before it or together with it. A branch may instead finish with
`result`, after its own local work, without returning to that continuation,
provided it lies outside the earlier merge's branch interval. Partial merges
inside one question or choice and a new selection after a completed merge remain
valid. This is a language restriction, independent of whether a renderer can
route a particular diagram.

An implicit merge closes its producer branches before a consumer may capture the
merged wire, and source order must already say so: every block a merge waits for
is declared above every block that captures the merged wire. Consider all
executions in which one of that wire's producers runs, including executions with
no capture. A question or choice selects the producer when two executions in
this context differ only in its selection (using the agreement rule of
section 2) and produce different occurrences. Every computational block whose
participation such a selection decides within this context must finish before
the merge whenever that block participates. This includes work using a value
provided by only one producer branch. Such a value remains local; it cannot be
carried past the merge as a hidden optional input. The context excludes
executions producing none of the wire's occurrences, such as a terminal case
outside a partial convergence.

Validation combines capture dependencies with producer-to-merge,
branch-local-work-to-merge, and merge-to-consumer order. This order must be
acyclic within a single visit to each sequence; the explicit return to a while
condition is not a wire dependency or a wire merge. A cycle means that
completing a producer branch requires a value from a merge that already waits
for that branch. For example, independently merging `left_value` and
`right_value` is invalid when left-only work needs the merged `right_value` and
right-only work needs the merged `left_value`. A one-way use of a fully merged
value in another question's branch remains valid.

Rust checks that all alternative producers have one type. A branch-local value
absent on another converging path cannot reappear after convergence: it must
finish its own branch before the merge, and the merged wire carries nothing of
it. Past the merge the wire is ordinary data. A later block may leave it
uncaptured, and a question or choice anywhere in the flow may decide which block
captures it, exactly as for any other action output.

`result` is the only way to finish an execution, and its producer is the last
participating computational block of that execution; statements below it belong
to other branches or the continuation of an enclosing loop that this execution
leaves. Every branch that completes the flow provides `result`, directly or
through its continuation, and branches may do so after different numbers of
computational blocks.

## 8. Grammar

The outer syntax is ordinary Rust:

```text
flow := "#[kaalang]" rust_function

flow_parameter := "mut"? identifier ":" rust_type | "_" ":" rust_type

action_statement :=
    "#[action(" block_description ")]"
    ("let" output_pattern "=")? "|" input_list? "|" rust_expression ";"

question_statement :=
    "#[question(" block_description ")]"
    question_answers?
    "let" "(" output_binding "," output_binding ","? ")" "="
    "|" input_list "|" rust_expression ";"

while_statement :=
    "#[question(" block_description ")]"
    question_answers?
    "while" "(" "|" input_list "|" rust_expression ")"
    "{" block_statement* "}"

block_statement := action_statement | question_statement | choice_statement | while_statement

question_answers :=
    yes_attribute no_attribute | no_attribute yes_attribute

choice_statement :=
    "#[choice(" block_description ")]"
    case_attribute case_attribute+
    "let" "(" output_binding "," output_binding ("," output_binding)* ","? ")" "="
    "|" input_list "|" choice_body ";"

choice_expression := rust_match_expression | "todo!()"
choice_body := choice_expression | "{" choice_expression "}"
choice_match_arm := rust_pattern rust_guard? "=>" rust_expression

case_attribute := "#[case(" block_description ")]"
yes_attribute := "#[yes]" | "#[yes(" block_description ")]"
no_attribute := "#[no]" | "#[no(" block_description ")]"

block_description := nonempty_rust_string_literal
output_binding := "mut"? identifier
output_pattern :=
    output_binding | "(" ")" | "(" output_binding "," ")"
    | "(" output_binding "," output_binding ("," output_binding)* ","? ")"
input_list := input ("," input)* ","?
input := "mut"? identifier | "&" "mut"? identifier
```

Outputs within one declaration are distinct. An expression statement without
`let` and a declaration with the pattern `()` both declare none, which only an
action may do in closure-statement form. A while has no output pattern. Every
other constraint the grammar leaves open is stated with its rule: descriptions
and control transfers in section 3, block kinds and the implicit end block in
section 4, function forms and flow parameters in section 5, and repeated output
names in section 6.
