import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { compileFunction } from "node:vm";
import test from "node:test";

test("a renamed reference in a closed workspace target resolves once per source revision", async () => {
  const root = "file:///project/lib.rs";
  const alias = "file:///project/names.rs";
  const review = "file:///project/integration.rs";
  const uri = (key) => ({
    toString: () => key,
    scheme: "file",
    fsPath: new URL(key).pathname,
  });
  const texts = new Map([
    [root, "struct Focus;"],
    [alias, "pub use crate::Focus as Author;"],
    [review, '#[kaa] struct Review { #[kaa::refer(Author, "author")] id: u8 }'],
  ]);
  const docs = [];
  const openDocument = async (key) => {
    const doc = {
      uri: uri(key),
      version: 1,
      positionAt: (offset) => ({ line: 0, character: offset }),
    };
    docs.push(doc);
    return doc;
  };
  const roots = [{ root, cfg: "" }];
  let scans = 0,
    queries = 0;
  let references = [root];
  const modules = {
    vscode: {
      Uri: { parse: uri },
      workspace: {
        textDocuments: docs,
        workspaceFolders: [{}],
        getWorkspaceFolder: () => ({}),
      },
      commands: {
        executeCommand: async () =>
          references.map((key) => ({
            uri: uri(key),
            range: {
              start: { line: 0, character: texts.get(key).indexOf("Focus") },
            },
          })),
      },
    },
    "./target": { target: async () => ({ roots, cfgForRoot: () => "" }) },
    "./documents": {
      openDocument,
      documentText: async (uri) => texts.get(uri.toString()),
    },
    "./card-places": {
      cardPlacements: async (places) =>
        places.map(() => [{ root: uri(root).fsPath }]),
      moduleFiles: async () => {
        ++scans;
        return { uris: [...texts.keys()], complete: true };
      },
      forgetModules: () => {},
    },
    "./analyzer": {
      analyzerClient: () => ({}),
      locations: async (_, method, key, position) => {
        ++queries;
        assert.equal(method, "textDocument/definition");
        assert.equal(key, review);
        assert.equal(position.character, texts.get(review).indexOf("Author"));
        if (!docs.some((doc) => doc.uri.toString() === key))
          throw Error("file not found");
        return [{ uri: root, position: { line: 0, character: 7 } }];
      },
    },
  };
  // Load the built CommonJS host with isolated editor, Cargo and LSP responses.
  const file = new URL("../out/references.js", import.meta.url);
  const require = createRequire(file);
  const host = {};
  compileFunction(await readFile(file, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(file),
  })(host, (name) => modules[name] ?? require(name));
  const { referringFiles, definitionMatches, forgetReferences } = host;
  const focus = { uri: root, offset: 7 },
    place = { uri: review, offset: texts.get(review).indexOf("Author") };
  assert.deepEqual(await referringFiles(root, focus.offset, () => {}), []);
  assert.equal(scans, 0);
  references = [root, alias];
  const files = await referringFiles(root, focus.offset, () => {});
  assert.deepEqual(
    files.map(({ uri, offsets }) => [uri, offsets]),
    [[review, []]],
  );
  assert.deepEqual(await definitionMatches(focus, [place]), [true]);
  assert.equal(queries, 2, "opens a missing file's text model and retries");
  await referringFiles(root, focus.offset, () => {});
  assert.deepEqual(await definitionMatches(focus, [place]), [true]);
  assert.equal(scans, 1);
  assert.equal(queries, 2);
  forgetReferences();
  assert.equal(await definitionMatches(focus, [place]), undefined);
});
