// The bindings as a page loads them: the module in V8, with positions in
// JavaScript's UTF-16 strings. `just wasm test` builds it first.

import assert from "node:assert";
import fs from "node:fs";
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
    kaalang.declaredAt(source, "", source.indexOf("#[kaa]"));
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

test("indexing a module shared by many Cargo targets fits the memory for separate files", () => {
  const text =
    Array.from({ length: 80 }, (_, i) => `pub struct Type${i};`).join("\n") +
    '\n#[kaa] pub struct Report { #[kaa::refer(Type0, "Field")] id: u8 }';
  const count = 160;
  function index(shared) {
    const files = [];
    const add = (uri, text) => files.push({ uri, text }) - 1;
    const common = shared ? add("shared.rs", text) : undefined;
    const crates = Array.from({ length: count }, (_, i) => ({
      packageId: "app",
      name: `test${i}`,
      library: false,
      cfg: "test",
      dependencies: [],
      modules: [
        { path: "", file: add(`test${i}.rs`, "mod model;") },
        { path: "model", file: common ?? add(`model${i}.rs`, text) },
      ],
    }));
    const workspace = new kaalang.Workspace(JSON.stringify({ files, crates }));
    try {
      const { uri } = files[crates[0].modules[1].file];
      const matches = JSON.parse(
        workspace.references(uri, text.indexOf("Type0"), JSON.stringify([uri])),
      );
      assert.equal(matches.length, shared ? count : 1);
      assert.ok(matches.every(({ offsets }) => offsets.length === 1));
      return module.memory.buffer.byteLength;
    } finally {
      workspace.free();
    }
  }
  const separate = index(false);
  const shared = index(true);
  assert.ok(
    shared <= separate * 1.25,
    `Shared files used ${shared} bytes, separate files used ${separate}`,
  );
});

test("positions count UTF-16 code units of a JavaScript string", () => {
  const source =
    "// Пример 🦀\n#[kaa]\nfn settle(first: u8) -> u8 {\n    #[kaa::call]\n    let end = |first| math::add(first);\n\n    |end| return end;\n}\n";
  const call = JSON.parse(kaalang.drawFlow(source, "", 0, false)).hotspots.find(
    (hotspot) => hotspot.kind === "call",
  );
  assert.equal(source.slice(call.start, call.end), "#[kaa::call]");
  assert.equal(source.slice(call.calleeStart, call.calleeEnd), "add");

  const declared = (offset) =>
    JSON.parse(kaalang.declaredAt(source, "", offset));
  assert.deepEqual(declared(source.indexOf("math::add")), {
    flow: 0,
    path: "settle",
  });
  assert.equal(declared(source.indexOf("🦀")), null);
});

test("the caret index includes active declarations, adjacent boundaries and Unicode offsets", () => {
  const source =
    "\ufeff#!/usr/bin/env rustx\n// 🦀\n#[cfg(on)] struct A;#[kaa] fn run() { || return; }\nmod m { struct B; }";
  for (const cfg of ["", "on"]) {
    const ranges = JSON.parse(kaalang.declaredRanges(source, cfg));
    assert.equal(ranges.length, cfg ? 3 : 2);
    for (let offset = 0; offset <= source.length + 1; offset++) {
      const active = ranges.findLast(
        ([start, end]) => start <= offset && offset <= end,
      );
      assert.deepEqual(
        active?.[2] ?? null,
        JSON.parse(kaalang.declaredAt(source, cfg, offset)),
      );
    }
    const flow = source.indexOf("#[kaa]");
    const [start, end, value] = ranges.find((range) => range[0] === flow);
    assert.equal(source.slice(start, end), "#[kaa] fn run() { || return; }");
    assert.deepEqual(value, { flow: 0, path: "run" });
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

test("one source can own different module files in two Cargo targets", () => {
  const source = "// 🦀\nstruct Focus;\n#[cfg(test)] enum Extra {}\nmod child;";
  const items = (cfg, owns) =>
    JSON.parse(kaalang.moduleItems(source, cfg, "shared", owns));
  const normal = items("", false);
  assert.deepEqual(normal.types, [source.indexOf("Focus")]);
  const [child] = normal.modules;
  assert.equal(child.module, "child");
  assert.equal(child.offset, source.indexOf("child"));
  const files = (entry) =>
    entry.files.map(({ parts, ownsDirectory }) => [
      path.posix.join(...parts),
      ownsDirectory,
    ]);
  assert.deepEqual(files(child), [
    ["shared/child.rs", false],
    ["shared/child/mod.rs", true],
  ]);
  const explicit = items("test", true);
  assert.deepEqual(explicit.types, [
    source.indexOf("Focus"),
    source.indexOf("Extra"),
  ]);
  assert.deepEqual(files(explicit.modules[0]), [
    ["child.rs", false],
    ["child/mod.rs", true],
  ]);
});

test("workspace body ranges use UTF-16 inside the braces of configured module items", () => {
  const source = `\ufeff#!/usr/bin/env rustx
// 🦀
fn f() { fn local() {} }
mod inner {
    #[cfg(extra)] fn gated() { "🦀"; }
    trait T { fn m(&self) {} }
    impl T for () { fn m(&self) {} }
    struct Fields { n: u8 }
    mod nested { fn empty() {} }
}
`;
  for (const cfg of ["", "extra"]) {
    const { bodies } = JSON.parse(
      kaalang.moduleItems(source, cfg, "lib", true),
    );
    assert.deepEqual(
      bodies.map(([start, end]) => source.slice(start, end)),
      [
        " fn local() {} ",
        ...(cfg ? [' "🦀"; '] : []),
        " fn m(&self) {} ",
        " fn m(&self) {} ",
        "",
      ],
    );
    for (const [start, end] of bodies) {
      assert.equal(source[start - 1], "{");
      assert.equal(source[end], "}");
    }
    assert.equal(
      bodies.at(-1)[0],
      source.indexOf("{}", source.indexOf("fn empty")) + 1,
    );
  }
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

test("Cargo owners resolve one physical target through their own dependencies and modules", (t) => {
  const model =
    "\ufeff#!/usr/bin/env rustx\n// 🦀\npub struct Focus; pub mod names { pub use crate::Focus as Author; }" +
    'mod expanded { generated!(); #[kaa] struct Late { #[kaa::refer(Generated, "The generated alias.")] id: u8 } }';
  const client =
    "pub use renamed_model::names::Author as Focus; pub use left::links::nested::Incoming as Preferred; pub mod left { pub use crate::Focus; pub mod links; } pub mod right { pub struct Focus; pub mod links; }";
  const other =
    "pub struct Focus; pub mod links; pub use links::nested::Incoming as Preferred;";
  const shared =
    '\ufeff#!/usr/bin/env rustx\n// 🦀\npub mod nested { use core::prelude::v1::derive as kaa; generated!(); #[kaalang::kaa] #[kaa(Debug)] pub struct Incoming { #[kaa::refer(super::super::Focus, "The owner.")] id: u8, #[kaa::refer(crate::Preferred, "Preferred.")] other: u8, #[kaa::refer(Generated, "Generated.")] unknown: u8 } }';
  const files = [model, client, other, shared].map((text, index) => ({
    uri: `file:///src/${index}.rs`,
    text,
  }));
  const mount = (path, file) => ({ path, file });
  const target = (id, dependencies, ...modules) => ({
    packageId: `pkg${id}`,
    name: `crate${id}`,
    library: true,
    cfg: "",
    dependencies,
    modules: [mount("", id), ...modules],
  });
  const snapshot = {
    files,
    crates: [
      target(0, []),
      target(
        1,
        [["renamed_model", "pkg0"]],
        mount("left::links", 3),
        mount("right::links", 3),
      ),
      target(2, [], mount("links", 3)),
    ],
  };
  const workspace = new kaalang.Workspace(JSON.stringify(snapshot));
  t.after(() => workspace.free());
  const references = (id, offset, candidates, selected) =>
    JSON.parse(
      workspace.references(
        files[id].uri,
        offset,
        JSON.stringify(candidates.map((id) => files[id].uri)),
        selected && JSON.stringify(selected),
      ),
    );
  const answer = references(0, model.indexOf("Focus"), [0, 3]);
  assert.deepEqual(
    answer.map(({ owner, module, offsets }) => [owner, module, offsets]),
    [
      [0, "", []],
      [1, "left::links", [shared.indexOf("Focus")]],
      [1, "right::links", []],
      [2, "links", []],
    ],
  );
  assert.ok(answer[0].focus);
  assert.deepEqual(answer[0].unknown, [model.indexOf("Generated")]);
  // One editor location cannot confirm a macro expansion for three different modules.
  assert.ok(answer.slice(1).every((entry) => entry.unknown === undefined));
  // Drawing between queries releases that call's spans.
  kaalang.drawType(model, "", "Focus", "");
  assert.deepEqual(
    references(2, other.indexOf("Focus"), [3]).map((entry) => entry.offsets),
    [[], [], [shared.indexOf("Focus")]],
  );
  // The same declaration is two distinct types in this crate. Its enclosing
  // external module identifies which one the editor is showing.
  assert.equal(references(3, shared.indexOf("Incoming"), [3]), null);
  const selected = references(3, shared.indexOf("Incoming"), [0, 3], {
    owner: 1,
    module: "right::links",
  });
  assert.deepEqual(
    selected.filter((entry) => entry.focus).map((entry) => entry.module),
    ["right::links"],
  );
  // The destination itself also has to identify one type by file and offset.
  assert.ok(selected.every((entry) => entry.unknown === undefined));
  const readings = references(3, shared.indexOf("Incoming"), [3], {
    owner: 1,
    module: "left::links",
  });
  const own = readings.find((reading) => reading.focus);
  const context = {
    ...own,
    resolved: true,
    files: readings
      .filter((reading) => !reading.focus && reading.offsets.length)
      .map((reading) => ({ ...reading, text: shared, resolved: true })),
  };
  // The second inclusion is a distinct incoming type at the same physical offset.
  assert.deepEqual(
    context.files.map((file) => file.module),
    ["right::links"],
  );
  const diagram = JSON.parse(
    kaalang.drawType(shared, "", "nested::Incoming", JSON.stringify(context)),
  );
  assert.deepEqual(
    diagram.cards.map(({ source, path }) => [source, path]),
    [
      [0, "nested::Incoming"],
      [1, "nested::Incoming"],
    ],
  );
});

test("two incoming types in one file keep their module while other targets share their card", (t) => {
  const root = {
    uri: "lib.rs",
    text: "pub struct Focus; pub struct Other; pub mod r#left; pub mod right;",
  };
  const shared = {
    uri: "shared.rs",
    text: '// 🦀\n#[kaa] pub struct Report { #[kaa::refer(crate::Focus, "Owner")] id: u8, #[kaa::refer(crate::Other, "Other")] other: u8 }',
  };
  const check = {
    uri: "check.rs",
    text: "pub use app::{Focus, Other}; pub mod left;",
  };
  const crates = [
    {
      name: "app",
      modules: [
        ["", 0],
        ["r#left", 1],
        ["right", 1],
      ],
    },
    {
      name: "check",
      modules: [
        ["", 2],
        ["left", 1],
      ],
    },
  ].map(({ name, modules }, owner) => ({
    packageId: "app",
    name,
    cfg: "",
    library: owner === 0,
    dependencies: [],
    modules: modules.map(([path, file]) => ({ path, file })),
  }));
  const workspace = new kaalang.Workspace(
    JSON.stringify({ files: [root, shared, check], crates }),
  );
  t.after(() => workspace.free());
  const matches = JSON.parse(
    workspace.references(
      root.uri,
      root.text.indexOf("Focus"),
      JSON.stringify([shared.uri]),
    ),
  );
  assert.deepEqual(
    matches.map(({ module, repeated, offsets }) => [
      module,
      !!repeated,
      offsets,
    ]),
    [
      ["r#left", true, [shared.text.indexOf("Focus")]],
      ["right", true, [shared.text.indexOf("Focus")]],
      ["left", false, [shared.text.indexOf("Focus")]],
    ],
  );
  const context = {
    uri: root.uri,
    modules: { Focus: "app" },
    files: matches.map((reading) => ({
      ...reading,
      text: shared.text,
      resolved: true,
      modules: { Report: crates[reading.owner].name + "::" + reading.module },
    })),
  };
  const drawing = JSON.parse(
    kaalang.drawType(root.text, "", "Focus", JSON.stringify(context)),
  );
  assert.deepEqual(
    drawing.cards.map(({ source }) => source),
    [0, 1, 2],
  );
  assert.ok(
    drawing.svg.includes("r#left::Report<") &&
      drawing.svg.includes("right::Report<"),
  );
  const targets = drawing.hotspots.filter((h) => h.kind === "target");
  for (const source of [1, 2]) {
    assert.deepEqual(
      targets
        .filter((h) => h.source === source)
        .map((h) => [shared.text.slice(h.start, h.end), !!h.focus]),
      [
        ["Focus", true],
        ["Other", false],
      ],
    );
  }
});
