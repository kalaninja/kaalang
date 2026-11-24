import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { compileFunction } from "node:vm";

test("the diagram keeps a pending external Rust editor when its webview takes focus", async () => {
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
  const external = editor("/outside.rs"),
    pending = editor("/other.rs"),
    local = editor("/workspace/lib.rs");
  const commands = new Map(),
    ownership = new Map(),
    shown = [],
    targets = [];
  const ignore = () => {};
  // Only active-editor changes fire in this scenario.
  const events = (api = {}) =>
    new Proxy(api, {
      get: (target, name) =>
        target[name] ??
        (typeof name === "string" && name.startsWith("onDid")
          ? ignore
          : undefined),
    });
  let changeEditor;
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
      activeTextEditor: external,
      createOutputChannel: () => ({ appendLine: ignore }),
      onDidChangeActiveTextEditor: (handle) => {
        changeEditor = handle;
      },
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
  const first = ownership.get(external.document.uri);
  changeEditor(undefined);
  first.resolve({ place: {}, cfg: "", roots: [] });
  await first.promise;
  assert.deepEqual(shown, [external.document.uri.toString()]);

  changeEditor(pending);
  const second = ownership.get(pending.document.uri);
  changeEditor(local);
  second.resolve({ place: {}, cfg: "", roots: [] });
  await second.promise;
  assert.deepEqual(
    shown,
    [external, local].map((e) => e.document.uri.toString()),
  );
  await new Promise(setImmediate);
  assert.deepEqual(targets.at(-1), {
    type: "target",
    uri: local.document.uri.toString(),
    cfg: "",
  });
});
