# Changelog

## 0.2.0 - unreleased

### Breaking

- Cycles no longer complete with a structural `break` or repeat at the end of
  their body, and the body is written as `loop { ... }` instead of a bare brace
  block. A bare `#[cycle("...")] loop { ... }` may omit its trailing semicolon,
  which rustfmt removes. A cycle declares named outputs, each produced by a
  same-named wire in its body, and repeats only through an explicit `continue`.
  Its header names at most one plain gate wire instead of a capture list, and
  inner blocks capture outer wires directly. See
  [RFC 0007](docs/rfcs/0007-language-refinements.md) and
  [RFC 0008](docs/rfcs/0008-cycle-forms.md).
- A block body can no longer read or assign a flow parameter it does not
  capture. 0.1.0 accepted this by mistake.
- The `--collapse-loops` option and `RenderOptions::collapse_loops` are renamed
  `--collapse-cycles` and `collapse_cycles`.
- Descriptions are interpreted as restricted Markdown, so existing `*`, `_`,
  `` ` ``, `~`, `^`, `$`, and supported tags render as formatting. Escape a
  marker with a backslash to keep it literal. See
  [RFC 0005](docs/rfcs/0005-markdown.md).

### Added

- Stages: named steps of a state machine whose dispatcher the compiler
  generates. See [RFC 0006](docs/rfcs/0006-stages.md).
- Formatted descriptions: bold, italic, strikethrough, code, superscript,
  subscript, quotes, underline, highlight, palette colors, and TeX formulas.
- A block initializer may omit an empty capture list.
- Flows with many alternative histories are analyzed through execution
  conditions instead of enumerating every history.
- `cargo kaalang --help` and `cargo kaalang --version`.
- The README is the `kaalang` crate documentation on docs.rs.

### Fixed

- `cargo kaalang` refuses an output path that resolves to its source file,
  including through symbolic and hard links.
- Arrangement verification rejects coordinates that overflow.
- README images and links resolve on crates.io.

## 0.1.0 - 2026-09-20

Initial release.
