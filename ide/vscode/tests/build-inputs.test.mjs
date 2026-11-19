import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { compileFunction } from "node:vm";

const uri = (fsPath) => ({ scheme: "file", fsPath });

async function editor(t) {
  const directory = await mkdtemp(
    path.join(os.tmpdir(), "kaalang-build-inputs-"),
  );
  t.after(() => rm(directory, { recursive: true, force: true }));
  const watchers = new Map();
  const saves = new EventEmitter();
  const vscode = {
    Uri: { file: uri },
    RelativePattern: class {
      constructor(baseUri, pattern) {
        this.baseUri = baseUri;
        this.pattern = pattern;
      }
    },
    Disposable: {
      from: (...items) => ({
        dispose: () => items.forEach((item) => item.dispose()),
      }),
    },
    workspace: {
      createFileSystemWatcher({ baseUri, pattern }) {
        const events = new EventEmitter();
        watchers.set(baseUri.fsPath, { pattern, events });
        return {
          onDidCreate: (fn) => events.on("create", fn),
          onDidChange: (fn) => events.on("change", fn),
          onDidDelete: (fn) => events.on("delete", fn),
          dispose: () => events.removeAllListeners(),
        };
      },
      onDidSaveTextDocument(fn) {
        saves.on("save", fn);
        return { dispose: () => saves.off("save", fn) };
      },
    },
  };
  const file = new URL("../out/build-inputs.js", import.meta.url);
  const require = createRequire(file);
  const host = {};
  compileFunction(await readFile(file, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(file),
  })(host, (name) => (name === "vscode" ? vscode : require(name)));
  return {
    directory,
    watchers,
    watch: host.watchBuildInputs,
    save: (file) => saves.emit("save", { uri: uri(file) }),
  };
}

test("declared files, including missing and external inputs, refresh on change or editor save", async (t) => {
  const { directory, watchers, watch, save } = await editor(t);
  const workspace = path.join(directory, "workspace");
  const external = path.join(directory, "external");
  await Promise.all([mkdir(workspace), mkdir(external)]);
  const script = path.join(workspace, "flags.rs");
  const missing = path.join(workspace, "later.txt");
  const settings = path.join(external, "flags.toml");
  await Promise.all([writeFile(script, ""), writeFile(settings, "")]);

  let changes = 0;
  const subscription = watch(
    { paths: [script, missing, settings], ignored: [] },
    () => ++changes,
  );
  const local = watchers.get(workspace);
  assert.equal(local.pattern, "*");
  local.events.emit("change", uri(path.join(workspace, "another.rs")));
  assert.equal(changes, 0);
  save(script);
  assert.equal(changes, 1);
  await writeFile(missing, "ready");
  local.events.emit("create", uri(missing));
  assert.equal(changes, 2);
  await rm(settings);
  const outside = watchers.get(external);
  outside.events.emit("delete", uri(settings));
  assert.equal(changes, 3);

  subscription.dispose();
  save(script);
  local.events.emit("change", uri(script));
  outside.events.emit("create", uri(settings));
  assert.equal(changes, 3);
});

test("a directory input includes nested files while ignoring Git and Cargo output", async (t) => {
  const { directory, watchers, watch, save } = await editor(t);
  const root = path.join(directory, "workspace");
  const output = path.join(root, "build");
  const nested = path.join(root, "schema", "v1");
  const similarlyNamed = path.join(root, "build-inputs");
  await Promise.all(
    [output, nested, similarlyNamed].map((dir) =>
      mkdir(dir, { recursive: true }),
    ),
  );
  const source = path.join(nested, "schema.json");
  await writeFile(source, "{}");
  let changes = 0;
  const subscription = watch(
    { paths: [root], ignored: [output] },
    () => ++changes,
  );
  const workspace = watchers.get(root);
  assert.equal(workspace.pattern, "**/*");
  workspace.events.emit("change", uri(source));
  assert.equal(changes, 1);
  workspace.events.emit("change", uri(path.join(root, ".git", "index")));
  save(path.join(nested, ".git", "HEAD"));
  workspace.events.emit("create", uri(path.join(output, "debug", "artifact")));
  save(path.join(output, "generated.rs"));
  assert.equal(changes, 1);
  workspace.events.emit(
    "create",
    uri(path.join(similarlyNamed, "options.json")),
  );
  assert.equal(changes, 2);
  subscription.dispose();
});
