# RFC 0007: Language refinements

- Status: accepted
- Language: [RFC 0001: kaalang Language](0001-language.md)
- Visual language: [RFC 0002: kaalang Visual Language](0002-visual-language.md)
- Renderer: [RFC 0003: kaalang SVG Renderer](0003-svg-renderer.md)
- Lowering: [RFC 0004: kaalang Rust Lowering](0004-rust-lowering.md)
- Markdown: [RFC 0005: Markdown](0005-markdown.md)
- Stages: [RFC 0006: Stages](0006-stages.md)

## 1. Motivation and scope

Cycles need explicit completion and repetition boundaries while their inner
blocks retain ordinary capture and ownership rules. This RFC replaces
cycle-header data aliases, structural `break`, and implicit repetition with an
optional entry gate, inherited outer data, named alternative outputs, and an
explicit structural `continue`.

It also permits block initializers with no capture list, follows Rust's optional
statement terminators, refines which routes decide and converge at a block, and
specifies the corresponding diagram rules. These changes apply throughout the
language, including preparation and stage bodies introduced by
[RFC 0006](0006-stages.md).

It also corrects parameter isolation during Rust lowering to enforce the
existing explicit capture rules.

This RFC defines syntax, execution, validation, Rust lowering, and the affected
visual representation. Section 7 identifies the earlier provisions it
supersedes. Earlier released RFC texts remain unchanged.

### 1.1 Terms

This RFC uses RFC 0001's wire, producer, capture, branch, and merge terms, with
the following additions:

- A **gate** is a wire whose availability establishes a block's or transfer's
  participation in a route. A cycle header may name one gate.
- A cycle **export** passes one selected local value from the cycle to its
  containing sequence at the cycle's **result boundary**.
- A **boundary consumer** is the implicit by-value use of the local wire
  selected for export. It is not an authored block and participates in ordinary
  scope, producer, branch, and merge checks.
- A **selector** is a question, a choice, or a cycle with alternative outputs: a
  block whose run selects one of its declared outcomes.
- A **frame** is one sequence level of a local flow: the root sequence,
  preparation, a stage body, or a cycle body describing one iteration. A cycle
  nested in a frame is one block of that frame; its body forms a frame of its
  own.

## 2. Cycle scope and interfaces

The `kaalang` attribute is assumed to be in scope in all examples.

### 2.1 Entry gates and inherited data

A cycle without a gate participates according to the containing sequence's
source and branch rules. A header `|start|` additionally gates that entry on the
signal; subsequent iterations are selected by `continue`. The header establishes
a capture dependency for producer usage and branch validation, but performs no
Rust move, copy, or borrow. Its gate may have any Rust type, including in a
`const fn`. The gate remains an ordinary outer wire available to explicit inner
captures. The header creates no separate data alias and does not remove that
wire from scope. Modified captures and multiple header names are invalid for
cycle headers.

A cycle inherits outer data visible at its declaration. Every outer wire used
inside must be available whenever that cycle is entered; inner captures do not
silently make cycle entry conditional. Their dependencies are recorded against
the original producers, including through nested cycles. They must respect
ordinary producer order and branch ancestry. The cycle's declared outputs are
provided only after it completes and are not outer inputs to its own body. Local
cycle producers cannot shadow visible outer wires, including the gate. The
cycle's own declared outputs are the exception: its body does not see outer
wires under those names, so a local producer of a declared output may share its
name with a visible outer wire, as when the cycle is a later alternative
producer of that wire. Inside the body such a name refers only to the local
wire, and capturing it before its local producer is invalid. Separate cycle
bodies may reuse their local names.

Borrowing, copying, and moving data happen at the inner block that captures it.
A cycle header does not create data aliases or hold data borrows across the
entire cycle. Create persistent local state explicitly before the cycle; nested
cycles can use state created earlier in their containing sequence. Those owners
retain their surrounding scope, while each iteration creates fresh body-local
wires. Uncaptured data reads inside a computational body remain invalid.

### 2.2 Alternative outputs

A cycle declares its alternative outputs with ordinary `mut` permission allowed
on output bindings. One name exposes the whole selected value. A tuple of names
declares alternative exits, in declaration order. A singleton `(found,)` is
equivalent to `found`: neither form destructures the selected value. Whenever
the cycle completes, it provides exactly one of them. Each declared output must
have a reachable same-named local producer. The cycle's **result boundary** is
where its selected output leaves the body and becomes available to the
containing sequence. Its boundary consumer consumes that local value after all
remaining work on the route, under the ordinary scope and merge rules.
Alternative internal producers must agree on type and mutability, while the
declaration sets the exported binding's mutability permission. Raw spellings
identify the same logical name.

Cycle outputs may carry data, and different exits may have different Rust types.
To carry several values together, produce one tuple-valued wire and declare it
as one output. A later action can destructure that tuple. Cycle outputs retain
ordinary moves and can carry non-`Copy` owners even in a `const fn`.

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

A structural return remains invalid inside a cycle. A cycle completes by
exporting one of its declared outputs, after which the containing sequence
continues along that output's branch.

### 2.3 Explicit repetition with continue

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

### 2.4 Shared state and a unit exit

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
flow returns the shared counter. Each iteration creates fresh local wires.
Branch-local owners drop before the merge continuation; remaining body-local
owners drop when the iteration completes or repeats. Outer owners retain their
surrounding scope.

### 2.5 Data exits and local ownership

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

## 3. Block initializers without captures

A `let` block without inputs may omit its empty capture list. For every block
kind, `let output = body;` whose `body` is not a closure is
`let output = || body;`, and that kind's rules then apply to the body. An action
may write `let count = 0;` and a cycle `let found = { ... };`. An initializer
that is itself a closure is always read as the capture list, so a closure-valued
action still writes `let f = || |x| x + 1;`. A statement without `let` keeps RFC
0001 §3's forms. Replace the action alternative with:

```text
action_statement :=
    "#[action(" block_description ")]"
    ("let" output_pattern "=" ("|" input_list? "|")? rust_expression
     | "|" input_list? "|" rust_expression
     | "{" rust_statement* "}") ";"
```

### 3.1 Statement terminators

A block statement's trailing semicolon is Rust's: kaalang reads a statement with
or without it wherever Rust accepts both spellings, and gives the semicolon no
meaning. This applies to all block kinds, including stages and structural
transfers. The `";"` ending each statement production in this RFC and RFC 0001
is Rust's terminator, optional wherever Rust allows a statement without it.

## 4. Branch participation and convergence

### 4.1 Deciding selections

A question or choice decides a block only through executions that reach the
block's comparison context. Reaching is distinct from running: an execution may
reach a block's position while unavailable captures keep that block from
running. Such an execution supplies the skipped outcome in RFC 0001's
deciding-selection comparison.

A completing finite history reaches every block in its local frame. A repeat
reaches the repeated cycle's own header and the blocks it encloses. For other
blocks, compare the block and repeated cycle in their nearest shared containing
sequence, using the position of an enclosing cycle when necessary. The repeat
reaches the block if those positions coincide, or if the block's position comes
first and both containing statements can run in one structural execution. A
repeat before that position, or on a mutually exclusive branch outside it, does
not count as skipping the block. Cycles on separate branches that merge their
results are therefore decided only by the selection between them, not by the
questions inside them.

### 4.2 Convergence groups

Convergence groups of one question or choice are compared by route, not by case.
A route is the selected case followed by the selections nested inside that case;
selections below the point where the cases converge belong to every case alike
and are not part of it. Two groups are invalid only when they share a route and
each also takes a route that the other reaches but does not take. A later
selection inside one case may therefore send its routes to different groups: a
timeout case can give up with the completing routes of other cases and retry
with their repeating routes. A route that repeats before reaching a group's
comparison context neither sets two groups apart nor separates the group's
cases, extending the adjacency trace of RFC 0001 §7. A repeat beyond that
context may still separate routes.

## 5. Validation and lowering

### 5.1 Finite cycle analysis

Analyze a cycle's sequence using the ordinary local branch, capture, merge, and
source-order rules, with the refinements in §§2–4. The cycle body receives its
available outer data scope. Inner blocks establish explicit dependencies on the
original producers; the entry gate and derived outer-data dependencies have
distinct roles.

Insert a boundary consumer for each declared output. Validate its producer,
scope, type, and merge order. Preserve separately declared mutability for cycle
output bindings. Validate every cycle route as one selected output, an explicit
continue, or nested divergence. Check the single continue separately for each
owning cycle. A finite cycle summary exposes its alternative exits to the
containing sequence and keeps its repeat outcome within the cycle. Preserve the
selected exit's identity in outer branch and convergence analysis.

For structural reachability, computational Rust bodies remain opaque as in
RFC 0001. Validation considers all question answers and choice cases possible
and does not prove termination or correlations between iterations. A Rust
expression does not establish structural divergence. Finite summaries describe
repeated execution without unfolding it; reject unreachable executable blocks
and require a conforming diagram for every reachable part.

Merge completion, block participation, and branch ordering compare executions
that disagree at exactly one selector they both run. A context first excludes
executions irrelevant to its check: those producing no alternative of a merge,
or those that repeat before reaching the block or boundary being checked. A
running block always participates in its own context. Compare the selections
visible in that context's cycle frame.

The explicit execution list can grow exponentially. Finite local flows also
admit an exact representation by execution conditions. Give every question,
choice, or cycle with alternative outputs one selection variable with its
declared outcomes and an additional inactive outcome. Store Boolean conditions
as an ordered, reduced decision graph: a node selects one variable, its outgoing
edges select that variable's outcomes, equal suffixes share a node, and a node
with identical successors disappears. This representation imposes no restriction
on the number of inputs, captures, actions, calls, or choice cases, or on which
earlier wires a block captures. Small flows may retain the ordinary enumeration
when it costs less. Count the finite domain with a capped decision-graph query
before choosing that strategy; the product of declared alternatives can greatly
overestimate a cycle's actual histories. The count does not list them and does
not overflow on a large domain. When the product is already within the
enumeration limit, no count is needed.

Walk source order once per finite cycle frame. A block runs where execution
continues in its frame and all its captures are available. Each producer
occurrence has its own condition. An action or call provides every output under
its execution condition; a question or choice provides only the selected output.
A capture dependency holds where both its consumer and that particular producer
run. A return or transition removes its condition from the execution that
continues below it; a continue records the repeated cycle and closes that finite
route. Reject overlapping producer conditions for one wire name, preserving the
ordinary walk's diagnostic priority. Require the inactive outcome exactly where
a question or choice does not run, or where a cycle selects no completed output.
The resulting condition for a complete execution therefore has one assignment
per structural history, including histories that do not visit every selector.

A cycle body starts with its entry condition and the available outer wire scope.
An export records its output condition, closes that body route, restores the
outer scope, and produces that output under the same condition. Couple the
cycle's selected output to its export condition. Restore no body-local wires in
the continuation. A repeating route or one diverging in a nested cycle produces
no cycle output and reaches no outer continuation. Keep their block and capture
facts in the finite history; iterations are never unfolded.

Transition boundaries check the same available scope for competing stage signals
and required common wires. Preparation's common data must be available on every
completing preparation route. Test that implication directly; repeats that never
reach a transition select no stage. Keep local stage analyses separate and
traverse their finite transition graph to decide reachability and constructor
order. These queries do not enumerate preparation or stage histories.

Perform validation over these exact conditions. Existence means a condition has
a satisfying assignment, implication means its counterexample condition is
empty, and transitive dependency paths combine edge conditions by intersection
and alternative paths by union. Preserve diagnostic priority: placement checks
use completing histories before a route missing its return is reported, just as
the ordinary walk does. Reachability, producer usage, branch-output consumers,
merge order, and convergence use the same predicates as enumerated validation.

To compare two executions, use two adjacent copies of each selection variable.
Require both histories to have a final return, transition, or repeat outcome,
both to run the selector being compared and to select different outcomes there.
For every other selector, require equal outcomes only if both histories run it;
an inactive selector takes no part in the comparison. In a cycle frame, compare
only visible selections. A visible cycle is a black box: equality of its
selection requires both its exported output and every selection in its body
route to agree. Its unfinished routes have an additional outcome. Inside that
cycle, its own selection is absent and the body's selections are visible. This
preserves the ordinary frame's route identity without numbering or listing its
body routes. The resulting relation is exactly the enumerated one: executions
that disagree at exactly one selector they both run. Query producer differences
and block participation within this relation to collect merge owners, all
branch-local work, and block deciders without listing its satisfying pairs. For
branch adjacency, project away selections that do not change the observed
outcome and check whether a missing producer lies between the first and last
producing traces. Choice and cycle routes retain their own selection and only
the visible selections nested within the chosen case, including a visible nested
cycle's body identity. Compare their projected route sets for adjacency and
crossing convergence groups. Each group also has its own reaching condition: a
repeat beyond its position can separate routes, while a repeat before it cannot.
Summarize authored ordering by the first and last output, a missing interval,
and any output-order reversal; equal remaining decision suffixes share that
summary. Check cycle output order and repeating intervals in the same way.

Lower sets of executions directly. A block can run in a plan scope when every
execution in that scope participates and its capture producers and merge-local
work have finished. Branches restrict that scope by their selected outcome;
joins combine the scopes yielding to them and carry the merged values available
throughout that combination. Shared computation is emitted once. Independently
replay the plan with conditions: check each body's participation, original
producer bindings, source order, completed merges, yielded values, lexical join
targets, cycle entry and scope restoration, exports, and the final return or
repeat against every history. This verification queries the entire execution
domain, including histories never materialized.

Project connections with conditions as well. A connection survives in the union
when some execution has its direct edge without an alternative path of two or
more edges to the same destination. Path conditions use intersection along a
path and union across alternative paths. This preserves per-execution transitive
reduction; reducing the unconditional connection union would discard edges that
are direct on some routes. Concrete witnesses may supply union facts such as
producer usage and capture successors, but do not substitute for validation,
plan verification, or conditional connection reduction.

This avoids the Cartesian product of independent earlier choices during
analysis, lowering, and projection. The decision graph can still grow large for
interacting conditions; no polynomial bound for all flows is claimed. Cycles and
transition boundaries use the same exact conditional representation. Explicit
requests to list all execution summaries enumerate every history on demand,
retaining the original ordering. Counting histories does not materialize them;
mutable access to that list replaces the conditional representation so later
projections observe the edited summaries.

### 5.2 Rust lowering

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
preserves an unconstrained generic `T` in a `const fn`; capture aliases and the
parameter prologue are omitted. Parameter isolation follows §5.3:

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
repeat from an unfinished body ending. Completing cycles, diverging cycles, and
partial convergence retain the same finite local plans in execution and drawing.
One iteration either transfers its chosen value outward, discards remaining
locals and repeats, or stays in a nested divergent execution. This preserves the
completion rules without unfolding repeated iterations.

### 5.3 Hygienic flow parameters

RFC 0001 §6 requires a computational body to receive local wire bindings only
for its explicitly listed inputs. The uninitialized parameter declarations in
RFC 0004 §2 do not enforce that rule: Rust permits their first initialization
without `mut`, and a later body can read the value without a capture. Correct
the lowering mechanism while retaining the existing capture semantics.

Keep each named parameter's authored spelling, type, pattern, mutability, and
source location, but give its binding the macro's hygienic context. The
parameter prologue references that hygienic binding when moving its value into
internal wire storage. Choose storage names that differ from every authored
parameter spelling, so creating one wire cannot shadow a parameter before its
value is moved. Keep the individual wire initializers: a tuple initializer would
introduce a generic tuple temporary whose destruction Rust rejects in otherwise
valid `const fn` flows. Omit the uninitialized authored-name declarations
entirely. A block receives authored names only through its capture aliases or
its own Rust declarations. The receiver remains unchanged under its existing
capture rules.

Lowering stays in the authored function's body. A nested implementation function
would hide the outer parameter names, but could not use `Self` or the generics
of the surrounding `impl` without additional transformations. Hygienic parameter
bindings preserve that Rust context without introducing another function.

Validate direct, unqualified parameter uses against the block's captures and
explicit Rust local bindings, including single-segment paths with generic
arguments and expressions in nested closure return types. A local `let` binding
is not in scope in its own initializer or type annotation. This prevents an
omitted capture from silently resolving to a module item with the same name
after the parameter is hidden. Respect Rust's local binding scopes. Nested items
and macro token streams remain opaque; their own name resolution and binding
introductions are left to Rust. Bindings introduced or removed by conditional
compilation and attributes, and scopes whose bindings require macro expansion,
are also left to Rust's expanded name resolution. These checks do not implement
a complete Rust name resolver; the original parameter storage remains
hygienically hidden in every case. Qualified item paths do not name parameter
wires.

The assignment below is rejected at `value`, even when the signature declares
the parameter `mut`:

```rust
#[kaalang]
fn invalid(value: u32) -> u32 {
    #[action("Assign and read an uncaptured input.")]
    let result = || {
        value = 9;
        value
    };

    |result| return result;
}
```

An explicit mutable value capture remains valid:

```rust
#[action("Change a local copy of the input.")]
let result = |mut value| {
    value += 8;
    value
};
```

It mutates the block's local alias. Changing the original wire uses `&mut value`
and requires a mutable producer, under the existing capture rules. An explicit
body-local binding such as `let mut value = 1;` may reuse the spelling and
retains ordinary Rust checks.

## 6. Visual representation

### 6.1 Cycles and continue

An expanded cycle retains its bounded region and ordinary local routing; only a
collapsed cycle node carries the loop marker. Its entry represents the available
outer data for local dependency routing, with original provenance retained in
the model. Those connections follow the existing control routes through the
entry. Its unique structural continue supplies the iteration tail and the back
edge to that entry. Continue has no computational figure; its captures and any
ordinary merges lead to that common tail. A cycle with no reachable continue has
no back edge. If such a cycle's entry leads only to one body block with no other
incoming route, that route replaces the entry junction. An explicit `continue;`
can connect entry to tail directly.

Each declared cycle output has a distinct result exit at the drawn cycle
boundary. All local producers of that output merge under the ordinary rules
before export. Result exits preserve declaration order from left to right and
retain their selected branch identity in the containing sequence. They hand over
only the selected wire. Different output exits converge only through ordinary
merges outside the cycle. A nested cycle's results first reach its immediate
containing sequence. Validation therefore rejects a body whose routes cannot
reach those exits without crossing: routes exporting a later output drawn left
of routes exporting an earlier one, or a repeating route between routes
exporting different outputs, which the back edge could not clear.

A collapsed cycle retains its described cycle node and loop marker. With several
declared outputs, it has one branch-specific exit per output, in declaration
order: the first leaves the lower edge, and later exits fan out to the right
using the existing ordered branch routing. A single output leaves through an
ordinary non-branching exit, as in §2.2. Each exit is labeled with its output
binding. A fully diverging cycle has no outgoing exit. Its receiving label lists
the external wires used by the cycle: begin with the authored gate, when
present, then walk the body in source order, including nested cycles, and append
each captured outer wire on its first occurrence. Resolve and deduplicate by
wire identity. Exclude wires produced inside this cycle, even when a nested
cycle captures them. A gate also captured inside the body appears only once.

This derived list is the collapsed node's input label, in the ordinary receiving
label position. Show literal names with their producer's `mut` permission and no
inferred types or aggregate borrow modifiers. Show `()` when the list is empty.
Actual inner capture forms remain visible in the expanded view. The list records
dependencies, not borrows held for the whole cycle. An expanded cycle retains
its unlabeled structural entry and does not repeat the full input list on its
boundary.

For label sharing, this derived input list is unordered: an adjacent hand-over
with the same nonempty set of displayed names, including producer `mut`
permissions, shares one label with the collapsed cycle even when the lists are
in different orders. The shared label uses the preceding hand-over's order. The
same applies to a shared hand-over at a merge whose sole consumer is the
collapsed cycle. All other adjacency and uniqueness conditions of RFC 0002 §6
still apply. Ordinary authored capture lists and hand-overs from alternative
exits still require matching order.

Both projections preserve the same selected outputs and surrounding branch
order. Check all result routes and the continue back edge in the expanded view,
then verify the collapsed projection. An arrangement must respect the existing
noncrossing and cycle-boundary rules, including the positions of all alternative
result exits. Derived inputs follow the existing reduced control routes through
the cycle entry; they do not introduce paths that bypass preceding work.

### 6.2 Result junctions and construction

A cycle has one result junction per declared output in place of the authored
break junction, so no break row or break capture label remains. A result whose
only incoming connection is a merge's only outgoing connection still reuses that
merge. In a cycle with several outputs, a result fed directly by a branch exit
keeps its own junction, so each alternative leaves the body at its own result.

An exit enclosed between repeating routes splits the merge before their one
continue, so validation rejects it before construction. Construction reports a
topology refusal when a search exhausts its space.

### 6.3 Aligning node heights

In every diagram, nodes on the same local row share the height of its tallest
measured node. Their upper and lower edges align while their centers and side
ports remain on the recorded rank line. Measuring the row includes each shape's
full height, including a case's triangular tip.

### 6.4 Oversized formulas

A formula whose absolute ascent plus absolute descent exceeds the renderer's
height limit stays literal, including its dollar delimiters. Use the unscaled
math layout in em for this check. This extends RFC 0005's literal fallback rules
to valid formulas whose height would make a label unreadable.

## 7. Changes to earlier RFCs

This RFC supersedes the provisions below in every flow, whether or not it
declares stages. Earlier released RFC texts remain unchanged.

- **RFC 0001 §§4.5–4.6 and §8:** replace the cycle's complete data-capture
  header with an optional single gate and inherited outer data scope. The gate
  imposes no trait bound or Rust value operation and remains available to inner
  captures as the original outer wire. Named outputs are alternative exits
  matched to same-named internal wires. Explicit result boundaries replace
  structural `break`. Each cycle owns at most one structural `continue`, and
  every repeating route must reach it; reaching the body end without an output
  or transfer is invalid.
- **RFC 0001 §§1–4:** wherever the overview, the definition of iteration, and
  the block-kind table describe a cycle completing by `break` or repeating at
  its body's end, a declared output completes it and `continue` repeats it. An
  outputless cycle no longer transfers `()`: it repeats or diverges as in §2.2
  here. RFC 0001 §4.7 keeps the capture, value, and computation restrictions RFC
  0001 §4.6 gave break, now for return alone.
- **RFC 0001 §3 and §8:** permit a block initializer without an empty capture
  list, with the grammar and closure-valued initializer rules in §3 here.
- **RFC 0001 §3 and §8:** replace the mandatory trailing semicolon with Rust's
  statement terminator rules, as in §3.1 here, including the `";"` endings in
  the grammar. Stage declarations follow the same rule.
- **RFC 0001 §7:** compare convergence groups by route, including nested
  selections and routes that repeat before reaching the group, as in §4.2 here.
- **RFC 0001 §2:** count a selection as deciding a block only through executions
  that reach that block, as in §4.1 here.
- **RFC 0001 §§5–7:** cycle bodies inherit outer data and create fresh local
  scopes per iteration. Multiple outputs participate as alternative branches.
  Ordinary producer order, ownership, local merges, and convergence rules apply.
- **RFC 0002 §§2–4 and 4.9:** breaks no longer exist. Each declared output's
  routes reach its own result junction as in §6.2 here, which limits §4.9's
  reuse of a merge or body exit as the result. A collapsed cycle has one exit
  per output as in §6.1 here, rather than one non-branching exit.
- **RFC 0002 §4.8 and RFC 0003 §3:** an expanded cycle shows its boundary
  without a loop marker; only the collapsed cycle node carries it.
- **RFC 0002 §§4.8 and 6–8:** cycles have one result exit per declared
  alternative output, and their continue supplies the single iteration tail.
  Collapsed cycles retain those alternative exits. Replace their authored
  capture list with the derived external-wire input list in §6.1 here, including
  the gate and transitive inner uses. Show producer mutability, and share an
  adjacent hand-over or shared merge label when the displayed names match
  regardless of order, using the preceding hand-over's order. Expanded cycles
  retain the unlabeled structural entry and boundary without a repeated full
  input list.
- **RFC 0003 §§1–2:** check expanded and collapsed cycle projections with their
  alternative result exits and explicit continue routes in the same validated
  model. In all diagrams, nodes sharing a row use its maximum measured node
  height so their edges align, as in §6.3 here.
- **RFC 0003 §2.3:** reject an exit enclosed between repeating routes before
  construction, as in §6.2 here; construction still reports a topology refusal
  when a search exhausts its space.
- **RFC 0003 §§2.1, 2.3 and 3:** replace the authored break junction with one
  result junction per declared output, preserving the junction and merge reuse
  rules in §6.2 here.
- **RFC 0003 §§4–5:** name the collapsed view after cycles: the
  `RenderOptions::collapse_cycles` field and the `--collapse-cycles` option
  replace `collapse_loops` and `--collapse-loops` with the same meaning.
- **RFC 0004 §2:** replace the uninitialized authored-name declarations and
  their permission to assign an uncaptured parameter spelling as an ordinary
  Rust local with the hygienic parameter bindings and capture checks in §5.3
  here. Preserve the authored signature text from RFC 0004 §6 while changing the
  named parameters' binding hygiene. Explicit capture semantics and authored
  body-local Rust bindings retain RFC 0001's existing rules.
- **RFC 0004 §7:** remove persistent cycle-header data aliases and implicit
  repetition at body endings. Lower inner data captures against their original
  storage. Explicit continue targets the nearest generated cycle label.
  Alternative outputs use the labeled result blocks from RFC 0004 §4, leaving
  the loop with the selected value before entering its matching outer
  continuation.
- **RFC 0005 §§3, 4–5:** apply literal fallback to formulas above the renderer's
  height limit, as in §6.4 here.
- **RFC 0006 §§1, 3, 4.1, 5–6, 8.3, 8.5 and 10:** apply the cycle rules in §§2
  and 5 here to preparation and stage bodies, replacing their captured
  cycle-input bindings, structural break, simultaneous results, and implicit
  repetition. A cycle inherits the available stage data; a gate naming the stage
  entry refers to that same wire and performs no move or copy. Inner captures
  resolve to the original producer. Mutable borrowing of the received stage
  entry remains forbidden, including inside a cycle; create a separate mutable
  wire for working state before the cycle. A selected cycle output becomes a
  stage transition only when the stage also declares that output.
- **RFC 0006 §6.1:** use the deciding-selection and convergence rules in §4 here
  for every local part. The execution-condition representation in §5.1 also
  covers preparation, stage bodies, and their transition boundaries. It checks
  common-wire availability across all completing preparation histories and
  traverses the finite stage graph without listing local histories.
- **RFC 0006 §7 and §10:** apply the cycle projection and row-height rules in §6
  here inside each part. Derived cycle-input labels replace the authored capture
  interface; alternative result exits and explicit continue routes replace the
  single break result and implicit repetition. Preserve stage declaration order,
  entry and transition icons, and inter-stage rails.

For cycle migration, create any formerly captured owned working state before the
cycle and keep data captures on its inner blocks. Replace the structural break's
value with a same-named local producer for a declared output. A unit completion
needs a named unit output. Replace each implicit repeating route with a path to
the cycle's common continue, merging its gate wires as needed. If a repeating
branch has consumed its non-`Copy` wire, use the consuming action's unit output
to gate the continue as in §2.3. A former tuple of simultaneous results becomes
one tuple-valued output followed by ordinary destructuring outside the cycle.
The selected output is exported only after its route's remaining work, so the
migrated branch structure must make that work and the continue mutually
exclusive.

Flows without cycles still use the initializer, branch, parameter-isolation,
node-height, and formula refinements where applicable. Concrete compiler and
renderer types remain implementation choices.

## 8. Acceptance scenarios

These scenarios define required language behavior and visual representation.

| Scenario                                                                                             | Required result                                                                                                     |
| ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Duplicate declared outputs or a declared output with no reachable local producer                     | Reject the invalid output interface.                                                                                |
| Remaining work after an exported wire is produced                                                    | Finish the selected route before exporting its output.                                                              |
| Branch-local exit wire consumed after a merge closes its branch                                      | Reject under ordinary scope and merge ordering rules.                                                               |
| Inner computational body reads an uncaptured wire                                                    | Reject according to ordinary capture rules.                                                                         |
| Computational body assigns an uncaptured parameter, including a mutable or underscore-prefixed one   | Reject; a parameter spelling alone does not introduce a body-local binding.                                         |
| Body assigns a parameter captured without `mut`                                                      | Let Rust reject the assignment to the immutable capture alias.                                                      |
| Body assigns a parameter captured through `mut name`                                                 | Allow mutation of the local alias under ordinary move or copy rules.                                                |
| Body declares its own Rust local with a parameter's spelling                                         | Accept the local binding and retain ordinary Rust scope and mutability checks.                                      |
| Uncaptured parameter spelling also names a module item                                               | Reject the direct unqualified use; explicitly qualified item paths remain available.                                |
| Parameter spelling matches a generated wire name                                                     | Choose distinct internal storage names and preserve every input value.                                              |
| Macro expansion, conditional compilation, or attributes determine a local binding                    | Leave expanded name resolution to Rust while keeping the original parameter bindings hygienic.                      |
| Repeated local producers in a cycle iteration                                                        | Require mutual exclusion, merge completion, and producer order before consumers.                                    |
| Mutable outer state and borrowing blocks                                                             | Later iterations observe updates; data borrows begin at their actual capturing blocks.                              |
| Repeated move of owned outer state on a continuing route                                             | Let Rust reject invalid repeated use in the generated loop.                                                         |
| Reference to a departing local stored in shared state                                                | Reject the escaping borrow.                                                                                         |
| Branch-local and body-local owners with destructors                                                  | Drop branch locals before the merge continuation and remaining body locals when the iteration completes or repeats. |
| Methods, generics, and otherwise valid const functions                                               | Preserve the Rust context and ordinary receiver capture rules.                                                      |
| Generic const function with alternative cycle outputs of an owned type                               | Compile and evaluate constant instances through labeled result blocks, without extra trait bounds.                  |
| Cycle with an empty header or one entry signal                                                       | Enter by control order or the named gate; the header performs no move, copy, borrow, or trait check.                |
| Invalid cycle header captures                                                                        | Reject multiple or modified header captures; inner data access remains explicit.                                    |
| Non-Copy cycle gate in an ordinary or const function                                                 | Accept the gate and let inner captures determine moves and borrows of its original storage.                         |
| Nested cycle reads surrounding data through an inner capture                                         | Resolve the original producer without repeated header data lists.                                                   |
| Outer data unavailable on one route entering a cycle                                                 | Reject; inner captures cannot silently make the cycle entry conditional.                                            |
| Cycle with one selected unit output                                                                  | Complete and provide that named unit wire to the containing sequence.                                               |
| Cycle with alternative outputs of different Rust types                                               | Provide only the selected output and apply ordinary alternative-branch consumer rules.                              |
| Cycle declares one tuple-valued output                                                               | Transfer that tuple intact; a later action may destructure it.                                                      |
| Exported cycle binding declared mutable                                                              | Permit later mutable captures according to the outer declaration.                                                   |
| One case splits between completing and repeating routes                                              | Compare convergence groups by route, so the completing and repeating groups stay disjoint.                          |
| Case diverging in a nested cycle between two merged cases                                            | Keep the merge adjacent: a route that never reaches it cannot separate its cases.                                   |
| Several repeating branches and one continue                                                          | Require their ordinary convergence before the shared transfer and draw one back edge.                               |
| Missing continue on a route with no declared output                                                  | Reject the forgotten iteration ending.                                                                              |
| More than one structural continue in one cycle                                                       | Reject, counting nested cycles separately.                                                                          |
| Declared output produced on a route reaching continue                                                | Reject the conflicting completion and repetition.                                                                   |
| Multiple cycle outputs produced on one route                                                         | Reject the ambiguous result.                                                                                        |
| Routes exporting a later cycle output drawn left of those exporting an earlier one                   | Reject; the declaration order is the drawn order of the result exits.                                               |
| Repeating route between routes exporting different cycle outputs                                     | Reject; the back edge cannot clear the exits on both sides.                                                         |
| Labeled, attributed, value-carrying, or out-of-cycle structural continue                             | Reject the invalid transfer.                                                                                        |
| Nested continue                                                                                      | Repeat only the directly containing cycle.                                                                          |
| Structural break in a cycle body                                                                     | Reject; completion is expressed by a declared output.                                                               |
| Empty cycle body                                                                                     | Reject the unfinished iteration.                                                                                    |
| Outputless cycle containing an unconditional continue                                                | Accept explicit divergence and reject work reachable only after it.                                                 |
| Owned cycle-local value selected for export                                                          | Move that value out and drop unexported locals at their ordinary scope exits.                                       |
| Cycle output borrows a departing body-local owner                                                    | Let Rust reject the escaping reference.                                                                             |
| Cycles in a flow without stages                                                                      | Apply the same unified scope, output, continue, and diagram rules.                                                  |
| Expanded and collapsed cycle views                                                                   | Preserve alternative output order, continue behavior, and surrounding branch order.                                 |
| Cycle-local producer named after its own declared output while an outer wire of that name is visible | Accept; inside the body the name refers only to the local wire.                                                     |
| Capture of a cycle's own declared output before its local producer                                   | Reject; the outer wire of that name is not visible inside the body.                                                 |
| Block initializer with outputs and no capture list                                                   | Treat `let output = body;` as `let output = \|\| body;` and apply the block kind's own rules.                       |
| Block statement without a trailing semicolon                                                         | Accept wherever Rust allows that statement without its terminator.                                                  |
| Singleton cycle output pattern                                                                       | Treat (found,) and found as the same single output, transferring its whole value.                                   |
| Inner capture of a cycle gate                                                                        | Keep the original outer wire available; create no persistent header alias.                                          |
| Bare continue after an open selection                                                                | Reject missing branch ancestry; the repeating branch must capture its gate.                                         |
| Bare continue after full convergence in an outputless cycle                                          | Accept intentional repetition after the common work.                                                                |
| Repeated capture of a unit branch gate before continue                                               | Accept copying the same wire while preserving its branch ancestry.                                                  |
| Continue after an action consumes a non-Copy branch output                                           | Capture the consuming action's unit output to preserve branch ancestry without cloning the consumed value.          |
| Braced continue with or without its inner semicolon                                                  | Accept both forms under the same transfer grammar.                                                                  |
| Derived input list of a collapsed cycle                                                              | Include its gate and all externally sourced inner captures once, including nested uses; omit its own locals.        |
| Multiple capture forms for one external cycle wire                                                   | Show one input name with producer mutability; keep actual borrow forms at expanded inner consumers.                 |
| Nested output export                                                                                 | Pass through each containing cycle's declared output before reaching its surrounding sequence.                      |
