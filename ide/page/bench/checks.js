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
 * unseen, so that it takes clicks, and keeps the `types` requests it sends.
 */
function open(overrides = {}) {
  const root = document.createElement("div");
  root.style.cssText =
    "position: fixed; top: 0; left: 0; opacity: 0; pointer-events: none";
  document.body.append(root);
  const requests = [];
  const receive = start(root, {
    send: (message) => message.type === "types" && requests.push(message),
    wasm: { ...wasm, ...overrides },
  });
  const banner = root.querySelector(".kaalang-banner");
  return {
    show: (text, cursor, uri = "a") =>
      receive({ type: "document", uri, text, cursor }),
    banner: () => (banner.hidden ? null : banner.textContent),
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
    on("posted", "drawFlow", () => page.show("struct X;\n", 0, "b"));
    // What shows once the last document's diagram has arrived.
    on("answering", "drawFlow", () => setTimeout(() => (late = page.drawn())));
    page.show(flow, inFlow);
    await settled();
    return late === false && !page.drawn();
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
  async "the first flow stands in for the one the cursor left and deleted"() {
    const page = open();
    const both = two("alpha", "beta");
    page.show(both, both.lastIndexOf("return"));
    await settled();
    page.show(`// a\n${flow.replace("settle", "alpha")}`, 0);
    await settled();
    return page.flow() === "alpha";
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
