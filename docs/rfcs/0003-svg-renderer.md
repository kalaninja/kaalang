# RFC 0003: kaalang SVG Renderer

- Status: accepted design draft
- Visual language: [RFC 0002](0002-visual-language.md)
- Artifact target: SVG

## 1. Overview

The kaalang SVG renderer implements the visual language defined by RFC 0002. It
consumes the validated semantic model also used for execution lowering and does
not independently reinterpret the authored source.

## 2. Layout

The validated model supplies the topology. After `kaalang_model::build`, the
renderer calls the shared constructor with the analyzed flow, wire merges, and
topology. The constructor searches for an arrangement and independently checks
it against RFC 0002. The renderer assigns dimensions and spacing to its ranks,
columns, corridors, lanes, and contours. Everything this section adds beyond RFC
0002 is a presentation preference, relaxable without making a topology invalid.

The current construction search is finite and deterministic, with no retry or
time budget. It varies iteration-tail ranks, return sides, and connection
corridors in response to routing conflicts. Its normalization and pruning have
no completeness proof: failure to find an arrangement is a rendering error, not
a semantic rejection or proof that no conforming diagram exists. The model
carries no mandatory or optional arrangement field, and macro expansion does not
run this search. Moving construction into `build` requires a complete procedure
for the unchanged RFC 0002 constraints.

Every emitted layout is computed deterministically, including node dimensions,
row and column assignments, label placement, and connection routing. It
preserves the row and column ordering and connection-routing constraints
required by RFC 0002. A question or choice nested inside a branch receives
columns that do not overlap those assigned to other branches of the enclosing
question or choice.

A **footprint** is the contiguous range of columns reserved by one convergence
group and its shared continuation. Disjoint convergence groups of the same
question or choice receive disjoint footprints in authored branch order. Nested
convergence groups of that question or choice may share columns.

When a shared continuation has one entry block, its node occupies the column in
which the first branch of its convergence group reaches it. "First" follows the
branch order defined by RFC 0002. This is normally that branch's own column; if
the branch contains a nested question or choice, its route may reach that node
in another column. Other branches of the group route to the same node.

When a shared continuation has several entry blocks, draw them sequentially in
source order. They share the column reached by the group's first branch. Other
branches meet before the sequence; connections carry later captures transitively
through it.

A group's footprint extends from its leftmost occupied column to its rightmost
occupied column. A terminal sibling branch that precedes the group remains to
the left of its footprint; one that follows the group remains to its right. With
several disjoint groups, intervening terminal branches remain between their
footprints.

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

The shared constructor records the side of the body, the outermost column of
that body, and the lane beside it each return climbs. Geometry realizes the
checked contour; failure to find one on either side remains a rendering error.
It puts the lane just past whatever the body itself occupies in the return's
vertical span — the node in the recorded column, or the column's own line when
none is there — and one lane step further out for each later lane.

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
Wider body nodes below that span do not widen the return. Keep the original
route when the compact one would cross another connection. Node columns and
branch order do not change.

After routing the returns, adjust a reachable terminal `end` merge and end using
the actual geometry. First try the usual vertical gap below all other nodes, so
independent terminal and return rails can share a row. Horizontal terminal and
return segments whose horizontal spans overlap must remain at least the usual
vertical gap apart. If the first position violates that clearance or introduces
a crossing, also leave the usual vertical gap below the lowest return. Move the
terminal merge and end together, preserving their common segment's length; when
the end block has no terminal merge, move the block alone. Move upward to remove
excess space or downward to provide the required clearance. Keep the original
placement if both adjusted positions would introduce a crossing or upward
segment. Place labels after this adjustment.

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

`RenderError::InvalidFlow` reports semantic errors from `kaalang_model::build`.
`RenderError::UnroutableTopology` reports failure to construct an arrangement,
an internal construction error, or geometry and label placement that cannot
satisfy RFC 0002 §8. A failed construction search does not change whether the
flow is valid. Final route and label verifiers remain necessary after assigning
dimensions and spacing.

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
