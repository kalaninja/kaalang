// The type of a value in the text of a hover at its binding, apart from VS Code
// so that tests reach it.

/** The first line of the first Rust block. */
const FIRST_LINE = /```rust\r?\n(.*)/;

/**
 * A binding and its type: `value: usize`, `self: &mut Counter`,
 * `let mut __kaalang_wire_1: i64`. The name may be the one the macro generated.
 */
const BINDING =
  /^(?:let\s+)?(?:mut\s+)?(?:r#)?[\p{ID_Start}_]\p{ID_Continue}*\s*:(?!:)\s*(.+)$/u;

/** What the names the macro generates begin with. */
const GENERATED = "__kaalang_";

/**
 * The type rust-analyzer's hover `markdown` gives a binding, or `null` when
 * its first Rust block names none: no API stabilizes the text. Until the macro
 * expands, a `let` binds the closure its block is written as, so a local's type
 * counts only once the hover shows a generated name. A type rust-analyzer has
 * not inferred, `{unknown}`, never counts.
 */
export function hoverType(markdown: string): string | null {
  const line = FIRST_LINE.exec(markdown)?.[1].trim() ?? "";
  const type = BINDING.exec(line)?.[1];
  const unexpanded = line.startsWith("let ") && !markdown.includes(GENERATED);
  return type === undefined || type.includes("{unknown}") || unexpanded
    ? null
    : type;
}
