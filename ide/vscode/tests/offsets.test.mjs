import assert from "node:assert";
import test from "node:test";

import { coordinates, editorText, offsets } from "../out/offsets.js";

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

test("a saved file's mixed line endings become the kind most of them are", () => {
  assert.equal(editorText("a\r\nb\nc\n"), "a\nb\nc\n");
  assert.equal(editorText("a\r\nb\r\nc\n"), "a\r\nb\r\nc\r\n");
  assert.equal(editorText("a\r\nb\n"), "a\nb\n");
  assert.equal(editorText("a\rb\n"), "a\nb\n");
});

test("a saved file's one kind of line ending and its text stay", () => {
  assert.equal(editorText("a\r\nb\r\n"), "a\r\nb\r\n");
  assert.equal(editorText("\uFEFFa\nb"), "a\nb");
});
