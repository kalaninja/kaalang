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
  const target = (name, kind, file, cfg) => ({
    package: "app",
    kind: [kind],
    name,
    root: pathToFileURL(file).href,
    cfg,
  });
  const roots = [
    target("app", "lib", lib, ""),
    target("it", "test", it, "test"),
  ];
  const shared = path.resolve("project", "src", "shared.rs");
  // The bindings' places, which the host passes on as they are.
  const place = (crate, group, file, order) => ({
    package: "app",
    group,
    crate,
    root: file,
    module: "shared",
    order,
  });
  const first = place("app", 0, lib, [4]);
  const second = place("it", 1, it, [10]);
  const { asked, workspaceTree } = await places({
    files: {
      [shared]: [
        { target: 0, place: first, types: [7] },
        { target: 1, place: second, types: null },
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
        { package: "app", kind: ["lib"], name: "app", root: lib, cfg: "" },
        { package: "app", kind: ["test"], name: "it", root: it, cfg: "test" },
      ],
      unsaved: [[lib, "unsaved"]],
    },
  ]);
  assert.deepEqual(graph.files.get(shared), [
    { place: first, cfg: "", types: new Set([7]) },
    { place: second, cfg: "test", types: null },
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
