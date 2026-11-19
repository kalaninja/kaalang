import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { compileFunction } from "node:vm";

import { forgetModules, moduleTree, sourceOrder } from "../out/card-places.js";

test("body edits reuse the resolver while changed imports or type offsets build a new workspace", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "kaalang-body-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const file = (name) => path.join(directory, name);
  const uri = (name) => pathToFileURL(file(name)).href;
  const roots = ["lib", "main"].map((name) => ({
    name,
    root: uri(name + ".rs"),
    package: "app",
    packageId: "app",
    directory,
    library: name === "lib",
    group: 0,
    cfg: "",
    dependencies: [],
  }));
  const shared = 'struct Link { #[kaa::refer(crate::Focus,"yes")] id:u8 }';
  const base =
    'mod links; use alpha::First; fn work() { "🔵" } struct Focus; struct Other; fn tail() { x }';
  await Promise.all([
    writeFile(file("lib.rs"), base),
    writeFile(file("main.rs"), "mod links;"),
    writeFile(file("links.rs"), shared),
  ]);
  const parse = async (text) => ({
    modules: text.startsWith("mod links;")
      ? [
          {
            module: "links",
            offset: 4,
            files: [{ parts: ["links.rs"], ownsDirectory: false }],
          },
        ]
      : [],
    types: [...text.matchAll(/struct (\w+)/g)].map((match) => match.index + 7),
    bodies: [...text.matchAll(/fn \w+\(\) \{(.*?)\}/g)].map((match) => [
      match.index + match[0].indexOf("{") + 1,
      match.index + match[0].length - 1,
    ]),
  });
  const reads = (key) => readFile(fileURLToPath(key), "utf8");
  let graph = await moduleTree(roots, reads, parse);
  const snapshots = [];
  let resolve = async () => [];
  const exports = {};
  const host = new URL("../out/referring-sources.js", import.meta.url);
  const require = createRequire(host);
  const modules = {
    "./card-places": { editorTree: async () => graph, sourceOrder },
    "./module-sources": {
      workspaceReferences: async (snapshot) => {
        snapshots.push(snapshot);
        return resolve();
      },
    },
  };
  compileFunction(await readFile(host, "utf8"), ["exports", "require"])(
    exports,
    (key) => modules[key] ?? require(key),
  );
  const query = (text, name = "Focus") =>
    exports.referringSources(
      roots,
      true,
      uri("lib.rs"),
      text.indexOf("struct " + name) + 7,
    );
  const edit = async (text) => {
    await writeFile(file("lib.rs"), text);
    forgetModules(file("lib.rs"));
    graph = await moduleTree(roots, reads, parse);
    await query(text);
    return snapshots.at(-1);
  };
  await query(base);
  await query(base);
  assert.equal(snapshots.length, 1, "the same graph caches an answer");
  const first = snapshots[0];
  const body = base.replace("🔵", "🟦").replace("{ x }", "{ return y; }");
  assert.equal(await edit(body), first);
  assert.equal(
    snapshots.length,
    2,
    "a new graph asks the resolver with its current candidates",
  );
  const imported = body.replace("alpha", "bravo");
  const second = await edit(imported);
  assert.notEqual(second, first, "a same-width import can change a target");
  const last = imported.replace("🟦", "longer_body");
  const moved = await edit(last);
  assert.notEqual(moved, second, "the name of a subsequent type moved");
  const count = snapshots.length;
  await query(last, "Other");
  await query(last);
  await query(last, "Other");
  assert.equal(snapshots.length, count + 3, "only the last focus is retained");

  resolve = async () => {
    throw new Error("worker stopped");
  };
  await assert.rejects(query(last), /worker stopped/);
  resolve = async () => [];
  await query(last);
  assert.equal(snapshots.length, count + 5, "a failed answer can be retried");

  let started, finish;
  const waiting = new Promise((ready) => {
    started = ready;
  });
  resolve = () =>
    new Promise((done) => {
      finish = done;
      started();
    });
  const old = query(last, "Other");
  await waiting;
  resolve = async () => [];
  await query(last);
  finish(null);
  await old;
  await query(last);
  assert.equal(
    snapshots.length,
    count + 7,
    "an older failure keeps the current answer",
  );
});
