# RFC 0006: Stages and unified cycle interfaces

- Status: proposed
- Language: [RFC 0001: kaalang Language](0001-language.md)
- Visual language: [RFC 0002: kaalang Visual Language](0002-visual-language.md)
- Renderer: [RFC 0003: kaalang SVG Renderer](0003-svg-renderer.md)
- Lowering: [RFC 0004: kaalang Rust Lowering](0004-rust-lowering.md)

## 1. Motivation and scope

An algorithm with several repeatable steps can already be expressed through a
cycle, a choice, and an explicitly maintained state that selects the next step.
A kaalang **stage** makes those steps explicit and lets the compiler generate
the dispatcher. The flow first prepares its data, then enters one stage at a
time through named control signals.

A stage behaves as a branch of that common dispatcher. Its entry signal selects
one visit and can carry a value into it. A completing visit selects one of the
stage's declared alternative exits, or the terminal stage completes the flow
with its structural return. A stage may instead diverge in a nested cycle.
Stages share the outer data scope established by preparation; their internal
blocks capture the entry value and other data they use.

This proposal defines stage syntax, execution, validation, Rust lowering, and
visual representation. It also unifies stages and cycles around inherited data
scope and explicitly declared alternative outputs. Cycle repetition requires an
explicit structural `continue`. These cycle rules apply throughout the language,
including flows without stages. The cycle contract is independent of stages;
staged flows use the same cycle rules as every other flow.

A transition's name selects its destination, and its value becomes that stage's
input. Ordinary functions transfer these values under Rust's ownership and
lifetime rules, including non-`Copy` owners. In a `const fn`, transition values
must implement `Copy`: this keeps the generated dispatch state free of
destructors even with generic payloads. The restriction follows the function's
`const` qualifier, including when it is called at runtime. Authored computations
still obey Rust's constant-evaluation rules. Data shared across visits can
remain in outer storage and be accessed through explicit captures.

### 1.1 Terms

This RFC uses RFC 0001's wire, producer, capture, branch, and merge terms, with
the following additions:

- A **stage** is a named kaalang sequence entered through one incoming signal.
- A **visit** is one execution of a stage body, with fresh local wires.
- A **transition** ends a completing preparation or stage route by selecting the
  next stage and supplying its entry value.
- A **signal** is a wire used to select control flow. A stage transition signal
  names its destination and carries the value received there.
- A stage's **entry** is its named incoming wire, bound for the current visit.
- A **gate** is a wire whose availability establishes a block's or transfer's
  participation in a route. A cycle header may name one gate.
- An **export** passes a selected value out of its containing sequence: to a
  destination stage through a transition, or from a cycle to its continuation.

## 2. Preparation and stage declarations

```rust
#[kaalang]
fn complete<T>(input: T) -> T {
    #[action("Begin processing.")]
    let finish = || ();

    #[stage("Return the input.")]
    |finish| {
        |input| return input;
    };
}
```

A flow containing stages has two consecutive sections:

1. **Preparation:** ordinary kaalang blocks that execute once, establish shared
   data, perform initial work, and select the initial stage.
2. **Stages:** the stage declarations, in their authored diagram order.

The first stage declaration ends preparation. Every later root statement must
also be a stage declaration. Preparation follows the ordinary source-order,
branch, merge, and cycle rules. Every preparation route that reaches the stage
section must supply exactly one entry signal.

A declaration has exactly one `#[stage("description")]` attribute containing a
nonempty Rust string literal, a header `|entry|` with one plain identifier, a
braced kaalang sequence, and a trailing semicolon. A stage with completing
transitions declares its possible outputs before the header, for example
`let (count, finish) = |count| { ... };`. These are alternative outputs: a
completed visit provides exactly one. A terminal stage or a stage whose every
route diverges omits the output declaration or uses `let ()`.

The header receives the stage's entry signal and activates its whole body. Its
identifier names the received value as an immutable wire, available to explicit
inner captures for that visit. Entering the body counts as use of the signal
even when no inner block captures its value again. Stage declarations are
permitted directly in the root flow body. Their order determines their diagram
positions; transitions determine their execution order.

Entry names are unique across the flow. Raw and ordinary spellings identify the
same name. The names in these headers identify the flow's transition signals.
Each such signal has a Rust type, with a `Copy` bound in a `const fn`. All
producers targeting one entry, including initial producers in preparation or the
function inputs, must agree on its Rust type. Different entries can have
different types. Each declared stage output resolves to an entry, including the
stage's own entry for a self-transition. All headers and output declarations are
resolved before executable sequences are analyzed, so a transition can target an
earlier or later stage.

Using RFC 0001's description, identifier, and block notation:

```text
staged_flow_body := block_statement* stage_declaration+
stage_declaration :=
    "#[stage(" block_description ")]"
    ("let" output_pattern "=")?
    "|" identifier ","? "|" "{" block_statement* "}" ";"
```

Here `output_pattern` uses RFC 0001's named output bindings: one binding, a
tuple of bindings, or `()`. A stage output binding cannot carry `mut`; duplicate
names are invalid. Each destination receives an immutable entry, regardless of
the source wire's mutability. A tuple lists alternative exits rather than
destructuring one result: `(output,)` and `output` declare the same single exit
and transfer its whole value. The preparation statements of a staged flow are
subject to §4's completion rules. Cycles use the unified rules in §5.

## 3. Data scope and signals

### 3.1 Outer data

Function inputs and preparation outputs establish the common outer data scope.
Preparation must make every outer wire used by a stage available at the stage
section's entry through ordinary branch and merge rules. A branch-local
preparation value must reach that common scope through its wire's merge before
stages can use it. Initial transition signals provide the selected entry; the
common outer data scope contains the remaining wires. A stage sees its current
entry value, not the preparation signal from an earlier visit.

Each stage body can use those outer data wires, its current entry, and its own
earlier local outputs. Each computational block or transfer explicitly captures
the wires it reads, using the existing `name`, `mut name`, `&name`, and
`&mut name` forms. For example, an action inside `|partition|` may capture
`|&mut values|`, where `values` was declared during preparation. Mutation
through a permitted mutable capture updates the shared wire for subsequent work
and subsequent stage visits.

The current entry cannot be captured with `&mut`, including from inside a nested
cycle. A value capture `mut entry` remains valid wherever that capture form is
supported: it moves or copies the entry into a mutable block-local alias. To
modify an owned entry across several blocks, first produce a local wire such as
`let mut work = |entry| entry;`, then capture `&mut work`. An initial parameter
or preparation wire may be mutable in preparation; each receiving stage still
binds its entry immutably. This concerns the binding, not the value's type: an
entry carrying `&mut T` or interior mutability retains its ordinary Rust
behavior.

The enclosing function's generics, `Self`, and receiver remain in their ordinary
Rust context. Inner blocks that use the receiver capture it with RFC 0001 §5's
receiver spelling. A nested cycle inherits the data visible at its entry under
the same rules (§5); its header controls entry, while its inner blocks capture
the data they use.

Stage-local wires are fresh on every visit. A declared output transfers its
selected value to the next stage by an ordinary Rust move or copy. Other local
values remain in the current stage; preserve them in outer state through a
mutable capture when needed by later visits. A stage-local producer cannot reuse
a name inherited from common outer data or the current entry, except for the
self-transition defined in §3.2. Different stages may reuse local names in their
separate scopes.

### 3.2 Entry and outgoing signals

A stage's entry signal activates one visit. Inside the body, captures resolve to
the visible entry, common outer data, or outputs of that visit. Each declared
stage output selects the same-named local wire produced directly in the body's
sequence. The declaration and the internal wire belong to separate scopes; the
declaration does not provide a wire inside the body. A matching internal
producer must exist, and each declared exit must be reachable. The incoming
entry alone does not supply a declared output; a transition requires a local
producer.

A question can select unit signals directly; a choice can provide different
values for its cases; an action, call, or completed cycle can produce a signal
alongside ordinary local outputs. Only names listed in the stage's output
declaration can leave it. An internal wire whose name matches another stage's
entry remains local when it is not listed. Its ordinary consumers and type
determine its local meaning.

A direct local output may reuse the stage's own entry name when that name is
also a declared output. This is the only exception to the prohibition on
shadowing an inherited wire. Captures before that first local declaration,
including captures of the declaring block, resolve to the entry. The declaration
then introduces a new body-local wire: in `let go = |go| ...`, the input is the
current entry and the output is the new local producer. Later captures resolve
to that local wire, with no fallback to the entry on another branch. The local
producer retains its ordinary `mut` permission; exporting its value still
creates an immutable entry for the next visit.

The entry and local output are distinct producer identities, not alternative
producers of one merged wire. Multiple local producers of the new name still
obey the ordinary alternative-producer, merge, and source-order rules. If later
work needs the incoming value as well, give it another local name before the
first declaration that shadows it. A nested cycle may read the visible wire but
cannot shadow an outer entry itself.

Exporting the new `go` requests another visit, whose header receives that value.
This also permits mutual transitions between stages. Every continuing visit must
explicitly select its successor.

A **transition boundary** is the end of a completing preparation or stage route,
where the selected signal is exported to its destination. A signal may be
produced before later work on that route; that remaining work executes in source
order before the transition.

A **boundary consumer** is the implicit by-value use of a wire selected for
export. It is not an authored block. At a transition boundary it consumes the
selected signal and supplies the destination's entry. In a `const fn`, that
value's type must implement `Copy`. Boundary consumers participate in the
ordinary scope, producer, branch, and merge checks. A branch-local signal cannot
be used after a merge has closed its branch. Alternative internal producers
retain their ordinary type and mutability agreement; the receiving stage binds
the exported value immutably.

Preparation selects its initial destination from its direct outputs or named
flow inputs matching stage entries. Such a name designates a transition signal,
including when it carries data; it is not also a common outer data wire. Rust
checks agreement with the destination's other incoming values and, in a
`const fn`, its `Copy` bound. The initial signal must remain available by value
until preparation's transition boundary, just like an outgoing stage signal. For
a non-`Copy` `go`, an action with `|go| go.clone()` moves the original into its
capture alias before cloning, so the initial signal is no longer available.
Borrowing forms such as `|&go| go.len()` or `|&go| go.clone()` preserve it; the
action's result follows the ordinary output and usage rules.

Inside a stage, only its declared outputs request transitions. A wire inside a
nested cycle must first be exported by that cycle to the containing sequence,
then by the stage, to request a transition.

### 3.3 Merge and source order

The ordinary rule remains in force for data: every producer is authored before
every consumer of its wire. Shared data is established before the stage section;
local producers precede local consumers. A mutable capture changes a stored
value under Rust's rules while keeping its original producer identity.

Within preparation or one stage, same-named output producers must be mutually
exclusive and merge before any consumer. This includes signal outputs. All work
that the merge waits for must precede its consumers, and alternative producers
must agree on type and mutability. Branch adjacency and convergence nesting
retain their existing rules.

A signal connection between stages has execution order across visits. Its
producer completes in one visit, then the receiving stage begins a fresh visit.
Its header may appear before the producer in the source. Producers in different
stages belong to separate local scopes and meet the same entry on different
visits. This signal graph can contain cycles while every visit retains an
acyclic local data-dependency order.

## 4. Execution and completion

Preparation executes once and leaves its shared data alive for the stage
section. Its initial signal determines the first stage. Each visit executes the
selected stage's body according to its verified local source and branch order.

At every route that reaches the end of preparation or a nonterminal stage:

1. In preparation, collect participating direct outputs matching stage entries.
   Named flow inputs matching entries can also supply the initial signal. In a
   stage, collect participating local wires named by its output declaration.
2. Require exactly one signal. Zero is an unfinished route; two or more are
   ambiguous and invalid. Alternative producers of one signal follow §3.3.
3. Require that signal to be available to the boundary consumer, consume it
   after the route's remaining work, finish the visit's local scope, and enter
   the stage with the matching entry name.

The rule is checked separately for each feasible route. A stage can have several
possible exits, like a select, while each completed visit chooses one. Export
consumes the internal producer, and the destination header consumes the exported
signal; both count toward ordinary usage and branch participation requirements.
A signal emitted during preparation is used only for the initial entry. Every
later transition uses a freshly produced signal.

Every route of a nonterminal stage either exports one declared signal or
diverges in a nested cycle. A stage whose every route diverges declares no
outputs. Reaching the end of the body requires a successor signal, including for
a transition back to the same stage. Divergence is structural under §6.1; an
opaque Rust expression does not establish it.

### 4.1 Terminal stage

A flow with stages may have one terminal stage. It contains the flow's sole
structural `return` and declares no outputs. Every completing route through that
stage reaches that return. It may perform ordinary computations, selections,
merges, and cycles before returning. The return completes the enclosing
function, and Rust checks its value against the function's declared result type.

In a staged flow the structural return belongs to this terminal stage.
Completing preparation and nonterminal-stage routes hand control onward through
signals. The terminal stage may appear anywhere in the stage section's
declaration order. An empty stage is invalid. A flow that diverges through
transitions or cycles may omit the terminal stage.

A structural return remains invalid inside a cycle. A cycle completes by
exporting one of its declared outputs, after which the containing sequence
continues along that output's branch.

### 4.2 Ownership and lifetimes

Outer data keeps its surrounding scope across stage visits. Each inner block's
captures perform ordinary Rust moves, copies, or borrows where that block runs.
A mutable borrow normally ends with its borrowing block; a derived reference
retains the usual Rust lifetime and borrowing restrictions. A terminal stage can
consume an owned outer value when it returns.

A continuing stage must leave outer data usable by subsequent executions. Rust
checks ownership in the generated dispatcher loop, including repeated moves and
borrows. Persistent owned values are normally borrowed or updated through
mutable captures. Storing a value in outer state moves it into that state's
ownership; remaining stage-local values leave scope before the next stage
begins.

References stored in outer state must have referents that live long enough. A
reference to a departing stage's owned local cannot survive the transition.
Destructors follow the scope of their owners. Block-owned values drop at block
exit; remaining branch-local owners drop when that branch closes, including
before a merge's shared continuation. Remaining owners in the stage body's
outermost scope drop at visit completion. Surviving outer owners retain their
enclosing scope until completion or an explicit move or drop. Export moves or
copies the selected transition value into the next dispatch state before those
local scopes end. A moved owner belongs to the receiving visit and is not
dropped by the departing one. A reference can cross a transition boundary only
when its referent outlives the receiving visits; a `Copy` bound does not extend
a local owner's lifetime.

A selected signal in preparation or a stage must remain usable by value until
its transition boundary. Earlier work can borrow it under ordinary Rust rules,
but moving a non-`Copy` signal into an earlier capture makes the later export
invalid. The same rules apply to entry captures: receiving an owned value does
not implicitly clone it for each consumer. A self-transition may move the
incoming entry through its local producer into the next visit.

## 5. Unifying cycles and stages

### 5.1 Shared scope and interfaces

A cycle and a stage both contain a kaalang sequence with its own local wires.
Their headers control entry, their internal blocks capture data, and their
output declarations identify the local wires that may leave the body. The cycle
inherits existing wires; a stage also receives the value of the transition that
selected its current visit.

| Property                                  | Cycle                                                    | Stage                                                  |
| ----------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------ |
| Entry header                              | Empty or one plain signal                                | One plain signal                                       |
| Data available inside                     | Outer wires available at cycle entry, including its gate | Common outer wires and the current entry value         |
| Data captures                             | Explicit on each inner computational block or transfer   | Explicit on each inner computational block or transfer |
| Output declaration                        | Named alternative outputs of arbitrary Rust types        | Named alternatives (`Copy` in a `const fn`)            |
| Completing route                          | Exports one selected output to the containing sequence   | Exports one selected value to the destination stage    |
| Repeat                                    | The cycle's structural `continue`                        | An output targeting the stage's own entry              |
| End reached without an output or transfer | Invalid                                                  | Invalid                                                |

A cycle without a gate participates according to the containing sequence's
source and branch rules. A header `|start|` additionally gates that entry on the
signal; subsequent iterations are selected by `continue`. The header establishes
a capture dependency for producer usage and branch validation, but performs no
Rust move, copy, or borrow. Its gate may have any Rust type, including in a
`const fn`. The gate remains an ordinary outer wire available to explicit inner
captures. The header creates no separate data alias and does not remove that
wire from scope. Modified captures and multiple header names are invalid for
both constructs.

When a stage receives `go`, a nested cycle uses `|go|`, and a block in that
cycle captures `go`, all three refer to the same logical incoming wire of that
stage visit. Inner captures preserve its original identity rather than creating
a new cycle input wire. A later transition supplies the next stage visit's
entry.

A cycle inherits outer data visible at its declaration. Every outer wire used
inside must be available whenever that cycle is entered; inner captures do not
silently make cycle entry conditional. Their dependencies are recorded against
the original producers, including through nested cycles. They must respect
ordinary producer order and branch ancestry. The cycle's declared outputs are
provided only after it completes and are not outer inputs to its own body. Local
cycle producers cannot shadow visible outer wires, including the gate. Separate
cycle bodies may reuse their local names.

Borrowing, copying, and moving data happen at the inner block that captures it.
A cycle header does not create data aliases or hold data borrows across the
entire cycle. Create persistent local state explicitly before the cycle; nested
cycles can use state created earlier in their containing sequence. Those owners
retain their surrounding scope, while each iteration creates fresh body-local
wires. Uncaptured data reads inside a computational body remain invalid.

### 5.2 Alternative outputs

Cycles use the same alternative-output notation and name resolution as stages,
with ordinary `mut` permission allowed on cycle output bindings. One name
exposes the whole selected value. A tuple of names declares alternative exits,
in declaration order. A singleton `(found,)` is equivalent to `found`: neither
form destructures the selected value. Whenever the cycle completes, it provides
exactly one of them. Each declared output must have a reachable same-named local
producer. The cycle's **result boundary** is where its selected output leaves
the body and becomes available to the containing sequence. Its boundary consumer
consumes that local value after all remaining work on the route, under the
ordinary scope and merge rules. Alternative internal producers must agree on
type and mutability, while the declaration sets the exported binding's
mutability permission. Raw spellings identify the same logical name.

Cycle outputs may carry data, and different exits may have different Rust types.
To carry several values together, produce one tuple-valued wire and declare it
as one output. A later action can destructure that tuple. Cycle outputs retain
ordinary moves and can carry non-`Copy` owners even in a `const fn`; stage
outputs in a `const fn` require `Copy`.

In the containing sequence, the selected cycle output has the cycle as its
producer. Multiple cycle outputs are alternative branch outputs, with the same
first-consumer, branch ancestry, adjacency, and convergence rules as choice
outputs. A single output is an ordinary completed-cycle result. Exporting one
output does not provide any of its sibling outputs.

A completed cycle with no data to transfer declares a unit-valued output such as
`finished`. A cycle declaring no outputs can only repeat explicitly or diverge
inside a nested cycle. Producing a local wire whose name is not declared as an
output has no exit effect. A nested cycle exports only to its immediate
containing sequence; further export requires each enclosing output declaration.

Using RFC 0001's output pattern and block notation, replace its cycle and
structural-transfer alternatives with:

```text
cycle_statement :=
    "#[cycle(" block_description ")]"
    ("let" output_pattern "=")?
    ("|" (identifier ","?)? "|")? "{" block_statement* "}" ";"
block_statement :=
    action_statement | call_statement | question_statement | choice_statement
    | cycle_statement | continue_statement | return_statement
```

Omitting `let` or using `let ()` declares no outputs. A cycle without a gate may
omit its empty header, with or without outputs: `let found = { ... };` is
`let found = || { ... };`.

### 5.3 Explicit repetition with continue

A cycle may contain at most one structural `continue` belonging to it. Each
nested cycle has its own limit. All routes that repeat a cycle reach that same
transfer. Several repeating branches converge through ordinary wire merges
before it, giving the iteration one shared return route.

The transfer has an optional ordinary capture list and no value operand. The
`transfer_prefix` below is RFC 0001's `"|" input_list? "|"`:

```text
continue_statement := "continue" ";" | transfer_prefix continue_body ";"
continue_body := "continue" | "{" continue_body ";"? "}"
```

For example, `continue;`, `|| continue;`, and `|again| continue;` are valid
forms. Captures gate participation, create the ordinary aliases, and count as
consumers for wire usage and branch validation. `continue` targets the directly
containing cycle and ends the current route immediately. Later authored
statements may belong to other branches; participating work must precede the
transfer. An attributed or labeled structural continue, a continue outside a
cycle, and a continue carrying a value are invalid. Native Rust transfers within
nested Rust control-flow scopes retain RFC 0001's computational-body rules.

Validate every possible iteration route separately:

- Reaching the end of the body with exactly one available declared output
  exports it and completes the cycle.
- Reaching the structural `continue` with no declared output produced starts the
  next iteration.
- A route may instead diverge in a nested cycle.

A body ending with no output and no transfer is invalid. Producing several
outputs, or producing an output and then continuing on the same route, is also
invalid. An empty body therefore fails validation. An outputless cycle may do
ordinary work and then repeat with `continue;`; a body consisting only of that
transfer is the smallest example. Reachability validation rejects work that can
only follow a fully diverging cycle.

A bare `continue;` or `|| continue;` participates wherever its containing cycle
body is entered. It is valid only after any open selection has fully converged
and when no declared output has been produced on that route. Source position
does not attach it to one selected branch. A repeating branch uses a capture
such as `|again| continue;` to establish its ancestry. An earlier block may
already have captured the same unit branch output: `()` is `Copy`, so the later
capture can reuse that wire under ordinary branch and scope rules.

When an earlier action consumes a non-`Copy` branch output, it can produce a
unit wire for the continue: `let kept = |value, &mut out| out.push(value);`,
followed by `|kept| continue;`. The unit wire preserves that branch's ancestry
after `value` has moved. The continue needs that dependency, not a clone of the
consumed value.

An explicit continue lets the compiler distinguish intentional repetition from a
forgotten exit. Its uniqueness keeps convergence visible: the author supplies
the common repeat signal through the same merge rules used elsewhere.

### 5.4 Shared state and a unit exit

```rust
#[kaalang]
fn count_with_cycle(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut counter = || 0usize;

    #[cycle("Count to the limit.")]
    let finish = {
        #[question("Has the limit been reached?")]
        let (finish, again) = |counter, limit| counter >= limit;

        #[action("Increment the counter.")]
        |again, &mut counter| {
            *counter += 1;
        };

        |again| continue;
    };

    |finish, counter| return counter;
}
```

The cycle reads the surrounding `counter` and `limit` through its inner blocks.
On the repeating route, the mutation finishes before continue. On the other
route, the local `finish` wire becomes the cycle's declared unit output and the
flow returns the shared counter. Each visit obeys the same local ownership and
drop rules as a stage (§4.2).

### 5.5 Data exits and local ownership

```rust
#[kaalang]
fn pop_nonempty(mut items: Vec<String>) -> Option<String> {
    #[cycle("Find a nonempty item.")]
    let (found, exhausted) = {
        #[choice("Is another item available?")]
        #[case("Inspect the item.")]
        #[case("The collection is exhausted.")]
        let (item, exhausted) = |&mut items| match items.pop() {
            Some(item) => item,
            None => (),
        };

        #[question("Is the item empty?")]
        let (skip, keep) = |&item| item.is_empty();

        #[action("Provide the nonempty item.")]
        let found = |keep, item| item;

        |skip| continue;
    };

    #[action("Return the found item.")]
    let result = |found| Some(found);

    #[action("Report exhaustion.")]
    let result = |exhausted| None;

    |result| return result;
}
```

Only one of `found: String` and `exhausted: ()` exists after completion. Their
outer continuations produce alternative `result` wires that merge before return.
A selected owned value moves out through its declared output; remaining locals
drop at their ordinary scope exits. A reference may cross a cycle's result
boundary only when its owner outlives that export. Exporting a reference does
not extend a body-local owner's lifetime. The same-named inner and outer wires
remain separate producer occurrences connected by the cycle's result boundary.

## 6. Validation and lowering

### 6.1 Finite analysis

Resolve stage entry and declared output names, then split a staged root into
preparation and stages. Analyze each sequence using the ordinary local branch,
capture, merge, and source-order rules, extended with the cycle rules in §5.
Stage and cycle bodies receive their available outer data scope. Inner blocks
establish explicit capture dependencies to those original producers; a cycle's
entry gate and its derived outer-data dependencies have distinct roles.

Insert a boundary consumer for each declared output and each possible initial
signal from preparation. Validate its producer, scope, type, and merge order.
Preserve separately declared mutability for cycle output bindings. Stage output
bindings reject `mut`, and received stage entries are immutable. Validate every
cycle route as one selected output, an explicit continue, or nested divergence.
Check the single continue separately for each owning cycle. A finite cycle
summary exposes its alternative exits to the containing sequence, and keeps its
repeat outcome within the cycle. Preserve the selected exit's identity in outer
branch and convergence analysis.

Each stage summary ends at a declared transition boundary, the terminal return,
or a repeating cycle outcome. The destination's body belongs to another visit.
Construct the directed stage graph from those transition boundaries and compute
reachability from the preparation's initial entries. Reject unreachable stages
and unreachable executable blocks.

Validate one entry per stage, one selected signal per completing nonterminal
route, the terminal-stage rules, and producer usage across the reachable model.
The header's position imposes diagram order; signal connections determine
inter-stage reachability. Reject `&mut` captures of a stage's received entry,
including through nested cycles. Rust checks type agreement of all values
targeting each stage, data operations and captures in generated scopes, and
`Copy` bounds on stage entry signals in a `const fn`.

As in RFC 0001, computational Rust bodies are opaque. Validation considers all
question answers and choice cases possible and does not prove termination or
correlations between visits. Finite local summaries and the stage graph describe
arbitrarily many transitions without unfolding them. Every reachable part must
have a conforming local diagram.

### 6.2 Rust lowering

Keep the parameter prologue and preparation's shared wire storage in the scope
surrounding the dispatcher. Generate an enum with one variant and one type
parameter per stage. Each variant carries its entry value. In a `const fn`,
every parameter has a `core::marker::Copy` bound; ordinary functions need no
such bound. Rust infers those types from the incoming values; all constructors
for one destination must agree on its type. The macro need not name the concrete
or enclosing generic types in the enum declaration.

A mutable state binding records the selected stage and its value. Each match arm
binds that entry immutably and executes the stage's local plan. Captures of that
entry, including through nested cycles, resolve to the same visit-local storage.
A self-transition output receives a separate hygienic binding, preserving §3.2's
distinction from the incoming wire. Common outer wires remain available through
ordinary generated capture aliases.

At a completed transition boundary, construct the destination variant with the
selected value and finish the current arm's local scopes. Its constructor checks
the destination type and, in a `const fn`, the value's `Copy` bound. The
constructor and its value argument retain the source location of the authored
output binding, or the function parameter when it supplies the initial signal,
while generated identifiers retain their hygiene. Type and bound diagnostics
must identify that authored producer, following RFC 0004 §2's span policy. The
destination is known from the resolved output name; choosing it requires no
extra join label. Ordinary merges of alternative producers still use RFC 0004's
labeled blocks and shared continuations. The next dispatch iteration enters the
selected stage. The terminal arm returns directly from the enclosing function. A
wholly diverging arm remains in its nested cycle and produces no next state.
When a transition boundary continues the dispatcher from inside generated
labeled blocks, its native `continue` names the hygienic dispatch-loop label.
Authored structural continue still targets its own cycle, as §6.3 defines.

For example, the counting flow in §8.1 has this illustrative lowering. As in RFC
0004, hygiene, parameter withdrawal, and lint details are omitted; the mutable
capture is shown explicitly:

```rust
fn count_to(limit: usize) -> usize {
    enum Stage<C, F> {
        Count(C),
        Finish(F),
    }

    let mut wire_counter = 0usize;
    let mut stage = Stage::Count(());
    loop {
        stage = match stage {
            Stage::Count(()) => {
                if wire_counter >= limit {
                    let wire_finish = ();
                    Stage::Finish(wire_finish)
                } else {
                    let wire_count = {
                        let counter = &mut wire_counter;
                        *counter += 1;
                    };
                    Stage::Count(wire_count)
                }
            }
            Stage::Finish(()) => return wire_counter,
        };
    }
}
```

For a `const fn`, the lowering adds `Copy` bounds to the enum's parameters. Here
two alternative producers merge into one outgoing `finish` value. The ordinary
merge closes their branches before constructing the next state:

```rust
const fn select_value<T: Copy>(take_left: bool, left: T, right: T) -> T {
    enum Stage<C: Copy, F: Copy> {
        Choose(C),
        Finish(F),
    }

    let mut stage = Stage::Choose(());
    loop {
        stage = match stage {
            Stage::Choose(()) => {
                let wire_finish = 'merge_finish: {
                    if take_left {
                        break 'merge_finish left;
                    } else {
                        break 'merge_finish right;
                    }
                };
                Stage::Finish(wire_finish)
            }
            Stage::Finish(finish) => return finish,
        };
    }
}
```

In a `const fn`, `Copy` excludes destructors from the generated state, including
for inferred generic payloads. This deliberately rejects even concrete
non-`Copy` transition types without destructors. The bound applies only to
transmitted entry values: common outer data may have arbitrary Rust types.
Ordinary functions use the same dispatcher with unbounded enum parameters;
matching the current variant moves its owned entry into the visit, and the next
variant takes ownership of its outgoing value. Authored operations and escaping
borrows retain ordinary Rust checks, including constant-evaluation restrictions
in a `const fn`.

Preserve each block's capture scope and all remaining route work before updating
the state. Keep storage accessible only through generated capture aliases as RFC
0004 requires. Ordinary branch lowering and the cycle lowering below apply
inside each arm, including merges of alternative outgoing producers. Shared
continuations are emitted once.

The dispatcher preserves the enclosing generic and receiver context. Its stack
usage is independent of the number of transitions. It stores one active variant
and its entry value; authored outer storage and computations retain their own
memory requirements.

Correctness follows one visit at a time: preparation establishes the common data
and first entry; the local plan preserves each stage's effects; the selected
output supplies both the next arm and its entry value; and that arm observes the
surviving outer data with fresh local wires. These steps preserve both
completing and indefinitely repeating executions. The diagram uses the same
local plans and transition graph.

### 6.3 Cycle lowering

Lower a cycle to a native Rust loop with a hygienic label. Its optional entry
gate selects entry to the whole loop once through the verified control
dependencies; the header emits no value operation or trait bound. Keep that gate
and other outer data in their original storage; inner capture aliases access
those wires directly when their blocks execute. Each native iteration opens
fresh local scopes.

At a verified result boundary, move the selected value out with a generated Rust
`break` after all remaining work on that route. One declared output can be the
loop's value directly. For alternative outputs, use the nested labeled blocks
from RFC 0004 §4: one result block per output surrounds the loop, and each
result boundary breaks to its own label with the selected value. That break
leaves the cycle's local scopes before the corresponding outer continuation
runs. Each result binding has its independently inferred Rust type.

Place the cycle body once inside the innermost result block. Each output's
continuation follows its result binding and reaches the appropriate ordinary
merge or structural transfer. Reuse RFC 0004's coercion contexts and merge
lowering, emitting each shared continuation once. Authored choices keep their
ordinary match lowering. All generated labels are hygienic.

For example, a cycle can repeat before selecting either of two exits that carry
the same owned value into a shared continuation. Its illustrative lowering
preserves an unconstrained generic `T` in a `const fn`; capture aliases and
parameter withdrawal are omitted as in §6.2:

```rust
const fn route_value<T>(value: T, mut remaining: usize, take_left: bool) -> T {
    let result = 'merge: {
        let wire_right = 'exit_right: {
            let wire_left = 'exit_left: {
                'cycle: loop {
                    if remaining > 0 {
                        remaining -= 1;
                        continue 'cycle;
                    }
                    if take_left {
                        break 'exit_left value;
                    } else {
                        break 'exit_right value;
                    }
                }
            };
            break 'merge wire_left;
        };
        wire_right
    };
    result
}
```

Passing each value directly into its result binding preserves ordinary Rust
ownership and constant evaluation. Remaining unexported owners drop on leaving
their scopes before the outer continuation, and a reference cannot escape an
owner dropped by that transfer.

A structural continue emits its capture aliases and a native `continue` to the
directly containing cycle's generated label. Nesting uses distinct labels.
Validation supplies an explicit outcome for every route; lowering generates no
repeat from an unfinished body ending. Returning cycles, diverging cycles, and
partial convergence retain the same finite local plans in execution and drawing.
One iteration either transfers its chosen value outward, discards remaining
locals and repeats, or stays in a nested divergent execution. This preserves the
completion rules without unfolding repeated iterations.

## 7. Visual representation

A **part** is the diagram area assigned to preparation or one stage, with its
own horizontal range. Preparation occupies the leftmost part and starts at the
existing flow start node. Stages occupy separate parts to its right in
declaration order. Their entry nodes align on a common row one row below start.
The parameter panel stays beside start on its row. Each part has its own
horizontal range, sized for its local arrangement, labels, data panel, and cycle
boundaries.

### 7.1 Stage entry

A stage begins with the same shape as a case node in RFC 0002: a rectangular
text body ending in a lower triangular point. Reuse the case outline, text
placement, and sizing rules. The downward control connection leaves the lower
apex and enters the stage body.

The stage's displayed name is its authored description, rendered using
[RFC 0005](0005-markdown.md). It appears inside the entry node. The entry signal
is shown separately as a literal caption beside the node; its authored spelling
comes from the stage header. This caption identifies the stage when two stages
have the same description.

A **data panel** lists the common outer wires used by a stage. It is
non-executable and sits to the right of the entry, vertically centered beside
it. Reuse the rectangular shape and one-item-per-row presentation of RFC 0002
§4.1's parameter panel. List each used common outer wire once in order of first
capture in the authored body, including nested cycles. Labels are literal wire
names with their producer's `mut` permission, not capture modifiers or inferred
types. The entry value has its own signal caption and is not repeated in the
panel. Omit an empty panel. Reserve space so it overlaps no node, caption,
connection, or other panel; it adds no control connection.

The model records each listed wire's original producer in preparation or the
function inputs. The entry represents those available sources and its received
signal for local dependency routing. Actual capture forms remain labeled at
their computational consumers. As in RFC 0002, data travels virtually along the
reduced control connections; the panel adds no separate routes to consumers.

### 7.2 Transition node

Each transition boundary ends its selected route with a **transition node**
after all work on that route. The node is last on its route and occupies a row
below every other vertex on that route. Transition nodes on alternative routes
may share a row under RFC 0002 §8.

The transition node mirrors the stage entry vertically: a rectangular text body
with an upper triangular point. The incoming control connection meets that upper
apex. Execution continues at the target stage's entry through the symbolic link
recorded by the model.

Inside the transition node, repeat the destination stage's displayed name
exactly, with the same description rendering. Its separate literal caption uses
the destination header's entry signal. Both labels come from that resolved
header, so changing a stage description updates its entry and all transitions
targeting it together.

A transition node is a synthetic projection of the transition boundary, like a
merge or end node. Alternative local producers merge before a shared boundary
consumer under the ordinary rules. Distinct terminal routes remain distinct
unless local convergence already joins them. Preparation ends its continuing
routes with these same transition nodes.

### 7.3 Backward and self-transition markers

Number the stages in declaration order, from left to right. A transition from
stage `i` to stage `j` is **backward** when `j < i` and a **self-transition**
when `j == i`. Both receive a **back-transition marker**. A transition to a
later stage is forward. Preparation precedes all stages, so its initial
transitions are forward.

The marker is a small solid triangular cap at the node's apex, filled with the
outline color. Its sloping sides follow the surrounding triangular point, and
its horizontal base lies halfway from the apex to the base of that point. This
keeps the marker inside the outline and clear of the text. It remains visible in
monochrome diagrams.

Mark the two ends of every backward or self-transition:

- Fill the upper tip of its transition node.
- Fill the lower tip of its destination stage's entry node.

An entry receives one marker if at least one backward or self-transition targets
it. Each outgoing transition is classified independently: a forward transition
to a marked entry retains an unmarked tip. The classification depends on stage
declaration order, including transitions to a terminal stage placed earlier in
the section. It is the same in expanded and collapsed cycle views.

For example, with stages `choose`, `partition`, and `finish` in that order:

| Transition             | Transition tip | Destination entry tip                           |
| ---------------------- | -------------- | ----------------------------------------------- |
| Preparation → `choose` | Unmarked       | Marked because of the backward transition below |
| `choose` → `partition` | Unmarked       | Unmarked                                        |
| `partition` → `choose` | Marked         | Marked                                          |
| `choose` → `finish`    | Unmarked       | Unmarked                                        |

A self-transition marks both its own outgoing node and its stage's entry.

### 7.4 Composition and completion

The entry signal links each transition node to its unique destination. Data
panels identify the listed wires' original producers by wire identity. The model
records that provenance and the derived back-transition markers. Within a part,
branch order, wire routing, and merges follow the ordinary visual language;
cycles use the projection in §7.5.

Check and arrange preparation and stages locally, then compose their verified
arrangements in declaration order. Inter-part links establish reachability and
data provenance; local arrangements supply the drawn routes. The shared model
owns these decisions, and rendering consumes the same stage graph as lowering.

There is one end node in the terminal stage, below all other vertices as RFC
0002 requires. That stage keeps its authored horizontal position. A fully
diverging flow has no end node.

### 7.5 Cycles and continue

An expanded cycle retains its bounded region, entry junction, loop marker, and
ordinary local routing. Its entry represents the available outer data for local
dependency routing, with original provenance retained in the model. Those
connections follow the existing control routes through the entry. Its unique
structural continue supplies the iteration tail and the back edge to that entry.
Continue has no computational figure; its captures and any ordinary merges lead
to that common tail. A cycle with no reachable continue has no back edge. An
explicit `continue;` can connect entry to tail directly.

Each declared cycle output has a distinct result exit at the drawn cycle
boundary. All local producers of that output merge under the ordinary rules
before export. Result exits preserve declaration order from left to right and
retain their selected branch identity in the containing sequence. They hand over
only the selected wire. Different output exits converge only through ordinary
merges outside the cycle. A nested cycle's results first reach its immediate
containing sequence.

A collapsed cycle retains its described cycle node and loop marker. It has one
branch-specific exit per declared output, in declaration order: the first leaves
the lower edge, and later exits fan out to the right using the existing ordered
branch routing. Each exit is labeled with its output binding. A fully diverging
cycle has no outgoing exit. Its receiving label lists the external wires used by
the cycle: begin with the authored gate, when present, then walk the body in
source order, including nested cycles, and append each captured outer wire on
its first occurrence. Resolve and deduplicate by wire identity. Exclude wires
produced inside this cycle, even when a nested cycle captures them. A gate also
captured inside the body appears only once.

This derived list is the collapsed node's input label, in the ordinary receiving
label position; it needs no separate data panel. Show literal names with their
producer's `mut` permission and no inferred types or aggregate borrow modifiers.
Show `()` when the list is empty. Actual inner capture forms remain visible in
the expanded view. The list records dependencies, not borrows held for the whole
cycle. An expanded cycle retains its unlabeled structural entry and does not
repeat the full input list on its boundary.

Both projections preserve the same selected outputs and surrounding branch
order. Check all result routes and the continue back edge in the expanded view,
then verify the collapsed projection. An arrangement must respect the existing
noncrossing and cycle-boundary rules, including the positions of all alternative
result exits. Derived inputs follow the existing reduced control routes through
the cycle entry; they do not introduce paths that bypass preceding work.

## 8. Examples

These definitions use the proposed syntax. The current compiler does not yet
implement stages or the unified cycle contract. The `kaalang` attribute is
assumed to be in scope in all examples.

### 8.1 Shared counter and a self-transition

```rust
#[kaalang]
fn count_to(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut counter = || 0usize;

    #[action("Begin counting.")]
    let count = || ();

    #[stage("Count to the limit.")]
    let (count, finish) = |count| {
        #[question("Has the limit been reached?")]
        let (finish, again) = |counter, limit| counter >= limit;

        #[action("Increment and count again.")]
        let count = |again, &mut counter| {
            *counter += 1;
        };
    };

    #[stage("Return the count.")]
    |finish| {
        |counter| return counter;
    };
}
```

`counter` is produced once during preparation. Each visit reads and may mutate
that same wire. The `count` output chooses another visit, while `finish` selects
the terminal stage. The header's signal activates all blocks participating in
that visit; each inner block captures its own data and branch dependencies.

### 8.2 Mutual transitions and a terminal stage in the middle

```rust
#[kaalang]
fn is_even(mut remaining: usize) -> bool {
    #[action("Prepare the result and start at even parity.")]
    let (mut result, even) = || (false, ());

    #[stage("An even number of steps has been taken.")]
    let (finish, odd) = |even| {
        #[question("Have all steps been taken?")]
        let (done, again) = |remaining| remaining == 0;

        #[action("Record an even result.")]
        let finish = |done, &mut result| {
            *result = true;
        };

        #[action("Take one step.")]
        let odd = |again, &mut remaining| {
            *remaining -= 1;
        };
    };

    #[stage("Return the parity.")]
    |finish| {
        |result| return result;
    };

    #[stage("An odd number of steps has been taken.")]
    let (finish, even) = |odd| {
        #[question("Have all steps been taken?")]
        let (done, again) = |remaining| remaining == 0;

        #[action("Record an odd result.")]
        let finish = |done, &mut result| {
            *result = false;
        };

        #[action("Take one step.")]
        let even = |again, &mut remaining| {
            *remaining -= 1;
        };
    };
}
```

The `remaining` and `result` wires outlive the stage visits. Each producing
stage selects `finish` on its own completing route. The terminal stage stays
between `even` and `odd` in the diagram.

### 8.3 A cycle inside a stage

```rust
#[kaalang]
fn drain(mut input: Vec<u8>) -> usize {
    #[action("Begin draining.")]
    let draining = || ();

    #[stage("Remove every item.")]
    let finish = |draining| {
        #[cycle("Drain the collection.")]
        let finish = {
            #[question("Is the collection empty?")]
            let (finish, occupied) = |&input| input.is_empty();

            #[action("Remove one item.")]
            |occupied, &mut input| {
                input.pop();
            };

            |occupied| continue;
        };
    };

    #[stage("Return the remaining length.")]
    |finish| {
        #[action("Read the length.")]
        let length = |&input| input.len();

        |length| return length;
    };
}
```

The cycle's inner blocks capture the surrounding `input`; the cycle itself binds
no data. The occupied route explicitly continues after removing an item. The
selected unit output is exported as `finish` first by the cycle and then by the
stage. Borrowing is local to the actual borrowing blocks, and the terminal stage
can read the same shared collection.

### 8.4 Divergence through transitions

```rust
#[kaalang]
fn spin() -> ! {
    #[action("Begin repeating.")]
    let repeat = || ();

    #[stage("Repeat indefinitely.")]
    let repeat = |repeat| {
        #[action("Select another visit.")]
        let repeat = || ();
    };
}
```

Each visit produces a fresh signal to select the next visit. The stage has one
exit and the flow is fully diverging.

A stage can also diverge entirely inside a nested cycle and declare no outputs:

```rust
#[kaalang]
fn wait_forever(go: ()) -> ! {
    #[stage("Wait indefinitely.")]
    |go| {
        #[cycle("Keep waiting.")]
        {
            continue;
        };
    };
}
```

The parameter supplies the initial signal. This stage has neither a transition
node nor an end node; its nested cycle supplies the repeating route.

### 8.5 Quicksort

Preparation creates a shared stack of pending ranges. The stages select a range,
send it to the partition stage, and eventually return the sorted values. Both
pieces of work after a partition are recorded in the pending stack.

```rust
use core::cmp::Ordering;

#[kaalang]
fn quick_sort<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    #[action("Prepare the pending ranges.")]
    let mut pending = |&values| {
        let len = values.len();
        if len > 1 {
            vec![(0, len)]
        } else {
            Vec::new()
        }
    };

    #[action("Begin selecting ranges.")]
    let next_range = || ();

    #[stage("Choose the next range.")]
    let (partition, sorted) = |next_range| {
        #[choice("Is another range waiting?")]
        #[case("Partition the next range.")]
        #[case("All ranges are sorted.")]
        let (partition, sorted) = |&mut pending| match pending.pop() {
            Some(range) => range,
            None => (),
        };
    };

    #[stage("Partition the range around its pivot.")]
    let next_range = |partition| {
        #[action("Move the middle value to the pivot position at the end.")]
        let (start, end, pivot) = |partition, &mut values| {
            let (start, end) = partition;
            let pivot = end - 1;
            values.swap(start + (end - start) / 2, pivot);
            (start, end, pivot)
        };

        #[action("Start the lower, equal, and upper regions.")]
        let (mut lower, mut cursor, mut upper) = |start, pivot| (start, start, pivot);

        #[cycle("Scan the range.")]
        let scanned = {
            #[question("Has the cursor reached the upper region?")]
            let (scanned, more) = |cursor, upper| cursor >= upper;

            #[choice("How does the current value compare with the pivot?")]
            #[case("Less: move it to the lower region.")]
            #[case("Equal: leave it in place.")]
            #[case("Greater: move it to the upper region.")]
            let (less, equal, greater) =
                |more, &values, cursor, pivot| match values[cursor].cmp(&values[pivot]) {
                    Ordering::Less => (),
                    Ordering::Equal => (),
                    Ordering::Greater => (),
                };

            #[action("Swap it into the lower region and advance both bounds.")]
            let stepped = |less, &mut values, &mut lower, &mut cursor| {
                values.swap(*cursor, *lower);
                *lower += 1;
                *cursor += 1;
            };

            #[action("Advance the cursor.")]
            let stepped = |equal, &mut cursor| {
                *cursor += 1;
            };

            #[action("Extend the upper region and swap the value into it.")]
            let stepped = |greater, &mut values, cursor, &mut upper| {
                *upper -= 1;
                values.swap(cursor, *upper);
            };

            |stepped| continue;
        };

        #[action("Place the pivot after the equal region.")]
        |scanned, &mut values, upper, pivot| {
            values.swap(upper, pivot);
        };

        #[action("Schedule both sides, larger first.")]
        let next_range = |&mut pending, start, end, lower, upper| {
            let (mut first, mut second) = ((start, lower), (upper + 1, end));
            if first.1 - first.0 < second.1 - second.0 {
                core::mem::swap(&mut first, &mut second);
            }
            for range in [first, second] {
                if range.1 - range.0 > 1 {
                    pending.push(range);
                }
            }
        };
    };

    #[stage("Return the sorted values.")]
    |sorted| {
        |values| return values;
    };
}
```

`values` and `pending` remain in the common outer scope. The `partition` signal
carries the selected `(usize, usize)` range into the stage. Pivot selection,
region initialization, scanning, pivot placement, and scheduling appear as
separate steps. The `next_range` and `sorted` signals carry unit.

The stage creates `lower`, `cursor`, and `upper` before entering the cycle. Its
inner actions update those same wires through mutable captures. The three
alternative `stepped` producers merge before the single `continue`; the
`scanned` route exports the cycle's result instead. Repetition follows the right
branch of the cycle's first question, so RFC 0002 §8 prefers the right contour
for the back edge. After the cycle, source order places the pivot before
scheduling the remaining ranges.

Ranges are half-open and contain at least two elements when partitioned. The
pivot stays at `end - 1` during the scan. The intervals `[start, lower)`,
`[lower, cursor)`, and `[upper, pivot)` hold values less than, equal to, and
greater than the pivot; `[cursor, upper)` remains unclassified. A greater value
extends the upper region without advancing the cursor, so the swapped-in value
is examined next. Only the unequal regions need further sorting. Larger ranges
are pushed first so the smaller range is processed next. Values stay in their
original allocation, and `T` needs neither `Copy` nor `Clone`. The algorithm is
not stable.

### 8.6 One incoming wire through a nested cycle

```rust
#[kaalang]
fn forward<T>(go: T) -> T {
    #[stage("Forward the incoming value.")]
    let finish = |go| {
        #[cycle("Complete on the first iteration.")]
        let finish = |go| {
            #[action("Provide the incoming value.")]
            let finish = |go| go;
        };
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}
```

The flow input selects the first stage. That visit's `go` is the same wire used
by the cycle header and its action. The cycle header creates no replacement
input binding and performs no move. The action moves or copies `go` into
`finish`, which crosses the cycle's result boundary and then the stage's
transition boundary before the terminal stage returns it. Both stage entries
carry the unconstrained type `T`; this ordinary function can forward a `String`
or another non-`Copy` owner. Declaring this staged function `const` would
require `T: Copy` for its stage entries; the cycle's gate itself adds no bound.

### 8.7 A self-transition with a new entry value

```rust
#[kaalang]
const fn count_down(count: usize) -> usize {
    #[stage("Count down to zero.")]
    let (count, finish) = |count| {
        #[choice("Is another step needed?")]
        #[case("Take the next step.")]
        #[case("The count is zero.")]
        let (count, finish) = |count| match count {
            value if value > 0 => value - 1,
            _ => 0usize,
        };
    };

    #[stage("Return zero.")]
    |finish| {
        |finish| return finish;
    };
}
```

The choice captures the incoming `count` before declaring its new local output
of that name. The selected output either supplies the next visit's smaller count
or enters `finish`. The incoming and outgoing `count` occurrences do not merge
within one visit.

## 9. Changes to earlier RFCs

If accepted, this RFC supersedes the provisions below. Stage rules apply to
flows declaring stages; the unified cycle rules apply to every cycle, including
those in flows without stages. Earlier accepted RFC texts remain unchanged.

- **RFC 0001 §§2–4 and §8:** introduce stage declarations after preparation. A
  stage has one entry and declared alternative outputs, which require `Copy`
  only in a `const fn`. A terminal stage contains the flow's sole return and
  declares no outputs; a wholly diverging stage also declares none. Entry values
  are visible to inner captures as immutable wires, and stage output bindings
  reject `mut`. A declared local self-transition output may shadow only its own
  stage entry, with the distinct producer identities and resolution order of
  §3.2.
- **RFC 0001 §§4.5–4.6 and §8:** replace the cycle's complete data-capture
  header with an optional single gate and inherited outer data scope. The gate
  imposes no trait bound or Rust value operation and remains available to inner
  captures as the original outer wire. Named outputs are alternative exits
  matched to same-named internal wires. Explicit result boundaries replace
  structural `break`. Each cycle owns at most one structural `continue`, and
  every repeating route must reach it; reaching the body end without an output
  or transfer is invalid.
- **RFC 0001 §3 and §8:** a `let` block without inputs may omit its empty
  capture list. For every block kind, `let output = body;` whose `body` is not a
  closure is `let output = || body;`, and that kind's rules then apply to the
  body. An action may write `let count = 0;` and a cycle `let found = { ... };`.
  An initializer that is itself a closure is always read as the capture list, so
  a closure-valued action still writes `let f = || |x| x + 1;`. A statement
  without `let` keeps RFC 0001 §3's forms. Replace the action alternative with:

  ```text
  action_statement :=
      "#[action(" block_description ")]"
      ("let" output_pattern "=" ("|" input_list? "|")? rust_expression
       | "|" input_list? "|" rust_expression
       | "{" rust_statement* "}") ";"
  ```

- **RFC 0001 §§4.7, 5–7:** place a staged flow's return in its terminal stage.
  Both stage and cycle bodies access outer data through inner block captures and
  create fresh local scopes per visit or iteration. Multiple cycle outputs
  participate as alternative branches. Ordinary data producer order, ownership,
  scope, local merges, and convergence restrictions remain. Stage signal links
  can cross declaration order and form cycles between visits.
- **RFC 0002 §§3–8:** use case-shaped stage entries and mirrored transition
  nodes with destination labels and backward/self markers. Data panels record
  provenance while existing control connections carry the dependency routes. A
  terminal stage keeps its declaration position and its end stays below all
  other vertices. Cycles have one result exit per declared alternative output;
  their continue supplies the single iteration tail. Collapsed cycles retain
  those alternative exits. In **§4.8 and §6**, replace the collapsed cycle's
  authored capture list with the derived external-wire input list defined by
  §7.5 here, including its gate and transitive inner uses. That list displays
  producer mutability rather than per-block capture forms. The expanded cycle
  retains the unlabeled structural entry and the boundary without a repeated
  full input list; its alternative result interfaces replace the former
  single-result interface. Stage data panels follow §7.1 here.
- **RFC 0003 §§1–2:** compose locally verified stage arrangements and preserve
  symbolic stage links. Expanded and collapsed cycle checks include alternative
  result exits and explicit continue routes in the same validated model.
- **RFC 0004 §§1–2 and §5:** retain shared data around the generated stage
  dispatcher, carry each selected entry in its enum variant, and emit the
  terminal return directly from its arm. Each arm binds its received entry
  immutably. Variant fields use inferred generic types with `Copy` bounds only
  in `const` functions. Ordinary functions transfer owned values without those
  bounds. Constructor checks preserve the authored producer's source location
  for diagnostics.
- **RFC 0004 §7:** remove persistent cycle-header data aliases and implicit
  repetition at body endings. Lower inner data captures against their original
  storage. Explicit continue targets the nearest generated cycle label.
  Alternative outputs use the labeled result blocks from RFC 0004 §4, leaving
  the loop with the selected value before entering its matching outer
  continuation.

For cycle migration, create any formerly captured owned working state before the
cycle and keep data captures on its inner blocks. Replace the structural break's
value with a same-named local producer for a declared output. A unit completion
needs a named unit output. Replace each implicit repeating route with a path to
the cycle's common continue, merging its gate wires as needed. If a repeating
branch has consumed its non-`Copy` wire, use the consuming action's unit output
to gate the continue as in §5.3. A former tuple of simultaneous results becomes
one tuple-valued output followed by ordinary destructuring outside the cycle.
The selected output is exported only after its route's remaining work, so the
migrated branch structure must make that work and the continue mutually
exclusive.

Flows containing neither stages nor cycles retain their existing contract.
Concrete compiler and renderer types remain implementation choices.

## 10. Acceptance scenarios

These scenarios define required language behavior and visual representation.

| Scenario                                                                         | Required result                                                                                                                           |
| -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Preparation followed by stage declarations                                       | Run preparation once and select the first stage through its signal.                                                                       |
| Ordinary root statement after the first stage                                    | Reject the interleaved preparation block.                                                                                                 |
| Nested stage declaration                                                         | Reject it; declarations belong to the root stage section.                                                                                 |
| Empty, multiple, or modified stage header captures                               | Reject; a stage header has one plain entry identifier.                                                                                    |
| Duplicate stage entries, including raw spellings                                 | Reject the ambiguous destination.                                                                                                         |
| Stage output without a matching entry                                            | Reject the unresolved destination.                                                                                                        |
| Duplicate declared outputs or a declared output with no reachable local producer | Reject the invalid output interface.                                                                                                      |
| Internal wire named like another stage but absent from declared outputs          | Keep it local, with its ordinary type and usage rules.                                                                                    |
| Non-Copy stage transition in an ordinary or const function                       | In an ordinary function, transfer ownership under normal Rust rules; in a const function, reject even a concrete type with no destructor. |
| Const function called at runtime                                                 | Keep the Copy requirement on its stage entries; the function declaration determines the bound.                                            |
| Owned stage entry forwarded through a self-transition                            | Move it through the local output into the next visit without cloning or dropping the transferred owner.                                   |
| Non-Copy initial or outgoing stage signal moved before its boundary consumer     | Reject the export as a use after move; borrowing before export follows ordinary Rust rules.                                               |
| Preparation action clones a non-Copy initial signal                              | A value capture moves the original before cloning and prevents export; a shared capture preserves the initial signal.                     |
| Stage transition fails its type or const Copy check                              | Locate the diagnostic at the authored outgoing binding or initial parameter, preserving its source span.                                  |
| Several declared exits on alternative stage routes                               | Each completed visit selects exactly one.                                                                                                 |
| Zero or simultaneous outputs on a completing stage route                         | Reject the unfinished or ambiguous route.                                                                                                 |
| Remaining work after an exported wire is produced                                | Finish the selected route before exporting its output.                                                                                    |
| Branch-local exit wire consumed after a merge closes its branch                  | Reject under ordinary scope and merge ordering rules.                                                                                     |
| Self-transition and mutual transitions                                           | Reuse outer data and create fresh local scopes with bounded dispatch stack usage.                                                         |
| Inner stage block captures a preparation wire                                    | Resolve it directly to the common outer producer.                                                                                         |
| Inner computational body reads an uncaptured wire                                | Reject according to ordinary capture rules.                                                                                               |
| Branch-local preparation data missing at entry to the stage section              | Require an ordinary merge before stage consumers.                                                                                         |
| Capture of a wire local to another stage                                         | Reject the unavailable wire.                                                                                                              |
| Repeated local producers in a stage or cycle iteration                           | Require mutual exclusion, merge completion, and producer order before consumers.                                                          |
| Mutable outer state and borrowing blocks                                         | Later visits observe updates; data borrows begin at their actual capturing blocks.                                                        |
| Owned outer value consumed by the terminal return                                | Move it into the result under ordinary Rust ownership rules.                                                                              |
| Repeated move of owned outer state on a continuing route                         | Let Rust reject invalid repeated use in the generated loop.                                                                               |
| Reference to a departing local stored in shared state                            | Reject the escaping borrow.                                                                                                               |
| Branch-local and body-local owners with destructors                              | Drop branch locals before the merge continuation and remaining body locals when the visit ends.                                           |
| Methods, generics, and otherwise valid const functions                           | Preserve the Rust context and ordinary receiver capture rules.                                                                            |
| Generic const function with alternative cycle outputs of an owned type           | Compile and evaluate constant instances through labeled result blocks, without extra trait bounds.                                        |
| Cycle with an empty header or one entry signal                                   | Enter by control order or the named gate; the header performs no move, copy, borrow, or trait check.                                      |
| Invalid cycle header captures                                                    | Reject multiple or modified header captures; inner data access remains explicit.                                                          |
| Non-Copy cycle gate in an ordinary or const function                             | Accept the gate and let inner captures determine moves and borrows of its original storage.                                               |
| Nested cycle reads surrounding data through an inner capture                     | Resolve the original producer without repeated header data lists.                                                                         |
| Outer data unavailable on one route entering a cycle                             | Reject; inner captures cannot silently make the cycle entry conditional.                                                                  |
| Cycle with one selected unit output                                              | Complete and provide that named unit wire to the containing sequence.                                                                     |
| Cycle with alternative outputs of different Rust types                           | Provide only the selected output and apply ordinary alternative-branch consumer rules.                                                    |
| Cycle declares one tuple-valued output                                           | Transfer that tuple intact; a later action may destructure it.                                                                            |
| Exported cycle binding declared mutable                                          | Permit later mutable captures according to the outer declaration.                                                                         |
| Nested output export                                                             | Pass through each enclosing declaration before becoming a stage transition.                                                               |
| Unit signal remains inside a cycle without export                                | Keep it local; it does not select a stage.                                                                                                |
| Several repeating branches and one continue                                      | Require their ordinary convergence before the shared transfer and draw one back edge.                                                     |
| Missing continue on a route with no declared output                              | Reject the forgotten iteration ending.                                                                                                    |
| More than one structural continue in one cycle                                   | Reject, counting nested cycles separately.                                                                                                |
| Declared output produced on a route reaching continue                            | Reject the conflicting completion and repetition.                                                                                         |
| Multiple cycle outputs produced on one route                                     | Reject the ambiguous result.                                                                                                              |
| Labeled, attributed, value-carrying, or out-of-cycle structural continue         | Reject the invalid transfer.                                                                                                              |
| Nested continue                                                                  | Repeat only the directly containing cycle.                                                                                                |
| Structural break in a cycle body                                                 | Reject; completion is expressed by a declared output.                                                                                     |
| Empty cycle body                                                                 | Reject the unfinished iteration.                                                                                                          |
| Outputless cycle containing an unconditional continue                            | Accept explicit divergence and reject work reachable only after it.                                                                       |
| Owned cycle-local value selected for export                                      | Move that value out and drop unexported locals at their ordinary scope exits.                                                             |
| Cycle output borrows a departing body-local owner                                | Let Rust reject the escaping reference.                                                                                                   |
| Cycles in a flow without stages                                                  | Apply the same unified scope, output, continue, and diagram rules.                                                                        |
| Terminal stage in the middle of the declarations                                 | Return from that stage and preserve diagram order.                                                                                        |
| Stage entry and outgoing transition                                              | Use the case outline for entry and its vertical mirror for transition, with matching destination names.                                   |
| Position of a transition node                                                    | Place it last and below every other vertex on its route; alternative transition nodes may share a row.                                    |
| Backward or self-transition                                                      | Mark the transition tip and the destination entry tip.                                                                                    |
| Forward transition to an entry also targeted backward                            | Keep that transition unmarked and mark the shared destination entry once.                                                                 |
| Transition to an earlier terminal stage                                          | Apply the same backward marker rule.                                                                                                      |
| Several stages with identical descriptions                                       | Distinguish them through their entry-signal captions.                                                                                     |
| Stage panels and collapsed-cycle inputs                                          | Show stage data in the defined panel and cycle data in derived input labels, preserving control order.                                    |
| Expanded and collapsed cycle views                                               | Preserve alternative output order, continue behavior, and surrounding stage links and markers.                                            |
| Terminal stage declares an output, return in preparation, or multiple returns    | Reject the invalid completion structure.                                                                                                  |
| Stage route ending without a signal or return                                    | Reject an empty stage or a route reaching its body end without a selected signal or return.                                               |
| Divergence through stage transitions                                             | Accept with declared signal exits and no terminal stage.                                                                                  |
| Unreachable stage                                                                | Reject using finite graph reachability.                                                                                                   |
| Quicksort on empty, duplicate, sorted, reversed, and owned inputs                | Match standard sorting while retaining the input allocation and requiring only Ord.                                                       |
| Copy values on stage transitions                                                 | Carry unit, primitives, tuples, arrays, shared references, and user-defined Copy types under ordinary Rust lifetime rules.                |
| Different entry types and several producers for one entry                        | Infer each destination type independently; reject mismatched incoming types.                                                              |
| &mut capture of a received stage entry or mut on a stage output binding          | Reject; received entries are immutable, including through nested cycles, and stage output declarations carry no mut permission.           |
| Mutable value capture of a stage entry                                           | Allow mut entry wherever that capture form is supported; it creates a mutable block-local alias under ordinary move or copy rules.        |
| Mutable initial parameter or preparation wire                                    | Allow mutation during preparation; bind the receiving stage entry immutably.                                                              |
| Stage entry carries a mutable reference or interior mutability                   | Preserve ordinary Rust behavior of the value; entry immutability concerns its binding.                                                    |
| Stage-local producer shadows a common outer wire or the current entry            | Reject except for a declared self-transition output; alternative local producers still follow ordinary merge rules.                       |
| Function input or direct preparation output named after an entry                 | Treat it as an initial transition value, with a Copy bound only in a const function; it is not common outer data.                         |
| Block initializer with outputs and no capture list                               | Treat `let output = body;` as `let output = \|\| body;` and apply the block kind's own rules.                                             |
| Singleton stage or cycle output pattern                                          | Treat (found,) and found as the same single output, transferring its whole value.                                                         |
| Stage entry captured through a nested cycle                                      | Resolve the stage, cycle, and inner captures to the same current entry wire.                                                              |
| Local self-transition output sharing the stage entry name                        | Captures through its first declaration read the entry; subsequent captures read the new local wire.                                       |
| Mutable local self-transition producer                                           | Allow mutable captures of the new local wire; exporting it creates an immutable entry for the next visit.                                 |
| Same-named local alternatives after entry shadowing                              | Apply normal merge ordering to the local producers; never fall back to the incoming entry on a sibling branch.                            |
| Inner capture of a cycle gate                                                    | Keep the original outer wire available; create no persistent header alias.                                                                |
| Bare continue after an open selection                                            | Reject missing branch ancestry; the repeating branch must capture its gate.                                                               |
| Bare continue after full convergence in an outputless cycle                      | Accept intentional repetition after the common work.                                                                                      |
| Repeated capture of a unit branch gate before continue                           | Accept copying the same wire while preserving its branch ancestry.                                                                        |
| Continue after an action consumes a non-Copy branch output                       | Capture the consuming action's unit output to preserve branch ancestry without cloning the consumed value.                                |
| Braced continue with or without its inner semicolon                              | Accept both forms under the same transfer grammar.                                                                                        |
| Stage whose every route diverges in a nested cycle                               | Accept with no declared outputs, transition node, or terminal return.                                                                     |
| Stage entry row and data panel                                                   | Place entries one row below start; keep the parameter panel at start and avoid panel overlap.                                             |
| Derived input list of a collapsed cycle                                          | Include its gate and all externally sourced inner captures once, including nested uses; omit its own locals.                              |
| Multiple capture forms for one external cycle wire                               | Show one input name with producer mutability; keep actual borrow forms at expanded inner consumers.                                       |
