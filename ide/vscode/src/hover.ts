// The type of a value in the text of a hover at its binding, and the module of
// a type in the text of a hover at its name, apart from VS Code so that tests
// reach them.

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

/** A path of names: `shop`, `shop::model::inner`. */
const MODULE =
  /^[\p{ID_Start}_]\p{ID_Continue}*(?:::[\p{ID_Start}_]\p{ID_Continue}*)*$/u;

/** The declaration of a type, after its visibility. */
const DECLARATION = /^(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|union|type)\s/;

/** Every Rust block, by its text. */
const BLOCKS = /```rust\r?\n([^]*?)```/g;

/**
 * The module rust-analyzer's hover `markdown` at a type's name places it in,
 * starting with its crate's name: the first Rust block, when the second
 * declares a type. Any other hover gives `null`, as one at a keyword or a
 * binding.
 */
export function hoverModule(markdown: string): string | null {
  const [module, declaration] = [...markdown.matchAll(BLOCKS)].map(
    ([, block]) => block.trim().split(/\r?\n/)[0],
  );
  return MODULE.test(module ?? "") && DECLARATION.test(declaration ?? "")
    ? module
    : null;
}
