# RFC 0003: kaalang SVG Renderer

- Status: accepted design draft
- Visual language: [RFC 0002](0002-visual-model.md)
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
group and its shared continuation. Distinct convergence groups of the same
question or choice receive disjoint footprints in authored branch order.

When a shared continuation has one convergence point, its node occupies the
column in which the first branch of its convergence group reaches it. "First"
follows the branch order defined by RFC 0002. This is normally that branch's own
column; if the branch contains a nested question or choice, its route may reach
that node in another column. Other branches of the group route to the same node.

When a shared continuation has several convergence points, the renderer assigns
their columns deterministically. All convergence-point nodes remain within its
group's footprint. Every branch of the group routes to every convergence point.

A group's footprint extends from its leftmost occupied column to its rightmost
occupied column. A terminal sibling branch that precedes the group remains to
the left of its footprint; one that follows the group remains to its right. With
several groups, intervening terminal branches remain between their footprints.
The core model's adjacency rule ensures that a branch outside a convergence
group cannot separate two of its members.

To form the collector defined by RFC 0002, each terminal path reaches its
assigned column, descends to the collector, and follows it into the end node. A
terminal path already in the end node's column descends straight through the
collector.

## 3. SVG output

The renderer produces standalone SVG with embedded styles and no JavaScript,
external fonts, or external rendering programs. It uses a monochrome style.
The presentation choices left open by RFC 0002 are internal to the renderer and
may change without changing the visual language.

The renderer preserves Unicode text and escapes XML content. For the start
label, it takes the signature content defined by RFC 0002 and collapses each run
of whitespace to one space before wrapping it to its allotted width. It applies
no other renderer-specific rewrite. Other long labels also wrap to their
allotted width. Wrapping breaks only between grapheme clusters; an otherwise
unbreakable word is split rather than drawn outside its node.

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
