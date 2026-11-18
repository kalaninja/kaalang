// Plays the page's races against the real worker, timing its answers through a
// wrapped `Worker`. Open checks.html while `just ide bench` runs; every check
// should pass.

import { start } from "../page.js";

const built = new URL("/target/wasm/web/", location.href);
const wasm = {
  worker: new URL("../worker.js", import.meta.url).href,
  glue: new URL("kaalang_js.js", built).href,
  module: new URL("kaalang_js_bg.wasm", built).href,
};
const missing = new URL("missing", built).href;

const flow =
  "#[kaa]\nfn settle(first: u8) -> u8 {\n    |first| return first;\n}\n";
const inFlow = flow.indexOf("return");
const types =
  '#[kaa]\nstruct Member {\n    id: u8,\n}\n\n#[kaa]\nstruct Team {\n    #[kaa::refer(Member, "The lead.")]\n    lead: u8,\n}\n';
const member = "#[kaa] pub struct Member;";
const review =
  '#[kaa] pub struct Review { #[kaa::refer(crate::api::Author, "The reviewer.")] id: u8 }';
/** Two flows named `first` and `second`. */
const two = (first, second) =>
  `${flow.replace("settle", first)}${flow.replace("settle", second)}`;
const nested = 200_000;
const deep = flow.replace(
  "return first",
  `return ${"(".repeat(nested)}first${")".repeat(nested)}`,
);

/** Each runs once with a call's name: as the page posts it, or before its answer lands. */
const hooks = { posted: null, answering: null };
let sent = 0;
let answered = 0;
window.Worker = class extends Worker {
  #names = new Map();

  postMessage(message, ...rest) {
    super.postMessage(message, ...rest);
    if (message.name) {
      sent += 1;
      this.#names.set(message.id, message.name);
      hooks.posted?.(message.name);
    }
  }

  set onmessage(listener) {
    super.onmessage = (event) => {
      answered += 1;
      hooks.answering?.(this.#names.get(event.data.id));
      listener(event);
    };
  }
};

/** Runs `action` the first time `name` is `when`. */
function on(when, name, action) {
  hooks[when] = (called) => {
    if (called === name) {
      hooks[when] = null;
      action();
    }
  };
}

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Waits until every call posted has its answer and the page has paused, or 5 s. */
async function settled() {
  for (let quiet = 0, waited = 0; quiet < 3 && waited < 5000; waited += 100) {
    await wait(100);
    quiet = sent > 0 && sent === answered ? quiet + 1 : 0;
  }
}

/**
 * A page of its own, with the bindings' files `wasm` overrides. It lays out
 * unseen, so that it takes clicks, and keeps the host requests under test.
 */
function open(overrides = {}) {
  const root = document.createElement("div");
  root.style.cssText =
    "position: fixed; top: 0; left: 0; opacity: 0; pointer-events: none";
  document.body.append(root);
  const requests = [];
  const receive = start(root, {
    send: (message) =>
      ["types", "referrers", "definitions", "reveal"].includes(message.type) &&
      requests.push(message),
    wasm: { ...wasm, ...overrides },
  });
  const banner = root.querySelector(".kaalang-banner");
  return {
    receive,
    show: (text, cursor, uri = "a") =>
      receive({ type: "document", uri, text, cursor }),
    target: (cfg) => receive({ type: "target", cfg }),
    banner: () => (banner.hidden ? null : banner.textContent),
    /** How many cards the type diagram drawn shows. */
    cards: () => root.querySelectorAll(".kaalang-diagram .card-outline").length,
    /** A navigating click on the incoming card's header. */
    header() {
      const header = root.querySelector(
        '.kaalang-hotspots rect[data-kind="header"]',
      );
      const rect = header.getBoundingClientRect();
      header.dispatchEvent(
        new MouseEvent("click", {
          bubbles: true,
          ctrlKey: true,
          clientX: rect.x + rect.width / 2,
          clientY: rect.y + rect.height / 2,
        }),
      );
    },
    /** Answers `request` with the references at `offsets` of `text`. */
    refer: (request, text, offsets) =>
      receive({
        type: "referrers",
        id: request.id,
        files: [{ uri: "a", text, cfg: "", offsets }],
      }),
    drawn: () => root.querySelector(".kaalang-diagram svg") !== null,
    /** The name of the flow drawn. */
    flow: () =>
      root
        .querySelector(".kaalang-diagram svg title")
        ?.textContent.split(" ")
        .at(-1),
    /** Clicks the middle of the first vertical connection. */
    click() {
      const strip = [...root.querySelectorAll(".kaalang-hotspots > rect")].find(
        (rect) =>
          rect.dataset.kind === "connection" &&
          rect.getAttribute("width") === "24",
      );
      const { x, y, width, height } = strip.getBoundingClientRect();
      const [clientX, clientY] = [x + width / 2, y + height / 2];
      strip.dispatchEvent(
        new MouseEvent("click", { bubbles: true, clientX, clientY }),
      );
    },
    requests,
    /** Answers `request` with `type` for each offset. */
    answer: (request, type) =>
      receive({
        type: "types",
        id: request.id,
        types: Object.fromEntries(request.offsets.map((at) => [at, type])),
      }),
    /** The types the open list shows, or `null` when none is open. */
    types: () =>
      root.querySelector(".kaalang-popup")?.querySelector("span").textContent ??
      null,
    collapse: () => root.querySelector(".kaalang-collapse").click(),
  };
}

const checks = {
  async "an answer about the last document is not drawn on the next"() {
    const page = open();
    let late;
    on("posted", "drawFlow", () => page.show("fn plain() {}\n", 0, "b"));
    // What shows once the last document's diagram has arrived.
    on("answering", "drawFlow", () => setTimeout(() => (late = page.drawn())));
    page.show(flow, inFlow);
    await settled();
    return late === false && !page.drawn();
  },
  async "an edit overtakes a drawing before the typing pause ends"() {
    const page = open();
    let stale;
    on("answering", "drawFlow", () => {
      page.show(flow.replace("settle", "updated"), inFlow);
      setTimeout(() => (stale = page.drawn()));
    });
    page.show(flow, inFlow);
    await settled();
    return stale === false && page.flow() === "updated";
  },
  async "a new document starts with its own cfg"() {
    const page = open();
    const source = `#[cfg(enabled)]\n${two("enabled", "fallback")}`;
    page.show(source, 0);
    page.target("enabled");
    await settled();
    const first = page.flow();
    page.show(source, 0, "b");
    await settled();
    return first === "enabled" && page.flow() === "fallback";
  },
  async "a flow found in the last document is not drawn from the next"() {
    const page = open();
    const last = two("alpha", "beta");
    on("answering", "declaredAt", () =>
      page.show(`// b\n${two("gamma", "delta")}`, 0, "b"),
    );
    // The cursor in the last document's second flow; the next draws its first.
    page.show(last, last.lastIndexOf("return"));
    await settled();
    return page.flow() === "gamma";
  },
  async "the last flow follows its path when another flow moves into its position"() {
    const page = open();
    const both = two("alpha", "beta") + flow.replace("settle", "gamma");
    page.show(both, both.indexOf("fn beta"));
    await settled();
    page.show(`// outside\n${flow}${both}`, 0);
    await settled();
    const kept = page.flow();
    page.show(`// outside\n${two("alpha", "gamma")}`, 0);
    await settled();
    return kept === "beta" && page.flow() === "alpha";
  },
  async "the banner goes once the text parses, the cursor outside every flow"() {
    const page = open();
    page.show("fn (\n", 0);
    await settled();
    const shown = page.banner() !== null;
    page.show("fn f() {}\n", 0);
    await settled();
    return shown && page.banner() === null;
  },
  async "bindings that do not load are an internal error"() {
    const page = open({ glue: missing });
    page.show(flow, inFlow);
    await settled();
    return page.banner()?.startsWith("Internal error") ?? false;
  },
  async "a module that does not load is an internal error"() {
    const page = open({ module: missing });
    page.show(flow, inFlow);
    await settled();
    return page.banner()?.startsWith("Internal error") ?? false;
  },
  async "a type asked for an earlier list does not fill a later one"() {
    const page = open();
    page.show(flow, inFlow);
    await settled();
    page.click();
    page.click();
    const [early, late] = page.requests;
    page.answer(early, "u16");
    const kept = page.types();
    page.answer(late, "u8");
    return kept === "" && page.types() === "u8";
  },
  async "a list opened while the diagram updates closes with it"() {
    const page = open();
    page.show(flow, inFlow);
    await settled();
    on("posted", "drawFlow", () => page.click());
    page.collapse();
    await settled();
    return page.requests.length === 1 && page.types() === null;
  },
  async "a type draws the types the host finds referring to it"() {
    const page = open();
    page.show(types, types.indexOf("Member {"));
    await settled();
    const alone = page.cards();
    const [asked] = page.requests;
    page.refer(asked, types, [types.indexOf("Member, ")]);
    await settled();
    return asked?.type === "referrers" && alone === 1 && page.cards() === 2;
  },
  async "a re-exported relationship draws and navigates from its own file"() {
    const page = open();
    page.show(member, member.indexOf("Member"));
    await settled();
    const [refs] = page.requests;
    const unrelated = review.replace("Review", "Other");
    page.receive({
      type: "referrers",
      id: refs.id,
      files: [
        { uri: "review.rs", text: review, cfg: "", offsets: [] },
        { uri: "other.rs", text: unrelated, cfg: "", offsets: [] },
      ],
    });
    await settled();
    const lookup = page.requests.at(-1);
    if (
      lookup?.type !== "definitions" ||
      page.cards() !== 1 ||
      lookup.offset !== member.indexOf("Member") ||
      JSON.stringify(lookup.places) !==
        JSON.stringify([
          { uri: "review.rs", offset: review.indexOf("Author") },
          { uri: "other.rs", offset: unrelated.indexOf("Author") },
        ])
    )
      return false;
    page.receive({
      type: "definitions",
      id: lookup.id,
      matches: [true, false],
    });
    await settled();
    if (page.cards() !== 2) return false;
    page.header();
    const jump = page.requests.at(-1);
    return (
      jump?.type === "reveal" &&
      jump.uri === "review.rs" &&
      jump.text === review &&
      jump.start === review.indexOf("Review") &&
      page.requests.filter(({ type }) => type === "definitions").length === 1
    );
  },
  async "a refreshed type ignores definitions from its older workspace snapshot"() {
    const page = open();
    page.show(member, member.indexOf("Member"));
    await settled();
    const files = [{ uri: "review.rs", text: review, cfg: "", offsets: [] }];
    page.receive({ type: "referrers", id: page.requests.at(-1).id, files });
    await settled();
    const old = page.requests.at(-1);
    page.receive({ type: "changed" });
    const newer = page.requests.at(-1);
    page.receive({ type: "referrers", id: newer.id, files });
    await settled();
    const latest = page.requests.at(-1);
    page.receive({ type: "definitions", id: old.id, matches: [true] });
    page.receive({ type: "definitions", id: latest.id, matches: [false] });
    await settled();
    return (
      old.type === "definitions" &&
      newer.type === "referrers" &&
      latest.type === "definitions" &&
      old.id !== latest.id &&
      page.cards() === 1
    );
  },
  async "references from a previous text or cfg cannot add a card"() {
    const page = open();
    const cursor = types.indexOf("Member {");
    const offset = types.indexOf("Member, ");
    page.show(types, cursor);
    await settled();
    const [first] = page.requests;
    page.refer(first, types, [offset]);
    await settled();
    const edited = types.replace("refer(Member", "refer(OtherX");
    page.show(edited, cursor);
    await settled();
    const second = page.requests.at(-1);
    page.target("test");
    await settled();
    page.refer(second, edited, [offset]);
    await settled();
    return (
      first !== second && second !== page.requests.at(-1) && page.cards() === 1
    );
  },
  async "after a trap, the next text draws"() {
    const page = open();
    page.show(deep, inFlow);
    await settled();
    const trapped = page.banner()?.startsWith("Internal error") ?? false;
    page.show(flow, inFlow);
    await settled();
    return trapped && page.banner() === null && page.drawn();
  },
};

const results = document.querySelector("#results");
let failed = 0;
for (const [name, check] of Object.entries(checks)) {
  hooks.posted = hooks.answering = null;
  sent = answered = 0;
  let passed = false;
  try {
    passed = await check();
  } catch (error) {
    console.error(error);
  }
  failed += passed ? 0 : 1;
  const item = document.createElement("li");
  item.textContent = `${passed ? "pass" : "FAIL"}: ${name}`;
  results.append(item);
}
document.querySelector("#summary").textContent = `${failed} failed`;
