// Plays the page's races against the real worker, timing its answers through a
// wrapped `Worker`. Open checks.html while `just ide browser` runs; every check
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
    send: (message) => {
      if (
        ["types", "referrers", "reveal", "definition"].includes(message.type)
      ) {
        requests.push(message);
      }
    },
    wasm: { ...wasm, ...overrides },
  });
  const banner = root.querySelector(".kaalang-banner");
  return {
    receive,
    visible: (shown) => {
      root.style.display = shown ? "" : "none";
    },
    clickType(field, name) {
      const line = root.querySelectorAll(".field-type")[field];
      const box = line.getExtentOfChar(line.textContent.lastIndexOf(name));
      const at = new DOMPoint(
        box.x + box.width / 2,
        box.y + box.height / 2,
      ).matrixTransform(line.getScreenCTM());
      line.dispatchEvent(
        new MouseEvent("click", {
          bubbles: true,
          ctrlKey: true,
          clientX: at.x,
          clientY: at.y,
        }),
      );
    },
    show: (text, cursor, uri = "a") =>
      receive({ type: "document", uri, text, cursor }),
    target: (cfg) => receive({ type: "target", cfg }),
    banner: () => (banner.hidden ? null : banner.textContent),
    /** How many cards the type diagram drawn shows. */
    cards: () => root.querySelectorAll(".kaalang-diagram .card-outline").length,
    fieldNames: () =>
      [...root.querySelectorAll(".field-name > text:not(.md-badge)")].map(
        (node) => node.textContent.trim(),
      ),
    /** A navigating click on the middle of a card hotspot. */
    clickCard(kind, index = 0) {
      const spot = root.querySelectorAll(
        `.kaalang-hotspots rect[data-kind="${kind}"]`,
      )[index];
      const rect = spot.getBoundingClientRect();
      spot.dispatchEvent(
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
    loading: () => !!root.querySelector('.kaalang-diagram [role="status"]'),
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
    on("answering", "declaredRanges", () =>
      page.show(`// b\n${two("gamma", "delta")}`, 0, "b"),
    );
    // The cursor in the last document's second flow; the next draws its first.
    page.show(last, last.lastIndexOf("return"));
    await settled();
    return page.flow() === "gamma";
  },
  async "cursor moves reuse one declaration index per source and cfg"() {
    const page = open();
    let indexes = 0;
    hooks.posted = (name) => {
      if (name === "declaredRanges") ++indexes;
    };
    const source = two("alpha", "beta");
    page.show(source, source.indexOf("alpha"));
    await settled();
    page.show(source, source.indexOf("beta"));
    await settled();
    if (indexes !== 1 || page.flow() !== "beta") return false;
    page.target("enabled");
    await settled();
    page.show("// 🦀\n" + source, 0);
    await settled();
    return indexes === 3 && page.flow() === "beta";
  },
  async "a click on a visible generic argument uses the browser's glyph geometry"() {
    const page = open();
    const source =
      "struct Profile<T> { a: std::collections::HashMap<AccountId, T>, b: std::collections::HashMap<𐐀ccount, T> }";
    page.visible(false);
    page.show(source, source.indexOf("Profile"));
    await settled();
    page.visible(true);
    for (const [field, name, offset] of [
      [0, "T", source.indexOf(", T>") + 2],
      [1, "𐐀", source.indexOf("𐐀")],
      [1, "T", source.lastIndexOf(", T>") + 2],
    ]) {
      page.clickType(field, name);
      const jump = page.requests.at(-1);
      if (jump.type !== "definition" || jump.offset !== offset) return false;
    }
    return true;
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
  async "a card from another file draws and navigates from its own file"() {
    const page = open();
    page.show(member, member.indexOf("Member"));
    await settled();
    const [refs] = page.requests;
    page.receive({
      type: "referrers",
      id: refs.id,
      files: [
        {
          uri: "review.rs",
          text: review,
          cfg: "",
          offsets: [review.indexOf("Author")],
        },
      ],
    });
    await settled();
    if (page.cards() !== 2) return false;
    page.clickCard("header");
    const jump = page.requests.at(-1);
    page.clickCard("target");
    const target = page.requests.at(-1);
    return (
      jump?.type === "reveal" &&
      jump.uri === "review.rs" &&
      jump.text === review &&
      jump.start === review.indexOf("Review") &&
      target.type === "reveal" &&
      target.uri === "a" &&
      target.text === member &&
      target.start === member.indexOf("Member") &&
      target.end === target.start + "Member".length &&
      target.origin?.uri === "review.rs" &&
      target.origin.text === review
    );
  },

  async "the same relationship token opens the focus it was drawn for"() {
    const page = open();
    const text =
      '#[kaa] pub struct Report { #[kaa::refer(crate::Focus, "Owner")] id: u8, #[kaa::refer(crate::Other, "Other")] other: u8 }';
    for (const uri of ["a.rs", "b.rs"]) {
      const root = `// 🦀 ${uri}\npub struct Focus; pub struct Other; mod shared;`;
      const start = root.indexOf("Focus");
      page.show(root, start, uri);
      await settled();
      const refs = page.requests.findLast((r) => r.type === "referrers");
      page.receive({
        ...refs,
        files: [
          { uri: "shared.rs", text, cfg: "", offsets: [text.indexOf("Focus")] },
        ],
      });
      await settled();
      if (page.cards() !== 2) return false;
      page.clickCard("target");
      const jump = page.requests.at(-1);
      if (
        jump.type !== "reveal" ||
        jump.uri !== uri ||
        jump.text !== root ||
        jump.start !== start ||
        jump.end !== start + "Focus".length ||
        jump.origin?.uri !== "shared.rs" ||
        jump.origin.text !== text
      )
        return false;
      page.clickCard("target", 1);
      const other = page.requests.at(-1);
      if (
        other.type !== "definition" ||
        other.uri !== "shared.rs" ||
        other.text !== text ||
        other.offset !== text.indexOf("Other")
      )
        return false;
    }
    return true;
  },
  async "a refreshed type ignores references from its older workspace snapshot"() {
    const page = open();
    page.show(member, member.indexOf("Member"));
    await settled();
    const old = page.requests.at(-1);
    page.receive({ type: "changed" });
    const newer = page.requests.at(-1);
    const files = (offsets) => [
      { uri: "review.rs", text: review, cfg: "", offsets },
    ];
    page.receive({ type: "referrers", id: newer.id, files: files([]) });
    page.receive({
      type: "referrers",
      id: old.id,
      files: files([review.indexOf("Author")]),
    });
    await settled();
    return old.id !== newer.id && page.cards() === 1;
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
  async "a type draws in the cfg of its document's target"() {
    const page = open();
    const text =
      "pub struct Focus { #[cfg(red)] red_field: u8, #[cfg(blue)] blue_field: u8 }";
    page.receive({
      type: "document",
      uri: "a",
      text,
      cursor: 0,
      waitForTarget: true,
    });
    await settled();
    if (page.drawn() || page.requests.length) return false;
    page.receive({ type: "target", uri: "a", cfg: "red" });
    await settled();
    if (!page.fieldNames().includes("red_field")) return false;
    page.receive({ type: "target", uri: "a", cfg: "blue" });
    await settled();
    return (
      page.fieldNames().includes("blue_field") &&
      !page.fieldNames().includes("red_field")
    );
  },

  async "a pending Cargo target leaves flows available and only its document's reply shows a type"() {
    const page = open();
    const pending = (uri, text, cursor) =>
      page.receive({
        type: "document",
        uri,
        text,
        cursor,
        waitForTarget: true,
      });
    pending("flow.rs", flow, inFlow);
    await settled();
    if (page.flow() !== "settle" || page.loading()) return false;
    pending("model.rs", member, member.indexOf("Member"));
    page.receive({ type: "target", uri: "flow.rs", cfg: "" });
    await settled();
    if (
      !page.loading() ||
      page.drawn() ||
      page.requests.some((r) => r.type === "referrers")
    )
      return false;
    // A file without a Cargo owner can draw as soon as the empty target arrives.
    page.receive({ type: "target", uri: "model.rs", cfg: "" });
    await settled();
    return page.cards() === 1 && !page.loading();
  },
  async "a finished type stays visible until refreshed references arrive"() {
    const page = open();
    page.show(member, member.indexOf("Member"));
    await settled();
    const reply = (offsets) =>
      page.receive({
        type: "referrers",
        id: page.requests.findLast((r) => r.type === "referrers").id,
        files: [{ uri: "review.rs", text: review, cfg: "", offsets }],
      });
    reply([review.indexOf("Author")]);
    await settled();
    if (page.cards() !== 2) return false;
    page.receive({ type: "changed" });
    await settled();
    if (page.cards() !== 2) return false;
    reply([]);
    await settled();
    return page.cards() === 1;
  },

  async "an edit of the focus keeps another file's cards until its new answer"() {
    const page = open();
    page.show(member, member.indexOf("Member"));
    await settled();
    const [asked] = page.requests;
    page.receive({
      type: "referrers",
      id: asked.id,
      files: [
        {
          uri: "review.rs",
          text: review,
          cfg: "",
          offsets: [review.indexOf("Author")],
        },
      ],
    });
    await settled();
    if (page.cards() !== 2) return false;
    const added = "// An edit above the type.\n";
    page.show(added + member, added.length + member.indexOf("Member"));
    await settled();
    const again = page.requests.findLast((r) => r.type === "referrers");
    return (
      again.id !== asked.id &&
      again.offset === asked.offset + added.length &&
      page.cards() === 2
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
