# RFC 0003: kaalang SVG Renderer

- Status: accepted design draft
- Visual language: [RFC 0002](0002-visual-language.md)
- Artifact target: SVG

## 1. Overview

The kaalang SVG renderer implements the visual language defined by RFC 0002. It
consumes the validated semantic model also used for execution lowering and does
not independently reinterpret the authored source.

## 2. Layout

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

Use one vertical gap throughout a diagram, at least 72 pixels and enlarged when
labels need more room. Leave at least that gap between consecutive node rows.
For horizontal runs in row gaps, reserve it both below the preceding node row
and above the following node row. A junction has no node or capture label, so it
reserves no extra space above its rail. Additional horizontal lanes stay 20
pixels apart. This gives select distributors, merge rails, and iteration tails
the same clearance.

While conditions use the ordinary question layout and authored answer order. An
unconditional loop has no node: its entry and iteration tail are junctions, and
an empty body connects them directly. Forward connections and iteration-tail
precedence form an acyclic placement graph. After separating every return from
the forward graph, carry each tail's precedence through continuation blocks and
loop-entry junctions. Stop at the first wire merge, iteration tail, or end on
each forward route. These boundaries stay below the preceding body to leave room
for its return; continuation blocks can fill their separate branch columns
without waiting for the body's final row. This rule applies equally to actions,
questions, choices, and further loops. Tail precedence is not drawn. Each loop
has an entry junction on its incoming line; a while follows it with one vertical
segment into the question, while an unconditional loop follows it with its body.
Return connections are routed separately, innermost loop first, around the body
and horizontally into that junction. The arrowhead belongs to that horizontal
arrival. Returns may move upward; all other geometry checks still apply. If no
checked return route fits, lower the blocked iteration tail and retry placement
and routing within the existing bounded attempt budget. Tail precedence carries
that delay to dependent boundaries; independent terminal branches may remain
above the return. If no attempt succeeds, rendering fails under §4.

An iteration tail meets its incoming branches at their leftmost approach for an
unconditional or yes-first loop and their rightmost approach for a no-first
loop. The return can then leave that end of the rail without retracing an
incoming branch.

A tail with one incoming connection may sit on a side exit's horizontal run, or
after the usual vertical gap below a straight exit. Prefer turning upward there
over descending to the tail's placement row and immediately returning upward.
Keep the lower route when the shorter one would cross another connection; the
tail's forward precedence still determines initial node placement.

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
description sits above its horizontal exit. Default while answer labels use the
same placement. Return arrowheads use an embedded SVG marker, and the accessible
description identifies the iteration tail and the loop entry, naming the while
condition when one exists.

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

When its deterministic layout cannot route every connection without violating
RFC 0002, it returns a rendering error instead of emitting a non-conforming
diagram. This does not make the validated textual flow invalid.

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
