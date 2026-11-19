import assert from "node:assert/strict";
import test from "node:test";

import { TypeContext } from "../type-context.js";

test("the latest answers describe one type in one source snapshot", () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  const cards = [{ source: 0, path: "Focus", start: 7 }];
  context.ask(host, cards, ["a.rs"]);
  context.ask(host, cards, ["a.rs"]);
  context.ask(host, cards, ["a.rs"], true);
  assert.equal(requests.length, 3);
  const [oldRefs, oldPaths, refs] = requests;

  const files = [{ uri: "a.rs", text: "first", offsets: [2], cfg: "" }];
  assert.ok(context.receive({ ...refs, files }));
  context.ask(host, cards, ["a.rs"]);
  const paths = requests.at(-1);
  const placement = {
    package: "app",
    group: 1,
    crate: "checks",
    root: "/test.rs",
    module: "new",
    order: [17, 7],
  };
  assert.ok(
    context.receive({
      ...paths,
      modules: ["crate::new"],
      placements: [[placement]],
    }),
  );
  assert.equal(context.receive({ ...oldRefs, files: [] }), false);
  assert.equal(
    context.receive({ ...oldPaths, modules: ["crate::old"] }),
    false,
  );

  const current = context.sources("a.rs", "first").context;
  assert.deepEqual(current.offsets, [2]);
  assert.equal(current.modules.Focus, "crate::new");
  assert.deepEqual(current.places.Focus, [placement]);
  assert.deepEqual(context.sources("a.rs", "edited").context.offsets, []);
  assert.equal(new TypeContext("Other").receive({ ...refs, files }), false);
});

test("a card outside the focus workspace stays excluded until references are refreshed", () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  const cards = [
    { source: 0, path: "Focus", start: 7 },
    { source: 1, path: "Outside", start: 11 },
  ];
  const files = [{ uri: "b.rs", text: "other", cfg: "", offsets: [2] }];
  const uris = ["a.rs", "b.rs"];
  context.ask(host, cards.slice(0, 1), uris);
  context.receive({ ...requests[0], files });
  context.ask(host, cards, uris);
  context.receive({
    ...requests.at(-1),
    modules: [null, null],
    placements: [null, []],
  });
  const sent = requests.length;
  context.ask(host, cards.slice(0, 1), uris);
  assert.equal(requests.length, sent);
  assert.deepEqual(
    context.sources("a.rs", "text").context.files[0].places.Outside,
    [],
  );
  context.ask(host, cards.slice(0, 1), uris, true);
  context.receive({ ...requests.at(-1), files });
  assert.equal(
    context.sources("a.rs", "text").context.files[0].places,
    undefined,
  );
});

test("only unconfirmed relationship targets are looked up in their source snapshot", async () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  const cards = [{ source: 0, path: "Focus", start: 7 }];
  context.ask(host, cards, ["a.rs"]);
  const files = [
    { uri: "a.rs", text: "focus", cfg: "host", offsets: [2] },
    { uri: "bad.rs", text: "invalid", cfg: "", offsets: [] },
    { uri: "b.rs", text: "incoming", cfg: "test", offsets: [] },
    { uri: "c.rs", text: "unrelated", cfg: "", offsets: [] },
  ];
  context.receive({ ...requests[0], files });
  assert.deepEqual(context.sources("a.rs", "focus").uris, ["a.rs"]);
  const reads = [];
  const engine = {
    async relationshipTargets(text, cfg) {
      reads.push([text, cfg]);
      if (text === "invalid") throw new Error("expected an item");
      return text === "focus" ? [2, 5] : [3];
    },
  };
  await context.checkDefinitions(engine, host, "focus", "drawn", () => true);
  assert.deepEqual(reads, [
    ["focus", "host"],
    ["focus", "drawn"],
    ["invalid", ""],
    ["incoming", "test"],
    ["unrelated", ""],
  ]);
  const request = requests.at(-1);
  assert.deepEqual(request, {
    type: "definitions",
    id: request.id,
    uri: "a.rs",
    offset: 7,
    places: [
      { uri: "a.rs", offset: 5 },
      { uri: "b.rs", offset: 3 },
      { uri: "c.rs", offset: 3 },
    ],
  });
  const answer = {
    type: "definitions",
    id: request.id,
    matches: [true, true, false],
  };
  assert.ok(context.receive(answer));
  assert.equal(context.receive(answer), false);
  const drawn = context.sources("a.rs", "focus");
  assert.deepEqual(drawn.uris, ["a.rs", "b.rs"]);
  assert.deepEqual(drawn.texts, ["focus", "incoming"]);
  assert.deepEqual(drawn.context.offsets, [2, 5]);
  assert.deepEqual(drawn.context.files[0].offsets, [3]);
  assert.deepEqual(files[2].offsets, []);
});

test("a second reference search overtakes target extraction and confirmation", async () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  const cards = [{ source: 0, path: "Focus", start: 7 }];
  const files = [
    { uri: "a.rs", text: "obsolete focus", cfg: "", offsets: [] },
    { uri: "b.rs", text: "incoming", cfg: "", offsets: [] },
  ];
  const refresh = () => {
    context.ask(host, cards, ["a.rs"], true);
    context.receive({ ...requests.at(-1), files });
  };
  refresh();
  let resume;
  const engine = {
    relationshipTargets: (text) => {
      assert.equal(text, "incoming");
      return new Promise((resolve) => {
        resume = resolve;
      });
    },
  };
  const pending = context.checkDefinitions(
    engine,
    host,
    "focus",
    "",
    () => true,
  );
  refresh();
  resume([3]);
  await pending;
  assert.equal(requests.at(-1).type, "referrers");

  const checking = context.checkDefinitions(
    engine,
    host,
    "focus",
    "",
    () => true,
  );
  resume([3]);
  await checking;
  const previous = requests.at(-1);
  assert.equal(previous.type, "definitions");
  context.ask(host, cards, ["a.rs"], true);
  assert.equal(context.receive({ ...previous, matches: [true] }), false);
  assert.deepEqual(context.sources("a.rs", "focus").uris, ["a.rs"]);
});

test("direct references need no definition lookup and unchanged syntax is read once", async () => {
  const context = new TypeContext("Focus");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  const cards = [{ source: 0, path: "Focus", start: 7 }];
  const file = { uri: "b.rs", text: "direct", cfg: "test", offsets: [2] };
  const reads = [];
  const engine = {
    relationshipTargets: async (text, cfg) => {
      reads.push([text, cfg]);
      return [2];
    },
  };
  for (const source of [
    file,
    file,
    { ...file, text: "edited" },
    { ...file, cfg: "" },
  ]) {
    context.ask(host, cards, ["a.rs"], true);
    context.receive({ ...requests.at(-1), files: [source] });
    await context.checkDefinitions(engine, host, "focus", "", () => true);
  }
  assert.deepEqual(reads, [
    ["direct", "test"],
    ["edited", "test"],
    ["direct", ""],
  ]);
  assert.ok(requests.every(({ type }) => type === "referrers"));
});

test("a target's meaning belongs to its Cargo owner and module even for the same cfg", async () => {
  const context = new TypeContext("Focus", "unix");
  const requests = [];
  const host = { send: (message) => requests.push(message) };
  const focus = { source: 0, path: "Focus", start: 4 };
  context.ask(host, [focus], ["lib.rs"]);
  const shared = {
    uri: "shared.rs",
    text: "shared",
    cfg: "unix",
    resolved: true,
  };
  context.receive({
    ...requests[0],
    files: [
      {
        uri: "lib.rs",
        text: "focus",
        cfg: "unix",
        owner: 1,
        module: "",
        offsets: [1],
        unknown: [7],
        resolved: true,
        focus: true,
      },
      { ...shared, owner: 0, module: "left", offsets: [] },
      { ...shared, owner: 1, module: "left", repeated: true, offsets: [5] },
      { ...shared, owner: 1, module: "right", repeated: true, offsets: [8] },
    ],
  });
  await context.checkDefinitions({}, host, "focus", "unix", () => true);
  const lookup = requests.at(-1);
  assert.deepEqual(lookup.places, [{ uri: "lib.rs", offset: 7 }]);
  context.receive({ ...lookup, matches: [true] });
  const { uris, readings, context: drawn } = context.sources("lib.rs", "focus");
  assert.deepEqual(uris, ["lib.rs", "shared.rs", "shared.rs"]);
  assert.deepEqual(
    drawn.files.map(({ owner, module, repeated, offsets }) => [
      owner,
      module,
      repeated,
      offsets,
    ]),
    [
      [1, "left", true, [5]],
      [1, "right", true, [8]],
    ],
  );
  assert.deepEqual(drawn.offsets, [1, 7]);
  const cards = [
    focus,
    { source: 1, path: "Incoming", start: 20 },
    { source: 2, path: "Incoming", start: 20 },
  ];
  context.ask(host, cards, uris, false, readings);
  const modules = requests.at(-1);
  assert.deepEqual(
    modules.places.map(({ owner, module }) => [owner, module]),
    [
      [1, ""],
      [1, "left"],
      [1, "right"],
    ],
  );
  context.receive({ ...modules, modules: [null, "left", "right"] });
  assert.deepEqual(
    context
      .sources("lib.rs", "focus")
      .context.files.map((file) => file.modules.Incoming),
    ["left", "right"],
  );
});
