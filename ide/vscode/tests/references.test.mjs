import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { compileFunction } from "node:vm";
import test from "node:test";

const root = "file:///project/lib.rs";
const alias = "file:///project/names.rs";
const review = "file:///project/integration.rs";
const author =
  '#[kaa] struct Review { #[kaa::refer(Author, "author")] id: u8 }';
const focus = { uri: root, offset: 7 };
const place = { uri: review, offset: author.indexOf("Author") };

/**
 * The built host with isolated editor, Cargo and language server answers:
 * `texts` holds each file, `references` the files the editor finds the focus
 * in, `resolved` what the workspace resolver matched in a scanned file.
 */
async function host(texts) {
  const state = { references: [root], resolved: undefined, queries: 0 };
  const uri = (key) => ({
    toString: () => key,
    scheme: "file",
    fsPath: new URL(key).pathname,
  });
  state.opened = [];
  const modules = {
    vscode: {
      Uri: { parse: uri },
      workspace: {
        textDocuments: [],
        workspaceFolders: [{}],
        getWorkspaceFolder: () => ({}),
      },
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
      target: async () => ({
        roots: [{ root, cfg: "" }],
        cfgForRoot: () => "",
        cfg: "selected",
        displayOwner: { owner: 2, module: "nested" },
      }),
    },
    "./documents": {
      openDocument: async (key) => {
        state.opened.push(key);
        return {
          uri: uri(key),
          version: 1,
          positionAt: (offset) => ({ line: 0, character: offset }),
        };
      },
      documentText: async (uri) => texts.get(uri.toString()),
    },
    "./card-places": {
      MAY_REFER: /\brefer\b/,
      fileOwners: async (places) =>
        places.map((key) =>
          key === review ? [{ cfg: "left" }, { cfg: "right" }] : [{ cfg: "" }],
        ),
      forgetModules: () => {},
    },
    "./referring-sources": {
      // A workspace scan finds the file that names the focus through `Author`.
      referringSources: async (_, all) =>
        all
          ? new Map([
              [
                review,
                {
                  text: texts.get(review),
                  cfgs: ["left", "right"],
                  matches: state.resolved,
                },
              ],
            ])
          : new Map(),
    },
    "./analyzer": {
      analyzerClient: () => ({}),
      locations: async () => {
        ++state.queries;
        return [{ uri: root, position: { line: 0, character: 7 } }];
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

const files = async ({ referringFiles }) =>
  (await referringFiles(root, focus.offset, () => {})).files.map(
    ({ uri, cfg }) => [uri, cfg],
  );

test("a reference search without a renaming use reads no other file", async () => {
  const texts = new Map([
    [root, "struct Focus; fn wide(id: u8) -> u32 { id as u32 }"],
    [review, author],
  ]);
  assert.deepEqual(await files(await host(texts)), []);
});

test("a renaming use brings in the files that name the type by its new name", async () => {
  const texts = new Map([
    [root, "struct Focus;"],
    [alias, "pub use crate::Focus as Author;"],
    [review, author],
  ]);
  const loaded = await host(texts);
  loaded.state.references = [root, alias];
  assert.deepEqual(await files(loaded), [
    [review, "left"],
    [review, "right"],
  ]);
});

test("the workspace resolver's matches stand for a scanned file's references", async () => {
  const texts = new Map([
    [root, "struct Focus;"],
    [alias, "pub use crate::Focus as Author;"],
    [review, author],
  ]);
  const loaded = await host(texts);
  loaded.state.references = [root, alias];
  loaded.state.resolved = [
    { uri: review, cfg: "right", owner: 1, module: "two", offsets: [7] },
  ];
  const { files } = await loaded.referringFiles(root, focus.offset, () => {});
  assert.deepEqual(
    files.map(({ text, ...file }) => file),
    [{ ...loaded.state.resolved[0], resolved: true }],
  );
});

test("a definition reaching the focus matches once per source revision without opening its file", async () => {
  const texts = new Map([
    [root, "struct Focus;"],
    [alias, "pub use crate::Focus as Author;"],
    [review, author],
  ]);
  const loaded = await host(texts);
  loaded.state.references = [root, alias];
  await loaded.referringFiles(root, focus.offset, () => {});
  assert.deepEqual(await loaded.definitionMatches(focus, [place]), [true]);
  assert.deepEqual(await loaded.definitionMatches(focus, [place]), [true]);
  assert.equal(loaded.state.queries, 1);
  assert.ok(!loaded.state.opened.includes(review));
});

test("a source change drops the definitions asked before it", async () => {
  const texts = new Map([[root, "struct Focus;"]]);
  const loaded = await host(texts);
  await loaded.referringFiles(root, focus.offset, () => {});
  loaded.forgetReferences();
  assert.equal(await loaded.definitionMatches(focus, [place]), undefined);
});
