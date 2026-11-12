# RFC 0001: kaalang Language

- Status: accepted

## 1. Overview

kaalang is a language for writing flows as valid Rust. It is inspired by DRAKON
but defines its own syntax and semantics.

Execution and diagrams share one validated model: a diagram shows the blocks,
branches, and execution order that kaalang lowers to Rust.

A **flow** is a kaalang computation composed of blocks. Named wires make values
available to blocks, questions and choices divide execution into alternative
branches, and cycles repeat self-contained nested sequences until an explicit
`break`. Every execution that finishes passes through the flow's one explicit
structural `return`; a cycle may instead diverge.

A flow is written as an ordinary Rust function marked with `#[kaalang]`. Source
order is execution order: each block runs where it is written, in every
execution that reaches it.

A validated flow is drawable: a flow whose required connections have no
conforming diagram under [RFC 0002](0002-visual-language.md) is rejected at
compile time, like violations of the syntactic and semantic rules below.

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
- the **flow output** is the value transferred by a structural `return` when a
  flow finishes;
- a **block** is one authored unit of a flow; its inputs name the wires it
  captures, and its outputs name the wires it produces;
- a **computational block** is an action, call, question, or choice with a Rust
  body; a cycle instead contains a nested kaalang sequence, and breaks and
  returns transfer control;
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
  is an ordinary wire from there on. A name with alternative producers reaches
  its implicit merge; a name without them is captured by its first consumer in
  source order whenever that output is selected, and by any later consumer under
  the ordinary rules;
- an **iteration** is one execution of an entered cycle's body; normal
  completion begins the next iteration, and a `break` completes that cycle;
- a **case** describes one branch of a choice and is not itself a block;
- the **branch ancestry** of a block is the set of branches it belongs to: a
  cycle body inherits its header's capture ancestry and its enclosing ancestry;
  otherwise a block belongs to a branch when it captures that branch's output,
  or a wire produced by a block that belongs to the branch. A wire with
  alternative producers carries only the ancestry all of them share;
- a question or choice **decides** a block when two executions that select
  different branches of it, and the same branch of every other question or
  choice they both run, differ in whether the block executes;
- the **continuation** of a branch is the set of authored blocks whose
  participation can transitively depend on that branch's selected output through
  a chain of capture dependencies and wire merges participating in one possible
  execution that selects the branch; a transfer destination is not part of a
  continuation;
- relative to one question or choice, the **branch set** of a downstream block
  is the exact set of its branches whose continuations contain that block;
- a **convergence group** is a branch set containing at least two branches;
- the **shared continuation** of a convergence group is the nonempty set of
  authored blocks whose branch set is exactly that group;
- a **continuing branch** for a convergence group is one of its branches. It may
  contain paths that return from the flow without entering the shared
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

`#[action]`, `#[question]`, `#[choice]`, and `#[cycle]` each carry one nonempty
Rust string literal: the block description. `#[call]` carries the same literal,
or nothing at all: a call names the function it runs, so that name describes it
when no description is written. A question may carry an ordered `#[yes]` and
`#[no]` pair, each optionally containing a nonempty branch description. A choice
carries two or more ordered `#[case("description")]` attributes. Transfers have
no attributes or descriptions.

Source comments remain ordinary Rust comments. Block, question-branch, and case
descriptions come from their attributes.

Actions, calls, and cycles may declare no outputs; questions and choices must
declare theirs. Such a block written without `let`, or with the pattern
`let ()`, declares none. An action body and a call's function must then evaluate
to `()`, while every completing route of an outputless cycle must transfer `()`.
An action body may be a Rust expression or a braced block:
`let output = |input| input + 1;` and `let output = |input| { input + 1 };` are
equivalent. A call body is always one application and never carries braces of
its own (§4.2), and a cycle body is always a braced nested kaalang sequence. An
action or cycle with neither inputs nor outputs may omit both the output
declaration and empty capture list by writing its attributed braced body
directly: `#[action("Log.")] { log(); };` is shorthand for
`#[action("Log.")] || { log(); };`, as `break;` and `return;` omit an empty
capture list.

Every block statement ends with a semicolon, including the last statement.

A call with no captures may omit the empty capture list and write its
application directly, with or without outputs: `#[call] record();` or
`#[call("Start at the origin.")] let point = origin();`. Other kinds require a
capture list unless they use the bare braced-body form above.

Block attributes precede the statement. Output patterns contain only simple
identifiers with optional `mut`, a flat tuple of those bindings, or `()`. Type
annotations, wildcard outputs, nested patterns, `ref` bindings, and `let else`
are not supported. Block closures have no return type annotation or `move`,
`async`, `const`, or lifetime modifier. A braced body has no label or body-level
attributes.

An authored computational block body must not use a `return` expression or the
`?` operator in its own control-flow scope. Occurrences inside a nested Rust
construct with its own control-flow scope, such as a closure or item definition,
belong to that construct and are permitted. A `break` or `continue` must target
a Rust loop or labeled block entirely inside that computational body; it cannot
transfer control to a kaalang cycle. Macro token streams are opaque to this
validation.

## 4. Block kinds

| Block kind   | Meaning                                                                           | Inputs       | Outputs                                               |
| ------------ | --------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------- |
| **action**   | performs a computation or effect                                                  | zero or more | zero or more                                          |
| **call**     | runs one function, passing the captured inputs it names                           | zero or more | zero or more                                          |
| **question** | evaluates a logical expression and selects one of two branches                    | one or more  | exactly two unit-valued control wires, one per branch |
| **choice**   | selects one of two or more cases and provides a value to the corresponding branch | one or more  | one per case, at least two                            |
| **cycle**    | repeats a self-contained nested sequence until a break                            | zero or more | zero or more result wires                             |
| **break**    | completes the directly containing cycle                                           | zero or more | none                                                  |
| **return**   | completes the root flow                                                           | zero or more | none                                                  |

### 4.1 action

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

An effect-only action declares no outputs:

```rust
#[action("Log the value.")]
|&value| {
    println!("{value}")
};
```

Without input wires, it uses the bare-body shorthand:

```rust
#[action("Log flow entry.")]
{
    println!("start");
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
let continued = |entered| {
    continue_work()
};
```

### 4.2 call

A call body is one application of a function path to an argument list, and
nothing else:

```rust
#[call("Subtract the right value from the left one.")]
let difference = |left, right| math::difference(left, right);
```

Every argument names one of the block's captured inputs and nothing else. An
expression, a literal, a nested application, or a reference is rejected; that
work belongs in an action. The capture list fixes the order the block reads its
wires in; the argument list fixes only where each value lands, so a call may
pass its captures in another order.

A capture need not appear in the argument list; it still establishes ordering
and branch membership and performs its ordinary move or borrow. For example, a
call can capture a question's control wire without passing it:

```rust
#[question("Is the value positive?")]
#[yes("YES")]
#[no("NO")]
let (positive, negative) = |&value| *value > 0;

#[call("Double a positive value.")]
let scaled = |positive, value| double(value);
```

One captured alias may appear more than once in the argument list. Rust checks
repeated uses under its ordinary move rules.

A call declares its outputs from the returned value the way an action does from
its body value.

The path may carry generic arguments (`str::parse::<u32>`) or a qualified self
type (`<u32 as FromStr>::from_str`), and is resolved by Rust in the block's own
scope. It cannot name a captured function value; applying a function supplied on
a wire requires an action. Parentheses around the path, braces around the
application, and body-level attributes are invalid. A path substituted from a
`macro_rules!` metavariable is accepted as that path.

A call requires a function path and has no `todo!()` placeholder. Use an action
while the function is unspecified.

Captures keep their ordinary meaning, so `&name` and `&mut name` pass a
reference and the callee's parameter takes one:

```rust
#[call("Add the length of the text to the total.")]
let extended = |&mut total, &text| extend(total, text);
```

A capture may not be written `mut`: a call passes its inputs on unchanged, so
nothing in the block could assign to one.

A bare `#[call]` is described by its path, without the arguments:

```rust
#[call]
let end = |a, b| math::gcd(a, b);
```

Rust checks the arguments against the function's parameters and the returned
value against the output pattern; kaalang adds no signature of its own.

### 4.3 question

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
it evaluates to false. The attributes may appear in either order; omitting both
is equivalent to `#[yes]` followed by `#[no]`:

```rust
#[question("Is the request eligible?")]
let (eligible, ineligible) = |valid, &request, &policy| {
    request.age_days <= policy.window_days
};
```

If either answer attribute is written, both must be present exactly once. A
branch description, when present, is one nonempty Rust string literal. The
question body is evaluated exactly once per visit to the question.

Question outputs are distinct unit-valued control wires and branch outputs
(section 6). A downstream block lists the selected control wire and every data
wire it needs as separate inputs.

### 4.4 choice

A choice makes the selected match arm's value available on the corresponding
output wire:

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
undergoes type checking. An individual match arm cannot consist solely of
`todo!()`, with or without braces. Match bindings and block locals remain scoped
to the selected arm; downstream code receives only declared wires.

The selected arm's value leaves the match before the branch continues. A case
value is therefore owned or borrows data that outlives the choice, such as a
borrowed input; it cannot borrow a match binding or another local of its arm.
Each choice branch continuation is entered at most once per visit to the choice
and cannot be reached from the authored body.

### 4.5 cycle

A cycle is a described, self-contained block whose body is a nested sequence of
kaalang statements. Its captures are its complete external input interface, and
its output pattern declares the complete result interface that becomes available
only when the cycle completes.

```rust
#[cycle("Count to the limit.")]
let total = |mut count, limit| {
    #[question("Has the limit been reached?")]
    let (done, again) = |count, limit| count >= limit;

    |done, count| break count;

    #[action("Increment the counter.")]
    |again, &mut count| *count += 1;
};
```

The `#[cycle("description")]` attribute and braces are required. A cycle has no
condition, authored label, or case declarations. Native Rust `loop`, `while`,
and `for` expressions are permitted inside computational bodies under §3.

Cycle captures are evaluated once before the first iteration and create bindings
local to the cycle body that remain alive across iterations. A `mut` value
capture therefore provides local state that later body blocks may mutate, while
a borrowed capture keeps its Rust borrow for the lifetime of the cycle block.
Each inner block must explicitly capture the cycle input bindings or earlier
iteration-local outputs it uses. Naming an uncaptured outer wire bypasses the
cycle interface and is invalid. A nested cycle declares its own complete
interface in the same way.

Each entered cycle executes at least one iteration, possibly completing
immediately. Normal completion of a body route starts another iteration; there
is no authored `continue`, and kaalang does not prove termination. An empty body
therefore repeats forever. A question or choice inside the body follows its
ordinary branch and capture rules. A break on one branch does not implicitly
attach later statements to another branch.

Iteration-local wires are fresh on every repeat and cannot be captured outside
the iteration. To retain a value, store it through a cycle input or transfer it
as the break value. Remaining iteration-local values drop on repeat; only the
persistent cycle input bindings remain. On completion, the cycle's declared
outputs become wires in the parent scope.

The output pattern binds the break value exactly as an action's binds its body
value. All outputs are conjunctive: every completing route provides them
together, and individual outputs cannot be selected or omitted. Alternative
result wires merge by the ordinary rules before the cycle's single break; outer
consumers resolve to the cycle block.

These cycles expose the same Rust pair as either one tuple-valued wire or two
distinct wires:

```rust
#[cycle("Keep the pair together.")]
let pair = |left, right| {
    |left, right| break (left, right);
};

#[cycle("Expose both elements.")]
let (first, second) = |pair| {
    |pair| break pair;
};
```

An outputless cycle completes with unit:

```rust
#[kaalang]
fn complete_once() {
    #[cycle("Complete on the first iteration.")]
    {
        break;
    };

    return;
}
```

A nested cycle hands its declared result to the enclosing body rather than
escaping that body:

```rust
#[kaalang]
fn nested(seed: usize) -> usize {
    #[cycle("Run the outer cycle.")]
    let result = |seed| {
        #[cycle("Run the inner cycle.")]
        let inner = |seed| {
            |seed| break seed;
        };

        |inner| break inner;
    };

    |result| return result;
}
```

Within one cycle body, completing routes and repeating routes must still admit a
conforming diagram in authored branch order. In particular, a repeating route
cannot be enclosed between completing routes when doing so would force a
crossing. Nested cycles are checked as bounded regions; each inner result
converges at its own boundary before the outer body continues. The compiler and
renderer never reorder question answers or choice cases.

A cycle with no reachable break has no normal continuation. It provides no
outputs at runtime, and blocks reachable only after it are rejected. A fully
diverging cycle remains valid without a return route.

```rust
#[kaalang]
fn spin() -> ! {
    #[cycle("Repeat forever.")]
    || {};
}
```

### 4.6 break

A cycle may contain at most one structural break belonging to that cycle. Nested
cycles each have their own limit; native Rust transfers inside computational
bodies do not count. When present, the break has one of these forms:

```rust
break;
break ();
|| break;
|done| break;
|done, value| break value;
|done, left, right| break (left, right);
```

It cannot name or leave an outer cycle. A break outside a cycle, a labeled
break, and an attributed break are invalid. The transfer immediately terminates
its body route and skips later statements in that route.

An optionally empty capture list gates participation and creates aliases under
the ordinary capture and branch rules. Those aliases remain in scope while the
transfer value is evaluated and moved. The value is one captured binding, `()`,
or a flat tuple assembled from captured bindings, including a singleton tuple. A
value-less `break;` is `break ();`. Calls, operators, field access, literals
other than `()`, and other computations belong in a described computational
block. A capture-free break cannot name a wire.

A captured transfer may also wrap its single `break` in braces, as in
`|value| { break value; };`. Parentheses around the whole transfer value are
permitted.

Every completing route executes that same break; a cycle with no completing
route may omit it.

Rust checks agreement between the break value and the cycle's output pattern.
Moving an iteration-local owned value out is permitted; a reference to a local
that ends at completion is rejected by Rust. Break captures are consumers for
producer-usage and branch-participation validation.

### 4.7 return

A flow may contain at most one structural return. When present, it has one of
the corresponding root-flow forms:

```rust
return;
return ();
|| return;
|value| return value;
|left, right| return (left, right);
```

It may appear in a branch continuation owned by the root sequence and is invalid
anywhere inside a cycle.

Return captures, values, braces, and computation restrictions are the same as
for break; a value-less `return;` is `return ();`. Rust checks the transferred
value against the function's declared return type. Return captures are consumers
and remain in scope until their value has been transferred.

Every root route that finishes executes the same structural return. A flow with
any completing route must contain that return; only a fully diverging flow may
omit it. Reaching the end of the root sequence without returning is invalid,
including for a function returning `()`.

## 5. Flow inputs and outputs

kaalang v0.1 supports synchronous Rust functions, including `const fn`. An
`async fn` cannot declare a flow. A flow may be a free function, an associated
function, or a method.

Function parameters declare flow inputs. Identifier parameters provide wires;
`name: T` declares an immutable binding and `mut name: T` explicitly permits
mutable borrowing of that wire. A wildcard parameter (`_`) accepts and discards
a flow input and provides no wire. Other parameter patterns, including `ref`
bindings and destructuring patterns, are invalid.

A receiver declares the flow input named `self`. Rust binds that name only as a
receiver, so no capture rebinds it: a block that captures the receiver reads it
exactly as the signature declares it, and spells the capture the same way.

| Receiver                                            | Capture         | Wire              |
| --------------------------------------------------- | --------------- | ----------------- |
| a value: `self`, `mut self`, `self: Box<Self>`      | `\|self\|`      | consuming         |
| a shared reference: `&self`, `self: &Self`          | `\|&self\|`     | borrowing         |
| a mutable reference: `&mut self`, `self: &mut Self` | `\|&mut self\|` | mutably borrowing |

Any other spelling of a `self` capture is invalid, as is a `self` capture in a
flow that declares no receiver, and a block output named `self`. A body that
reads `self` must capture it, and a cycle that captures it imports the one wire
rather than creating a fresh instance per iteration (§4.5).

```rust
use kaalang::kaalang;

impl Rectangle {
    #[kaalang]
    fn scaled_area(&self, scale: u32) -> u32 {
        #[action("Scale the width.")]
        let scaled = |&self, scale| { self.width * scale };

        #[action("Multiply by the height.")]
        let end = |&self, scaled| { scaled * self.height };

        |end| return end;
    }
}
```

A `todo!()` body is a placeholder that keeps its Rust behavior and panics if
execution reaches it:

```rust
use kaalang::kaalang;

#[kaalang]
fn decide(request: Request) -> Decision {
    #[question("Is the request valid?")]
    let (valid, invalid) = |&request| { todo!() };

    #[action("Approve the valid request.")]
    let decision = |valid, &request| { todo!() };

    #[action("Reject the invalid request.")]
    let decision = |invalid, &request| { todo!() };

    |decision| return decision;
}
```

Unreachable-code warnings remain visible for unfinished flows;
`#[allow(unreachable_code)]` on the function silences them.
[RFC 0004 §6](0004-rust-lowering.md#6-placeholders-and-function-attributes)
describes how lowering preserves placeholders and function attributes.

### 5.1 Zero-computation flow

A completing flow with no computational blocks still writes its transfer:

```rust
#[kaalang]
fn identity<T>(value: T) -> T {
    |value| return value;
}
```

An empty body is invalid: it reaches the end of the root sequence without
returning. A flow that computes nothing but must finish returns unit directly:

```rust
#[kaalang]
fn nothing() {
    return;
}
```

An ignored parameter remains explicit:

```rust
#[kaalang]
fn discard(_value: Value) {
    return;
}

#[kaalang]
fn discard_unnamed_input(_: Value) {
    return;
}
```

`_value` is a named flow input whose wire may remain uncaptured; `_` is unnamed
and provides no wire.

## 6. Wires, producers, and captures

Several blocks may declare the same output name only when they are alternative
producers. This is not sequential shadowing: one execution cannot produce the
same logical wire twice within its declaring scope, whatever a block did with
the earlier value. Each cycle iteration creates fresh instances of its local
wires (§4.5).

All alternative producers of a logical wire must declare the same mutability,
even when the wire is unused or never mutably borrowed; the merge preserves that
declaration.

Repeating an output name always declares a merge, including when different
blocks capture that name or its leading underscore permits it to remain unused.
Every capture names the merged wire. A consumer's other inputs cannot select a
particular producer occurrence before the merge. Branch-local values must use
different names; a local borrow of a repeated name does not bypass its merge.

The selected value passes into the merge's shared lexical scope even when no
later block captures it; a leading underscore does not shorten that scope.

Raw and ordinary spellings of the same Rust identifier name the same wire, so
`value` and `r#value` are interchangeable. Every producer occurrence is authored
before every consumer of its logical wire; a later producer cannot retroactively
join a wire that has already appeared as an input.

Every producer occurrence, a named flow input or block output, must have at
least one capture dependency to a later block or transfer unless its name begins
with `_`. A block may leave every output uncaptured, and an action, call, or
cycle may declare none at all: source order places it whether or not anything
reads what it provides. For flow inputs, action, call, or cycle outputs, and
merged wires, this requirement is existential rather than per-execution: one
possible execution establishing the dependency is sufficient. A question or
choice output without alternative producers must be captured by its first
consumer in source order in every execution selecting the output. For such an
output, the `_` prefix permits no consumer, but does not make an existing first
consumer optional. Later consumers are ordinary downstream work and may be
conditional.

```rust
#[question("Which value should be used?")]
let (yes, no) = |condition| { condition };

#[action("Build the yes value.")]
let selected = |yes| { yes_value() };

#[action("Build the no value.")]
let selected = |no| { no_value() };

#[action("Use the selected value.")]
let result = |selected| { use_value(selected) };

|result| return result;
```

The alternative `selected` outputs merge before the shared consumer, which
captures the merged wire by name and executes once.

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

These forms apply to actions, questions, choices, cycles, breaks, and returns.
Calls accept all except `mut name` (§4.2); receivers follow §5. A wire can be
captured through `&mut name` only when its flow parameter declares `mut name: T`
or its output binding declares `mut name`. The modifier grants permission; an
unused permission is accepted. A mutation through `&mut name` changes the
original wire's stored value, which later captures observe in source order,
without declaring another producer or changing the wire's capture dependencies.
A `mut name` value capture needs no mutable producer: it creates a mutable local
binding after the move or copy. References inside a captured value retain their
ordinary Rust behavior.

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

|result| return result;
```

The end of a borrowing block ends its input alias. A produced reference can keep
the shared or mutable borrow live beyond the block. The underlying owner follows
the scope of its wire binding. A value created only inside one branch and not
carried out through the merge remains local to that branch; any remaining owned
value is dropped when the branch scope ends. Its scope is not extended to keep a
derived reference alive. Rust rejects a reference crossing the merge when its
owner remains inside the finished branch.

An owner bound before a selection remains in its enclosing scope and is not
transferred again by that selection's merge. An owner from a partial merge stays
in that partial continuation's scope unless the same name has alternatives
reaching the wider merge.

For release earlier than scope exit, capture the resource by value and
explicitly drop it before the operation that needs it released. Rust rejects the
move if a live output still borrows the resource. Capturing a branch-specific
resource in a common action still requires it to be provided wherever that
action executes.

The first consumer or transfer of a branch output without alternative producers
carries its branch's single entry, so its other inputs must be available in
every execution selecting the output. For example, an action capturing the yes
outputs of two independent questions is invalid: either question can select yes
while the other selects no, leaving the selected output without its first
consumer. The flow must express a nested question or converge alternative
producers before a consumer that needs both results. Forwarding a branch output
through an action does not lift this: the questions and choices that decide the
action's consumers stay the same (section 7).

Later consumers of the same branch output may borrow it, and a nested question
or choice may decide which of them executes; section 7 forbids two independent
questions or choices from deciding one block, whether that block is the first
consumer or a later one. Every consumer of a branch output belongs to that
branch, so a branch output stays branch-local: it must finish its own branch
before any merge that closes it, like any other branch-local value.

When a branch output shares its name with an alternative producer, its one
continuation is the implicit merge. Captures after that merge use ordinary
merged data and do not capture the raw branch output.

Every input names a wire provided by a flow input, an earlier block output, or a
cycle interface binding in the current nested scope. Duplicate inputs are
invalid regardless of their capture modifiers. Other patterns, including
`ref name`, `&mut mut name`, and destructuring, are invalid. Whenever a consumer
executes, each captured name resolves to exactly one available producer or
imported cycle binding. No source is an error; more than one proves that the
supposed alternative producers are not mutually exclusive and is also an error.

A computational body and transfer receive local bindings only for their listed
inputs, so omitted wires are out of scope. Rust locals declared inside a
computational body remain local to that body and may reuse a wire's spelling
without changing wire resolution.

Rust checks block body types, concrete wire types, repeated uses, moves,
borrows, match exhaustiveness, alternative producer type agreement, and output
destructuring. kaalang keeps types out of wire declarations and relies on Rust
inference. It does not detect `Copy`, insert clones, or reorder captures to
satisfy ownership or borrowing rules.

## 7. Execution and implicit convergence

Validation considers every question answer and choice case possible. It treats
computational bodies as opaque: their conditions, patterns, panics, and native
Rust loops do not establish structural reachability or divergence. In this RFC,
an execution means a structural route under those possible branch selections.

Source order is the order of block declarations and transfers in each lexical
sequence. For each branch selection, every participating item executes once in
that order within its enclosing iteration. The compiler neither rearranges
blocks nor stops an execution early to hide later participating work.

Named flow-input producers participate in every execution. A block or transfer
participates when its enclosing cycle bodies are entered and every one of its
inputs has a participating earlier source, so an item with no inputs
participates wherever it is written. All outputs of a participating action,
call, or completed cycle participate together, while a participating question or
choice provides only its selected output. Two participating producers of one
logical wire are an error, even when the wire is unused.

Every authored block or transfer participates in at least one execution, and
each participating item executes once per visit to its enclosing sequence.

After a question or choice selects a branch, every participating block declared
below it must belong to that branch, by the branch ancestry of section 2, until
a merge joins the selected branch with the others, a structural transfer ends
the route, or the enclosing iteration ends. Shared inputs do not attach a block
to a selected branch, and source position alone does not supply missing
ancestry. Lexical nesting supplies the ancestry described by the enclosing cycle
(§§4.5–4.7). The rule applies to every block and transfer: a second selection
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
capture dependencies or normal cycle completion: each lies downstream of a
branch of the previous one, or after the cycle containing it. A selection after
a cycle requires a reachable break result instead of an endlessly repeating
route. This control order adds no capture dependency or wire merge. Two
independent questions or choices cannot both decide whether one block executes,
whichever way a selection withholds an input. A partial merge admits only work
belonging to the branches it joins. Two disjoint partial merges do not permit a
common action or independent selection shared by both groups; a merge must join
the groups before their common work can run.

Validation computes continuations and convergence groups separately for each
question and choice. One question or choice may have several convergence groups.
Groups may be disjoint or one may contain the other; partially overlapping
groups are invalid. This also applies to the producer branches of implicit wire
merges, even when the merged name is unused. Nesting allows an earlier partial
merge followed by a merge with the remaining alternatives before the root route
returns.

The shared continuation of each convergence group is represented once in the
semantic model, and each of its blocks executes at most once per visit to its
enclosing sequence. One merge can precede several consumers, and a consumer can
capture several merged wires.

The branches of a convergence group are adjacent in authored branch order,
within one choice and across nested questions and choices alike: a branch that
does not produce a merged wire cannot separate two that do, even if it reaches
the flow's return, and this holds for unused merged names too. Several disjoint
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
branch, allow `join` and `direct` to merge into `shared` while `skip` provides
an alternative value for a wider merge before the flow's return. Ordering them
`join, skip, direct` is invalid: `skip` separates the two producers of `shared`.

A nested branch may bypass an inner convergence and rejoin the other branches at
a later, wider ordinary wire merge. The bypassing branch does not execute the
inner merge or its shared continuation. Both merges still obey convergence-group
nesting, branch adjacency, and the source and merge order below; no wire name or
destination receives special treatment.

An implicit merge closes its producer branches before a consumer may capture the
merged wire, and source order must already say so: every block a merge waits for
is declared above every block that captures the merged wire. Consider all
executions in which one of that wire's producers runs, including executions with
no capture. A question or choice selects the producer when two executions in
this context differ only in its selection (using the agreement rule of
section 2) and produce different occurrences. Every authored block whose
participation such a selection decides within this context must finish before
the merge whenever that block participates. This includes work using a value
provided by only one producer branch. Such a value remains local to that branch.
The context excludes executions producing none of the wire's occurrences, such
as a terminal case outside a partial convergence.

Validation combines capture dependencies with producer-to-merge,
branch-local-work-to-merge, and merge-to-consumer order. This order must be
acyclic within a single visit to each sequence; an iteration back edge to a
cycle entry is not a wire dependency or a wire merge. A dependency cycle means
that completing a producer branch requires a value from a merge that already
waits for that branch. For example, independently merging `left_value` and
`right_value` is invalid when left-only work needs the merged `right_value` and
right-only work needs the merged `left_value`. A one-way use of a fully merged
value in another question's branch remains valid.

A branch-local value absent on another converging path cannot reappear after
convergence; the merged wire carries nothing of it. Past the merge the wire is
ordinary data. A later block may leave it uncaptured, and a question or choice
anywhere in the flow may decide which block captures it, exactly as for any
other action output.

Validation summarizes at most one iteration of each cycle. A summary ends either
at the flow's return or at a cycle's next iteration. A break continues the
summary through its cycle's result boundary. Repeating summaries do not reach
the root return boundary and therefore do not split the adjacency of routes that
reach the return. The return is the last participating item of every finishing
execution; statements below it belong to non-finishing branches. Finishing
routes may converge after different numbers of computational blocks.

## 8. Grammar

The outer syntax is ordinary Rust:

```text
flow := "#[kaalang]" rust_function

flow_parameter :=
    "mut"? identifier ":" rust_type | "_" ":" rust_type | rust_receiver

action_statement :=
    "#[action(" block_description ")]"
    (("let" output_pattern "=")? "|" input_list? "|" rust_expression ";"
     | "{" rust_statement* "}" ";")

call_statement :=
    ("#[call(" block_description ")]" | "#[call]")
    ("let" output_pattern "=")? ("|" input_list? "|")? call_application ";"
call_application := rust_path "(" call_argument_list? ")"
call_argument_list := identifier ("," identifier)* ","?

question_statement :=
    "#[question(" block_description ")]"
    question_answers?
    "let" "(" output_binding "," output_binding ","? ")" "="
    "|" input_list "|" rust_expression ";"

cycle_statement :=
    "#[cycle(" block_description ")]"
    (("let" output_pattern "=")? "|" input_list? "|" "{" block_statement* "}" ";"
     | "{" block_statement* "}" ";")

transfer_prefix := "|" input_list? "|"
break_statement := "break" transfer_value? ";" | transfer_prefix break_body ";"
return_statement := "return" transfer_value? ";" | transfer_prefix return_body ";"
break_body := "break" transfer_value? | "{" break_body ";"? "}"
return_body := "return" transfer_value? | "{" return_body ";"? "}"
transfer_value :=
    identifier | "(" ")" | "(" identifier "," ")"
    | "(" identifier "," identifier ("," identifier)* ","? ")"
    | "(" transfer_value ")"

block_statement :=
    action_statement | call_statement | question_statement | choice_statement
    | cycle_statement | break_statement | return_statement

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

Outputs within one declaration are distinct. In a `call_statement`, `rust_path`
may not be a single identifier that names one of the block's captured aliases.
Tuple construction in a transfer value does not implicitly destructure a
captured tuple-valued wire. Every other constraint the grammar leaves open is
stated with its rule: descriptions and computational control flow in section 3,
block and transfer kinds in section 4, function forms and flow parameters in
section 5, and repeated output names in section 6.
