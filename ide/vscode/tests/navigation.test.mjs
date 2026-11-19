import assert from "node:assert/strict";
import { createRequire, registerHooks } from "node:module";
import test from "node:test";

test("jumps preserve the editor's history and require current source and destination snapshots", async (t) => {
  const uri = "file:///workspace/example.rs";
  const document = {
    uri,
    version: 1,
    getText: () => "struct Example;",
    positionAt: (offset) => offset,
    offsetAt: (position) => position,
  };
  const closed = { document: { ...document, isClosed: true }, viewColumn: 1 };
  const editor = { document, selection: { active: 0 } };
  let externalText = "struct Review;";
  const external = {
    uri: "file:///workspace/review.rs",
    getText: () => externalText,
  };
  const started = Promise.withResolvers();
  const found = Promise.withResolvers();
  const jumps = [];
  class Range {
    constructor(start, end) {
      this.start = start;
      this.end = end;
    }
  }
  const api = {
    Uri: { parse: (value) => value },
    Range,
    Selection: Range,
    Location: class {
      constructor(uri, range) {
        this.uri = uri;
        this.range = range;
      }
    },
    workspace: {
      openTextDocument: async () => document,
      textDocuments: [document, external],
    },
    window: { showTextDocument: async () => editor, activeTextEditor: editor },
    commands: {
      async executeCommand(command, ...args) {
        if (command === "vscode.executeDefinitionProvider") {
          started.resolve();
          return found.promise;
        }
        assert.equal(command, "editor.action.goToLocations");
        jumps.push(args);
      },
    },
  };
  globalThis.vscodeNavigationTest = api;
  const hook = registerHooks({
    resolve(name, context, next) {
      return name === "vscode"
        ? { url: "test:vscode", shortCircuit: true }
        : next(name, context);
    },
    load(url, context, next) {
      return url === "test:vscode"
        ? {
            format: "commonjs",
            source: "module.exports = globalThis.vscodeNavigationTest",
            shortCircuit: true,
          }
        : next(url, context);
    },
  });
  t.after(() => {
    hook.deregister();
    delete globalThis.vscodeNavigationTest;
  });
  const { navigate } = createRequire(import.meta.url)("../out/navigation.js");
  let current = true;
  const pending = navigate(
    closed,
    { type: "definition", uri, offset: 7 },
    () => current,
  );
  await started.promise;
  current = false;
  await navigate(
    closed,
    { type: "reveal", uri, start: 7, end: 14, text: "struct Renamed;" },
    () => true,
  );
  assert.equal(jumps.length, 0);
  await navigate(
    closed,
    { type: "reveal", uri, start: 7, end: 14 },
    () => true,
  );
  found.resolve([new api.Location(uri, new Range(0, 1))]);
  await pending;
  assert.equal(jumps.length, 1);
  assert.deepEqual(editor.selection, new Range(7, 14));
  const confirmed = {
    type: "reveal",
    uri,
    start: 7,
    end: 14,
    text: document.getText(),
    origin: { uri: external.uri, text: externalText },
  };
  editor.selection = { active: 2 };
  await navigate(closed, confirmed, () => true);
  assert.equal(jumps.length, 2);
  assert.deepEqual(jumps[1].slice(0, 3), [
    uri,
    2,
    [new api.Location(uri, new Range(7, 14))],
  ]);
  externalText = "struct Renamed;";
  await navigate(closed, confirmed, () => true);
  assert.equal(jumps.length, 2);
  externalText = confirmed.origin.text;
  api.window.showTextDocument = async () => {
    externalText = "struct ChangedDuringTheJump;";
    return editor;
  };
  await navigate(closed, confirmed, () => true);
  assert.equal(jumps.length, 2);
});
