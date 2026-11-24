import assert from "node:assert/strict";
import fs from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const require = createRequire(import.meta.url);

test("the module worker survives invalid Rust and restarts after a Wasm trap", async (t) => {
  const directory = await fs.mkdtemp(
    path.join(os.tmpdir(), "kaalang-module-worker-"),
  );
  t.after(() => fs.rm(directory, { recursive: true, force: true }));
  const reader = path.join(directory, "module-sources.js");
  await fs.copyFile(
    new URL("../out/module-sources.js", import.meta.url),
    reader,
  );
  for (const name of ["cache.js", "offsets.js"]) {
    await fs.copyFile(
      new URL(`../out/${name}`, import.meta.url),
      path.join(directory, name),
    );
  }
  const wasm = path.join(directory, "wasm");
  await fs.mkdir(wasm);
  await fs.writeFile(path.join(wasm, "kaalang_js_bg.wasm"), "");
  await fs.writeFile(
    path.join(wasm, "kaalang_js.mjs"),
    `
    export function initSync() {}
    export function relationshipTargets(text) {
      if (text === "invalid") throw new Error("parse error");
      if (text === "trap") throw new WebAssembly.RuntimeError("unreachable");
      return JSON.stringify([text.length]);
    }`,
  );
  const { relationshipTargets, stopModuleWorker } = require(reader);
  t.after(stopModuleWorker);
  const read = (text) => relationshipTargets(text, "");
  assert.deepEqual(await Promise.all([read("invalid"), read("ok")]), [
    null,
    [2],
  ]);
  assert.equal(await read("trap"), null);
  assert.deepEqual(await read("again"), [5]);
});

test("a module path keeps its dot segment after a directory symlink until a change forgets it", async (t) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "kaalang-tree-host-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const file = (name) => path.join(root, name);
  await fs.mkdir(file("branch/inner"), { recursive: true });
  await fs.writeFile(file("child.rs"), "struct Decoy;");
  try {
    await fs.symlink(file("branch/inner"), file("linked"), "dir");
  } catch (error) {
    if (error.code !== "EPERM") throw error;
    return t.skip(
      "Creating a directory symlink needs this platform's permission",
    );
  }
  const { forgetFound, treeHost } = require("../out/module-sources.js");
  const found = new Map();
  const consulted = new Map();
  const unsaved = new Map();
  const host = treeHost(found, unsaved, consulted);
  const candidate = host.join(file("lib.rs"), ["linked/../child.rs"]);
  assert.equal(host.canonical(candidate), undefined);
  // The watch for a missing module stands where the file system would put it.
  const physical = path.join(await fs.realpath(file("branch")), "child.rs");
  assert.ok(consulted.has(physical));
  await fs.writeFile(physical, "struct Chosen;");
  assert.equal(host.canonical(candidate), undefined);
  forgetFound(found, physical);
  assert.equal(host.canonical(candidate), physical);
  assert.equal(host.read(physical), "struct Chosen;");
  unsaved.set(physical, "struct Unsaved;");
  assert.equal(host.read(physical), "struct Unsaved;");
});

test("a saved file is read without its byte order mark", async (t) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "kaalang-tree-host-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const saved = path.join(root, "lib.rs");
  await fs.writeFile(saved, "\uFEFFstruct Saved;");
  const { treeHost } = require("../out/module-sources.js");
  const host = treeHost(new Map(), new Map(), new Map());
  assert.equal(host.read(saved), "struct Saved;");
});
