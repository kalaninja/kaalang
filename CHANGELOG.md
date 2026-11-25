# Changelog

## 0.4.0 - unreleased

### Breaking

- A choice body is always a `match`: a placeholder `todo!()` goes in an arm, and
  a whole-body `todo!()` is rejected.

### Added

- The VS Code extension shows the flow or type at the cursor. Click a connection
  or merge to inspect the wires it carries. Ctrl-click (Cmd-click on macOS) a
  node, wire name, or type path to open its source or definition. Diagrams honor
  the `cfg` flags each package's build script sets.

### Fixed

- `cargo kaalang diagram` no longer keeps the carriage returns of a description
  written as a raw string in a file with CRLF line endings.
- rust-analyzer no longer reads a stage's entry or signal name as `core` or
  `Result`, so hover and go-to-definition on it reach the wire's own type.
- A staged flow's diagram no longer lists a signal, or drops another name, in a
  hand-over that also passes an unnamed output.
- A flow's parameter, or a datatype's generic parameter, that a `#[cfg]` leaves
  out no longer counts: a block cannot capture it, and it no longer hides a
  relationship target of the same name.
- `cargo kaalang diagram --type` reads a file included several times, through
  `mod` in several crates or `#[path]` in several modules, as a type per
  inclusion, as rustc compiles it, each with its own card.
- `cargo kaalang diagram` reports a module that has both `name.rs` and
  `name/mod.rs` instead of reading one of them.
- A datatype with a relationship no longer fails to compile under
  `forbid(unused_imports)`.
- `cargo kaalang diagram --target` reads a procedural macro and the libraries it
  depends on for the named target, as it reads every other crate, so a type they
  declare keeps its card.

## 0.3.0 - 2026-11-12

### Breaking

- The attribute is `#[kaa]`, or `#[kaalang::kaa]`, instead of `#[kaalang]`, and
  block attributes take its namespace: `#[kaa::action("...")]` instead of
  `#[action("...")]`. See [RFC 0009](docs/rfcs/0009-types.md).
- `cargo kaalang diagram --flow` takes the flow's Rust path from the crate root,
  as `review::Report::approve` or `'<&Report as Approve>::approve'`, instead of
  a source file and a bare name, and searches the whole cargo workspace; `-p`
  picks the package. The default output is named after the last path segment.
- Only Rust 2024 crates are supported.
- The minimum supported Rust version is 1.99.

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
- The `--collapse-loops` option is renamed `--collapse-cycles`.
- Descriptions are restricted Markdown: existing `*`, `_`, `` ` ``, `~`, `^`,
  `$`, supported tags, a leading `>`, backslash escapes, and `&...;` character
  references render as notation unless escaped with a backslash. See
  [RFC 0005](docs/rfcs/0005-markdown.md).
- In SVG output, a collapsed cycle node has the `cycle` class instead of `loop`,
  and the back-edge arrow marker is `back-edge-arrow` instead of `loop-arrow`.

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
