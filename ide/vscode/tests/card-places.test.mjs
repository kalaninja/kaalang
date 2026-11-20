import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { compileFunction } from "node:vm";

/** The built `card-places` module over a worker that answers `tree`. */
async function places(tree) {
  const built = new URL("../out/card-places.js", import.meta.url);
  const require = createRequire(built);
  const asked = [];
  const modules = {
    "./module-sources": {
      moduleTree: async (roots, unsaved) => {
        asked.push({ roots, unsaved });
        return tree;
      },
      forgetTrees: () => {},
    },
  };
  const host = {};
  compileFunction(await readFile(built, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(built),
  })(host, (name) => modules[name] ?? require(name));
  return { ...host, asked };
}

test("each reading of the bindings' trees takes its target's place and set", async () => {
  const lib = path.resolve("project", "src", "lib.rs");
  const it = path.resolve("project", "tests", "it.rs");
  const target = (name, group, file, cfg) => ({
    package: "app",
    group,
    name,
    root: pathToFileURL(file).href,
    packageId: "app 0.1.0",
    cfg,
  });
  const roots = [target("app", 0, lib, ""), target("it", 1, it, "test")];
  const shared = path.resolve("project", "src", "shared.rs");
  const { asked, workspaceTree } = await places({
    files: {
      [shared]: [
        { target: 0, module: "shared", order: [4], types: [7] },
        { target: 1, module: "shared", order: [10], types: null },
      ],
    },
    referring: [shared],
    complete: true,
    stamps: [[shared, "stamp"]],
  });
  const graph = await workspaceTree(roots, [[lib, "unsaved"]]);
  assert.deepEqual(asked, [
    {
      roots: [
        { package: "app", group: 0, name: "app", root: lib, cfg: "" },
        { package: "app", group: 1, name: "it", root: it, cfg: "test" },
      ],
      unsaved: [[lib, "unsaved"]],
    },
  ]);
  const place = (crate, group, file, order) => ({
    package: "app",
    group,
    crate,
    root: file.split(path.sep).join("/"),
    module: "shared",
    order,
  });
  assert.deepEqual(graph.files.get(shared), [
    { place: place("app", 0, lib, [4]), cfg: "", types: new Set([7]) },
    { place: place("it", 1, it, [10]), cfg: "test", types: null },
  ]);
  assert.deepEqual(graph.paths, new Map([[shared, "stamp"]]));
  assert.deepEqual(graph.referring, [shared]);
  assert.equal(graph.complete, true);
});

test("a failed worker gives an incomplete, empty tree", async () => {
  const { workspaceTree } = await places(null);
  const graph = await workspaceTree([], []);
  assert.equal(graph.complete, false);
  assert.equal(graph.files.size, 0);
});
