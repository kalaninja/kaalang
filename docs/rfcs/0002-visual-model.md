# RFC 0002: kaalang Visual Language

- Status: accepted design draft

## 1. Overview

kaalang's visual language represents the [core model](0001-core-model.md)
defined by RFC 0001. A diagram is a projection of the validated semantic model
and represents the same flow semantics. It does not introduce a separate
execution model.

## 2. Terminology

Terms defined by RFC 0001 keep their meanings. The visual language adds the
following terms:

- a **diagram** is the complete visual representation of one flow;
- a **node** is a drawn unit that represents all or part of one block, authored
  or implicit, except for the synthetic start node;
- a **connection** is one drawn control-flow link between nodes, not a wire;
  connections need not correspond one-to-one with wire dependencies;
- a **label** is text attached to a node or connection;
- an **exit** is an outgoing attachment point of a node; start and action nodes
  each have one non-branching exit, a question has one branch-specific exit per
  output, a select has one distributor exit, each case has one exit associated
  with its choice branch, and end has no exit;
- a **hand-over** is the ordered sequence of newly provided wires labeled at a
  node exit;
- a **column** is a vertical layout position at which nodes and vertical
  connection segments may be aligned;
- a **row** is a horizontal layout position at which nodes and horizontal
  connection segments may be aligned;
- a **distributor** is the shared horizontal connection segment from which a
  select node's branches fan out to its case nodes.

## 3. Diagram structure

A diagram contains a visual representation of each block the flow declares,
authored or implicit, so the implicit end block has a node like any other. No
block is duplicated to simplify layout.

Connections preserve the validated model's dependencies, implicit wire order,
and branch routes. The diagram shows the permitted serial order chosen by the
model's verified execution plan. Independent blocks therefore form a sequence
on the current column instead of suggesting parallel execution. This displayed
order adds no capture and is not an additional language dependency: RFC 0001
still permits other orders of independent ready blocks. Block bodies and source
comments do not affect control topology. Wire names label connection ends as
section 6 sets out; wires are not drawn as separate data connections.

## 4. Node kinds

An action, a question, and the unique end block each become one node. A choice
becomes one select node and one case node per authored case. Case nodes are
visual projections, not additional semantic blocks.

| Node kind | Represents | Label source | Shape |
| --- | --- | --- | --- |
| **start** | the beginning of a flow | the flow signature | capsule |
| **action** | an action block | the block description | rectangle |
| **question** | a question block | the block description | elongated hexagon |
| **select** | a choice block | the choice description | skewed parallelogram |
| **case** | one case of a choice | the case description | a shape with a lower triangular point |
| **end** | the end block | the flow's return type | capsule |

### 4.1 start

A diagram contains one synthetic start node. It uses the authored flow signature
without its `fn` keyword. The label retains parameter types, generics, the return
type, a where clause, a wildcard parameter, and the `r#` of a raw identifier.

### 4.2 action

An action block becomes one action node.

### 4.3 question

A question block becomes one question node. Its branches preserve their
positional meaning from RFC 0001: the first output is yes/true and the second is
no/false. The diagram does not add `yes` or `no` labels.

### 4.4 select

A choice block becomes one select node. Its branches preserve authored case
order, and the diagram must not reorder them according to their patterns or
layout position. Its distributor exit has exactly one connection to each of its
case nodes.

### 4.5 case

Each authored case becomes one derived case node. A select-to-case connection
names nothing at either end, because the case row belongs to the select above
it. The branch's output wire is handed over at the case node's exit. A choice
output may carry data or serve as a unit-valued control wire; the diagram shows
its wire name in either case.

### 4.6 end

The end node represents the flow's implicit end block. That block has no
description, so its node carries the flow's return type instead: the authored
return type preceded by `->`, and `-> ()` when the function declares none. The
label is therefore the tail of the start node's signature, and the two read as
one contract split across the diagram. A diagram has no separate return node.

The end node's label names the type and its incoming connection names the
`result` wire, so neither repeats the other; section 6 governs that capture
label exactly as it governs any other node's. The end node is an ordinary
consumer: alternative producers of `result` merge above it (section 7), and it
is not itself the merge.

## 5. Flow inputs and outputs

The start node's signature shows the flow input parameters and, when present,
the return type that constrains the flow output. Every named flow input is also
shown as an output of the start node, even if no block captures it. A wildcard
flow input produces no wire label. A zero-computation flow is the start node
connected to the end node by the `result` wire a flow input provides.

## 6. Wires and labels

Description labels carry the exact authored text. A presentation may wrap or
escape that text but must not paraphrase, normalize, or synthesize it. No node
carries a caption naming its block kind. The end node's `-> ()` for an absent
return type is the one synthesized label, and it states the contract rather
than paraphrasing authored text.

Every named flow input and every block output is labeled once at the exit that
provides it, or by a shared label as defined below, including an intentionally
unused wire whose name begins with `_`.
The labels at one exit form its hand-over. The start node hands over its named
flow inputs in signature order, an action hands over all its outputs in
declaration order, and each question branch or case hands over its exact branch
output. A hand-over names newly provided wires only; it neither lists wires that
remain available nor implies that the next node captures every named wire.

Every block input is labeled beside its receiving node. A consuming input is
shown as `name`, while a borrowed input is shown as `&name`. The captures of a
node are drawn once however many connections arrive there, because the capture
list belongs to the node rather than to an incoming connection. Consuming and
borrowed captures both establish execution dependencies. A wire that remains
available for a later capture may pass virtually along a transitive connection
path without appearing in intermediate hand-overs. This version of the visual
language does not show wire lifetimes or assign a wire to one particular
sequence of connections.

A computational node with an incoming connection and no authored inputs shows
`()` beside its receiving end. This marks an empty capture list, not a wire or
a unit-valued input: a connection passing through a node does not mean that the
node captures the preceding hand-over. Start and case nodes have no such label.
The first computational node also receives a connection from start when it has
no inputs.

A hand-over and an adjacent capture may share one label only when they are the
two ends of the same connection, that connection is the only one leaving its
source exit and the only one entering its destination node, and the two lists
have the same displayed names in the same order. `name` and `&name` do not
match. The shared label represents both connection-end labels.

Identical hand-overs from alternative exits may share one label beside their
merge when each exit has only one outgoing connection and all those connections
reach that merge. The displayed lists must match in both names and order; a
different hand-over remains at its own exit. The shared label represents the
alternative hand-overs, not a new producer at the junction. If the merge has
one outgoing connection, it is the sole connection entering its consumer, and
that consumer's capture list matches the shared hand-over, the same label also
represents the capture. This applies to `result` at the end node as well.

Wire labels use logical wire names. A raw identifier appears without its `r#`:
the raw and ordinary spellings of one wire name the same wire, and only the
signature in the start node preserves the authored raw spelling.

## 7. Connections and implicit convergence

For dependency routing, the start node represents the producers of named flow
inputs, a question's two outputs are distinct exits of its node, and a case node
represents its corresponding choice output.

Consider one possible execution after its branches have been selected. A node
participates in that execution when its represented block executes or its
represented case is selected; the start and end nodes always participate. A
producer node precedes a consumer node when a capture dependency from the
represented producer occurrence to that consumer participates in the execution,
and this order is transitive. A select node precedes its selected case node.
Wire production and implicit merges also establish the precedence defined by
RFC 0001 §7, including before a question or choice that selects their consumers
without capturing the wire itself. These orderings participate in the same
per-execution reduction as capture dependencies.
RFC 0001 makes every participating block either the producer of `result` or a
transitive predecessor of it, so every node representing one precedes the end
node.

The verified execution plan supplies the serial order of participating blocks.
For each execution, add precedence from start to its first computational block,
between consecutive computational blocks, and from the last one to end. A
question leaves through its selected exit; a choice continues through its
selected case. With no computational blocks, start leads directly to end.
These relations participate in the same reduction as dependencies and merges.
They express the order actually chosen for lowering without inventing captures
or changing the language's readiness rules.

For example, if two actions independently capture `left` and `right` after a
merge, draw them in the chosen serial order on the happy path. A connection
between them does not imply that the second captures the first's output. If a
zero-input setup action precedes a question capturing `condition`, draw
`start → setup → question`: the action's capture label is `()`, its hand-over
still names only `setup`, and the question's capture still names `condition`.

A direct connection joins an exit of a participating source node to a
participating destination node exactly when the source node precedes the
destination node and no other participating node lies between them in this
order. A connection is identified by its source exit and destination node,
so connections from distinct exits remain distinct even when they join the same
pair of nodes.

The complete diagram is the union of these connections over all possible
executions. A connection may therefore be direct in one execution and
transitively redundant in another. Each select-to-case connection and every
connection leaving a question or case exit remains associated with its branch.

A drawn connection participates in an execution when both endpoint nodes
participate and its branch, if any, is selected. A connection that is
transitively redundant in that execution adds no new ordering requirement.

The producer occurrence that supplies a capture therefore reaches its consumer
through one or more connections in every execution where that resolution
occurs. A dependency needs no direct producer-to-consumer connection when a
sequence of connections through participating intermediate nodes already
represents it. For example, if `P` produces `x`, `A` borrows `x` and produces
`a`, and `C` captures `x` and `a`, the diagram contains `P → A → C` but no
direct `P → C` connection.

Every computational node is reachable from start, including zero-input actions.
A start or action exit continues to the next step of the chosen serial order;
independent consumers are reached transitively through that sequence. A question
activates exactly one of its two exits; a select activates exactly one outgoing
connection from its distributor exit and therefore exactly one case. Branch
connections retain their selected output even when the next step does not
capture it directly. A node with several incoming connections waits for every
one that participates in the current execution. At a convergence, incoming
connections from alternative branches never participate together.

kaalang has no authored merge block or merge icon. Equally named alternative
outputs meet at an implicit junction of connections before any consumer of the
merged wire. The common segment from that junction leads to its consumer or
consumers; the junction is the convergence point, not a consumer node. Which
branches converge there, what finishes before it, and what may bypass it are
RFC 0001 §7's rules; the diagram draws them and adds none.
When the merge implicitly precedes a consumer-selecting question or choice,
the junction reaches that selection before its consumers. This connection
adds no capture label to the selection. Connections already represented through
that selection are omitted by the per-execution transitive reduction.
Likewise, an ordinary action output used by selected branch actions establishes
the eligible producer-to-selection order. Its producer sits above the selection;
the reduction carries its consumers' dependencies through that selection while
preserving their authored capture labels.
The junction adds neither a computational block nor a producer occurrence.
Consumers display the captured logical wire name once.

## 8. Spatial notation

Along each connection, execution time runs from top to bottom: the destination
node occupies a lower row than its source node, and the route never moves
upward. Nodes on alternative branches may share a row.

The visual language uses columns and rows. Branches are arranged from left to
right in authored order. The first question output and the first choice case
continue down the current column; remaining branches appear to their right.

Consecutive independent blocks continue down the current column. After a
convergence, its independent entry blocks form a sequence in the column reached
by the group's first branch. Exact lower rows and routing space remain
presentation choices.

A convergence group reserves enough columns for its shared continuation,
including any nested question or choice. Later sibling branches start to the
right of that whole area, even when the shared continuation is wider than the
group's incoming branches.

Alternative producers of `result` meet at their implicit merge above the end
node, mirroring the distributor that fans a select node out to its case nodes.
Connection routes are simple: they do not intersect or overlap themselves. They
do not cross one another or pass through a non-endpoint node. Meeting at a common
endpoint or deliberately sharing a collinear segment is not a crossing;
connections may share such a segment only when they have the same source exit
or the same destination node. Other connections do not overlap. Connections
are plain lines without arrowheads. A route contains only straight horizontal
and vertical segments, so every bend is a right angle.

At an implicit merge, side routes finish horizontally at the junction on the
merge rail. The outgoing connection alone owns the vertical below that point:
an incoming side route must not turn down and overlap that continuation. Routes
already in the junction's column descend straight to the same point.

The visual-language contract covers the diagram's nodes, roles, labels, the
connection end or ends to which each connection label belongs, branch order,
implicit convergence, dependency reachability, connections, crossing-free
orthogonal routing, top-to-bottom row order, and branch column order. Exact
dimensions, colors, typography, spacing, and routing offsets are presentation
choices.

A validated flow whose required connections cannot be drawn under these rules
has no conforming diagram.
