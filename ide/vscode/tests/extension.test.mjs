import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { compileFunction } from "node:vm";

const ignore = () => {};
/** An API whose events never fire, unless `api` takes their handlers. */
const events = (api = {}) =>
  new Proxy(api, {
    get: (target, name) =>
      target[name] ??
      (typeof name === "string" && name.startsWith("onDid")
        ? ignore
        : undefined),
  });

const uri = (path) => ({
  scheme: "file",
  path,
  toString: () => `file://${path}`,
});
const editor = (path) => ({
  document: {
    uri: uri(path),
    languageId: "rust",
    getText: () => path,
    offsetAt: () => 0,
  },
  selection: { active: 0 },
});
const named = (editors) => editors.map((e) => e.document.uri.toString());
const located = { place: {}, cfg: "", roots: [] };

/**
 * The extension showing a diagram of `active` with the folder `/workspace`
 * open, where locating an editor's source waits for `pending(editor)`.
 */
async function following(active) {
  const commands = new Map(),
    ownership = new Map(),
    shown = [],
    targets = [];
  const handlers = {};
  const webview = events({
    asWebviewUri: (uri) => uri,
    postMessage(message) {
      if (message.type === "document") {
        assert.equal(message.waitForTarget, true);
        shown.push(message.uri);
      }
      if (message.type === "target") targets.push(message);
    },
  });
  const vscode = {
    Uri: { joinPath: (base, name) => uri(`${base.path}/${name}`) },
    ViewColumn: { Active: 1 },
    commands: { registerCommand: (name, handle) => commands.set(name, handle) },
    window: events({
      activeTextEditor: active,
      createOutputChannel: () => ({ appendLine: ignore }),
      onDidChangeActiveTextEditor: (handle) => (handlers.changeEditor = handle),
      onDidChangeTextEditorSelection: (handle) =>
        (handlers.changeSelection = handle),
      createWebviewPanel: () => events({ webview }),
    }),
    workspace: events({
      isTrusted: true,
      workspaceFolders: [uri("/workspace")],
      getWorkspaceFolder: (uri) =>
        uri.path.startsWith("/workspace/") ? uri : undefined,
      createFileSystemWatcher: () => events(),
    }),
    languages: events(),
  };
  const modules = {
    vscode,
    "node:path": path,
    "./card-places": { watchModuleSources: ignore },
    "./target": {
      forgetAnalyzedCfg: ignore,
      watchOutsideManifests: ignore,
      analyzedCfg: async () => "",
      located: (uri) => {
        if (!ownership.has(uri)) ownership.set(uri, Promise.withResolvers());
        return ownership.get(uri).promise;
      },
    },
  };
  const host = {};
  const source = await readFile(
    new URL("../out/extension.js", import.meta.url),
    "utf8",
  );
  compileFunction(source, ["exports", "require"])(
    host,
    (name) => modules[name] ?? {},
  );
  host.activate({ extensionUri: uri("/extension"), subscriptions: [] });
  commands.get("kaalang.showDiagram")();
  return {
    ...handlers,
    pending: (editor) => ownership.get(editor.document.uri),
    shown,
    targets,
  };
}

test("the diagram keeps a pending external Rust editor when its webview takes focus", async () => {
  const external = editor("/outside.rs"),
    pending = editor("/other.rs"),
    local = editor("/workspace/lib.rs");
  const host = await following(external);
  const first = host.pending(external);
  host.changeEditor(undefined);
  first.resolve(located);
  await first.promise;
  assert.deepEqual(host.shown, named([external]));

  host.changeEditor(pending);
  const second = host.pending(pending);
  host.changeEditor(local);
  second.resolve(located);
  await second.promise;
  assert.deepEqual(host.shown, named([external, local]));
  await new Promise(setImmediate);
  assert.deepEqual(host.targets.at(-1), {
    type: "target",
    uri: local.document.uri.toString(),
    cfg: "",
  });
});

test("an edit in the source leaves a pending external Rust editor to follow", async () => {
  const local = editor("/workspace/lib.rs"),
    external = editor("/outside.rs");
  const host = await following(local);
  host.changeEditor(external);
  const pending = host.pending(external);
  host.changeSelection({ textEditor: local });
  pending.resolve(located);
  await pending.promise;
  assert.deepEqual(host.shown.at(-1), external.document.uri.toString());
});

/**
 * The extension activated with the `target` module's `forget…` functions,
 * where `out` is a build directory, and a watcher's handlers by a part of its
 * pattern.
 */
async function watching(target) {
  const watchers = new Map();
  const modules = {
    vscode: {
      Uri: { joinPath: ignore },
      commands: { registerCommand: ignore },
      window: events({ createOutputChannel: ignore }),
      workspace: events({
        asRelativePath: ({ fsPath }) =>
          fsPath.replace("/target/workspace/", ""),
        createFileSystemWatcher: (pattern) => {
          const handlers = {};
          watchers.set(pattern, handlers);
          return events({
            onDidCreate: (handle) => (handlers.created = handle),
            onDidDelete: (handle) => (handlers.deleted = handle),
          });
        },
      }),
      languages: events(),
    },
    "./card-places": { watchModuleSources: ignore },
    "./references": { forgetReferences: ignore },
    "./target": {
      forgetCfg: ignore,
      forgetAnalyzedCfg: ignore,
      forgetTargets: ignore,
      inBuildDirectory: (file) => file.startsWith("/target/workspace/out/"),
      watchOutsideManifests: ignore,
      ...target,
    },
  };
  const host = {};
  const source = await readFile(
    new URL("../out/extension.js", import.meta.url),
    "utf8",
  );
  compileFunction(source, ["exports", "require"])(
    host,
    (name) => modules[name] ?? {},
  );
  host.activate({ subscriptions: [] });
  return (part) => [...watchers].find(([pattern]) => pattern.includes(part))[1];
}

test("a removed target folder reads the targets again, a source or built file does not", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  let forgotten = 0;
  const { deleted: removed } = (
    await watching({ forgetCfg: () => ++forgotten })
  )("tests");

  removed({ fsPath: "/target/workspace/tests/api.rs" });
  removed({ fsPath: "/target/workspace/target/debug/examples/api-1f" });
  removed({ fsPath: "/target/workspace/out/debug/examples/api-1f" });
  t.mock.timers.tick(500);
  assert.equal(forgotten, 0);
  removed({ fsPath: "/target/workspace/tests" });
  t.mock.timers.tick(500);
  assert.equal(forgotten, 1);
});

test("a source file a build writes leaves the targets, a workspace one forgets them", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  let forgotten = 0;
  const { created, deleted } = (
    await watching({ forgetTargets: () => ++forgotten })
  )("*.rs");

  created({ fsPath: "/target/workspace/target/debug/build/api-1f/out/gen.rs" });
  deleted({ fsPath: "/target/workspace/out/package/api-0.1.0/src/lib.rs" });
  assert.equal(forgotten, 0);
  created({ fsPath: "/target/workspace/tests/api.rs" });
  assert.equal(forgotten, 1);
});
