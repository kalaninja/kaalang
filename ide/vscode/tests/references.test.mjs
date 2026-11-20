import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { compileFunction } from "node:vm";
import test from "node:test";

const root = "file:///project/lib.rs";
const alias = "file:///project/names.rs";
const review = "file:///project/integration.rs";
const other = "file:///project/other.rs";
const author =
  '#[kaa] struct Review { #[kaa::refer(Author, "author")] id: u8 }';
const focus = { uri: root, offset: 7 };

/** Where a file stands: in `module` of crate `app`. */
const place = (module) => ({
  package: "app",
  group: 0,
  crate: "app",
  root: "/project/lib.rs",
  module,
  order: [],
});

/**
 * The built host with isolated editor, Cargo and language server answers:
 * `texts` holds each file, `references` the files the editor finds the focus
 * in. The workspace's module tree holds every file that may refer to it, and
 * only `Author` in `review` names the focus.
 */
async function host(texts) {
  const state = { references: [root], queries: 0 };
  const uri = (key) => ({
    toString: () => key,
    scheme: "file",
    fsPath: new URL(key).pathname,
  });
  const modules = {
    vscode: {
      Uri: { parse: uri },
      workspace: { getWorkspaceFolder: () => ({}) },
      commands: {
        executeCommand: async () =>
          state.references.map((key) => ({
            uri: uri(key),
            range: {
              start: { line: 0, character: texts.get(key).indexOf("Focus") },
            },
          })),
      },
    },
    "./target": {
      located: async (file) => ({
        cfg: `cfg of ${file.toString()}`,
        place: place(new URL(file.toString()).pathname),
        roots: [],
      }),
    },
    "./documents": {
      openDocument: async (key) => ({
        uri: uri(key),
        version: 1,
        positionAt: (offset) => ({ line: 0, character: offset }),
      }),
      documentText: async (uri) => texts.get(uri.toString()),
    },
    "./card-places": {
      forgetModules: () => {},
      editorTree: async () => ({
        referring: [review, other]
          .filter((key) => texts.has(key))
          .map((key) => fileURLToPath(key)),
      }),
    },
    "./module-sources": {
      relationshipTargets: async (text) => [text.indexOf("Author")],
    },
    "./analyzer": {
      analyzerClient: () => ({}),
      locations: async (_client, uri) => {
        ++state.queries;
        return uri === review
          ? [{ uri: root, position: { line: 0, character: 7 } }]
          : [];
      },
    },
  };
  const file = new URL("../out/references.js", import.meta.url);
  const require = createRequire(file);
  const exports = {};
  compileFunction(await readFile(file, "utf8"), ["exports", "require"], {
    filename: fileURLToPath(file),
  })(exports, (name) => modules[name] ?? require(name));
  return { ...exports, state };
}

const referring = ({ referringFiles }) =>
  referringFiles(root, focus.offset, () => {});

test("a reference search without a renaming use asks no definition", async () => {
  const texts = new Map([
    [root, "struct Focus; fn wide(id: u8) -> u32 { id as u32 }"],
    [review, author],
  ]);
  const loaded = await host(texts);
  const answer = await referring(loaded);
  assert.deepEqual(answer.files, []);
  assert.deepEqual(answer.focus, { place: place("/project/lib.rs") });
  assert.equal(loaded.state.queries, 0);
});

test("a renaming use brings in each file whose target's definition is the focus, once, with its place", async () => {
  const texts = new Map([
    [root, "struct Focus;"],
    [alias, "pub use crate::Focus as Author;"],
    [review, author],
    [other, author.replace("Review", "Other")],
  ]);
  const loaded = await host(texts);
  loaded.state.references = [root, alias];
  const { files } = await referring(loaded);
  assert.deepEqual(files, [
    {
      uri: review,
      text: author,
      cfg: `cfg of ${review}`,
      offsets: [author.indexOf("Author")],
      place: place("/project/integration.rs"),
    },
  ]);
  // A second search in the same source revision asks again only what had no answer.
  const asked = loaded.state.queries;
  await referring(loaded);
  assert.equal(loaded.state.queries, asked + 1);
});

test("a source change asks the definitions again", async () => {
  const texts = new Map([
    [root, "struct Focus;"],
    [alias, "pub use crate::Focus as Author;"],
    [review, author],
  ]);
  const loaded = await host(texts);
  loaded.state.references = [root, alias];
  await referring(loaded);
  loaded.forgetReferences();
  await referring(loaded);
  assert.equal(loaded.state.queries, 2);
});
