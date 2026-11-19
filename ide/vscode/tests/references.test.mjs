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
  let resolved;
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
    "./target": {
      target: async (_uri, _log, _hover, offset) => {
        assert.equal(offset, 7);
        return {
          roots,
          cfgForRoot: () => "",
          cfg: "selected",
          displayOwner: { owner: 2, module: "nested" },
        };
      },
    },
    "./documents": {
      openDocument,
      documentText: async (uri) => texts.get(uri.toString()),
    },
    "./card-places": {
      MAY_REFER: /\brefer\b/,
      fileOwners: async (places) =>
        places.map((key) =>
          key === review
            ? [{ cfg: "left" }, { cfg: "right" }, { cfg: "left" }]
            : [{ cfg: "" }],
        ),
      forgetModules: () => {},
    },
    "./referring-sources": {
      referringSources: async (_, all) => {
        if (!all) return new Map();
        ++scans;
        return new Map([
          [
            review,
            {
              text: texts.get(review),
              cfgs: ["left", "right"],
              matches: resolved,
            },
          ],
        ]);
      },
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
  assert.deepEqual(await referringFiles(root, focus.offset, () => {}), {
    files: [],
    focus: { owner: 2, module: "nested", cfg: "selected" },
  });
  assert.equal(scans, 0);
  references = [root, alias];
  const { files } = await referringFiles(root, focus.offset, () => {});
  assert.deepEqual(
    files.map(({ uri, cfg, offsets }) => [uri, cfg, offsets]),
    [
      [review, "left", []],
      [review, "right", []],
    ],
  );
  assert.deepEqual(await definitionMatches(focus, [place]), [true]);
  assert.equal(queries, 2, "opens a missing file's text model and retries");
  await referringFiles(root, focus.offset, () => {});
  assert.deepEqual(await definitionMatches(focus, [place]), [true]);
  assert.equal(scans, 2);
  assert.equal(queries, 2);
  resolved = [
    { uri: review, cfg: "left", owner: 0, module: "one", offsets: [] },
    {
      uri: review,
      cfg: "right",
      owner: 1,
      module: "two",
      offsets: [place.offset],
      unknown: [4],
    },
  ];
  const { files: checked } = await referringFiles(root, focus.offset, () => {});
  assert.deepEqual(
    checked.map(({ text, ...file }) => file),
    resolved.map((match) => ({ ...match, resolved: true })),
  );
  forgetReferences();
  assert.equal(await definitionMatches(focus, [place]), undefined);
});
