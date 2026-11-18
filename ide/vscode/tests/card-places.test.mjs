import assert from "node:assert/strict";
import {
  mkdir,
  mkdtemp,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import { moduleTree, placements } from "../out/card-places.js";

async function workspace(t, sources) {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-modules-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const file = (name) => path.join(root, name);
  const uri = (name) => pathToFileURL(file(name)).href;
  for (const [name, source] of Object.entries(sources)) {
    await mkdir(path.dirname(file(name)), { recursive: true });
    await writeFile(file(name), source);
  }
  const target = (name, crate, group = 0, cfg = "") => ({
    root: uri(name),
    name: crate,
    package: "sample",
    group,
    cfg,
  });
  const card = (name, type) => ({
    uri: uri(name),
    path: type,
    offset: sources[name].indexOf(type.split("::").at(-1)),
  });
  return { file, uri, target, card };
}

const read = (uri) => readFile(fileURLToPath(uri), "utf8");
const candidate = (ownsDirectory, ...parts) => ({ parts, ownsDirectory });

test("shared source has distinct children and source order in each directory", async (t) => {
  const lib = 'struct A; mod shared; #[path="shared.rs"] mod alias; struct B;';
  const bin = '#[path="lib.rs"] mod library; #[path="shared.rs"] mod second;';
  const shared = "mod child; mod nested { struct Same; }";
  const child = "struct First;";
  const other = "struct Second;";
  const files = {
    "lib.rs": lib,
    "main.rs": bin,
    "shared.rs": shared,
    "shared/child.rs": child,
    "child.rs": other,
  };
  const { file, target, card } = await workspace(t, files);
  const counts = new Map();
  const readOnce = async (uri) => {
    counts.set(uri, (counts.get(uri) ?? 0) + 1);
    return read(uri);
  };
  const parses = [];
  // The compiler's configured declarations. Only a default shared.rs owns
  // shared/; a #[path] reading of that same text looks next to shared.rs.
  const parse = async (text, _cfg, stem, owns) => {
    parses.push([text, stem, owns]);
    const mod = (module, ...files) => ({
      module,
      offset: text.indexOf(module),
      files,
    });
    switch (text) {
      case lib:
        return {
          types: [lib.indexOf("A"), lib.indexOf("B")],
          modules: [
            mod(
              "shared",
              candidate(false, "shared.rs"),
              candidate(true, "shared", "mod.rs"),
            ),
            mod("alias", candidate(true, "shared.rs")),
          ],
        };
      case bin:
        return {
          types: [],
          modules: [
            mod("library", candidate(true, "lib.rs")),
            mod("second", candidate(true, "shared.rs")),
          ],
        };
      case shared: {
        const dir = owns ? [] : [stem];
        return {
          types: [text.indexOf("Same")],
          modules: [
            mod(
              "child",
              candidate(false, ...dir, "child.rs"),
              candidate(true, ...dir, "child", "mod.rs"),
            ),
          ],
        };
      }
      default:
        return { types: [text.indexOf(" ") + 1], modules: [] };
    }
  };
  const graph = await moduleTree(
    [target("lib.rs", "model"), target("main.rs", "example", 2)],
    readOnce,
    parse,
  );
  assert.equal(graph.complete, true);
  assert.equal(graph.files.size, 5);
  assert.deepEqual([...counts.values()], [1, 1, 1, 1, 1]);
  assert.equal(parses.filter(([text]) => text === shared).length, 2);

  const names = [
    ["lib.rs", "A"],
    ["shared/child.rs", "First"],
    ["child.rs", "Second"],
    ["lib.rs", "B"],
    ["shared.rs", "nested::Same"],
  ];
  const [a, first, second, b, same] = await placements(
    graph,
    names.map(([name, type]) => card(name, type)),
  );
  assert.deepEqual(first.map((p) => p.module).sort(), [
    "library::shared::child",
    "shared::child",
  ]);
  assert.deepEqual(second.map((p) => p.module).sort(), [
    "alias::child",
    "library::alias::child",
    "second::child",
  ]);
  assert.deepEqual(same.map((p) => p.module).sort(), [
    "alias::nested",
    "library::alias::nested",
    "library::shared::nested",
    "second::nested",
    "shared::nested",
  ]);
  const model = (choices) => choices.find((p) => p.crate === "model");
  assert.deepEqual(
    [model(a).order, model(first).order, model(b).order],
    [
      [lib.indexOf("A")],
      [lib.indexOf("shared"), shared.indexOf("child"), child.indexOf("First")],
      [lib.indexOf("B")],
    ],
  );
  assert.deepEqual(
    b.find((p) => p.crate === "example"),
    {
      package: "sample",
      group: 2,
      crate: "example",
      root: file("main.rs").split(path.sep).join("/"),
      module: "library",
      order: [bin.indexOf("library"), lib.indexOf("B")],
    },
  );
});

test("only the targets whose cfg declares the type get a placement", async (t) => {
  const text = "#[cfg(test)] struct OnlyTests;";
  const { uri, target, card } = await workspace(t, {
    "lib.rs": text,
    "outside.rs": "struct Other;",
  });
  const graph = await moduleTree(
    [target("lib.rs", "library"), target("lib.rs", "tests", 1, "test")],
    read,
    async (_text, cfg) => ({
      modules: [],
      types: cfg === "test" ? [text.indexOf("OnlyTests")] : [],
    }),
  );
  const [enabled, file, excluded] = await placements(graph, [
    card("lib.rs", "OnlyTests"),
    { uri: uri("lib.rs"), offset: 0 },
    card("outside.rs", "Other"),
  ]);
  assert.deepEqual(
    enabled.map((p) => p.crate),
    ["tests"],
  );
  assert.deepEqual(file.map((p) => p.crate).sort(), ["library", "tests"]);
  assert.deepEqual(excluded, []);
});

test("an unreadable module leaves other placements available and unknown sources retryable", async (t) => {
  const lib = "struct Ready; mod broken;";
  const { target, card } = await workspace(t, {
    "lib.rs": lib,
    "broken.rs": "struct",
    "outside.rs": "struct Other;",
  });
  const graph = await moduleTree(
    [target("lib.rs", "app")],
    read,
    async (text) =>
      text === lib
        ? {
            modules: [
              {
                module: "broken",
                offset: lib.indexOf("broken"),
                files: [candidate(false, "broken.rs")],
              },
            ],
            types: [lib.indexOf("Ready")],
          }
        : null,
  );
  assert.equal(graph.complete, false);
  const [known, uncertain] = await placements(graph, [
    card("lib.rs", "Ready"),
    card("outside.rs", "Other"),
  ]);
  assert.equal(known[0].crate, "app");
  assert.equal(uncertain, null);
});

test("a symlink preserves the original Cargo root and physical cycles terminate", async (t) => {
  const source = "struct A;";
  const { file, target, card } = await workspace(t, { "real.rs": source });
  try {
    await symlink(file("real.rs"), file("link.rs"), "file");
  } catch (error) {
    if (error.code !== "EPERM") throw error;
    return t.skip("Creating a file symlink needs this platform's permission");
  }
  const graph = await moduleTree(
    [target("real.rs", "same"), target("link.rs", "same")],
    read,
    async () => ({
      modules: [
        { module: "again", offset: 0, files: [candidate(true, "real.rs")] },
      ],
      types: [source.indexOf("A")],
    }),
  );
  const [found] = await placements(graph, [card("real.rs", "A")]);
  assert.deepEqual(
    found.map((p) => p.root).sort(),
    [file("link.rs"), file("real.rs")].map((p) => p.split(path.sep).join("/")),
  );
  assert.equal(graph.files.size, 1);
});

test("a dot segment is resolved after a symlink in an explicit module path", async (t) => {
  const lib = '#[path="linked/../child.rs"] mod selected;';
  const good = "struct Chosen;";
  const { file, target, card } = await workspace(t, {
    "lib.rs": lib,
    "branch/inner/leaf.rs": "",
    "branch/child.rs": good,
    "child.rs": "struct Decoy;",
  });
  try {
    await symlink(file("branch/inner"), file("linked"), "dir");
  } catch (error) {
    if (error.code !== "EPERM") throw error;
    return t.skip(
      "Creating a directory symlink needs this platform's permission",
    );
  }
  const graph = await moduleTree(
    [target("lib.rs", "app")],
    read,
    async (text) =>
      text === lib
        ? {
            modules: [
              {
                module: "selected",
                offset: lib.indexOf("selected"),
                files: [candidate(true, "linked/../child.rs")],
              },
            ],
            types: [],
          }
        : { modules: [], types: text === good ? [good.indexOf("Chosen")] : [] },
  );
  const [chosen, decoy] = await placements(graph, [
    card("branch/child.rs", "Chosen"),
    card("child.rs", "Decoy"),
  ]);
  assert.equal(chosen[0].module, "selected");
  assert.deepEqual(decoy, []);
});

test("a superseded snapshot stops before reading further modules", async (t) => {
  const { target } = await workspace(t, {
    "lib.rs": "mod child;",
    "child.rs": "struct A;",
  });
  let current = true;
  const seen = [];
  const graph = await moduleTree(
    [target("lib.rs", "app")],
    read,
    async (text) => {
      current = false;
      seen.push(text);
      return {
        modules: [
          { module: "child", offset: 4, files: [candidate(false, "child.rs")] },
        ],
        types: [],
      };
    },
    () => current,
  );
  assert.deepEqual(seen, ["mod child;"]);
  assert.equal(graph.complete, false);
  assert.equal(graph.files.size, 0);
});

test("only an edited source is reparsed in the next snapshot of a workspace", async (t) => {
  const root = "mod child; struct Root;";
  const child = "struct A;";
  const { file, target, uri } = await workspace(t, {
    "lib.rs": root,
    "child.rs": child,
  });
  const roots = [target("lib.rs", "app")];
  const calls = [];
  const parse = async (text) => {
    calls.push(text);
    return {
      types: [text.lastIndexOf(" ") + 1],
      modules:
        text === root
          ? [
              {
                module: "child",
                offset: 4,
                files: [candidate(false, "child.rs")],
              },
            ]
          : [],
    };
  };
  await moduleTree(roots, read, parse);
  assert.deepEqual(calls, [root, child]);
  await moduleTree(roots, read, parse);
  assert.deepEqual(calls, [root, child]);
  const edited = "// Changed\nstruct B;";
  await writeFile(file("child.rs"), edited);
  const graph = await moduleTree(roots, read, parse);
  assert.deepEqual(calls, [root, child, edited]);
  const [card] = await placements(graph, [
    { uri: uri("child.rs"), path: "B", offset: edited.indexOf("B") },
  ]);
  assert.equal(card[0].module, "child");
});
