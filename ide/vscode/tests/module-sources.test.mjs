import assert from "node:assert/strict";
import fs from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";

test("the module reader survives invalid Rust and restarts after a Wasm trap", async (t) => {
  const directory = await fs.mkdtemp(
    path.join(os.tmpdir(), "kaalang-module-worker-"),
  );
  t.after(() => fs.rm(directory, { recursive: true, force: true }));
  const reader = path.join(directory, "module-sources.js");
  await fs.copyFile(
    new URL("../out/module-sources.js", import.meta.url),
    reader,
  );
  const wasm = path.join(directory, "wasm");
  await fs.mkdir(wasm);
  await fs.writeFile(path.join(wasm, "kaalang_js_bg.wasm"), "");
  await fs.writeFile(
    path.join(wasm, "kaalang_js.mjs"),
    `
    export function initSync() {}
    export function moduleItems(text) {
      if (text === "invalid") throw new Error("parse error");
      if (text === "trap") throw new WebAssembly.RuntimeError("unreachable");
      return JSON.stringify({modules: [], types: [text.length]});
    }`,
  );
  const { moduleItems, stopModuleWorker } = createRequire(import.meta.url)(
    reader,
  );
  t.after(stopModuleWorker);
  const read = (text) => moduleItems(text, "", "lib", true);
  assert.deepEqual(await Promise.all([read("invalid"), read("ok")]), [
    null,
    { modules: [], types: [2] },
  ]);
  assert.equal(await read("trap"), null);
  assert.deepEqual(await read("again"), { modules: [], types: [5] });
});
