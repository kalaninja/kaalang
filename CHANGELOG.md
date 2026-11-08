# Changelog

## 0.3.0 - unreleased

### Breaking

- `cargo kaalang diagram` selects a flow by its Rust path from the crate root,
  as `--flow review::Report::approve`, instead of a source file and a bare name.
  It searches every crate of the cargo workspace, and `-p` names the package
  when the path exists in several. The default output is named after the last
  path segment. See [RFC 0009](docs/rfcs/0009-types.md).
- `kaalang-svg`'s `render_source` and `render_source_with_options` select a flow
  by its path within the file, and `flow_names` lists those paths, including
  flows in inline modules. A method's path starts with its type or trait.
- `kaalang-compiler`'s `flows` returns each flow with its path within the file
  and descends into inline modules.
- `kaalang-compiler`'s `is_flow_attribute` is renamed `is_kaalang_attribute`.
- The attribute is `#[kaa]`, or `#[kaalang::kaa]` through the crate path,
  instead of `#[kaalang]`, and block attributes take its namespace:
  `#[kaa::action("...")]` instead of `#[action("...")]`. See
  [RFC 0009](docs/rfcs/0009-types.md).

### Added

- `#[kaa]` marks structs, enums, unions, and type aliases as annotated types,
  with an optional description. Fields take `kaa::value` or `kaa::refer`
  annotations, enum variants take `kaa::case`, and an alias may carry its own
  `kaa::refer`. Rust resolves each `refer` target as the field's module sees it.
  See [RFC 0009](docs/rfcs/0009-types.md).

### Fixed

- A branch that could not descend in its merge column no longer takes a step
  sideways right below its brancher; it leaves in the column it descends.

## 0.2.0 - 2026-10-05

### Breaking

- A loop cycle no longer completes with a structural `break` or repeats at the
  end of its body, and the body is written as `loop { ... }` instead of a bare
  brace block. It declares named outputs, each produced by a same-named wire in
  its body, and repeats only through an explicit `continue`. Its header names at
  most one plain gate wire instead of a capture list, and inner blocks capture
  outer wires directly. A tuple of outputs now declares alternative exits, one
  completing per route, instead of simultaneous results. See
  [RFC 0007](docs/rfcs/0007-language-refinements.md) and
  [RFC 0008](docs/rfcs/0008-cycle-forms.md).
- A block body can no longer read or assign a flow parameter it does not
  capture. 0.1.0 accepted this by mistake.
- The `--collapse-loops` option and `kaalang-svg`'s
  `RenderOptions::collapse_loops` are renamed `--collapse-cycles` and
  `collapse_cycles`.
- Descriptions are interpreted as restricted Markdown, so existing `*`, `_`,
  `` ` ``, `~`, `^`, `$`, supported tags, a leading `>`, backslash escapes, and
  `&...;` character references render as notation. Escape a marker with a
  backslash to keep it literal. See [RFC 0005](docs/rfcs/0005-markdown.md).
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

- Nodes on the same row share the height of the tallest one, so a diagram of an
  unchanged 0.1.0 source can differ by a few pixels.
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
