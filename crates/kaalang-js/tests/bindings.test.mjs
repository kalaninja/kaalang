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
