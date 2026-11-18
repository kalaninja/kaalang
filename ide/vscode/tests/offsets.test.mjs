import assert from "node:assert";
import test from "node:test";

import { coordinates, offsets } from "../out/offsets.js";

test("a position counts the line breaks before it, whatever their kind", () => {
  const at = offsets("a\r\nbc\rdef\ng");
  assert.equal(at(0, 0), 0);
  assert.equal(at(1, 1), 4);
  assert.equal(at(2, 2), 8);
  assert.equal(at(3, 0), 10);
});

test("a character counts UTF-16 code units", () => {
  assert.equal(offsets("🦀\nx")(1, 0), 3);
  assert.equal(offsets("// 🦀 Member")(0, 6), 6);
});

test("a position past its line stands at the line's end", () => {
  assert.equal(offsets("ab\ncd")(0, 9), 2);
  assert.equal(offsets("ab")(5, 0), 2);
});

test("positions sent to the language server round-trip Unicode and mixed line endings", () => {
  const at = coordinates("ab\r\n🦀\rx\n");
  for (const [offset, line, character] of [
    [0, 0, 0],
    [2, 0, 2],
    [6, 1, 2],
    [7, 2, 0],
    [9, 3, 0],
  ]) {
    assert.deepEqual(at.positionAt(offset), { line, character });
    assert.equal(at.offsetAt(line, character), offset);
  }
  assert.deepEqual(at.positionAt(3), { line: 0, character: 2 });
});
