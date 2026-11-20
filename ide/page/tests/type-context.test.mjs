import assert from "node:assert/strict";
import test from "node:test";

import { TypeContext } from "../type-context.js";

const place = {
  package: "app",
  group: 0,
  crate: "app",
  root: "/src/lib.rs",
  module: "",
  order: [],
};

test("the type asks once, and only the latest answer counts", () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  context.ask(host, "a.rs", 7);
  context.ask(host, "a.rs", 7);
  assert.deepEqual(
    requests.map(({ type, uri, offset }) => [type, uri, offset]),
    [["referrers", "a.rs", 7]],
  );
  context.refresh(host);
  const [old, latest] = requests;
  const files = [{ uri: "b.rs", text: "b", cfg: "", offsets: [1], place }];
  assert.equal(context.receive({ ...old, files }), false);
  assert.ok(context.receive({ ...latest, files }));
  assert.equal(context.version, 1);
});

test("the focus's own references count only in the text they were found in", () => {
  const context = new TypeContext("Focus");
  const requests = [];
  context.ask({ send: (message) => requests.push(message) }, "a.rs", 7);
  const other = { uri: "b.rs", text: "b", cfg: "x", offsets: [1], place };
  context.receive({
    ...requests[0],
    focus: { place },
    files: [{ uri: "a.rs", text: "first", cfg: "y", offsets: [2] }, other],
  });
  const current = context.sources("a.rs", "first");
  assert.deepEqual(current.uris, ["a.rs", "b.rs"]);
  assert.deepEqual(current.texts, ["first", "b"]);
  assert.deepEqual(current.context, {
    offsets: [2],
    place,
    files: [{ text: "b", cfg: "x", offsets: [1], place }],
  });
  assert.deepEqual(context.sources("a.rs", "edited").context.offsets, []);
});

test("a refresh keeps the finished diagram until its answer arrives", () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  context.ask(host, "a.rs", 7);
  assert.ok(context.canPublish());
  context.refresh(host);
  assert.equal(context.canPublish(), false);
  context.receive({ ...requests.at(-1), files: [] });
  assert.ok(context.canPublish());
});

test("an edit asks again and draws the other files' cards meanwhile", () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  context.ask(host, "a.rs", 7);
  context.refresh(host);
  context.edited();
  assert.ok(context.canPublish());
  context.ask(host, "a.rs", 9);
  context.ask(host, "a.rs", 9);
  assert.deepEqual(
    requests.map(({ offset }) => offset),
    [7, 7, 9],
  );
  assert.ok(context.canPublish());
});
