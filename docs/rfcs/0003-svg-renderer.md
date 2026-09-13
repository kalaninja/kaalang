# RFC 0003: kaalang SVG Renderer

- Status: accepted design draft
- Visual language: [RFC 0002](0002-visual-language.md)
- Artifact target: SVG

## 1. Overview

The kaalang SVG renderer implements the visual language defined by RFC 0002. It
consumes the validated semantic model also used for execution lowering and does
not independently reinterpret the authored source.

## 2. Layout

The validated model supplies both the topology and its arrangement.
`kaalang_model::build` constructs an arrangement and independently checks it
against RFC 0002 before it returns, so every accepted model carries one and the
renderer chooses no structure of its own. It assigns dimensions and spacing to
the recorded ranks, columns, corridors, lanes, and contours. Everything this
section adds beyond RFC 0002 is a presentation preference, relaxable without
making a topology invalid.

### 2.1 The decision procedure

Construction first tries the finite family of compact arrangements described in
§2.5. A failed candidate, or exhaustion of that family, invokes the deciding
sweep. Within the sweep, straight return climbs are tried first; exhaustion
invokes the same search with variable return positions. Only exhaustion of this
last search establishes impossibility. Each successful candidate is checked by
an independent arrangement verifier before `build` stores it. None of these
searches uses a time, retry, coordinate, or bend limit to reject a topology.

#### Events and persistent constraints

Reverse each return from entry to tail. Forward connections, reversed returns,
and placement-only precedence form a DAG. In particular, precedence puts end
after every other vertex. A **lifeline** is a connection or reversed return
crossing a horizontal cut. A **frontier** lists those lifelines from left to
right; it does not assign them permanent columns.

The persistent horizontal variables are vertex columns, exit columns, and two
bounds for each return's climb. Mandatory column identities are unified before
search: the first branch continues the current column, as do serial blocks; case
nodes reached from a distributor and iteration tails need not continue that
incoming route's temporary column. Branch columns increase in authored order.
Each shared entry equals the minimum approach column of its group's first
branch, as derived by `Regions::approaches`. This is represented by weak
inequalities against all those approaches and a finite choice of which one is
equal. Later-sibling reservation contributes strict inequalities against every
reserved vertex that the sibling does not itself draw.

A return's lower and upper bounds enclose every horizontal coordinate its climb
uses. On the left, its upper bound is strictly below every body column and the
lower bound of every nested return. On the right, its lower bound is strictly
above the body and the upper bounds of nested returns. Body membership includes
junctions and is independent of ranks. The first, straight-return pass equates
the bounds; the deciding pass does not. Extra bends therefore require neither a
new language restriction nor an assumed straightening theorem.

A state consists of the placed vertex set, frontier, chosen return sides, and
the full transitive closure of weak and strict inequalities accumulated so far.
A strict cycle refuses the state; a cycle of weak inequalities identifies equal
coordinates. Global constraints remain in the state after their lifelines have
ended.

There are three kinds of event:

- A vertex other than a case is eligible after all predecessors are placed. Its
  consumed lifelines must be contiguous: its arrival rail cannot cross a
  lifeline that does not end there. It replaces them with its outgoing
  lifelines. Distinct source ports keep their authored order. Routes sharing one
  source may leave in any order. An entry emits its reversed return on the
  chosen side; a tail consumes it at the corresponding outer end of its arrival
  run.
- All case nodes of one choice form a single event, with one anchor per case
  column. Their incoming distributor lifelines must be contiguous and in
  authored case order. Each arrives at its own case column; outgoing groups
  leave in that same order. No other event or routing row can separate cases.
- Adjacent forward lifelines with a common source or destination may exchange
  positions on a shared horizontal rail. No vertex is placed. This is exactly
  the collinear sharing RFC 0002 permits; unrelated routes and returns cannot
  exchange positions this way.

At a vertex or case-row event, its nodes, ports and incident return positions
are anchors; untouched returns also supply anchors. A fixed column has interval
`[x, x]`, a return has its persistent `[low, high]` interval. Ordered anchors
have positions `x_1 < ... < x_k` precisely when each interval is nonempty and
`low_i < high_j` for every `i < j`, allowing arbitrary elastic spacing. These
pairwise inequalities are the exact projection of the event's local variables.
Necessity follows from `low_i <= x_i < x_j <= high_j`. For sufficiency, multiply
any integer solution of the bounds by more than the number of anchors, then
place anchors greedily from left to right, at least one unit apart. Each earlier
lower bound is strictly below the current upper bound; the scale leaves room for
all preceding choices. Extra scaling leaves room for every unanchored forward
lifeline on either side of an event. The implementation reserves four such bands
per anchor interval. Local forward coordinates are otherwise free.

Every eligible vertex or complete case row is explored. Before choosing the next
event, the search enumerates the finite orbit of the frontier under legal
adjacent exchanges, keeping one shortest exchange trace per resulting order.
Exchange rows add no persistent constraint: the return intervals stay in their
order, and the two exchanging forward coordinates are local. An emission
preference groups routes that reach the same merge or tail; the exchange orbit
still includes every permutation within a source group.

#### Coverage: a drawing gives a search path

Take any finite conforming orthogonal drawing. Separate independent event rows
by arbitrarily small vertical perturbations and stretching; all cases of one
choice, ports of a single vertex and a junction's incident ends remain together.
A nonincident route cannot occupy a vertex, so this preserves incidence and
order. Reverse returns and read the frontier between successive vertex events.
Forward and reversed-return paths are monotone, so each crosses the cut once,
apart from horizontal runs which can be separated into routing rows.

Before a case-row event, its incoming paths form an uninterrupted group. They
start at a single distributor, and none ends before that row. An unrelated
lifeline cannot enter the group without crossing one of them: it has neither
their source nor their destination. At their common row the group's order is the
authored case-column order. Thus the case-row transition includes every
conforming placement, and grouping the cases removes no conforming drawing.

When two routes change their left-to-right order, they must meet. RFC 0002
permits this away from a common vertex only on a shared collinear segment of
routes with the same source or destination. Resolve each such change into
adjacent exchanges of the participating routes. Nonsharing pairs keep their
order. The finite exchange orbit contains the resulting permutation; repeated
visits to one cut can be removed because they have no persistent coordinates.
All other sideways motion preserves frontier order and can be replaced by the
strip construction below.

At each vertex, consumed lifelines are contiguous, ports are ordered and the
return lies outside its arrival or departure rail. The drawing's actual vertex
and exit columns satisfy the static constraints. The minimum and maximum x of
each return's climb supply its bounds, including whole-body and nested-return
clearance. The drawing's row positions satisfy every projected anchor
inequality. Its first-branch approach attaining each shared-entry minimum is
among the enumerated equalities. Thus this path is never refused by the column
solver. It reaches a complete state in the deciding pass, even if its returns
need bends or its distributor routes descend out of case order.

#### Soundness: a finished search path gives a drawing

Settle the minimum equalities and number the acyclic inequality relation. Weak
components receive one number; each strict step increases it. Scale these
numbers to reserve the anchor bands, and choose the local anchor positions by
the greedy construction above. Place the event's consumed paths immediately
beside its arrival column, and its emissions beside their port columns, in the
recorded order. A case-row event instead places each sole arrival at its own
case anchor, and each case's emissions beside that anchor. All case boxes share
one rank; the ordered anchors leave room between them. Untouched lifelines fit
between adjacent anchors. Their previous coordinates are retained whenever the
new interval permits it: a left-to-right clamp keeps the frontier ordered and
leaves one position for every remaining path. A sole arrival at a node may
already occupy its column. A tail's arrivals stand on the side opposite its
return, so the return leaves along a rail which overlaps no incoming segment
beyond the tail.

Between events, the lifelines have the same order. Split common-source bundles
on a first horizontal lane. Move leftward paths from left to right, then
rightward paths from right to left, using one lane per move. A path cannot hit a
neighbour: an earlier leftward path has already moved, and a later one starts to
its right; the rightward case is symmetric. This applies to returns as well as
forward paths. A return moves between two points in its interval, so every
horizontal segment stays inside that interval and outside its whole body. Merge
the next event's consumed forward paths on a final common rail. For an exchange
event, its two forward paths use that final rail in opposite directions, as
allowed by their common source or destination.

Node, port, arrival and return rails span only their own contiguous group; no
other lifeline or vertex lies between their ends. Every route is monotone and
simple. Side arrivals into a junction finish on the merge rail rather than
descending over its outgoing connection. Distinct event rows keep precedence,
and the last vertex is end. Each finite strip uses at most one move per live
path plus departure and arrival lanes. Removing unused lanes and compressing all
used x coordinates to consecutive integers preserves every strict order and
equality. Return coordinates use lane zero beside those integer columns; the
separation between distinct columns keeps the harmless lane offset from changing
any order.

The resulting `Arrangement` records all forward and return runs, including
routing-only rows. The verifier reconstructs the polylines independently and
checks coverage, columns, regions, precedence, whole-body and nested-return
clearance, simplicity, permitted meetings, crossings and end placement. A failed
reconstruction is an internal implementation error, never an authored
impossibility diagnostic.

#### State sufficiency and exact reductions

Future eligibility reads only the placed set and frontier. Return sides,
identities, minima and accumulated column inequalities determine every future
constraint. Local coordinates of earlier rows have been eliminated exactly; they
constrain the future only through the retained closure. Any completion of one
prefix with this state is consequently a completion of any other such prefix:
number their common constraints and reconstruct each prefix's strips. A memoized
refusal is therefore exact, including a refusal of the final minimum equalities.
There is no first-ready-vertex commutation reduction.

The other reduction detects a necessary separator obstruction. For an unplaced
vertex `v`, suppose a live path `b` cannot reach `v` but reaches a vertex
required strictly after `v`. It must stay live until `v` is placed. If paths
reaching `v` stand on both sides, neither can cross `b` to reach the common
arrival rail. A future meeting before `v` would make `b` an ancestor of `v`,
contradicting the premise. Already-live edges sharing a source or destination
with `b` are excluded from this argument, because they can exchange positions
immediately. The remaining separation is unavoidable, so refusing it loses no
drawing.

The memo cache stops accepting entries after 64 MiB of estimated payload. This
limits a performance optimization, not the search: uncached states are explored
again and no continuation is omitted. Disabling both memoization and separator
refusal gives the same finite search for comparison tests.

#### Termination and cost

Let `N` be the number of vertices, `W` the number of forward paths and returns,
`L` the number of loops, and `M` the number of persistent column variables after
unification. There are at most `2^N` placed sets, `W!` frontier orders, `3^L`
partially chosen side assignments, and `3^(M*M)` inequality matrices. These are
loose finite bounds; most combinations are inconsistent or unreachable. A
frontier exchange orbit has at most `W!` orders and its retained trace has fewer
than `W!` exchanges. Every recursive placement adds a vertex, so recursion depth
is at most `N`, with a finite orbit and finite choices at each level even when
memoization stores nothing. Settling shared-entry minima is a finite product of
their approach counts. Thus termination does not rely on caching or any budget.

A closure update is at most quadratic in `M`; numbering and reachability are
polynomial. Enumeration is factorial/exponential in the worst case. A witness
uses at most `N * W!` event rows under the loose orbit bound and `W + 2` lanes
per intervening strip. There is no fixed two-bend or two-lane limit. Measured
latency targets are engineering gates and never define validity.

For an explicit loose time bound without memoization, let `A` be the product of
the shared-entry approach counts. The work is bounded by
`N! * 2^L * (W!)^N * A * poly(N + M + W)`: vertex permutations, return sides,
one exchange orbit per vertex, and minimum equalities. If `S` denotes the state
count above and `G` the number of minimum clauses, a loose space bound including
memoization is `O(S * (M² + N + W + L) + N * W! * W + N² + G * M²)`. The terms
account for stored keys, active exchange queues and witness rows, reachability,
and recursive minimum settlement. The actual cache threshold reduces retained
keys; termination and these upper bounds do not depend on it.

### 2.2 Withdrawn restrictions

The reservation rule holds later siblings beyond the area their convergence
group draws and they do not. It does not impose symmetric constraints on earlier
or enclosed siblings. Such a sibling can finish before a group vertex is placed.
The proposed symmetric normalization refused the drawable fixture
`loop/behavior/diverging_middle_branch.rs`; it is absent from both searches and
the verifier.

Authored case-node order does not fix the temporary descent order of routes from
their common distributor: shared-rail exchanges remain legal. However, RFC 0002
§4.5 requires all cases to end on one common row in authored order. A
distributor detour cannot lower an enclosed exit case below its siblings or
their iteration tail. `loop/compile_fail/enclosed_break.rs`,
`terminal_case_between_repeats.rs`, and `two_terminal_cases_between_repeats.rs`
record these refusals. The author must change the case order to make such a flow
drawable; construction never reorders cases.

### 2.3 Independent checks

Before rendering, `SemanticModel::compact_arrangement` may simplify a witness
with long routing detours. This is optional presentation work, outside macro
compilation. It shortens successive runs, joins compatible horizontal lanes,
closes unused column space, brings straight contours toward their bodies and
lifts vertices into earlier ranks. It may not split a choice's common case row.
Each candidate is normalized and checked by the complete arrangement verifier
before replacing the current witness. The SVG renderer realizes that checked
replacement. An unsuccessful simplification keeps the current witness and never
rejects a flow; it is not a second decision procedure or an additional language
restriction. The normal form remains the fallback, not the required appearance.

Normalization preserves the position of junctions relative to routing lanes. An
unused last lane above a junction can separate a foreign bend from its straight
arrival or return. Such a lane cannot be removed merely because no sideways run
occupies it.

The arrangement verifier derives body and region membership from the topology,
not from the chosen event order. Pixel verification independently checks all
segments, junction incidence, contour extent, common case rows and end
placement. Witness correspondence additionally checks columns and recorded
return corridors; a crossing-free drawing of a different witness does not
satisfy it.

The test-only reference uses separate event enumeration, a sparse difference
constraint graph with strongly connected components and longest paths, and a
separate strip router. It retains fresh coordinates for every live path on each
row instead of production's interval projection and transitive-closure key.
Positive-only preliminary attempts speed up witness discovery; their exhaustion
always falls through to the full enumeration. Its external test guard panics on
exhaustion and is never counted as a negative answer. It shares topology facts
and the independent verifier, not production transitions, pruning, column solver
or reconstruction.

Exhaustive comparisons include both distributor and ordered-question bodies,
with drawn, impossible and earlier-semantic outcomes counted separately. Both
domains contain topology refusals, including exits enclosed between converging
routes when the case row cannot be split. Separate tests compare the production
search with both reductions disabled and check that shared-rail exchanges cannot
separate a case from its siblings. Mutated witnesses check far contours, return
bends, nested return order, body junctions, lanes, reserved columns, end
placement and disconnected incident routes. Agreement of test sets supplements
the correspondence argument; it does not replace it.

The rule-to-constraint inventory is:

| Required rule                                | Construction and independent check                                                                     | Regression                                                                                                                                |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Forward and placement order                  | DAG predecessors; `verify::order`                                                                      | `the_verifier_rejects_every_mutation`                                                                                                     |
| Authored port/case order and common case row | Static exit/case inequalities; indivisible case-row events; `choice::verify`, `verify::branch_columns` | `the_verifier_rejects_a_case_on_a_different_row`, `all_cases_of_a_choice_share_a_row`                                                     |
| Serial and shared-entry columns              | Column identities and minimum clauses; `serial_columns` and `branch_columns`                           | `shared_entries_keep_the_first_branch_approach_column`                                                                                    |
| Later-sibling reservation                    | Persistent region inequalities; `reserved_columns`                                                     | `a_sibling_inside_a_reserved_footprint_is_caught`                                                                                         |
| Noncrossing simple orthogonal routes         | Contiguous events and ordered strips; `verify::routes`                                                 | `a_shared_rail_cannot_lower_a_case_past_its_siblings`                                                                                     |
| Actual junction incidence and merge arrival  | Final common rail; junction coordinates and pixel incidence                                            | `a_crossing_uses_the_junctions_actual_merge_lane`, `side_routes_end_horizontally_at_the_merge`                                            |
| Whole-body and nested-return clearance       | Return envelopes; `verify::returns` and pixel `route::verify_returns`                                  | `body_columns_do_not_shrink_when_a_body_vertex_moves_below_the_tail`, `a_return_inside_a_nested_return_is_caught_by_the_geometry_check`   |
| Recorded contour column, lane and bends      | Complete return routes; coverage, polyline and correspondence checks                                   | `a_return_can_bend_outside_its_body`, `a_return_keeps_its_recorded_bend_and_clears_labels`, `four_nested_returns_are_drawn_in_four_lanes` |
| End last, including after tails              | End precedence; both end verifiers                                                                     | `a_misplaced_end_is_caught_by_the_geometry_check`                                                                                         |
| Finite captions and panel clearance          | Measured rows and gaps; label, geometry and canvas checks                                              | `every_generated_shape_the_model_accepts_also_renders`, caption and contour mutation tests                                                |

### 2.4 Why every checked arrangement has a realization

Measured boxes decide spacing, never structure. The map from the arrangement to
pixels is order-preserving and its spacing is chosen after the measuring, so a
checked arrangement always has a drawing and the renderer never has to look for
another one.

A column becomes an x through one map, `column_x`, used by every node, every
route and every return alike, and that map is strictly increasing in the column
— so two nodes share an x exactly when the arrangement gave them one column, and
the branch order and every reserved footprint carry over into the nodes that
occupy them. A junction is the exception, and deliberately: it owns no box, its
position is where its routes meet, and §2.5 lets a compaction move that in
across a column boundary rather than reserve an empty column for it. What holds
a junction is the crossing rules over the routes that meet there and the contour
rules over the rail that leaves it, both checked after every compaction. An
outermost column used only by an iteration tail, its sole straight incoming
corridor and its return may close with that arrival. Its contour boundary moves
with the tail, staying outside every other drawn route. Any other vertex, exit,
route or contour using that column or a column beyond it prevents this
compaction; a separately recorded far contour keeps its original boundary.
Geometry, labels and witness correspondence are checked before keeping the
change. The fallback distance between two columns is chosen once, as the widest
of the standard column, a node box with the deepest rail reaching into the gap
from each side and a lane between them, and whatever slack a previous pass asked
for. So every node box fits inside its own column and every contour lane the
arrangement used fits beside it, because the width was measured from those two
things. A rank becomes a row whose height is the tallest box on it and whose gap
holds the lanes the arrangement recorded for it plus the labels that hang there,
each measured before the row is placed.

For straight-return drawings, a presentation first tries smaller gaps around
columns carrying only routes or junctions. Node and exit columns retain their
measured half-width allocation, including space for branch descriptions; empty
columns need only a lane and any requested slack. All geometry, label and
correspondence checks still apply. If these smaller gaps fail, the uniform
spacing above realizes the same witness. Drawings with recorded return bends
retain the common uniform map below.

One conflict is not settled by measuring, and it is the only one: a label
wrapped to a fixed width beside one column and a return climbing in the same gap
can want the same pixel, and no rail position clears it, because stepping out
goes further into the label and stepping in goes into the body. The room comes
from the columns, through the slack above, and that ends: the label width and
the node boxes are fixed and the slack does not change them, so once the gap
holds the widest label a connection can hang beside the deepest a rail reaches
past a body, the two cannot overlap. A pass asking for more than that is a
renderer defect rather than an arrangement the columns cannot hold.

When a witness records bends in a return, all returns use one common map from
column, side and lane to pixels. It reserves the measured node half-width and
the fixed wrapped-label width before the first contour lane on each side. Column
gaps hold both sides at once. Each recorded run uses its own rank-gap lane and
is emitted in reverse order, because return routes are recorded from entry to
tail. This direct realization keeps its bends and does not undergo
straight-return compaction. The ordinary straight-return realization retains the
compact geometry and bounded label-clearance adjustment above.

The compactions below are transformations of a realization that already
conforms, each kept only if the result still conforms and still realizes the
arrangement; the uncompacted realization is kept and drawn when one does not.

The arrangement records how many lanes each return climbs beside its column, and
a presentation holds the columns far enough apart for them. Nothing about a
renderer's spacing limits which arrangements are admissible: a topology never
needs more lanes beside one column than it has loops, because only a loop return
climbs there.

### 2.5 Presentation

Every emitted layout is computed deterministically, including node dimensions,
row and column assignments, label placement, and connection routing. It
preserves the row and column ordering and connection-routing constraints
required by RFC 0002. A question or choice nested inside a branch receives
columns that do not overlap those assigned to other branches of the enclosing
question or choice.

A **footprint** is the columns one convergence group and its shared continuation
occupy. Disjoint convergence groups of the same question or choice receive
disjoint footprints in authored branch order. Nested convergence groups of that
question or choice may share columns. The footprint is not asked to be a
contiguous range: RFC 0002 §8 holds a later sibling right of what the group
draws and that sibling does not, vertex by vertex, and an earlier or enclosed
sibling is held to nothing (§2.2).

When a shared continuation has one entry block, its node occupies the column in
which the first branch of its convergence group reaches it. "First" follows the
branch order defined by RFC 0002. This is normally that branch's own column; if
the branch contains a nested question or choice, its route may reach that node
in another column. Other branches of the group route to the same node.

When a shared continuation has several entry blocks, draw them sequentially in
source order. They share the column reached by the group's first branch. Other
branches meet before the sequence; connections carry later captures transitively
through it.

A later sibling inside a footprint is an arrangement the construction discards,
not a diagram this renderer declines to draw: the model derives each group and
its area from the topology and holds every arrangement to RFC 0002 §8. What the
sibling is held clear of is what the group draws and it does not; a vertex both
reach is common ground.

To draw an implicit merge, including the `end` merge above end, each producer
descends in its approach column to one horizontal merge rail whose junction lies
in the continuation's column; routes meet it as RFC 0002 §8 requires.

Every iteration tail sits on a row of its own, with no node beside it. A return
leaves its tail horizontally, across every column between the tail and its
contour, so anything else on that row would stand in its way. This costs one row
gap per repeating loop and changes no column.

That gap is given back where the return does not use it. Turning upward at a
side exit leaves the tail's row holding nothing but the connections that pass
through it, and such a row is closed: the connections crossing it shorten, every
row below it moves up, and two consecutive blocks are left one gap apart. A row
is closed only when no route bends, starts, or ends on it, so closing one moves
nothing into anything else, and the crossing rules of RFC 0002 §8 are checked
again over the result.

Any other junction may share its row with a node: a merge rail reaches only from
its own producers to the continuation's column, and the crossing rules of RFC
0002 §8 refuse an arrangement in which such a rail would run through a node, so
no separate row is needed to keep it clear.

Use one vertical gap throughout a diagram, at least 72 pixels and enlarged when
labels need more room. Leave at least that gap between consecutive node rows.
For horizontal runs in row gaps, reserve it both below the preceding node row
and above the following node row. Wire-merge junctions, loop entries, iteration
tails, and breaks have no capture labels. Additional horizontal lanes stay 20
pixels apart. This gives select distributors, merge rails, and iteration tails
the same clearance.

A loop entry does not by itself reserve an extra node row. When there is room,
lift the entry and its first body node together to align the node with an
earlier sibling row. Keep the usual gap on both sides of the entry and retain
the original geometry if crossings, labels or witness correspondence prevent the
move.

Questions in loop bodies use the ordinary question layout and authored answer
order. Loops and breaks have no computational nodes. Each repeating loop has an
iteration tail and a return to its entry; an empty body connects entry directly
to tail. A break with no data captures redirects the incoming route without an
intermediate vertex. Data captures use an unlabeled exit junction, which stays
in its incoming branch column before connecting to the target continuation.
Chains of structural junctions use their downstream columns instead of falling
back to the root column. Unit-valued question outputs that only gate structural
statements remain labeled at the question exit and add no capture label or
placement row.

A loop without a repeating route or data captures contributes no entry vertex to
the placement graph. Its body keeps the same geometry it would have without that
enclosing loop.

Forward connections and iteration-tail precedence form an acyclic placement
graph. Carry each exited region's tail precedence through continuation blocks
and structural entry or exit junctions. Stop at the first wire merge, iteration
tail, or end; those boundaries stay below the body. The continuation's preceding
blocks may fill their branch columns alongside the body, regardless of block
kind or branch order. This precedence is not drawn. A break that finishes an
enclosing iteration connects directly to that iteration's tail. Equivalent
question-and-break loop routes retain their geometry; structural syntax alone
does not introduce rows or detours.

Return connections are drawn separately, innermost loop first, around the body
and horizontally into its entry junction. The arrowhead belongs to that
horizontal arrival. Returns may move upward; all other geometry checks still
apply. The iteration tail meets its incoming branches at the end of its rail
nearest the side the checked arrangement chose for that return.

The arrangement records the side of the body, the outermost column of that body,
and the lane beside it each return climbs. Geometry realizes that side and that
lane: it puts lane 0 just past everything the body draws, and one lane step
further out for each later lane. Body membership is independent of ranks, so
neither placing a body vertex below the tail nor turning the return upward early
shrinks what it has to clear. The recorded column is not read as a pixel
position, because a vertex's pixel position is not `column_x` of its abstract
column once a return turns upward at a side exit or a sole arrival is compacted.

Columns stand far enough apart to hold the lanes the arrangement used, and a gap
two returns climb into from opposite sides is widened until the two cannot land
on the same line.

A tail with one incoming connection may sit on a side exit's horizontal run, or
after the usual vertical gap below a straight exit. Prefer turning upward there
over descending to the tail's placement row and immediately returning upward.
This is a compaction of already valid geometry: it brings the tail and its
return in together, keeps the side and lane the arrangement chose, and is
dropped when the shortened route would break RFC 0002 §8, leaving the
uncompacted arrangement in place.

A sole rightward horizontal arrival may also shorten toward the body instead of
reserving an empty branch column. Move its tail and return together, keeping the
return beyond the nodes and the ink and halo of labels along its vertical span.
The return still clears the whole body's extent. Keep the original route when
the compact one would cross another connection. Node columns and branch order do
not change.

End is ordered after every other vertex during construction, as RFC 0002 §8
requires. No construction or compaction may put a loop return below it.

After routing the returns, adjust a reachable terminal `end` merge and end using
the actual geometry. Leave the usual vertical gap below all other nodes. A
terminal rail may align with an independent return; where their horizontal spans
overlap, leave the usual vertical gap between them. Move the terminal merge and
end together, preserving their common segment's length; when the end block has
no terminal merge, move the block alone. A return beside end may align with its
top edge, but no connection may descend below that edge. This keeps end last
without adding an empty row when the return already clears it. Move upward to
remove excess space or downward to provide the required clearance. Keep the
original placement if the adjustment would introduce a crossing or upward
segment. The geometry check also requires end below all other nodes and loop
returns. Place labels after this adjustment.

## 3. SVG output

The renderer produces standalone SVG with embedded styles and no JavaScript,
external fonts, or external rendering programs. It lightly tints nodes by block
kind while retaining shape and labels as independent type indicators. The
presentation choices left open by RFC 0002 are internal to the renderer and may
change without changing the visual language.

Serialize forward connections as subpaths of one SVG path, so shared
distributors and merge rails are stroked once without darkening their
antialiased edges. Keep return paths separate for their arrowhead markers.

The renderer preserves Unicode text and escapes XML content. For the start label
and each row of the parameter panel, it takes the source content defined by RFC
0002 and collapses each run of whitespace to one space before wrapping it to its
allotted width. It applies no other renderer-specific rewrite. Other long labels
also wrap to their allotted width. Wrapping breaks only between grapheme
clusters; an otherwise unbreakable word is split rather than drawn outside its
node or panel.

A question-branch description uses its own branch-label style beside the exit
and replaces the output label. Its font is larger than a wire label's font, and
layout uses that size when wrapping text and placing the following row. The
first branch's description hangs below its downward exit; the second branch's
description sits above its horizontal exit. Return arrowheads use an embedded
SVG marker, and the accessible description identifies the iteration tail and the
loop entry. Loop-entry and break captures have no labels.

## 4. Library interface

The library entry point is:

```rust
kaalang_svg::render_source(source: &str, flow_name: &str)
    -> Result<String, RenderError>
```

It finds the named top-level `#[kaalang]` function in the provided UTF-8 Rust
source text and builds its validated semantic model. It then either lays out and
serializes that model or returns a rendering error. It does not invoke
`cargo check` or perform full Rust type checking.

`RenderError::InvalidFlow` reports every error from `kaalang_model::build`,
including an impossible topology and an internal construction error.
`RenderError::UnroutableTopology` reports a renderer defect in geometry, labels
or witness correspondence after assigning dimensions and spacing. Failed
compaction falls back to the uncompacted realization, so it cannot normally
produce this error. Final route, label and correspondence checks remain
necessary after that assignment.

## 5. Command-line interface

The command-line interface is:

```text
cargo kaalang diagram <source.rs> --flow <name> [-o <path>]
```

Without `-o`, the command writes `./<name>.svg`. It creates or replaces the
output only after the complete source has been validated and rendered.

## 6. Scope and limits

The renderer does not search an entire crate or resolve external modules,
conditional compilation, or macro-expanded source. It does not support batch
rendering, official `build.rs` integration, interactive HTML, a public JSON
descriptor, or alternative output formats.

These limits constrain the SVG renderer, not the semantic model or other visual
representations.
