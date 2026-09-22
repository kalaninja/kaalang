# RFC 0003: kaalang SVG Renderer

- Status: accepted
- Visual language: [RFC 0002](0002-visual-language.md)
- Artifact target: SVG

## 1. Overview

The kaalang SVG renderer implements the visual language defined by RFC 0002. It
consumes the validated semantic model also used for execution lowering and does
not independently reinterpret the authored source.

## 2. Layout

The validated model supplies both the topology and its arrangement.
`kaalang_compiler::build` validates the complete expanded topology and
independently checks its arrangement against RFC 0002 before it returns. For a
collapsed view, the model layer projects each validated cycle boundary to one
node and checks a second arrangement for that projection. Before any concrete
renderer measures it, `kaalang-render` may compact that witness into the common
rows, columns, corridors, lanes, and contours. A concrete renderer chooses no
structure of its own; it assigns dimensions and spacing to the shared
arrangement. Sections 2.1–2.4 describe construction and verification of RFC
0002's rules; §2.5 defines presentation preferences.

### 2.1 The decision procedure

Construction first tries the finite family of compact arrangements described in
§2.5. Exhausting that family, or a candidate failing independent verification,
invokes the deciding sweep. Within the sweep, straight iteration back edge
climbs are tried first; exhaustion invokes the same search with variable back
edge positions. Only exhaustion of this last search establishes impossibility.
Each successful candidate is checked by an independent arrangement verifier
before `build` stores it. None of these searches uses a time, retry, coordinate,
or bend limit to reject a topology.

#### Events and persistent constraints

Reverse each cycle back edge from entry to tail. Forward connections, reversed
back edges, and placement-only precedence form a DAG. In particular, precedence
puts end after every other vertex. A **lifeline** is a connection or reversed
back edge crossing a horizontal cut. A **frontier** lists those lifelines from
left to right; it does not assign them permanent columns.

The persistent horizontal variables are vertex columns, exit columns, and two
bounds for each back edge's climb. Mandatory column identities are unified
before search: the first branch continues the current column, as do serial
blocks; case nodes reached from a distributor and iteration tails need not
continue that incoming route's temporary column. Branch columns increase in
authored order. Each shared entry equals the minimum approach column of its
group's first branch, as derived by `Regions::approaches`. This is represented
by weak inequalities against all those approaches and a finite choice of which
one is equal. Later-sibling reservation contributes strict inequalities against
every reserved vertex that the sibling does not itself draw.

A back edge's lower and upper bounds enclose every horizontal coordinate its
climb uses. On the left, its upper bound is strictly less than every body column
and the lower bound of every nested back edge. On the right, its lower bound is
strictly greater than every body column and the upper bounds of nested back
edges. Body membership includes junctions and is independent of ranks. The
first, straight back edge pass equates the bounds; the deciding pass does not.
The deciding pass therefore includes back edges with bends.

A state consists of the placed vertex set, frontier, chosen back edge sides, and
the full transitive closure of weak and strict inequalities accumulated so far.
A strict cycle refuses the state; a cycle of weak inequalities identifies equal
coordinates. Global constraints remain in the state after their lifelines have
ended.

There are three kinds of event:

- A vertex other than a case is eligible after all predecessors are placed. Its
  consumed lifelines must be contiguous: its arrival rail cannot cross a
  lifeline that does not end there. It replaces them with its outgoing
  lifelines. Distinct source ports keep their authored order. Routes sharing one
  source may leave in any order. An entry emits its reversed back edge on the
  chosen side; a tail consumes it at the corresponding outer end of its arrival
  run.
- All case nodes of one choice form a single event, with one anchor per case
  column. Their incoming distributor lifelines must be contiguous and in
  authored case order. Each arrives at its own case column; outgoing groups
  leave in that same order. No other event or routing row can separate cases.
- Adjacent forward lifelines with a common source or destination may exchange
  positions on a shared horizontal rail. No vertex is placed. This is exactly
  the collinear sharing RFC 0002 permits; unrelated routes and back edges cannot
  exchange positions this way.

At a vertex or case-row event, its nodes, ports and incident back edge positions
are anchors; untouched back edges also supply anchors. A fixed column has
interval `[x, x]`, a back edge has its persistent `[low, high]` interval.
Ordered anchors have positions `x_1 < ... < x_k` precisely when each interval is
nonempty and `low_i < high_j` for every `i < j`, allowing arbitrary elastic
spacing. These pairwise inequalities are the exact projection of the event's
local variables. Necessity follows from `low_i <= x_i < x_j <= high_j`. For
sufficiency, multiply any integer solution of the bounds by more than the number
of anchors, then place anchors greedily from left to right, at least one unit
apart. Each earlier lower bound is strictly below the current upper bound; the
scale leaves room for all preceding choices. Extra scaling leaves room for every
unanchored forward lifeline on either side of an event. Local forward
coordinates are otherwise free.

Every eligible vertex or complete case row is explored. Before choosing the next
event, the search enumerates the finite orbit of the frontier under legal
adjacent exchanges, keeping one shortest exchange trace per resulting order.
Exchange rows add no persistent constraint: the back edge intervals stay in
their order, and the two exchanging forward coordinates are local. An emission
preference groups routes that reach the same merge or tail; the exchange orbit
still includes every permutation within a source group.

#### Coverage: a drawing gives a search path

Take any finite conforming orthogonal drawing. Separate independent event rows
by arbitrarily small vertical perturbations and stretching; all cases of one
choice, ports of a single vertex and a junction's incident ends remain together.
A nonincident route cannot occupy a vertex, so this preserves incidence and
order. Reverse back edges and read the frontier between successive vertex
events. Forward and reversed back edge paths are monotone, so each crosses the
cut once, apart from horizontal runs which can be separated into routing rows.

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
back edge lies outside its arrival or departure rail. The drawing's actual
vertex and exit columns satisfy the static constraints. The minimum and maximum
x of each back edge's climb supply its bounds, including whole-body and
nested-edge clearance. The drawing's row positions satisfy every projected
anchor inequality. Its first-branch approach attaining each shared-entry minimum
is among the enumerated equalities. Thus this path is never refused by the
column solver. It reaches a complete state in the deciding pass, even if its
back edges need bends or its distributor routes descend out of case order.

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
already occupy its column. A tail's arrivals stand on the side opposite its back
edge, so the back edge leaves along a rail which overlaps no incoming segment
beyond the tail.

Between events, the lifelines have the same order. Split common-source bundles
on a first horizontal lane. Move leftward paths from left to right, then
rightward paths from right to left, using one lane per move. A path cannot hit a
neighbour: an earlier leftward path has already moved, and a later one starts to
its right; the rightward case is symmetric. This applies to back edges as well
as forward paths. A back edge moves between two points in its interval, so every
horizontal segment stays inside that interval and outside its whole body. Merge
the next event's consumed forward paths on a final common rail. For an exchange
event, its two forward paths use that final rail in opposite directions, as
allowed by their common source or destination.

Node, port, arrival and back edge rails span only their own contiguous group; no
other lifeline or vertex lies between their ends. Every route is monotone and
simple. Side arrivals into a junction finish on the merge rail rather than
descending over its outgoing connection. Distinct event rows keep precedence; a
later side exit may meet its wire merge on the same row, with precedence carried
by the horizontal segment. Compaction may also place an iteration tail on the
row of its sole side arrival. When end is present, it is the last vertex. Each
finite strip uses at most one move per live path plus departure and arrival
lanes. Removing unused lanes and compressing all used x coordinates to
consecutive integers preserves every strict order and equality. Back edge
coordinates use lane zero beside those integer columns; the separation between
distinct columns keeps the harmless lane offset from changing any order.

The resulting `Arrangement` records all forward and back edge runs, including
routing-only rows. Each sideways run explicitly names either a rank line or a
lane in a rank gap. Every junction's final sideways arrival names its rank and
does not occupy a gap lane, including cycle entries, iteration tails, breaks and
results. The planner checks the route at that recorded position; verification
and rendering never relocate the final run based on its destination kind. The
verifier reconstructs the polylines independently and checks coverage, columns,
regions, precedence, whole-body and nested back edge clearance, simplicity,
permitted meetings, crossings and end placement. A failed reconstruction is an
internal implementation error, never an authored impossibility diagnostic.

#### State sufficiency and exact reductions

Future eligibility reads only the placed set and frontier. Back edge sides,
identities, minima and accumulated column inequalities determine every future
constraint. Local coordinates of earlier rows have been eliminated exactly; they
constrain the future only through the retained closure. Any completion of one
prefix with this state is consequently a completion of any other such prefix:
number their common constraints and reconstruct each prefix's strips. A memoized
refusal is therefore exact, including a refusal of the final minimum equalities.

The other reduction detects a necessary separator obstruction. For an unplaced
vertex `v`, suppose a live path `b` cannot reach `v` but reaches a vertex
required strictly after `v`. It must stay live until `v` is placed. If paths
reaching `v` stand on both sides, neither can cross `b` to reach the common
arrival rail. A future meeting before `v` would make `b` an ancestor of `v`,
contradicting the premise. Already-live edges sharing a source or destination
with `b` are excluded from this argument, because they can exchange positions
immediately. The remaining separation is unavoidable, so refusing it loses no
drawing.

Bounding the memo cache limits a performance optimization, not the search:
uncached states are explored again and no continuation is omitted. Disabling
both memoization and separator refusal gives the same finite search for
comparison tests.

#### Termination and cost

Let `N` be the number of vertices, `W` the number of forward paths and back
edges, `L` the number of cycles, and `M` the number of persistent column
variables after unification. There are at most `2^N` placed sets, `W!` frontier
orders, `3^L` partially chosen side assignments, and `3^(M*M)` inequality
matrices. These are loose finite bounds; most combinations are inconsistent or
unreachable. A frontier exchange orbit has at most `W!` orders and its retained
trace has fewer than `W!` exchanges. Every recursive placement adds a vertex, so
recursion depth is at most `N`, with a finite orbit and finite choices at each
level even when memoization stores nothing. Settling shared-entry minima is a
finite product of their approach counts. Thus termination does not rely on
caching or any budget.

A closure update is at most quadratic in `M`; numbering and reachability are
polynomial. Enumeration is factorial/exponential in the worst case. A witness
uses at most `N * W!` event rows under the loose orbit bound and `W + 2` lanes
per intervening strip. Measured latency targets never define validity.

For an explicit loose time bound without memoization, let `A` be the product of
the shared-entry approach counts. The work is bounded by
`N! * 2^L * (W!)^N * A * poly(N + M + W)`: vertex permutations, back edge sides,
one exchange orbit per vertex, and minimum equalities. If `S` denotes the state
count above and `G` the number of minimum clauses, a loose space bound including
memoization is `O(S * (M² + N + W + L) + N * W! * W + N² + G * M²)`. The terms
account for stored keys, active exchange queues and witness rows, reachability,
and recursive minimum settlement. The actual cache threshold reduces retained
keys; termination and these upper bounds do not depend on it.

### 2.2 Branch and case placement

The reservation rule constrains later siblings to remain beyond the vertices
that their convergence group draws and they do not. Earlier or enclosed siblings
may finish before a group vertex is placed.

Routes from a common distributor may exchange positions on shared rails, but all
cases must occupy one row in authored order. A detour cannot lower an enclosed
completion case below its siblings or their iteration tail. If that order makes
the flow impossible to draw, validation rejects it.

### 2.3 Independent checks

Before rendering, `kaalang_render::compact_arrangement` simplifies repeating
cycles and witnesses with long routing detours. This is optional presentation
work, outside macro compilation and shared by concrete renderers. It shortens
successive runs, joins compatible horizontal lanes, closes unused column space,
brings straight contours toward their bodies and lifts vertices into earlier
ranks. It may not split a choice's common case row. Each candidate is normalized
and checked through `kaalang_render::ArrangementVerifier` before replacing the
current witness. The verifier composes `kaalang_compiler::ArrangementChecks`
with cached cycle boundaries and the renderer-only exception that lets a
completion rise beside the cycle it leaves. A concrete renderer realizes that
checked replacement. An unsuccessful simplification keeps the current witness
and never rejects a flow; it is not a second decision procedure or an additional
language restriction. The normal form remains the fallback, not the required
appearance. Compaction reaches a deterministic local fixed point; it does not
promise a global minimum of bends or area.

Normalization keeps all junctions on their rank lines and removes unused gap
lanes. Rank runs do not reserve gap lanes. Lifting a junction moves its arrival
runs with its rank, and folding rows remaps both rank runs and gap lanes before
the candidate is checked.

A cycle's authored break and result are one topology junction. This avoids an
empty transfer row without merging separate vertices during measurement.
Alternative exit routes converge at ordinary wire merges. When the result's only
incoming connection is the merge's only outgoing connection, the topology uses
that merge as the result, retaining its marker and wire label. No separate
result row is needed after the final merge. The lower boundary contains the
merge's complete wrapped label even when it extends beyond the usual result
padding.

The arrangement verifier derives body and region membership from the topology,
not from the chosen event order. Pixel verification independently checks all
segments, junction incidence, contour extent, common case rows and end
placement. Witness correspondence additionally checks columns, cycle boundaries,
and recorded back edge corridors; a crossing-free drawing of a different witness
does not satisfy it.

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
separate a case from its siblings. Mutated witnesses check far contours, back
edge bends, nested back edge order, cycle boundaries, body junctions, lanes,
reserved columns, end placement and disconnected incident routes. Agreement of
test sets supplements the correspondence argument; it does not replace it.

The independent checks cover:

| Rule                                           | Check                                                 |
| ---------------------------------------------- | ----------------------------------------------------- |
| Forward order and end placement                | DAG precedence and geometry order                     |
| Authored branch order and common case row      | Exit inequalities and case-row verification           |
| Serial and shared-entry columns                | Column identities and minimum constraints             |
| Later-sibling reservation                      | Region inequalities and reserved-column checks        |
| Simple orthogonal routes and permitted sharing | Arrangement polylines and pixel intersections         |
| Junction incidence and merge arrivals          | Recorded rank lines and endpoint coordinates          |
| Whole-body and nested back edge clearance      | Body membership, contours, and pixel envelopes        |
| Cycle boundary containment                     | Owned vertices, routes, labels, and nested boundaries |
| Label and parameter-panel clearance            | Measured text, geometry, and canvas bounds            |

### 2.4 Why every checked arrangement has a realization

Measurement assigns pixels to the checked arrangement without changing its
structure. One strictly increasing `column_x` map positions every node,
junction, route, and back edge. Nodes sharing a column share an x coordinate;
branch order and reserved areas retain their horizontal order.

Each column gap is large enough for its content, the contours that enter that
gap from either side, a separating lane, and any label-clearance slack. A deep
contour widens only the gap that contains it. A rank's height fits its tallest
node or merge marker. Its gap holds the recorded routing lanes and measured
connection labels. All junctions retain their recorded row centres, including
those without markers.

For straight back edges, the renderer first tries narrower gaps around columns
containing only routes or junctions. Node and exit columns retain their measured
widths, including branch descriptions. If that attempt fails geometry, label, or
correspondence checks, standard spacing realizes the same arrangement. Drawings
with bent back edges use standard spacing except where their recorded contours
need a wider gap.

A label and a straight back edge can compete for the same column gap. The
renderer widens that gap in finite steps. Wrapped label widths and node sizes
are fixed, so the maximum necessary slack is bounded by the widest label plus
the contours' reach and one lane. A conflict beyond that bound is a renderer
defect, not an invalid topology.

Bent back edges use a common map from column, side, and lane to pixels. It
reserves node and label width before the first contour lane, with room for both
sides in each column gap. Recorded back edge runs are reversed from their
entry-to-tail arrangement order for drawing. Straight back edges retain their
entry and tail rows while their climbs may move outward to clear labels. Neither
form moves individual vertices or junctions.

An expanded cycle boundary encloses its owned nodes, junctions, labels, routes,
and back edges. Nested boundaries contribute their complete boxes to the parent
footprint. Padding extends above the entry and below the lowest body content;
labels can enlarge that footprint. When an entry is the first body node or a
result is supplied directly by a body exit, it adds no separate row. Nested
boundaries ending on one row stack their bottom padding, with room reserved
before the following row. A collapsed cycle uses ordinary node spacing.

The cycle caption fits at most two small lines in the existing top padding,
right-aligned beyond incoming routes and labels. It may end in an ellipsis or be
omitted if even that cannot fit. The full description remains in the tooltip and
accessible text. Captions never enlarge cycle boundaries.

### 2.5 Presentation

Layout is deterministic. Compact construction first tries arrangements with
preferred back edge sides, unsunk iteration tails, and direct routes into
destination columns. On a conflict, it varies three finite choices: which tails
sink below otherwise independent vertices, which side each back edge takes, and
whether a forward route turns near its source, near its destination, or outside
the intervening content. Exhausting these choices invokes §2.1's sweep.

A convergence group's **footprint** comprises the columns of its branches and
shared continuation. Shared continuation blocks run sequentially in source order
in the column reached by the group's first branch. A nested selection can make
that approach differ from the branch's initial column. Later siblings stay to
the right of reserved vertices under §2.2; footprints need not be contiguous.

At a wire merge, earlier producers descend to the junction's row. A later
branch's side exit may reach it horizontally on that same row. Side arrivals end
at the merge marker; only the outgoing connection continues below it. End has a
separate arrival rail and remains below every other vertex and back edge.

Iteration tails initially occupy separate rows. Checked compaction may align a
tail with its sole side arrival, or a cycle result with its tail when their
routes are disjoint. It may also remove unused rows, columns, and routing lanes.
Compaction preserves complete nested cycle envelopes as well as individual
routes. Measurement retains the resulting rows and columns.

Carry a completed region's tail precedence through its result and continuation
to the first wire merge or end. These boundaries stay below the body, while
preceding continuation blocks may stand alongside it. An enclosing tail reached
only by a side exit needs no extra precedence from alternative inner outcomes;
other tails retain it to clear nested boundaries. These are placement
constraints, not drawn execution connections.

Each vertical row gap is at least 72 pixels and is enlarged only for labels
drawn in that gap. Measure it from the edges of occupied rows, including merge
markers. Horizontal routes in a row gap leave that gap's clearance above and
below; additional lanes are 20 pixels apart. Structural junctions have no box
height or duplicate capture labels. End labels its transferred value under RFC
0002 §6.

Route back edges innermost first. The tail meets its arrivals at the end of its
rail nearest the chosen contour. The climb stays outside the complete body,
including nested boundaries, regardless of the rows occupied by that body. Its
recorded contour column uses the same `column_x` map as vertices; lane zero sits
beyond that column's content, and each later lane adds 20 pixels. Leave at least
one lane between an enclosing back edge and a nested boundary. Opposing climbs
in one gap must remain distinct. A bent climb follows its recorded runs. The
arrowhead belongs to the horizontal arrival at the entry junction.

## 3. SVG output

The renderer produces standalone SVG with embedded styles and no JavaScript,
external fonts, or external rendering programs. It lightly tints nodes, mostly
by block kind, while retaining shape and labels as independent type indicators.
An action and a call share one tint; their shapes tell them apart. The
presentation choices left open by RFC 0002 are internal to the renderer and may
change without changing the visual language.

Serialize forward connections as subpaths of one SVG path, so shared
distributors and merge rails are stroked once without darkening their
antialiased edges. Draw merge nodes over that path. Keep iteration back edge
paths separate for their arrowhead markers.

The renderer preserves Unicode text and escapes XML content. For the start and
end labels and each row of the parameter panel, it takes the source content
defined by RFC 0002 and collapses each run of whitespace to one space before
wrapping it to its allotted width. An undescribed call's path label is already
normalized under RFC 0002 §4.3 and needs only wrapping. Other long labels also
wrap to their allotted width. Expanded cycle captions may shorten the final
visible line with an ellipsis; all other labels retain their complete text.
Wrapping and shortening break only between grapheme clusters; an otherwise
unbreakable word is split rather than drawn outside its node or panel.

A question-branch description uses its own branch-label style beside the exit
and replaces the output label. Its font is larger than a wire label's font, and
layout uses that size when wrapping text and placing the following row. The
first branch's description hangs below its downward exit; the second branch's
description sits above its horizontal exit. Back edge arrowheads use an embedded
SVG marker, and the accessible description identifies the iteration tail and the
cycle entry. An expanded cycle emits a visible boundary, loop marker, caption
when space permits, but no repeated input or output list. A `<title>` on each
cycle boundary group preserves its complete authored description as a tooltip,
including when the caption is shortened or omitted. A collapsed cycle emits the
same marker and description on its node and its external capture and hand-over
labels. Break capture junctions have no duplicate labels. End shows the value of
the flow's structural return beside its receiving edge.

## 4. Library interface

The default and options-bearing library entry points are:

```rust
kaalang_svg::flow_names(source: &str) -> Result<Vec<String>, RenderError>

kaalang_svg::render_source(source: &str, flow_name: &str)
    -> Result<String, RenderError>

kaalang_svg::render_source_with_options(
    source: &str,
    flow_name: &str,
    options: RenderOptions,
) -> Result<String, RenderError>

pub struct RenderOptions {
    pub collapse_loops: bool,
}
```

The renderer parses the complete UTF-8 Rust source file and selects one function
by its unqualified name. It recognizes `#[kaalang]` on top-level free functions,
`impl` methods, and trait methods with default bodies. It does not descend into
inline modules. The selected function alone undergoes kaalang validation.
`flow_names` lists these functions in source order without validating their
flows; duplicate names remain in the list. `render_source` is equivalent to
options with `collapse_loops: false`. The options-bearing entry point selects
the all-expanded or all-collapsed projection only after the full expanded flow
has passed semantic and topology validation. It then serializes the shared
arrangement or returns a rendering error. It does not invoke `cargo check` or
perform full Rust type checking.

`RenderError::Parse` reports invalid Rust syntax; `FlowNotFound` and
`AmbiguousFlow` report missing and duplicate flow names. `InvalidFlow` reports
invalid kaalang attributes and errors from model construction, including an
impossible expanded topology. `InvalidLabelCharacter` reports authored
descriptions, source-derived signature labels, and default call captions
containing characters that XML 1.0 cannot represent.
`RenderError::UnroutableTopology` reports a renderer defect in geometry, labels
or witness correspondence for the selected projection after assigning dimensions
and spacing. Optional arrangement compaction retains the original witness when a
candidate fails verification. Final route, label and correspondence checks
remain necessary after measurement.

## 5. Command-line interface

The command-line interface is:

```text
cargo kaalang diagram <source.rs> --flow <name> [--collapse-loops] [-o <path>]
```

Without `-o`, the expanded command writes `./<name>.svg` and `--collapse-loops`
writes `./<name>_collapsed.svg`. An explicit output path overrides either
default. The command creates or replaces the output only after the file has been
parsed, the selected flow validated, and its view rendered. On success it prints
the output path; on failure it writes a diagnostic to stderr and exits with a
nonzero status.

## 6. Scope and limits

The renderer does not search an entire crate or resolve external modules,
conditional compilation, or macro-expanded source. It does not support batch
rendering, official `build.rs` integration, interactive or per-cycle folding, a
public JSON descriptor, or alternative output formats.

These limits constrain the SVG renderer, not the semantic model or other visual
representations.
