import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { promisify } from "node:util";
import { compileFunction } from "node:vm";
import { readCargoProject } from "../out/cargo.js";
import { workspaceTargets } from "../out/metadata.js";

const command = promisify(execFile);

const uri = (fsPath) => ({
  scheme: "file",
  fsPath,
  toString: () => pathToFileURL(fsPath).href,
});

async function targetHost(workspace) {
  const built = new URL("../out/target.js", import.meta.url);
  const require = createRequire(built);
  const host = {};
  compileFunction(await readFile(built, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(built),
  })(host, (name) =>
    name === "vscode" ? { workspace, Uri: { file: uri } } : require(name),
  );
  return host;
}

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
  const host = await targetHost(workspace);

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
  const alternate = path.join(temp, "Cargo.lock");
  await writeFile(alternate, oldLock);
  host.forgetCfg();
  settings["cargo.buildScripts.enable"] = true;
  settings["cargo.buildScripts.overrideCommand"] = [
    "cargo",
    "--config",
    `resolver.lockfile-path=${JSON.stringify(alternate)}`,
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
  assert.equal(await readFile(alternate, "utf8"), oldLock);
});

test("a new project without build scripts resolves default features and gated targets privately", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-fresh-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, "src"));
  await mkdir(path.join(root, "examples"));
  await writeFile(
    path.join(root, "Cargo.toml"),
    '[package]\nname="fresh"\nversion="0.1.0"\nedition="2024"\n[features]\ndefault=["drawing"]\ndrawing=[]\n[[example]]\nname="scene"\nrequired-features=["drawing"]\n',
  );
  await writeFile(
    path.join(root, "src/lib.rs"),
    '#[cfg(feature="drawing")] pub struct View;',
  );
  await writeFile(path.join(root, "examples/scene.rs"), "fn main() {}");
  const options = {
    target: null,
    features: [],
    noDefaultFeatures: false,
    noDeps: false,
    metadataArgs: [],
    buildScripts: false,
    rebuildOnSave: true,
    extraArgs: [],
  };
  const { metadata } = await readCargoProject(
    root,
    [],
    [],
    options,
    { ...process.env, CARGO_NET_OFFLINE: "true" },
    () => {},
  );
  assert.deepEqual(metadata.resolve.nodes[0].features, ["default", "drawing"]);
  assert.deepEqual(
    workspaceTargets(metadata).map((root) => root.name),
    ["fresh", "scene"],
  );
  assert.equal(existsSync(path.join(root, "Cargo.lock")), false);
  const locked = await readCargoProject(
    root,
    [],
    [],
    { ...options, extraArgs: ["--locked"] },
    { ...process.env, CARGO_NET_OFFLINE: "true" },
    () => {},
  );
  assert.equal(locked.metadata.resolve, null);
  assert.equal(existsSync(path.join(root, "Cargo.lock")), false);
});

test("a private retry copies the effective Cargo lock and overrides configured paths only for that invocation", async (t) => {
  const directory = await mkdtemp(
    path.join(os.tmpdir(), "kaalang-configured-lock-"),
  );
  t.after(() => rm(directory, { recursive: true, force: true }));
  for (const name of ["app/src", "app/.cargo", "app/authored", "pins"])
    await mkdir(path.join(directory, name), { recursive: true });
  const root = path.join(directory, "app");
  async function publish(version) {
    const crate = path.join(
      directory,
      "vendor",
      "lock-probe-dependency-" + version,
    );
    await mkdir(path.join(crate, "src"), { recursive: true });
    await writeFile(
      path.join(crate, "Cargo.toml"),
      `[package]\nname="lock-probe-dependency"\nversion="${version}"\nedition="2024"\n`,
    );
    await writeFile(
      path.join(crate, "src/lib.rs"),
      'pub const VERSION: &str = env!("CARGO_PKG_VERSION");',
    );
    await writeFile(
      path.join(crate, ".cargo-checksum.json"),
      '{"files":{},"package":null}',
    );
  }
  await publish("1.0.0");
  const file = (...names) => path.join(root, ...names);
  const config = file(".cargo", "editor.toml");
  await writeFile(
    config,
    '[resolver]\nlockfile-path = "authored/Cargo.lock"\n',
  );
  await writeFile(
    file(".cargo", "config.toml"),
    '[source.crates-io]\nreplace-with = "local"\n[source.local]\ndirectory = "../vendor"\n',
  );
  await writeFile(
    file("build.rs"),
    `fn main() { println!("cargo::rustc-cfg=version_{}", lock_probe_dependency::VERSION.replace('.', "_")); }`,
  );
  await writeFile(file("src/lib.rs"), "");
  const manifest =
    '[package]\nname="probe"\nversion="0.1.0"\nedition="2024"\n[workspace]\n' +
    '[build-dependencies]\nlock-probe-dependency="1"\n';
  await writeFile(file("Cargo.toml"), manifest);
  const env = { ...process.env, CARGO_NET_OFFLINE: "true" };
  await command(
    "cargo",
    ["--config", config, "generate-lockfile", "--offline"],
    { cwd: root, env },
  );
  const lock = await readFile(file("authored", "Cargo.lock"), "utf8");
  await publish("1.1.0");
  await writeFile(
    file("Cargo.toml"),
    manifest.replace('version="0.1.0"', 'version="0.2.0"'),
  );
  const options = {
    target: null,
    features: [],
    noDefaultFeatures: false,
    noDeps: false,
    metadataArgs: [],
    buildScripts: false,
    rebuildOnSave: true,
    extraArgs: [],
    targetDir: null,
  };
  const logs = [];
  const result = await readCargoProject(
    root,
    [],
    ["--config", config],
    options,
    env,
    (line) => logs.push(line),
  );
  assert.ok(result.metadata.resolve, logs.join("\n"));
  assert.equal(
    result.metadata.packages.find((pkg) => pkg.name === "lock-probe-dependency")
      .version,
    "1.0.0",
    logs.join("\n"),
  );
  const external = path.join(directory, "pins", "Cargo.lock");
  const select = `resolver.lockfile-path=${JSON.stringify(external)}`;
  await writeFile(external, lock);
  const custom = [
    "cargo",
    "check",
    "--config",
    select,
    "--message-format=json",
  ];
  const build = (extra, cwd = root) =>
    readCargoProject(
      root,
      [],
      [],
      {
        ...options,
        buildScripts: true,
        rebuildOnSave: false,
        buildDirectory: cwd,
        ...extra,
      },
      env,
      (line) => logs.push(line),
    );
  for (const extra of [
    { extraArgs: ["--config", select] },
    { buildCommand: custom },
  ]) {
    const found = await build(extra);
    assert.deepEqual(
      [...found.buildCfgs.values()],
      [["version_1_0_0"]],
      logs.join("\n"),
    );
    assert.ok(found.buildInputs.paths.includes(external));
    assert.equal(await readFile(external, "utf8"), lock);
  }
  // A once-per-window override may run in the parent of the Cargo workspace.
  const fromParent = await build(
    {
      buildCommand: [
        "cargo",
        "check",
        "--manifest-path=app/Cargo.toml",
        "--config",
        file(".cargo", "config.toml"),
        "--config",
        'resolver.lockfile-path="pins/Cargo.lock"',
        "--message-format=json",
      ],
    },
    directory,
  );
  assert.deepEqual(
    [...fromParent.buildCfgs.values()],
    [["version_1_0_0"]],
    logs.join("\n"),
  );
  assert.ok(fromParent.buildInputs.paths.includes(external));
  for (const flag of ["--locked", "--frozen"]) {
    const strict = await build({ buildCommand: [...custom, flag] });
    assert.deepEqual([...strict.buildCfgs.values()], []);
    assert.match(logs.at(-1), new RegExp(flag));
  }

  // Both locks are current. A metadata-only read still reports the selected lock.
  await writeFile(file("Cargo.toml"), manifest);
  const found = await readCargoProject(
    root,
    [],
    ["--config", select],
    options,
    env,
    (line) => logs.push(line),
  );
  assert.ok(found.metadata.resolve);
  assert.ok(found.buildInputs.paths.includes(external));
  assert.equal(await readFile(file("authored", "Cargo.lock"), "utf8"), lock);
  assert.equal(existsSync(file("Cargo.lock")), false);
});

test(
  "an override checks its own Cargo toolchain before allowing a private lock",
  { skip: process.platform === "win32" },
  async (t) => {
    const root = await mkdtemp(
      path.join(os.tmpdir(), "kaalang-cargo-version-"),
    );
    t.after(() => rm(root, { recursive: true, force: true }));
    await mkdir(path.join(root, "src"));
    await writeFile(
      path.join(root, "Cargo.toml"),
      '[package]\nname="app"\nversion="0.1.0"\nedition="2021"\n[workspace]\n',
    );
    await writeFile(path.join(root, "src/lib.rs"), "");
    await writeFile(path.join(root, "build.rs"), "fn main() {}");
    const program = path.join(root, "cargo");
    await writeFile(
      program,
      `#!${process.execPath}
const fs = require("node:fs");
const args = process.argv.slice(2);
if (args.shift() !== "+old") process.exit(1);
if (args.includes("-V")) console.log("cargo 1.96.0");
else if (args.includes("locate-project")) console.log(require("node:path").resolve("Cargo.toml"));
else if (args.includes("check")) fs.writeFileSync("invocation.json", JSON.stringify(args));
else process.exit(1);
`,
      { mode: 0o755 },
    );
    const result = await readCargoProject(
      root,
      [],
      [],
      {
        target: null,
        features: [],
        noDeps: false,
        noDefaultFeatures: false,
        metadataArgs: [],
        extraArgs: [],
        buildScripts: true,
        rebuildOnSave: false,
        buildCommand: [program, "+old", "check"],
      },
      process.env,
      () => {},
    );
    const args = JSON.parse(
      await readFile(path.join(root, "invocation.json"), "utf8"),
    );
    assert.ok(args.includes("--locked"));
    assert.equal(
      args.some((arg) => arg.includes("resolver.lockfile-path")),
      false,
    );
    assert.ok(result.metadata.resolve);
    assert.equal(existsSync(path.join(root, "Cargo.lock")), false);
  },
);
