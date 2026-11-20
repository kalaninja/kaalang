// The bindings as a page loads them: the module in V8, with positions in
// JavaScript's UTF-16 strings. `just wasm test` builds it first.

import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import * as kaalang from "../../../target/wasm/web/kaalang_js.js";

const built = path.join(import.meta.dirname, "../../../target/wasm/web");
const module = kaalang.initSync({
  module: fs.readFileSync(path.join(built, "kaalang_js_bg.wasm")),
});
const tests = path.join(import.meta.dirname, "../../kaalang/tests");
const target = fs.readFileSync(path.join(tests, "diagram-target.cfg"), "utf8");

/** Every fixture directory, compile-fail fixtures left out. */
function* directories(directory) {
  yield directory;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && entry.name !== "compile_fail") {
      yield* directories(path.join(directory, entry.name));
    }
  }
}

/** The flow a fixture declares, drawn, or `null` for one that declares none. */
function draw(source, collapsed) {
  return JSON.parse(kaalang.drawFlow(source, target, 0, collapsed));
}

test("every fixture flow draws its golden diagrams", () => {
  let drawn = 0;
  for (const directory of directories(tests)) {
    for (const file of fs
      .readdirSync(directory)
      .filter((name) => name.endsWith(".rs"))) {
      const source = fs.readFileSync(path.join(directory, file), "utf8");
      // A fixture declares one flow, named after its file, or its directory for `mod.rs`.
      const name =
        file === "mod.rs"
          ? path.basename(directory)
          : path.basename(file, ".rs");
      const golden = (suffix) => {
        const diagram = path.join(directory, `${name}${suffix}.svg`);
        return fs.existsSync(diagram) ? fs.readFileSync(diagram, "utf8") : null;
      };
      const expanded = draw(source, false);
      if (expanded === null) {
        continue;
      }
      assert.equal(expanded.svg, golden(""), `${directory}/${file}`);
      drawn += 1;
      if (source.includes("#[kaa::cycle(")) {
        // A collapsed view equal to the expanded one has no file of its own.
        const collapsed = draw(source, true);
        assert.equal(
          collapsed.svg,
          golden("_collapsed") ?? golden(""),
          `${directory}/${file}`,
        );
        drawn += 1;
      }
    }
  }
  assert.ok(drawn > 300, `${drawn} diagrams`);
});

test("drawing and asking many times keeps the memory level", () => {
  // A long comment makes every text the module might keep weigh 200 KB.
  const source =
    fs.readFileSync(
      path.join(tests, "flow/stage/add_through_stages.rs"),
      "utf8",
    ) + `// ${"x".repeat(200_000)}\n`;
  const memory = () => module.memory.buffer.byteLength;
  const round = () => {
    kaalang.drawFlow(source, "", 0, false);
    kaalang.declaredRanges(source, "");
  };
  for (let warm = 0; warm < 5; warm++) {
    round();
  }
  const settled = memory();
  for (let again = 0; again < 50; again++) {
    round();
  }
  assert.equal(memory(), settled);
});

test("positions count UTF-16 code units of a JavaScript string", () => {
  const source =
    "// Пример 🦀\n#[kaa]\nfn settle(first: u8) -> u8 {\n    #[kaa::call]\n    let end = |first| math::add(first);\n\n    |end| return end;\n}\n";
  const call = JSON.parse(kaalang.drawFlow(source, "", 0, false)).hotspots.find(
    (hotspot) => hotspot.kind === "call",
  );
  assert.equal(source.slice(call.start, call.end), "#[kaa::call]");
  assert.equal(source.slice(call.calleeStart, call.calleeEnd), "add");

  const [[start, end, declared]] = JSON.parse(
    kaalang.declaredRanges(source, ""),
  );
  assert.equal(
    source.slice(start, end),
    source.slice(source.indexOf("#[kaa]")).trimEnd(),
  );
  assert.deepEqual(declared, { flow: 0, path: "settle" });
});

test("declaration ranges cover what the target compiles from its first attribute", () => {
  const source =
    "\ufeff#!/usr/bin/env rustx\n// 🦀\n#[cfg(on)] struct A;#[kaa] fn run() { || return; }\nmod m { struct B; }";
  for (const cfg of ["", "on"]) {
    const ranges = JSON.parse(kaalang.declaredRanges(source, cfg));
    assert.equal(ranges.length, cfg ? 3 : 2);
    const flow = source.indexOf("#[kaa]");
    const [start, end, value] = ranges.find((range) => range[0] === flow);
    assert.equal(source.slice(start, end), "#[kaa] fn run() { || return; }");
    assert.deepEqual(value, { flow: 0, path: "run" });
    if (cfg) {
      assert.equal(
        source.slice(ranges[0][0], ranges[0][1]),
        "#[cfg(on)] struct A;",
      );
    }
  }
});

test("an active relationship exposes its final identifier in UTF-16", () => {
  const source = `// 🦀\n#[kaa] struct Review {
    #[cfg_attr(unix, kaa::refer(crate::names::Author, "The author."))] id: u8,
}`;
  assert.deepEqual(JSON.parse(kaalang.relationshipTargets(source, "unix")), [
    source.indexOf("Author"),
  ]);
  assert.deepEqual(JSON.parse(kaalang.relationshipTargets(source, "")), []);
});

test("a module tree gives each file its places in card order", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "kaalang-tree-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const sources = {
    "src/lib.rs": '// 🦀\nmod shared;\n#[path = "shared.rs"] mod alias;',
    "src/shared.rs": "#[cfg(test)] struct Tested;\nstruct Same;\nmod child;",
    "src/shared/child.rs": "struct First; // refer",
    "src/child.rs": "struct Second;",
    "tests/it.rs": '#[path = "../src/shared.rs"] mod shared;',
  };
  for (const [name, text] of Object.entries(sources)) {
    fs.mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    fs.writeFileSync(path.join(root, name), text);
  }
  const file = (name) => fs.realpathSync(path.join(root, name));
  let reads = [];
  const host = {
    join: (parent, parts) => path.join(path.dirname(parent), ...parts),
    canonical: (at) =>
      fs.statSync(at, { throwIfNoEntry: false })?.isFile()
        ? fs.realpathSync(at)
        : undefined,
    read(at) {
      reads.push(at);
      return fs.readFileSync(at, "utf8");
    },
  };
  const target = (name, kind, source, cfg) => ({
    package: "app",
    kind: [kind],
    name,
    root: path.join(root, source),
    cfg,
  });
  // A build script has no tree; a target's crate name has `_` for `-`.
  const roots = [
    target("it", "test", "tests/it.rs", "test"),
    target("build-script-build", "custom-build", "build.rs", ""),
    target("app-core", "lib", "src/lib.rs", ""),
  ];
  const tree = () =>
    JSON.parse(kaalang.moduleTree(JSON.stringify(roots), host));
  const { files, referring, complete } = tree();
  assert.equal(complete, true);
  const at = (name, text) => sources[name].indexOf(text);
  const same = at("src/shared.rs", "Same");
  const place = (crate, group, source, module, order) => ({
    package: "app",
    group,
    crate,
    root: path.join(root, source),
    module,
    order,
  });
  const library = (module, order) =>
    place("app_core", 0, "src/lib.rs", module, order);
  assert.deepEqual(files[file("src/shared.rs")], [
    {
      target: 2,
      place: library("shared", [at("src/lib.rs", "shared")]),
      types: [same],
    },
    {
      target: 2,
      place: library("alias", [at("src/lib.rs", "alias")]),
      types: [same],
    },
    {
      target: 0,
      place: place("it", 1, "tests/it.rs", "shared", [
        at("tests/it.rs", "shared;"),
      ]),
      types: [at("src/shared.rs", "Tested"), same],
    },
  ]);
  const modules = (name) => files[file(name)].map(({ place }) => place.module);
  assert.deepEqual(modules("src/shared/child.rs"), ["shared::child"]);
  assert.deepEqual(modules("src/child.rs"), ["alias::child", "shared::child"]);
  assert.deepEqual(referring, [file("src/shared/child.rs")]);

  // Only a forgotten file is read again; one that cannot be read leaves the tree incomplete.
  reads = [];
  kaalang.forgetModules(file("src/child.rs"));
  host.read = (at) => {
    reads.push(at);
    throw new Error("denied");
  };
  const unread = tree();
  assert.deepEqual([...new Set(reads)], [file("src/child.rs")]);
  assert.equal(unread.complete, false);
  assert.deepEqual(
    unread.files[file("src/child.rs")].map(({ types }) => types),
    [null, null],
  );
});

test("every fixture's first type draws its golden diagram", () => {
  const datatypes = path.join(tests, "datatype");
  let drawn = 0;
  for (const name of fs.readdirSync(datatypes)) {
    const directory = path.join(datatypes, name);
    if (!fs.existsSync(path.join(directory, "mod.rs"))) {
      continue;
    }
    const source = fs.readFileSync(path.join(directory, "mod.rs"), "utf8");
    const focus = source.match(
      /^(?:pub(?:\([^)]*\))? )?(?:struct|enum|union|type) ([A-Za-z_]\w*)/m,
    )[1];
    // The engine's references, as the fixtures stand in for them: every
    // relationship that names the type bare.
    const offsets = [
      ...source.matchAll(new RegExp(`refer\\(${focus}\\b`, "g")),
    ].map((found) => found.index + "refer(".length);
    const context = JSON.stringify({ offsets });
    const diagram = JSON.parse(
      kaalang.drawType(source, target, focus, context),
    );
    const golden = fs.readFileSync(path.join(directory, `${name}.svg`), "utf8");
    assert.equal(diagram.svg, golden, directory);
    drawn += 1;
  }
  assert.ok(drawn > 10, `${drawn} type diagrams`);
});

test("a card's place is its file's place inside the inline modules around it", () => {
  const root = "pub struct Focus; pub mod r#left;";
  const shared =
    '// 🦀\nmod inner { #[kaa] pub struct Report { #[kaa::refer(crate::Focus, "Owner")] id: u8 } }';
  const place = (module, ...order) => ({
    package: "app",
    group: 0,
    crate: "app",
    root: "src/lib.rs",
    module,
    order,
  });
  const drawing = JSON.parse(
    kaalang.drawType(
      root,
      "",
      "Focus",
      JSON.stringify({
        place: place("", 0),
        files: [
          {
            text: shared,
            cfg: "",
            offsets: [shared.indexOf("Focus")],
            place: place("r#left", root.indexOf("mod r#left")),
          },
        ],
      }),
    ),
  );
  assert.deepEqual(
    drawing.cards.map(({ source, path }) => [source, path]),
    [
      [0, "Focus"],
      [1, "inner::Report"],
    ],
  );
  assert.ok(drawing.svg.includes("r#left::inner::Report<"), drawing.svg);
});
