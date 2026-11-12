# Changelog

## 0.3.0 - unreleased

### Breaking

- The attribute is `#[kaa]`, or `#[kaalang::kaa]`, instead of `#[kaalang]`, and
  block attributes take its namespace: `#[kaa::action("...")]` instead of
  `#[action("...")]`. See [RFC 0009](docs/rfcs/0009-types.md).
- `cargo kaalang diagram --flow` takes the flow's Rust path from the crate root,
  as `review::Report::approve` or `'<&Report as Approve>::approve'`, instead of
  a source file and a bare name, and searches the whole cargo workspace; `-p`
  picks the package. The default output is named after the last path segment.
- Only Rust 2024 crates are supported.
- `kaalang-svg`: `render_source` and `render_source_with_options` are removed;
  `render_flow` renders the flow at a position from `flow_names`, which lists
  flows by their path within the file and takes a `Target`. `RenderOptions`
  gains `target`, and `RenderError` gains `TypeNotFound` and `InvalidType`.
- `kaalang-compiler`: `flows` returns `FlowItem`s with their paths and descends
  into inline modules; `is_flow_attribute` is renamed `is_kaalang_attribute`.

### Added

- Type diagrams. `#[kaa]` marks structs, enums, unions, and type aliases as
  datatypes; fields take `kaa::value` or `kaa::refer`, variants `kaa::case`.
  `cargo kaalang diagram --type <path>` draws a type under every datatype that
  refers to it, and `--columns` sets the cards per row. See
  [RFC 0009](docs/rfcs/0009-types.md).
- `cargo kaalang diagram` reads the workspace for one compilation target:
  `--target`, the host by default, and cargo's feature flags decide which `cfg`
  and `cfg_attr` hold.
- Diagrams mark visibility before a name as UML does: `+` for `pub`, `~` for a
  restricted form, `-` for private.
- `kaalang-compiler`: `declarations`, `FlowPath`, and `select` resolve a flow
  path; `type_items`, `type_declaration`, and `expand_type` read and expand type
  declarations; `configure` removes what a cfg set excludes.
- `kaalang-svg`: `render_type` draws a type diagram from `TypeCard`s with
  `TypeOptions`; `Target` holds a target's cfg set.

### Changed

- Flow diagrams draw cycles with the same stroke width as every other line, and
  set Rust source in a monospace font, as type diagrams do.

### Fixed

- A branch that cannot descend in its merge column no longer steps sideways
  right below its brancher.

## 0.2.0 - 2026-10-05

### Breaking

- A loop cycle is written `loop { ... }` and repeats only at `continue`; it no
  longer completes with `break` or at the end of its body. It declares named
  outputs, each produced by a same-named wire in its body, and a tuple of
  outputs declares alternative exits instead of simultaneous results. Its header
  names at most one gate wire instead of a capture list, and inner blocks
  capture outer wires directly. See
  [RFC 0007](docs/rfcs/0007-language-refinements.md) and
  [RFC 0008](docs/rfcs/0008-cycle-forms.md).
- A block body can no longer read or assign a flow parameter it does not
  capture.
- The `--collapse-loops` option and `kaalang-svg`'s
  `RenderOptions::collapse_loops` are renamed `--collapse-cycles` and
  `collapse_cycles`.
- Descriptions are restricted Markdown: existing `*`, `_`, `` ` ``, `~`, `^`,
  `$`, supported tags, a leading `>`, backslash escapes, and `&...;` character
  references render as notation unless escaped with a backslash. See
  [RFC 0005](docs/rfcs/0005-markdown.md).
- In SVG output, a collapsed cycle node has the `cycle` class instead of `loop`,
  and the back-edge arrow marker is `back-edge-arrow` instead of `loop-arrow`.
- The public types of `kaalang-compiler` follow the cycle terminology and the
  new cycle forms, for example `BlockKind::Cycle`, `ExecutionPlan::Cycle`,
  `topology::Cycle`, and `ExecutionOutcome::Repeat { cycle_index }`;
  loop-specific items such as `ProducerId::CycleInput`, `Execution::repeats`,
  and `SemanticModel::body_vertices` are removed. In `kaalang-render`,
  `ArrangementVerifier::may_rise_beside` and `body_vertices` are no longer
  public. `Analysis::executions` is now `Executions`, and `Branch` is an alias
  for `Box<ExecutionPlan>`.

### Added

- `for` cycles: `#[cycle("...")] |captures| for item in items { ... }` runs its
  body once per item of a Rust iterator, drawn between a for-entry and a for-end
  node. See [RFC 0008](docs/rfcs/0008-cycle-forms.md).
- Stages: named steps of a state machine whose dispatcher the compiler
  generates. See [RFC 0006](docs/rfcs/0006-stages.md).
- Formatted descriptions: bold, italic, strikethrough, code, superscript,
  subscript, quotes, underline, highlight, palette colors, and TeX formulas.
- A block initializer may omit an empty capture list.
- A block statement may omit its trailing semicolon wherever Rust allows it; the
  semicolon has no meaning in kaalang.
- `cargo kaalang --help` and `cargo kaalang --version`.
- The README is the `kaalang` crate documentation on docs.rs.

### Changed

- Nodes on the same row share the height of the tallest one, so 0.1.0 diagrams
  can shift by a few pixels.
- Convergence groups are compared by route, so one case of a question or choice
  may feed different merges when a nested selection splits it.
- Flows with many independent questions and choices are analyzed without
  enumerating every history.

### Fixed

- `cargo kaalang` finds flows marked `#[kaalang::kaalang]`.
- Blocks, cycles, and stages written through `macro_rules!` fragments parse like
  directly written ones.
- A choice arm that diverges, such as `_ => unreachable!()`, no longer makes the
  generated code warn.
- Generated code no longer silences the user's own lints inside cycles, and it
  compiles under `#![forbid(unused_mut)]`.
- `cargo kaalang` refuses an output path that resolves to its source file,
  including through symbolic and hard links.
- Arrangement verification rejects coordinates that overflow.
- README images and links resolve on crates.io.

## 0.1.0 - 2026-09-20

Initial release.
