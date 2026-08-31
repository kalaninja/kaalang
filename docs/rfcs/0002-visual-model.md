# RFC 0002: Contour Visual Model

- Status: implementation draft
- Model version: `0.1`
- Artifact target: SVG

## 1. Overview

Contour has a visual representation of the flow model defined by RFC 0001. The
visual graph is a projection of a validated semantic `Graph`; it does not parse
or reinterpret the authored Rust syntax independently.

The graph builder remains the source of truth for blocks, wires, branches,
paths, and their order. Rust lowering and visual rendering are separate
consumers of that graph. A diagram therefore represents the same validated flow
that Contour lowers for execution.

## 2. Visual graph

A visual graph contains one node for each authored block. An action, question,
choice, and merge retain distinct visual roles. A choice is drawn as a Select
node followed by one derived Case node for each authored `#[case]` attribute.
Case nodes are visual projections, not additional semantic blocks. No authored
block is duplicated to simplify layout.

The renderer adds two synthetic nodes:

- **Start** represents the flow boundary and its source wires;
- **Return** receives the result of every path that ends at a terminal action.

These nodes have no corresponding block statements. Connections between nodes
come from the validated plan. Rust bodies, source comments, and data-wire
dependencies do not add or change control topology in version 0.1. A wire name
may label a connection, but data dependencies are not drawn as a separate
graph.

## 3. Labels and branches

Action, question, and choice nodes use the exact text value of their block
description. The renderer does not paraphrase, normalize, or synthesize that
text; output-format escaping and line wrapping do not change its value. A merge
has no authored description and is identified by its visual role.

Question branches preserve their positional meaning from RFC 0001: the first
output is yes/true and the second is no/false. A diagram labels each branch with
only its exact output wire name; it does not add `yes` or `no`.

Choice branches preserve authored case order. The Select node uses the choice
description. Each Case node uses the exact text value of its corresponding case
description. A Select-to-Case connection is unlabeled, and the connection from
the Case node into its branch uses only the exact output wire name. The visual
graph must not reorder cases according to their Rust patterns or layout
position.

Merge connections preserve the validated convergence of sibling branches.
Terminal action outputs connect to the synthetic Return node.

## 4. Layout and style

The version 0.1 renderer uses a deterministic primitive skewer layout.
The first question output and the first choice case continue down the current
vertical skewer. Remaining branches occupy successive skewers to the right in
authored order. Nested branches receive non-overlapping groups of skewers, and
a merge, with the continuation after it, takes the skewer of the first branch
that reaches it. That is the branching block's own skewer unless a branch which
ends the flow leads the continuing ones and already holds it.

Terminal branches converge on one horizontal collector above the Return node,
the mirror of the distributor that fans a Select out to its cases. Each terminal
skewer descends onto the collector, and the collector makes the one vertical
descent into the node; a terminal skewer already in the Return column descends
straight through it. Terminal connections therefore share the collector and that
descent, and the shared run is deliberate: it draws one common path, not
connections hidden behind one another.

A clear terminal skewer descends in its own column. An obstructed one is routed
outside the continuing branches and joins the same collector at the bottom. A
terminal skewer is obstructed only by a continuation wider than the branches
beside it, never by a sibling branch: the branch adjacency RFC 0001 requires
leaves no connection crossing another.

Connections use horizontal and vertical segments without arrowheads. Action
nodes are rectangles, question nodes are elongated hexagons, choice nodes are
skewed Select parallelograms, Case nodes have a lower triangular point, Start
and Return are capsules, and merge nodes remain circles marked `M`. The renderer
uses one monochrome style and does not print block-kind captions inside nodes.

Exact dimensions, colors, typography, spacing, and routing offsets remain
rendering choices. The stable contract is the visual graph's nodes, roles,
labels, branch order, connections, and skewer ordering.

## 5. SVG renderer

The renderer produces a standalone SVG with embedded styles and no JavaScript,
external fonts, or external rendering programs. It preserves Unicode text,
escapes XML content, and wraps long labels without changing their values.

The library entry point is:

```rust
contour_svg::render_source(source: &str, flow_name: &str)
    -> Result<String, RenderError>
```

It finds the named top-level `#[contour]` function in one UTF-8 Rust source
file, builds its validated semantic graph, lays it out, and serializes it. It
does not invoke `cargo check` or perform full Rust type checking.

The command-line interface is:

```text
cargo contour diagram <source.rs> --flow <name> [-o <path>]
```

Without `-o`, the command writes `./<name>.svg`. It creates or replaces the
output only after the complete source has been validated and rendered.

## 6. Version 0.1 limits

The renderer does not search an entire crate or resolve external modules,
conditional compilation, or macro-expanded source. It does not support batch
rendering, official `build.rs` integration, interactive HTML, a public JSON
descriptor, or alternative output formats.

These limits constrain the first renderer, not the semantic graph or future
visual representations.
