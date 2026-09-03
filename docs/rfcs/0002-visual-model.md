# RFC 0002: kaalang Visual Model

- Status: accepted design draft
- Model version: `0.1`
- Artifact target: SVG

## 1. Overview

kaalang has a visual representation of the flow model defined by RFC 0001. The
visual graph is a projection of a validated semantic `Graph`; it does not parse
or reinterpret the authored Rust syntax independently.

The graph builder remains the source of truth for blocks, wires, branches,
paths, implicit convergence, and their order. Rust lowering and visual rendering
are separate consumers of that graph. A diagram therefore represents the same
validated flow that kaalang lowers for execution.

## 2. Visual graph

A visual graph contains one node for each authored block. Actions, questions,
choices, and the unique End retain distinct visual roles. A choice is drawn as
a Select node followed by one derived Case node for each authored `#[case]`
attribute. Case nodes are visual projections, not additional semantic blocks.
No authored block is duplicated to simplify layout.

The renderer adds one synthetic **Start** node representing the flow boundary,
labeled with the authored flow signature. End comes from the mandatory authored
`#[end]` statement; the renderer does not add a separate Return node.

Connections between nodes come from the validated plan. Rust bodies, source
comments, and data-wire dependencies do not add or change control topology. Wire
names label a connection's ends as section 3 sets out, but data dependencies are
not drawn as a separate graph.

kaalang has no merge node. Path-exclusive producers of the same logical wire
converge directly at their first shared consumer or at End.

## 3. Labels and branches

Action, question, and choice nodes use the exact text value of their block
description. The renderer does not paraphrase, normalize, or synthesize that
text; output-format escaping and line wrapping do not change its value. Wrapping
breaks between grapheme clusters, so a cluster spelled with several code points
stays on one line and a word with no other break opportunity is still divided
rather than drawn outside its node. End has no authored description and is
identified by its visual role.

Start uses the authored flow signature without its `fn` keyword, with each run
of whitespace collapsed to one space so the label wraps to its own budget.
Nothing else is rewritten: parameter types, generics, the return type, a where
clause, a wildcard parameter, and the `r#` of a raw identifier all appear as
authored. The signature states the flow's calling contract, which wire names
alone do not: a boundary input is otherwise drawn only where a connection
happens to carry its name.

A connection is labeled at the end whose wires it names. Its hand-over is what
the node it leaves passes on: the boundary's source wires for Start, the branch's
own output wire for a question or a case, and every output for any other node. A
node's capture is what it consumes, and is drawn once above that node however
many connections arrive there, because every connection into one node delivers
the same captured wires. A borrowed input is read where its wire lies rather than
taken off the flow, so it is a data dependency and no connection names it. An
underscore-prefixed wire that no block uses is omitted because it names no
value carried onward. If a block does use it, its name is drawn like any other
logical wire. Where a hand-over and the capture it meets name the same wires in
the same order, the connection carries one label instead of two.

A connection label carries the logical wire name, so a raw identifier appears
without its `r#`: the raw and ordinary spellings of one wire name the same wire,
and only the Start signature quotes the source as authored.

Question branches preserve their positional meaning from RFC 0001: the first
output is yes/true and the second is no/false. A branch's hand-over is only its
exact output wire name; a diagram does not add `yes` or `no`.

Choice branches preserve authored case order. The Select node uses the choice
description. Each Case node uses the exact text value of its corresponding case
description. A Select-to-Case connection names nothing at either end, because the
case row belongs to the Select above it and the branch's output wire rides the
connection leaving the Case node. The visual graph must not reorder cases
according to their Rust patterns or layout position.

When sibling paths produce the same logical wire, they meet at the one downstream
consumer that captures the name, which names that wire once above itself. No
synthetic node is inserted between them.

A zero-computation flow contains Start and the authored End. Those nodes are
connected only when End captures a source wire; when End captures nothing they
are drawn unconnected, whether or not the boundary declares source wires. Rust's
unit result does not appear as a wire.

## 4. Layout and style

The renderer uses a deterministic primitive skewer layout. The first
question output and the first choice case continue down the current vertical
skewer. Remaining branches occupy successive skewers to the right in authored
order. Nested branches receive non-overlapping groups of skewers.

When several branches continue into one shared consumer, that consumer occupies
the skewer of the first continuing branch. Other continuing branches route into
it directly. A branch that reaches End instead routes to the unique End node;
branches may do so after different numbers of computational blocks.

Paths into End converge on one horizontal collector above the node, the mirror
of the distributor that fans a Select out to its cases. Each terminal skewer
descends onto the collector, and the collector makes the one vertical descent
into End; a terminal skewer already in the End column descends straight through
it. Shared runs are deliberate common paths, not connections hidden behind one
another.

A clear terminal skewer descends in its own column. An obstructed one is routed
outside continuing branches and joins the same collector at the bottom.
Connections use horizontal and vertical segments without arrowheads.
The core model's adjacency rule prevents a terminal sibling from separating two
branches that enter one shared continuation, which would otherwise force a
crossing regardless of the outer lane used.

Action nodes are rectangles, question nodes are elongated hexagons, choice
nodes are skewed Select parallelograms, Case nodes have a lower triangular
point, and Start and End are capsules. The renderer uses one monochrome style
and does not print block-kind captions inside nodes. There is no merge icon.

Exact dimensions, colors, typography, spacing, and routing offsets remain
rendering choices; which end of a connection a label belongs to does not. The
stable contract is the visual graph's nodes, roles, labels and the ends they
name, branch order, implicit convergence, connections, and skewer ordering.

## 5. SVG renderer

The renderer produces a standalone SVG with embedded styles and no JavaScript,
external fonts, or external rendering programs. It preserves Unicode text,
escapes XML content, and wraps long labels without changing their values.

The library entry point is:

```rust
kaalang_svg::render_source(source: &str, flow_name: &str)
    -> Result<String, RenderError>
```

It finds the named top-level `#[kaalang]` function in one UTF-8 Rust source
file, builds its validated semantic graph, lays it out, and serializes it. It
does not invoke `cargo check` or perform full Rust type checking.

The command-line interface is:

```text
cargo kaalang diagram <source.rs> --flow <name> [-o <path>]
```

Without `-o`, the command writes `./<name>.svg`. It creates or replaces the
output only after the complete source has been validated and rendered.

## 6. Renderer limits

The renderer does not search an entire crate or resolve external modules,
conditional compilation, or macro-expanded source. It does not support batch
rendering, official `build.rs` integration, interactive HTML, a public JSON
descriptor, or alternative output formats.

These limits constrain the renderer, not the semantic graph or future visual
representations.
