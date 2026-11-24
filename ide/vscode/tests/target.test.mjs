import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { compileFunction } from "node:vm";
import { statusCrates } from "../out/analyzer.js";

const uri = (fsPath) => ({
  scheme: "file",
  fsPath,
  toString: () => pathToFileURL(fsPath).href,
});

/** rust-analyzer's status for a file, for each crate its name, package, and set. */
const status = (...crates) =>
  "Analysis:\nCrates for file 7:\n" +
  (crates.length
    ? crates
        .map(
          ({ name, package: pkg, cfg }, id) =>
            `Crate: ${name}(Crate(Id(${id})))\n` +
            `    Enabled cfgs: CfgOptions(${JSON.stringify(cfg)})\n` +
            `    Env: Env { entries: {"CARGO_PKG_NAME": "${pkg}"} }\n`,
        )
        .join("")
    : "Does not belong to any crate\n");

/** A module tree in which each crate root is read only as its own crate's root. */
const roots = {
  fileReadings: async (uri, targets) =>
    targets
      .filter(({ root }) => root === uri)
      .map(({ cfg, package: pkg, name, root }) => ({
        cfg,
        place: {
          package: pkg,
          group: 0,
          crate: name,
          root,
          module: "",
          order: [],
        },
      })),
};

/** The built `target` module with an isolated editor and rust-analyzer, which answers `answer(file)`. */
async function targetHost(workspace, answer, cardPlaces) {
  const built = new URL("../out/target.js", import.meta.url);
  const require = createRequire(built);
  const asked = [];
  const client = {
    sendRequest: async (method, { textDocument }) => {
      assert.equal(method, "rust-analyzer/analyzerStatus");
      const file = fileURLToPath(textDocument.uri);
      asked.push(file);
      return answer(file);
    },
  };
  const modules = {
    vscode: {
      workspace,
      Uri: { file: uri },
      RelativePattern: class {
        constructor(base, pattern) {
          Object.assign(this, { base, pattern });
        }
      },
    },
    "./analyzer": { analyzerClient: () => client, statusCrates },
    ...(cardPlaces && { "./card-places": cardPlaces }),
  };
  const host = {};
  compileFunction(await readFile(built, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(built),
  })(host, (name) => modules[name] ?? require(name));
  return { ...host, asked };
}

test("each target takes the set rust-analyzer reads its root with, gated ones included, and Cargo writes no lockfile", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-target-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const file = (...parts) => path.join(root, ...parts);
  await writeFile(
    file("Cargo.toml"),
    '[workspace]\nmembers = ["catalog", "other"]\nresolver = "3"\n',
  );
  for (const name of ["catalog", "other"]) {
    await mkdir(file(name, "src"), { recursive: true });
    await writeFile(file(name, "src/lib.rs"), "");
  }
  await writeFile(
    file("catalog/Cargo.toml"),
    '[package]\nname = "catalog"\nversion = "0.1.0"\nedition = "2024"\n' +
      '[dependencies]\nother = { path = "../other" }\n[features]\naudit = []\n' +
      '[[example]]\nname = "gated"\nrequired-features = ["audit"]\n',
  );
  await mkdir(file("catalog/examples"));
  await writeFile(file("catalog/examples/gated.rs"), "fn main() {}\n");
  await writeFile(
    file("other/Cargo.toml"),
    '[package]\nname = "other"\nversion = "0.1.0"\nedition = "2024"\n',
  );
  const folder = { uri: uri(root) };
  const workspace = {
    isTrusted: false,
    workspaceFolders: [folder],
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({ get: () => undefined }),
  };
  // rust-analyzer has loaded `catalog` but not yet `other`.
  let loaded = false;
  const host = await targetHost(
    workspace,
    (source) =>
      source.includes(`${path.sep}catalog${path.sep}`)
        ? status({
            name: "catalog",
            package: "catalog",
            cfg: ["feature=audit", "test"],
          })
        : loaded
          ? status({ name: "other", package: "other", cfg: ["test", "unix"] })
          : status(),
    roots,
  );
  const logs = [];
  const target = () =>
    host.located(uri(file("catalog/src/lib.rs")), (line) => logs.push(line));

  const restricted = await target();
  assert.deepEqual([restricted.cfg, restricted.roots], ["", []]);
  assert.deepEqual(host.asked, []);

  workspace.isTrusted = true;
  const first = await target();
  assert.equal(first.cfg, 'feature="audit"\ntest');
  const cfgs = (read) =>
    read.roots.map(({ name, cfg }) => [name, cfg.replaceAll("\n", " ")]);
  assert.deepEqual(cfgs(first), [
    ["catalog", 'feature="audit" test'],
    ["gated", 'feature="audit" test'],
    ["other", ""],
  ]);
  assert.match(logs.join("\n"), /no cfg set yet/);

  loaded = true;
  const second = await target();
  assert.deepEqual(cfgs(second).at(-1), ["other", "test unix"]);
  // Only the unanswered crate is asked again, and an unchanged list keeps its identity.
  assert.equal(host.asked.length, 4);
  assert.equal((await target()).roots, second.roots);
  assert.equal(existsSync(file("Cargo.lock")), false);
});

test("a source file created after the targets were listed becomes a target once they are forgotten", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-new-target-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, "src"));
  await writeFile(
    path.join(root, "Cargo.toml"),
    '[package]\nname="hub"\nversion="0.1.0"\nedition="2024"\n',
  );
  await writeFile(path.join(root, "src/lib.rs"), "");
  const folder = { uri: uri(root) };
  const workspace = {
    isTrusted: true,
    workspaceFolders: [folder],
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({ get: () => undefined }),
  };
  const host = await targetHost(
    workspace,
    () => status({ name: "hub", package: "hub", cfg: ["test"] }),
    roots,
  );
  const targets = async () =>
    (
      await host.located(uri(path.join(root, "src/lib.rs")), () => {})
    ).roots.map(({ name }) => name);
  assert.deepEqual(await targets(), ["hub"]);

  await mkdir(path.join(root, "tests"));
  await writeFile(path.join(root, "tests/extra.rs"), "");
  assert.deepEqual(await targets(), ["hub"]);
  host.forgetTargets();
  assert.deepEqual(await targets(), ["hub", "extra"]);
});

test("a manifest outside the workspace folders is watched once, until the extension stops", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-outside-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const file = (...parts) => path.join(root, ...parts);
  await writeFile(
    file("Cargo.toml"),
    '[workspace]\nmembers = ["app", "billing"]\nresolver = "3"\n',
  );
  for (const name of ["app", "billing"]) {
    await mkdir(file(name, "src"), { recursive: true });
    await writeFile(file(name, "src/lib.rs"), "");
    await writeFile(
      file(name, "Cargo.toml"),
      `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n`,
    );
  }
  // The folder is opened below its Cargo workspace's root.
  const folder = { uri: uri(file("app")) };
  const watched = new Map();
  const workspace = {
    isTrusted: true,
    workspaceFolders: [folder],
    getWorkspaceFolder: ({ fsPath }) =>
      fsPath.startsWith(file("app") + path.sep) ? folder : undefined,
    getConfiguration: () => ({ get: () => undefined }),
    createFileSystemWatcher({ base, pattern }) {
      assert.equal(pattern, "Cargo.toml");
      const watch = {
        onDidCreate() {},
        onDidChange: (handle) => (watch.change = handle),
        onDidDelete() {},
        dispose: () => watched.delete(base.fsPath),
      };
      watched.set(base.fsPath, watch);
      return watch;
    },
  };
  const host = await targetHost(
    workspace,
    () => status({ name: "app", package: "app", cfg: [] }),
    roots,
  );
  let changes = 0;
  const watches = host.watchOutsideManifests(() => ++changes);
  await host.located(uri(file("app/src/lib.rs")), () => {});
  assert.deepEqual([...watched.keys()].sort(), [root, file("billing")]);
  const first = watched.get(root);
  first.change();
  assert.equal(changes, 1);
  // Forgetting the targets leaves no time in which a change goes unseen.
  host.forgetCfg();
  await host.located(uri(file("app/src/lib.rs")), () => {});
  assert.equal(watched.get(root), first);
  assert.equal(watched.size, 2);
  watches.dispose();
  assert.equal(watched.size, 0);
});

test("the last set of rust-analyzer stands while it gives none", async () => {
  const workspace = { isTrusted: true };
  let cfg = ["test", "unix"];
  const host = await targetHost(workspace, () =>
    cfg ? status({ name: "hub", package: "hub", cfg }) : status(),
  );
  const file = path.resolve("hub", "src", "lib.rs");
  const logs = [];
  const read = () => {
    host.forgetAnalyzedCfg();
    return host.analyzedCfg(file, (line) => logs.push(line));
  };
  assert.equal(await read(), "test\nunix");
  // It restarts: the set stays, and it is asked again each time.
  cfg = undefined;
  assert.equal(await read(), "test\nunix");
  assert.equal(await read(), "test\nunix");
  assert.equal(host.asked.length, 3);
  assert.deepEqual(logs, []);
  cfg = ["test"];
  assert.equal(await read(), "test");
});

test("a file several targets include stands where rust-analyzer reads it", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-root-owner-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, "src"));
  await writeFile(
    path.join(root, "Cargo.toml"),
    '[package]\nname="hub"\nversion="0.1.0"\nedition="2024"\n[[bin]]\nname="view"\npath="src/main.rs"\n',
  );
  await writeFile(path.join(root, "src/lib.rs"), "pub struct Focus;");
  await writeFile(
    path.join(root, "src/main.rs"),
    '#[path="lib.rs"] mod mount; fn main() {}',
  );
  const folder = { uri: uri(root) };
  const workspace = {
    isTrusted: true,
    workspaceFolders: [folder],
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({ get: () => undefined }),
  };
  let shown = "view";
  const source = uri(path.join(root, "src/lib.rs"));
  const host = await targetHost(
    workspace,
    () => status({ name: shown, package: "hub", cfg: ["test"] }),
    {
      fileReadings: async (asked, targets, typeOffset) => {
        assert.equal(asked, source.toString());
        // In diagram order: the library first.
        return targets
          .map(({ cfg, name }) => ({
            cfg,
            place: {
              package: "hub",
              group: 0,
              crate: name,
              root: name === "view" ? "src/main.rs" : "src/lib.rs",
              module: name === "view" ? "mount" : "",
              order: [],
            },
          }))
          .filter(({ place }) => typeOffset !== 17 || place.module === "")
          .sort((a, b) => (a.place.crate === "hub" ? -1 : 1));
      },
    },
  );
  const read = await host.located(source, () => {});
  assert.deepEqual([read.place.crate, read.place.module], ["view", "mount"]);
  assert.equal(read.cfg, "test");
  // A place whose reading lacks the type falls back to the first that has it.
  const typed = await host.located(source, () => {}, 17);
  assert.deepEqual([typed.place.crate, typed.place.module], ["hub", ""]);
  host.forgetAnalyzedCfg();
  shown = "hub";
  const direct = await host.located(source, () => {});
  assert.deepEqual([direct.place.crate, direct.place.module], ["hub", ""]);
});
