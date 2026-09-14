# RFC 0002: kaalang Visual Language

- Status: accepted design draft

## 1. Overview

kaalang's visual language represents the flows defined by
[RFC 0001](0001-language.md). A diagram is a projection of the validated
semantic model and represents the same flow semantics. It does not introduce a
separate execution model.

## 2. Terminology

Terms defined by RFC 0001 keep their meanings. The visual language adds the
following terms:

- a **diagram** is the complete visual representation of one flow;
- a **node** is a drawn unit that represents all or part of one block, except
  for the synthetic start and end nodes. An expanded cycle is a bounded region,
  a collapsed cycle is one node, a break is represented by routes and any needed
  structural junction, and a return is represented by its route to end;
- a **connection** is one drawn control-flow link between nodes, not a wire;
  connections need not correspond one-to-one with wire dependencies;
- a **label** is text attached to a node or connection;
- a **parameter panel** is the non-executable rectangle to the right of start
  that lists the flow's authored Rust parameters;
- an **exit** is an outgoing attachment point of a node; start, action, and a
  normally completing collapsed cycle each have one non-branching exit, a
  question has one branch-specific exit per answer, a select has one distributor
  exit, each case has one exit associated with its choice branch, and end has no
  exit;
- a **hand-over** is the ordered sequence of newly provided wires labeled at a
  node exit;
- a **column** is a vertical layout position at which nodes and vertical
  connection segments may be aligned;
- a **row** is a horizontal layout position at which nodes and horizontal
  connection segments may be aligned;
- a **distributor** is the shared horizontal connection segment from which a
  select node's branches fan out to its case nodes.

## 3. Diagram structure

A diagram contains a visual representation of each reachable block the flow
declares. Breaks use structural junctions as needed, while the flow's structural
return connects directly to end. The implicit visual end boundary has a node
only when at least one finite execution summary has a `Return` outcome. No block
is duplicated to simplify layout.

Every cycle is rendered in one of two representations selected for the whole
diagram. The expanded representation draws its boundary, body, local result
routes, and iteration back edge. The collapsed representation replaces that
region with one described loop node. Both project the same validated cycle block
and preserve the same captures, outputs, branch participation, and source order.
Internal wires and transfers never cross a cycle boundary directly.

Expanded is the default. A presentation may request that all cycles be
collapsed; this version has no source attribute, interactive folding state, or
per-cycle selection for that choice.

Connections preserve the validated model's dependencies, wire merges, and branch
routes. The serial order they show is the source order RFC 0001 §7 defines, so
blocks that capture nothing from each other still form a sequence on the current
column instead of suggesting parallel execution. That sequence adds no capture.
Block bodies and source comments do not affect control topology. Wire names
label connection ends as section 6 sets out; wires are not drawn as separate
data connections.

## 4. Node kinds

An action and a question each become one node. A choice becomes one select node
and one case node per authored case. A collapsed cycle becomes one cycle node. A
reachable flow-completion boundary becomes one end node. Case nodes and the end
node are visual projections, not additional semantic blocks; breaks and returns
have no nodes of their own.

| Node kind    | Represents              | Label source           | Shape                                 |
| ------------ | ----------------------- | ---------------------- | ------------------------------------- |
| **start**    | the beginning of a flow | the flow header        | capsule                               |
| **action**   | an action block         | the block description  | rectangle                             |
| **question** | a question block        | the block description  | elongated hexagon                     |
| **select**   | a choice block          | the choice description | skewed parallelogram                  |
| **case**     | one case of a choice    | the case description   | a shape with a lower triangular point |
| **cycle**    | a collapsed cycle block | the cycle description  | loop-marked rectangle                 |
| **end**      | flow completion         | the flow's return type | capsule                               |

### 4.1 start

A diagram contains one synthetic start node. Its label uses the authored flow
header without its `fn` keyword, parameters, or return type. It retains the
`const` marker, generic parameter declarations, where clause, and the `r#` of a
raw flow name.

When the flow has parameters, a rectangular parameter panel sits to the right of
start and connects to it with a horizontal line. The panel lists one authored
parameter per row, including its Rust type. It retains a wildcard parameter and
the `r#` of a raw identifier. The panel is not a node and does not participate
in execution.

### 4.2 action

An action block becomes one action node.

### 4.3 question

A question block becomes one question node. Its answer attributes, outputs, and
branches preserve their positional order from RFC 0001. Each branch is labeled
with its authored description when present, otherwise with its output name. An
ordinary question therefore needs no synthesized answer labels.

### 4.4 select

A choice block becomes one select node. Its branches preserve authored case
order, and the diagram must not reorder them according to their patterns or
layout position. Its distributor exit has exactly one connection to each of its
case nodes. The first connection leaves the select's lower edge; the remaining
connections leave its right edge and fan out horizontally, like a question's
later branches.

### 4.5 case

Each authored case becomes one derived case node. All case nodes belonging to
one select occupy the same row, in authored order. A case may not be lowered
below its siblings to route around a convergence or iteration back edge; a
topology that requires this has no conforming diagram. A select-to-case
connection names nothing at either end, because the case row belongs to the
select above it. The branch's output wire is handed over at the case node's
exit. A choice output may carry data or serve as a unit-valued control wire; the
diagram shows its wire name in either case.

### 4.6 end

When at least one execution has a `Return` outcome, the end node represents the
flow's visual completion boundary. It has no authored description, so its label
is the authored return type, and `()` when the function declares none. A diagram
has no separate return node. A fully diverging flow omits the unreachable end
node.

The structural return terminates at the end node. Its arrival is not a
logical-wire merge. The end node displays the value transferred into the flow
boundary, which Rust checks against the function signature.

### 4.7 cycle

An expanded cycle is enclosed by a visible boundary with a separate loop marker.
Its description is secondary: the caption wraps into the available space and may
end with an ellipsis when no more lines fit. If even an ellipsis cannot fit, the
visible caption may be omitted. The full authored description remains available
in a tooltip and the accessible diagram description. Caption length does not
enlarge the boundary or move routes. The boundary does not repeat the cycle's
input or output list; its entry and result routes show the interface.

Inside the boundary, an entry junction receives the initial route and every
iteration back edge. The body follows that junction once in the finite diagram.
Normal body endings meet at an unlabeled iteration tail whose back edge returns
to the entry. An empty body connects entry directly to tail. A cycle with no
repeating route has no tail or back edge. Break routes meet at the result
boundary, and the cycle's one normal continuation leaves that boundary carrying
all declared outputs together. A cycle with no reachable break has no normal
outgoing connection.

Nested cycles have nested boundaries. Every inner route either remains within
the inner boundary or completes at its result interface before the enclosing
body continues; no inner wire or transfer connects directly to an outer
boundary. The boundary remains visible for an empty cycle or a cycle that
completes on its first iteration; its caption follows the same space limits.

A collapsed cycle becomes one cycle node carrying the same loop marker and
authored description. Its receiving label lists the authored captures, and its
hand-over lists the authored output bindings. It has one normal exit when
completion is reachable, regardless of the number of output wires, and none when
the cycle fully diverges. Its body, break routes, result junction, and iteration
back edge are hidden, not removed from validation.

Unit-valued question outputs captured by a cycle or break still select an
existing branch route. The branch description or output name remains at the
question exit; the structural dependency adds no duplicate label. A transfer's
capture junction is likewise unlabeled. A collapsed cycle shows its complete
capture list on the node; an expanded cycle adds no interface list.

### 4.8 break

A break redirects its branch route to the directly containing cycle's result
boundary. Break routes from nested cycles stop at their own boundary and never
bypass an enclosing cycle. A break capturing data uses an unlabeled structural
junction for its dependencies; a break with no data captures adds no vertex or
spacing. It has no computational figure, description, or output hand-over of its
own.

### 4.9 return

The flow's structural return redirects its root-owned branch route to the flow's
end boundary. Its explicit captures participate in dependency and serial-order
routing, but the route ends directly at end and adds no vertex or spacing. The
return has no computational figure, description, or wire hand-over of its own.
RFC 0001 permits at most one root-owned return and none inside a cycle; a fully
diverging flow has no return or end node.

## 5. Flow inputs and outputs

The parameter panel shows every flow input with its Rust type. Every named flow
input is also shown as an output of the start node, even if no block captures
it. A wildcard flow input produces no wire label. A zero-computation flow with
an explicit return connects start to end through that return's route and any
capture dependency.

## 6. Wires and labels

Description labels carry the exact authored text. A presentation may wrap or
escape that text but must not paraphrase, normalize, or synthesize it. The
expanded-cycle caption is the exception in §4.7: it may show only a prefix and
an ellipsis while retaining the complete description outside the caption. No
node carries a caption naming its block kind; the cycle's non-textual loop
marker is independent of its description. The end node's `()` for an absent
return type states the contract rather than paraphrasing authored text.

An authored question-branch description replaces that branch's output hand-over
label. It appears beside the branch's exit and remains there when the connection
is shared with a later capture or moved to an implicit merge.

Every named flow input and every block output is normally labeled once at the
exit that provides it, or by a shared label as defined below, except for a
question output replaced by its branch description. An expanded cycle is the
exception: neither structural interface repeats the cycle's input or output
list. This includes ordinary labels for wires named `end`, `out`, or `result`
and an intentionally unused wire whose name begins with `_`. The labels at one
exit form its hand-over. The start node hands over its named flow inputs in
signature order, an action and a collapsed cycle hand over all their outputs in
declaration order, and an undescribed question branch or case hands over its
branch output. A block declaring no outputs therefore carries no hand-over
label. A hand-over names newly provided wires only; it neither lists wires that
remain available nor implies that the next node captures every named wire.

A hand-over displays each producer's authored mutability as `name` or
`mut name`, including named flow inputs. This is permission to mutably borrow
the wire, not a capture. Alternative producers declare the same mutability and
retain it in shared merge labels. An authored question-branch description still
replaces its output label.

Every computational block input and collapsed-cycle input is labeled beside its
receiving node. An expanded cycle adds no label for its structural entry. Value
captures are shown as `name` or `mut name`, and borrowed captures as `&name` or
`&mut name`, according to the authored form. The captures are drawn once however
many connections arrive, because the list belongs to the block rather than to an
incoming connection. All forms establish execution dependencies. A wire that
remains available for a later capture may pass virtually along a transitive
connection path without appearing in intermediate hand-overs. This version of
the visual language does not show wire lifetimes or assign a wire to one
particular sequence of connections.

A computational node with an incoming connection and no authored inputs shows
`()` beside its receiving end. This marks an empty capture list, not a wire or a
unit-valued input: a connection passing through a node does not mean that the
node captures the preceding hand-over. Start and case nodes have no such label.
The first computational node also receives a connection from start when it has
no inputs.

The end node labels the value transferred by `return`, rather than the return's
complete capture list. Its input wires are shown as the same comma-separated
list used by every other receiving label, without the transfer expression's
tuple punctuation; a unit value is `()`. This receiving label follows the same
sharing rules as every other capture.

A hand-over and an adjacent capture may share one label only when they are the
two ends of the same connection, that connection is the only one leaving its
source exit and the only one entering its destination node, and the two lists
have the same nonempty displayed names in the same order. `name`, `mut name`,
`&name`, and `&mut name` do not match each other, and an empty hand-over shares
nothing. The shared label represents both connection-end labels.

Identical hand-overs from alternative exits may share one label beside their
merge when each exit has only one outgoing connection and all those connections
reach that merge. The displayed lists must match in both names and order; a
different hand-over remains at its own exit. The shared label represents the
alternative hand-overs, not a new producer at the junction. If the merge has one
outgoing connection, it is the sole connection entering its consumer, and that
consumer's capture list matches the shared hand-over, the same label also
represents the capture. Structural cycle-result junctions are not wire merges
and share no wire label.

Wire labels use logical wire names. A raw identifier appears without its `r#`:
the raw and ordinary spellings of one wire name the same wire, and only the
parameter panel preserves the authored raw spelling of a flow input.

## 7. Connections and implicit convergence

For dependency routing, the start node represents the producers of named flow
inputs, a question's two outputs are distinct exits of its node, and a case node
represents its corresponding choice output.

Consider one finite structural execution after its branches have been selected.
A node participates when its represented block executes or its represented case
is selected. Start always participates; end participates only in a `Return`
outcome. A producer node precedes a consumer node when a capture dependency from
the represented producer occurrence to that consumer participates in the
execution, and this order is transitive. A select node precedes its selected
case node. Wire production and implicit merges also establish the precedence
defined by RFC 0001 §7: a merge follows every producer and every block it
closes, and precedes every block that captures the merged wire. These orderings
participate in the same per-execution reduction as capture dependencies. In a
`Return` outcome, RFC 0001 makes the structural return the last participating
item, so every other participating vertex precedes the end node.

Source order supplies the serial order of participating blocks and transfers.
For each execution, add precedence from start to its first authored item and
between consecutive authored items, including cycle-interface and transfer
junctions. A `Return` outcome continues from its structural return to end; a
`Repeat` outcome continues to the corresponding iteration tail. A question
leaves through its selected exit; a choice continues through its selected case.
With no computational blocks, start leads through the authored cycle or transfer
junctions to an iteration tail or end according to the recorded route. These
relations participate in the same reduction as dependencies and merges. They
express the order the flow is written in without inventing captures.

For example, if two actions capture `left` and `right` after a merge, draw them
in the order they are written on the happy path. A connection between them does
not imply that the second captures the first's output. If a zero-input setup
action is written above a question capturing `condition`, draw
`start → setup → question`: the action's capture label is `()`, its hand-over
still names only `setup`, and the question's capture still names `condition`.

A direct connection joins a participating source exit or junction to a
participating destination node or junction exactly when the source precedes the
destination and no other participating vertex lies between them in this order. A
connection is identified by its source and destination, so connections from
distinct exits remain distinct even when they join the same pair of nodes.

The complete diagram is the union of these connections over all possible
executions. A connection may therefore be direct in one execution and
transitively redundant in another. Each select-to-case connection and every
connection leaving a question or case exit remains associated with its branch.

A drawn connection participates in an execution when both endpoint nodes
participate and its branch, if any, is selected. A connection that is
transitively redundant in that execution adds no new ordering requirement.

The producer occurrence that supplies a capture therefore reaches its consumer
through one or more connections in every execution where that resolution occurs.
A dependency needs no direct producer-to-consumer connection when a sequence of
connections through participating intermediate nodes already represents it. For
example, if `P` produces `x`, `A` borrows `x` and produces `a`, and `C` captures
`x` and `a`, the diagram contains `P → A → C` but no direct `P → C` connection.

Every computational or collapsed-cycle node is reachable from start, including
zero-input blocks. A start, action, or completed-cycle exit continues to the
next item in source order; a consumer that captures nothing from it is reached
transitively through that sequence. A question activates exactly one of its two
exits; a select activates exactly one outgoing connection from its distributor
exit and therefore exactly one case. Branch connections retain their selected
output even when the next step does not capture it directly. A node with several
incoming connections waits for every one that participates in the current
execution. At a convergence, incoming connections from alternative branches
never participate together.

kaalang has no authored merge block or merge icon. Equally named alternative
outputs meet at an implicit junction of connections before any consumer of the
merged wire. The common segment from that junction leads to its consumer or
consumers; the junction is the convergence point, not a consumer node. Which
branches converge there, what finishes before it, and what may bypass it are RFC
0001 §7's rules; the diagram draws them and adds none. When a question or choice
is written between a junction and the consumers of the merged wire, the serial
order routes the junction through that selection. This connection adds no
capture label to it, and connections already represented through it are omitted
by the per-execution transitive reduction. The junction adds neither a
computational block nor a producer occurrence. Consumers display the captured
logical wire name once.

For each expanded cycle, use one representative iteration per finite summary. A
break reaches that cycle's result boundary; a repeating outcome ends at its
iteration tail, whose back edge reaches the entry. The structural return reaches
end. Cycle-interface and transfer junctions participate in the forward
dependency and serial-order rules, including dependencies from their explicit
captures. No iteration-local wire travels along a back edge or crosses the
boundary directly.

The result route of a break occupies its branch column beside the other body
routes. Carry precedence from the tails of the bounded region through its result
boundary and outer continuation to the first wire merge, enclosing iteration
tail, or end. Those boundaries stay below the body; preceding actions,
questions, choices, and cycle entries can start alongside it. This is layout
precedence only, never a drawn execution edge from a repeating tail. Nested back
edges are routed innermost first. A completed inner cycle may feed outer work or
reach the outer iteration tail through that work's normal route. Each authored
block is drawn once; finite summaries do not unroll runtime iterations or prove
termination.

## 8. Spatial notation

Along each forward connection, execution time runs from top to bottom: the
destination node occupies a lower row than its source node, and the route never
moves upward. Nodes on alternative branches may share a row.

The visual language uses columns and rows. Branches are arranged from left to
right in authored order: question answer/output order and choice case order. The
first question answer and the first choice case continue down the current
column; remaining branches appear to their right.

The parameter panel is vertically centred beside start. It does not overlap a
node, connection, or connection label.

Consecutive blocks continue down the current column. After a convergence, the
entry blocks of its shared continuation form a sequence in the column reached by
the group's first branch. An expanded cycle boundary encloses every vertex,
junction, label, and back edge owned by the cycle and leaves its external
interface on the boundary. Nested boundaries do not overlap or interleave. Exact
lower rows, boundary padding, and routing space remain presentation choices.

A convergence group reserves enough columns for its shared continuation,
including any nested question or choice. Later sibling branches start to the
right of that whole area, even when the shared continuation is wider than the
group's incoming branches. The area one branch is held clear of is what the
group draws and that branch does not: a branch is not asked to keep clear of a
vertex it draws itself, which the first branch of a selection could never do at
all, since its column is the selection's own. Branches that never converge
reserve nothing, and their subtrees may interleave.

When end is reachable, it is the final node of the whole diagram: it occupies a
row below every other node and junction, including all iteration tails. Every
iteration back edge stays above end. This is placement order only; root branches
may converge at different depths before the single return. A topology that
cannot keep end last without crossing connections or changing authored branch
order has no conforming diagram.

The structural return reaches end on its arrival rail without declaring a
logical wire merge. Connection routes are simple: they do not intersect or
overlap themselves. They do not cross one another or pass through a non-endpoint
node. Meeting at a common endpoint or deliberately sharing a collinear segment
is not a crossing; connections may share such a segment only when they have the
same source exit or the same destination node. Other connections do not overlap.
Forward connections are plain lines without arrowheads. An iteration back edge
travels upward inside its cycle boundary, outside the body's content, and ends
horizontally with an arrowhead at its entry junction. It clears the whole column
range of that body, including nested cycle boundaries and back edges,
independently of the rows assigned to its vertices. The cycle's outer
continuation does not belong to the body. The common segment below the junction
enters the first body block without an arrowhead. Prefer the right contour when
every repeating route takes the rightmost branch of the first selection in the
body; otherwise prefer the left contour. A cycle without a selection prefers the
left contour. The back edge is the only exception to downward routing and the
only arrowhead. A route contains only straight horizontal and vertical segments,
so every bend is a right angle.

At an implicit merge, side routes finish horizontally at the junction on the
merge rail. The outgoing connection alone owns the vertical below that point: an
incoming side route must not turn down and overlap that continuation. Routes
already in the junction's column descend straight to the same point.

The visual-language contract covers the diagram's nodes, cycle boundaries and
interfaces, parameter panel, roles, labels, the connection end or ends to which
each connection label belongs, branch order, implicit convergence, dependency
reachability, connections, crossing-free orthogonal routing, forward
top-to-bottom row order, and branch column order. Exact dimensions, colors,
typography, spacing, and routing offsets are presentation choices.

Every accepted flow has a conforming expanded diagram. A flow whose expanded
connections and boundaries cannot be drawn under these rules is rejected before
view selection. The collapsed projection is then derived from that validated
model and its own arrangement and geometry are checked. Acceptance carries the
chosen arrangement with the model, so a presentation realizes that one rather
than searching for another.
