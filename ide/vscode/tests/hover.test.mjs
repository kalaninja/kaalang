import assert from "node:assert";
import test from "node:test";

import { hoverType } from "../out/hover.js";

// Hovers rust-analyzer 0.3.3121 gave in the repository's fixtures, with the
// macro expanded and, for the last two, before it expands, cut after the parts
// that matter.

test("a local's type comes from the first Rust block, not the generated code after it", () => {
  const hover =
    "\n```rust\nlet finish: usize\n```\n\n---\n\nsize = 8, align = 0x8, no Drop\n\n---\n\n" +
    "```rust\nlet __kaalang_stage_0___kaalang_wire_2: usize\n```\n\n---\n\n```rust\nvalue: T\n```";
  assert.equal(hoverType(hover), "usize");
});

test("a parameter's type follows its name", () => {
  assert.equal(
    hoverType("\n```rust\nself: &mut Counter\n```\n\n---\n\nsize = 8"),
    "&mut Counter",
  );
});

test("a raw, mutable parameter name gives its type", () => {
  assert.equal(
    hoverType(
      "\n```rust\nmut r#type: String\n```\n\n---\n\nsize = 24 (0x18), align = 0x8, needs Drop",
    ),
    "String",
  );
});

test("a mutable binding under a generated name gives its type", () => {
  assert.equal(
    hoverType(
      "\n```rust\nlet mut __kaalang_stage_1___kaalang_wire_1: &mut [T]\n```\n\n---\n\nsize = 16 (0x10)",
    ),
    "&mut [T]",
  );
});

test("a type with generic arguments is kept whole", () => {
  assert.equal(
    hoverType(
      "\n```rust\nlet __kaalang_wire_1: Result<u32, ParseIntError>\n```",
    ),
    "Result<u32, ParseIntError>",
  );
});

test("a hover whose first Rust block names no binding gives no type", () => {
  const hover =
    "\n```rust\ncore::num\n```\n\n```rust\nimpl FromStr for u32\nfn from_str(src: &str) -> Result<u32, ParseIntError>\n```";
  assert.equal(hoverType(hover), null);
});

test("a hover without a Rust block gives no type", () => {
  assert.equal(hoverType("size = 8, align = 0x8"), null);
});

test("a local before the macro expands gives no type", () => {
  assert.equal(
    hoverType(
      "\n```rust\nlet parsed: impl Fn(&str) -> Result<u32, ParseIntError>\n```\n\n---\n\nsize = 0, align = 0x1, no Drop",
    ),
    null,
  );
});

test("a type rust-analyzer has not inferred gives none", () => {
  assert.equal(
    hoverType("\n```rust\nadd: {unknown}\n```\n\n---\n\nno Drop"),
    null,
  );
});
