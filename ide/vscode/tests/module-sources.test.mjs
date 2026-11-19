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
    let opened=0,live=0;
    export class Workspace {
      constructor(snapshot) {
        if(live++) throw Error("old workspace was not freed");
        this.files=JSON.parse(snapshot).files;
        this.id=++opened;
      }
      references(focus,offset,candidates,place) {
        return JSON.stringify(JSON.parse(candidates).map(uri=>({
          uri,focus,offset,text:this.files[0].text,id:this.id,place:place&&JSON.parse(place)
        })));
      }
      free() { --live; }
    }
    export function moduleItems(text) {
      if (text === "invalid") throw new Error("parse error");
      if (text === "trap") throw new WebAssembly.RuntimeError("unreachable");
      return JSON.stringify({modules: [], types: [text.length]});
    }`,
  );
  const { moduleItems, workspaceReferences, stopModuleWorker } = createRequire(
    import.meta.url,
  )(reader);
  t.after(stopModuleWorker);
  const read = (text) => moduleItems(text, "", "lib", true);
  assert.deepEqual(await Promise.all([read("invalid"), read("ok")]), [
    null,
    { modules: [], types: [2] },
  ]);
  assert.equal(await read("trap"), null);
  assert.deepEqual(await read("again"), { modules: [], types: [5] });
  const source = { files: [{ uri: "one", text: "unsaved" }], crates: [] };
  const place = { owner: 0, module: "nested" };
  const lookup = () => workspaceReferences(source, "focus", 6, ["one"], place);
  const expected = [
    { uri: "one", focus: "focus", offset: 6, text: "unsaved", id: 1, place },
  ];
  assert.deepEqual(await lookup(), expected);
  const second = await workspaceReferences(source, "other", 0, ["one"], place);
  assert.equal(
    second[0].id,
    1,
    "the next focus reuses the parsed Rust workspace",
  );
  const edited = await workspaceReferences(
    { ...source },
    "other",
    0,
    ["one"],
    place,
  );
  assert.equal(edited[0].id, 2);
  assert.equal(await read("trap"), null);
  assert.deepEqual(await lookup(), expected);
});
