import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { compileFunction } from "node:vm";

test("each target reads its package's build cfgs without creating or rewriting Cargo.lock", async (t) => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "kaalang-target-"));
  t.after(() => rm(temp, { recursive: true, force: true }));
  const root = path.join(temp, "workspace");
  const file = (...parts) => path.join(root, ...parts);
  await mkdir(root);
  await writeFile(
    file("Cargo.toml"),
    '[workspace]\nmembers = ["catalog", "other"]\nresolver = "2"\n',
  );
  const script = `use std::{env, fs, path::Path};
fn main() {
    let package = env::var("CARGO_PKG_NAME").unwrap();
    let mode = env::var("KAALANG_TEST_MODE").unwrap();
    fs::write(Path::new(&env::var_os("KAALANG_TEST_MARKERS").unwrap()).join(&package), &mode).unwrap();
    println!("cargo::rerun-if-env-changed=KAALANG_TEST_MODE");
    println!("cargo:rustc-cfg=from_{package}");
    println!("cargo::rustc-cfg=chosen_mode=\\"{mode}\\"");
    if env::var_os("CARGO_FEATURE_AUDIT").is_some() {
        println!("cargo::rustc-cfg=script_saw_feature");
    }
}
`;
  for (const name of ["catalog", "other"]) {
    await mkdir(file(name, "src"), { recursive: true });
    await writeFile(
      file(name, "Cargo.toml"),
      `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n` +
        (name === "catalog" ? "[features]\naudit = []\n" : ""),
    );
    await writeFile(file(name, "build.rs"), script);
    await writeFile(
      file(name, "src/lib.rs"),
      name === "catalog"
        ? "pub struct Member;\n"
        : 'compile_error!("this ordinary library does not compile");\n',
    );
  }
  await writeFile(file("catalog/src/main.rs"), "fn main() {}\n");

  const uri = (fsPath) => ({
    scheme: "file",
    fsPath,
    toString: () => pathToFileURL(fsPath).href,
  });
  const folder = { uri: uri(root) };
  const env = {
    CARGO_NET_OFFLINE: "true",
    CARGO_TARGET_DIR: path.join(temp, "target"),
    KAALANG_TEST_MARKERS: temp,
    KAALANG_TEST_MODE: "editor",
  };
  const settings = {
    "cargo.features": ["catalog/audit"],
    "cargo.extraEnv": env,
    "cargo.buildScripts.enable": true,
    "procMacro.enable": false,
  };
  const workspace = {
    isTrusted: false,
    workspaceFolders: [folder],
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({ get: (key) => settings[key] }),
  };
  const built = new URL("../out/target.js", import.meta.url);
  const require = createRequire(built);
  const host = {};
  compileFunction(await readFile(built, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(built),
  })(host, (name) =>
    name === "vscode" ? { workspace, Uri: { file: uri } } : require(name),
  );

  const catalog = uri(file("catalog/src/lib.rs"));
  const other = uri(file("other/src/lib.rs"));
  const logs = [];
  const target = (source) => host.target(source, (line) => logs.push(line));
  const restricted = await target(catalog);
  assert.equal(restricted.cfg, "");
  assert.deepEqual(restricted.roots, []);
  assert.equal(existsSync(path.join(temp, "catalog")), false);
  assert.equal(existsSync(path.join(temp, "other")), false);
  assert.equal(existsSync(file("Cargo.lock")), false);

  workspace.isTrusted = true;
  const found = await target(catalog);
  const flags = (cfg) =>
    cfg
      .split("\n")
      .filter((line) =>
        /^(from_|chosen_mode=|script_saw_feature$|feature=)/.test(line),
      )
      .sort();
  const catalogFlags = [
    'chosen_mode="editor"',
    'feature="audit"',
    "from_catalog",
    "script_saw_feature",
  ];
  const otherFlags = ['chosen_mode="editor"', "from_other"];
  assert.deepEqual(flags(found.cfg), catalogFlags, logs.join("\n"));
  assert.deepEqual(flags((await target(other)).cfg), otherFlags);
  const byRoot = new Map(
    found.roots.map(({ root, cfg }) => [root, flags(cfg)]),
  );
  assert.deepEqual(
    byRoot,
    new Map([
      [catalog.toString(), catalogFlags],
      [uri(file("catalog/src/main.rs")).toString(), catalogFlags],
      [other.toString(), otherFlags],
    ]),
  );
  assert.equal(await readFile(path.join(temp, "catalog"), "utf8"), "editor");
  assert.equal(await readFile(path.join(temp, "other"), "utf8"), "editor");
  assert.equal(existsSync(file("Cargo.lock")), false);
  assert.doesNotMatch(logs.join("\n"), /could not compile/i);

  host.forgetCfg();
  settings["cargo.buildScripts.enable"] = false;
  env.KAALANG_TEST_MODE = "disabled";
  const disabled = await target(catalog);
  assert.ok(disabled.roots.length > 0);
  assert.deepEqual(
    flags(disabled.cfg).filter((line) => !line.startsWith("feature=")),
    [],
  );
  assert.equal(await readFile(path.join(temp, "catalog"), "utf8"), "editor");
  assert.equal(await readFile(path.join(temp, "other"), "utf8"), "editor");
  assert.equal(existsSync(file("Cargo.lock")), false);

  // A Cargo override can resolve the updated package while the existing lock stays as authored.
  const oldLock =
    'version = 4\n\n[[package]]\nname = "catalog"\nversion = "0.0.1"\n';
  await writeFile(file("Cargo.lock"), oldLock);
  host.forgetCfg();
  settings["cargo.buildScripts.enable"] = true;
  settings["cargo.buildScripts.overrideCommand"] = [
    "cargo",
    "check",
    "--package",
    "catalog",
    "--features",
    "audit",
    "--message-format=json",
  ];
  env.KAALANG_TEST_MODE = "override";
  const overridden = await target(catalog);
  assert.ok(
    flags(overridden.cfg).includes('chosen_mode="override"'),
    logs.join("\n"),
  );
  assert.equal(await readFile(file("Cargo.lock"), "utf8"), oldLock);
});
