import assert from "node:assert";
import * as path from "node:path";
import test from "node:test";

import { applyExtraCfgs, fileCfgs } from "../out/cfgs.js";

test("extra cfgs add names and values, and remove those marked with `!`", () => {
  const lines = new Set(["debug_assertions", "unix"]);
  applyExtraCfgs(
    lines,
    ["miri", "edition=2024", "!debug_assertions"],
    () => {},
  );
  assert.deepEqual([...lines].sort(), ['edition="2024"', "miri", "unix"]);
});

test("an extra cfg that names no cfg is left out and logged", () => {
  const lines = new Set(["unix"]);
  const logged = [];
  applyExtraCfgs(lines, ['feature="x"', "my-flag", "mode=a\nb", 3], (line) =>
    logged.push(line),
  );
  assert.deepEqual([...lines], ["unix"]);
  assert.equal(logged.length, 4);
});

/** A workspace with a member nested in another, and a dependency outside it. */
const metadata = {
  packages: [
    { id: "outer", manifest_path: path.resolve("/w/Cargo.toml"), source: null },
    {
      id: "inner",
      manifest_path: path.resolve("/w/inner/Cargo.toml"),
      source: null,
    },
    {
      id: "dependency",
      manifest_path: path.resolve("/registry/dependency/Cargo.toml"),
      source: "registry+https://github.com/rust-lang/crates.io-index",
    },
  ],
  resolve: {
    nodes: [
      { id: "outer", features: ["default", "outer_feature"] },
      { id: "inner", features: ["inner_feature"] },
      { id: "dependency", features: ["std"] },
    ],
  },
};

const cfgs = (file, setTest = true, extras = []) =>
  fileCfgs(
    [],
    metadata,
    path.resolve(file),
    { setTest, cfgs: extras },
    () => {},
  )
    .split("\n")
    .filter(Boolean)
    .sort();

test("a file takes the features of the package that holds it most closely", () => {
  assert.deepEqual(cfgs("/w/inner/src/lib.rs"), [
    'feature="inner_feature"',
    "rust_analyzer",
    "test",
  ]);
});

test("a directory that only starts like a package's is not inside it", () => {
  assert.deepEqual(cfgs("/w/inner_tools/main.rs"), [
    'feature="default"',
    'feature="outer_feature"',
    "rust_analyzer",
    "test",
  ]);
});

test("a workspace crate takes no `test` when rust-analyzer sets none", () => {
  assert.deepEqual(cfgs("/w/src/lib.rs", false), [
    'feature="default"',
    'feature="outer_feature"',
    "rust_analyzer",
  ]);
});

test("settings override local crate flags before cargo adds features", () => {
  assert.deepEqual(
    cfgs("/w/inner/src/lib.rs", true, [
      "!test",
      "!rust_analyzer",
      "!feature=inner_feature",
      "!miri",
      "miri",
    ]),
    ['feature="inner_feature"'],
  );
});

test("a dependency takes its features but no cfg of a workspace crate", () => {
  assert.deepEqual(cfgs("/registry/dependency/src/lib.rs"), ['feature="std"']);
});

test("a file in no package takes nothing", () => {
  assert.deepEqual(cfgs("/elsewhere/main.rs"), []);
});
