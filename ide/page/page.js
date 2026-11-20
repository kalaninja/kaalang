// The page every supported IDE shows: the flow or the type at the editor's
// cursor, drawn again as the text is typed, and the ribbon of a clicked
// connection or merge.
// A host gives the text, the cursor, and what the editor's engine knows: the
// wires' types, the references to a type, and its module. It carries out what a
// click asks for and never touches the module.

import { Crash, createEngine } from "./engine.js";
import { carryingAt, innermost, request, wires } from "./hotspots.js";
import { TypeContext } from "./type-context.js";

/** How long typing pauses before the page draws again, in milliseconds. */
const PAUSE = 100;

/** Counts the type cells of every page, so that each has its own id. */
let cells = 0;

/**
 * @typedef {object} Host
 * @property {(message: object) => void} send Carries a message to the host.
 * @property {import('./engine.js').Wasm} wasm Where the bindings' files are.
 */

/**
 * Starts the page in `root`. Messages are plain objects; positions count
 * UTF-16 code units of the text.
 *
 * From the host:
 * - `{type: "document", uri, text, cursor, waitForTarget?}` on opening a
 *   document, on every edit, and on every cursor move. A host that will send
 *   a target can defer the first type drawing until then;
 * - `{type: "target", uri, cfg}` with the cfg set the editor's engine reads the
 *   document with;
 * - `{type: "types", id, types}` in answer to `types`, at once or in parts:
 *   each offset answered maps to the type's text, or to `null` when the host
 *   learns none;
 * - `{type: "referrers", id, focus?: {cfg, place?}, files: [{uri, text, cfg, offsets, place?}]}`
 *   in answer to `referrers`: each file the engine finds references to the
 *   focus in, the focus's own included, read once, in the crate the engine
 *   reads it in, with the references' `offsets`. A `place` is where a text
 *   stands in the workspace's module tree, as `drawType` takes it;
 * - `{type: "changed"}` when the engine may answer `referrers` otherwise: it
 *   has analyzed more, or a document was saved.
 *
 * To the host:
 * - `{type: "ready"}` once started;
 * - `{type: "reveal", uri, start, end}` to select a node's code;
 * - `{type: "definition", uri, offset}` and `{type: "typeDefinition", uri, offset}`
 *   to run the editor's own go-to action in the document drawn. The editor's
 *   cursor moves to the target, and the diagram follows it. A go-to or reveal
 *   from a card in another file includes its drawn `text`;
 * - `{type: "types", id, uri, offsets}` for the types of the values bound at these
 *   offsets, as the editor's hover gives them; each answer repeats `id`;
 * - `{type: "referrers", id, uri, offset}` for the references to the type
 *   named at this offset of that document, as the editor's engine finds them;
 * - `{type: "back"}` and `{type: "forward"}` for a mouse's back and forward
 *   buttons, which go through the editor's own history.
 *
 * A host colors the page with the `--vscode-*` variables `page.css` reads, as a
 * VS Code webview sets them.
 *
 * @param {HTMLElement} root
 * @param {Host} host
 * @returns {(message: object) => void} Takes a message from the host.
 */
export function start(root, host) {
  root.classList.add("kaalang-page");
  root.innerHTML = `
    <div class="kaalang-toolbar">
      <button type="button" class="kaalang-collapse" aria-pressed="false">Collapse cycles</button>
    </div>
    <p class="kaalang-banner" role="status" hidden></p>
    <div class="kaalang-diagram"></div>`;
  const toolbar = root.querySelector(".kaalang-toolbar");
  const collapse = root.querySelector(".kaalang-collapse");
  const banner = root.querySelector(".kaalang-banner");
  const diagram = root.querySelector(".kaalang-diagram");
  const engine = createEngine(host.wasm);

  const state = {
    uri: "",
    text: "",
    cursor: 0,
    cfg: "",
    waitingForTarget: false,
    /**
     * What the cursor was last in, by its path in the file. Until then the
     * first flow is drawn.
     */
    declared: null,
    collapseCycles: false,
  };
  /** References and module paths belong to one type in one source snapshot. */
  let type = null;
  /** What the diagram on screen was drawn from. */
  let drawn = null;
  /** Counts updates, so that an answer an update has overtaken is dropped. */
  let generation = 0;
  let timer;
  let drawing = false;

  function schedule(delay) {
    ++generation;
    clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      if (!drawing) {
        void update();
      }
    }, delay);
  }

  async function update() {
    drawing = true;
    const current = generation;
    const { text, cfg, cursor, collapseCycles } = state;
    try {
      const declared = await engine.declaredAt(text, cfg, cursor);
      if (current !== generation) {
        return;
      }
      // The first target can uncover a type hidden in the default cfg.
      if (state.waitingForTarget && declared?.flow === undefined) return;
      if (declared !== null) {
        state.declared = declared;
      }
      const shown = state.declared ?? { flow: 0 };
      if (shown.type !== type?.path || shown.start !== type?.start) {
        type =
          shown.type === undefined
            ? null
            : new TypeContext(shown.type, cfg, shown.start);
      }
      const version = type?.version;
      if (
        drawn?.text === text &&
        drawn.cfg === cfg &&
        drawn.shown.flow === shown.flow &&
        drawn.shown.path === shown.path &&
        drawn.shown.type === shown.type &&
        drawn.collapseCycles === collapseCycles &&
        drawn.type === type &&
        drawn.version === version
      ) {
        show(null);
        return;
      }
      let uris = [state.uri];
      let texts;
      let result;
      if (shown.type === undefined) {
        result = await engine.drawFlow(
          text,
          cfg,
          shown.flow,
          collapseCycles,
          shown.path,
        );
      } else {
        const context = type.sources(state.uri, text);
        uris = context.uris;
        texts = context.texts;
        result = await engine.drawType(
          text,
          type.cfg,
          shown.type,
          JSON.stringify(context.context),
        );
      }
      if (current !== generation) {
        return;
      }
      if (result === null && state.declared !== null) {
        // What the cursor was last in is gone: the first flow stands in.
        state.declared = null;
        schedule(0);
        return;
      }
      if (result?.cards) {
        type.ask(host, uris[0], result.cards[0].start);
        if (current !== generation || !type.canPublish()) return;
      }
      toolbar.hidden = shown.type !== undefined;
      let outlines = new Map();
      let measured = true;
      if (result === null) {
        redraw('<p class="kaalang-empty">No flow to draw.</p>');
      } else {
        redraw(result.svg);
        const svg = diagram.querySelector("svg");
        outlines = outline(svg, result.hotspots);
        if (result.cards) measured = measureFieldText(svg, outlines);
      }
      drawn = {
        text,
        cfg,
        shown,
        type,
        collapseCycles,
        version,
        hotspots: result?.hotspots ?? [],
        wires: result?.wires ?? [],
        cards: result?.cards,
        uris,
        texts,
        outlines,
        measured,
      };
      show(null);
    } catch (error) {
      if (current !== generation) {
        return;
      }
      if (!drawn) redraw("");
      show(
        error instanceof Crash
          ? `Internal error: ${error.message}. Edit the text to try again.`
          : error.message,
      );
    } finally {
      drawing = false;
      // If typing paused while the worker was busy, draw just the latest text.
      if (current !== generation && timer === undefined) {
        void update();
      }
    }
  }

  /**
   * Puts `html` in place of the diagram, closing the list and the outlines
   * that belonged to the last one, even if they came after the update began.
   */
  function redraw(html) {
    close();
    highlight([]);
    diagram.innerHTML = html;
  }

  /** Shows `message` over the last diagram, dimmed, or clears it. */
  function show(message) {
    banner.hidden = message === null;
    banner.textContent = message ?? "";
    root.classList.toggle("kaalang-stale", message !== null);
  }

  collapse.addEventListener("click", () => {
    state.collapseCycles = !state.collapseCycles;
    collapse.setAttribute("aria-pressed", String(state.collapseCycles));
    schedule(0);
  });

  /**
   * The hotspots a pointer event lands on, while the diagram on screen shows
   * the current text: the ranges of an older text would lead astray. Under the
   * modifier, the place a click leads to; without, those whose ribbons a click
   * lists.
   */
  function targets(event, navigate) {
    const svg = diagram.querySelector("svg");
    if (svg === null || drawn?.text !== state.text || drawn.cfg !== state.cfg) {
      return [];
    }
    // A retained editor tab can receive its drawing while hidden.
    if (!drawn.measured) drawn.measured = measureFieldText(svg, drawn.outlines);
    const { x, y } = new DOMPoint(event.clientX, event.clientY).matrixTransform(
      svg.getScreenCTM().inverse(),
    );
    if (!navigate) {
      return carryingAt(drawn.hotspots, x, y);
    }
    const place = innermost(drawn.hotspots, x, y);
    return place === null ? [] : [place];
  }

  /**
   * Adds an outline for each hotspot to `svg`, in the hotspots' order, and
   * returns each by its hotspot. Each stays hidden until the pointer is on it,
   * as an editor underlines a link.
   */
  function outline(svg, hotspots) {
    const group = document.createElementNS(svg.namespaceURI, "g");
    group.setAttribute("class", "kaalang-hotspots");
    const outlines = new Map();
    for (const hotspot of hotspots) {
      const rect = document.createElementNS(svg.namespaceURI, "rect");
      rect.setAttribute("x", hotspot.left);
      rect.setAttribute("y", hotspot.top);
      rect.setAttribute("width", hotspot.right - hotspot.left);
      rect.setAttribute("height", hotspot.bottom - hotspot.top);
      rect.dataset.kind = hotspot.kind;
      outlines.set(hotspot, rect);
      group.append(rect);
    }
    svg.append(group);
    return outlines;
  }

  /** The outlines on show. */
  let hovered = [];
  /** Shows the outlines of `hotspots`, and of no other. */
  function highlight(hotspots) {
    const rects = hotspots.map((hotspot) => drawn.outlines.get(hotspot));
    if (
      rects.length === hovered.length &&
      rects.every((rect, index) => rect === hovered[index])
    ) {
      return;
    }
    for (const rect of hovered) {
      rect.classList.remove("kaalang-hovered");
    }
    hovered = rects;
    for (const rect of hovered) {
      rect.classList.add("kaalang-hovered");
    }
    diagram.classList.toggle("kaalang-linking", hovered.length > 0);
  }

  /** The open list of the wires under a click. */
  let popup = null;
  /** Numbers the lists, so that types asked for one never fill a later one. */
  let listing = 0;
  /** The type cell of each wire listed, by its binding's offset. */
  const typeCells = new Map();

  function close() {
    popup?.remove();
    popup = null;
    typeCells.clear();
  }

  /** Lists `listed` at the pointer of `event`, and asks the host for their types. */
  function open(listed, event) {
    close();
    const id = ++listing;
    const uri = drawn.uris[0];
    popup = document.createElement("div");
    popup.className = "kaalang-popup";
    popup.setAttribute("role", "dialog");
    popup.setAttribute("aria-label", "Wires");
    if (listed.length === 0) {
      popup.textContent = "No wires.";
    }
    for (const wire of listed) {
      const name = document.createElement("button");
      name.type = "button";
      name.textContent = wire.name;
      name.addEventListener("click", () => {
        close();
        host.send({ type: "typeDefinition", uri, offset: wire.start });
      });
      const type = document.createElement("span");
      type.id = `kaalang-type-${++cells}`;
      name.setAttribute("aria-describedby", type.id);
      typeCells.set(wire.start, type);
      popup.append(name, type);
    }
    // It grows away from the nearer edges, so that it stays in view.
    const { clientX: x, clientY: y } = event;
    if (x > innerWidth / 2) {
      popup.style.right = `${innerWidth - x}px`;
    } else {
      popup.style.left = `${x}px`;
    }
    if (y > innerHeight / 2) {
      popup.style.bottom = `${innerHeight - y}px`;
    } else {
      popup.style.top = `${y}px`;
    }
    root.append(popup);
    // The first name takes the focus, or the list itself when it names none.
    popup.tabIndex = -1;
    (popup.querySelector("button") ?? popup).focus();
    if (listed.length > 0) {
      host.send({
        type: "types",
        id,
        uri,
        offsets: listed.map((wire) => wire.start),
      });
    }
  }

  /** Ctrl, or Cmd on macOS, as an editor goes to a definition. */
  const navigating = (event) => event.ctrlKey || event.metaKey;
  /** The last pointer move over the diagram, for a modifier pressed in place. */
  let pointer = null;
  diagram.addEventListener("pointermove", (event) => {
    pointer = event;
    highlight(targets(event, navigating(event)));
  });
  diagram.addEventListener("pointerleave", () => {
    pointer = null;
    highlight([]);
  });
  for (const name of ["keydown", "keyup"]) {
    window.addEventListener(name, (event) => {
      if (event.key === "Escape") {
        close();
      }
      if (pointer !== null) {
        highlight(targets(pointer, navigating(event)));
      }
    });
  }
  diagram.addEventListener("click", (event) => {
    const navigate = navigating(event);
    const found = targets(event, navigate);
    if (found.length > 0 && navigate) {
      event.preventDefault();
      host.send(request(found[0], drawn.uris, drawn.texts, drawn.cards?.[0]));
    } else if (found.length > 0) {
      open(wires(found, drawn.wires), event);
    }
  });
  // A popup closes on a click elsewhere, even in the editor, and as its line
  // scrolls away.
  window.addEventListener("pointerdown", (event) => {
    if (popup?.contains(event.target) === false) {
      close();
    }
  });
  window.addEventListener("blur", close);
  diagram.addEventListener("scroll", close);

  // A frame keeps a mouse's back and forward buttons from the editor, which
  // goes through its history with them everywhere else. Pressed, they and a
  // navigating click leave the focus in the editor: a diagram that takes it
  // is a stop in that history where nothing moves, and drops every stop ahead.
  root.addEventListener("mousedown", (event) => {
    if (event.button === 3 || event.button === 4 || navigating(event)) {
      event.preventDefault();
    }
  });
  root.addEventListener("mouseup", (event) => {
    if (event.button === 3 || event.button === 4) {
      event.preventDefault();
      host.send({ type: event.button === 3 ? "back" : "forward" });
    }
  });

  // After `start` returns, so that the host holds the receiver by then.
  queueMicrotask(() => host.send({ type: "ready" }));

  return (message) => {
    switch (message.type) {
      case "document":
        if (message.uri !== state.uri) {
          type = null;
          state.cfg = "";
          state.waitingForTarget = !!message.waitForTarget;
          state.uri = message.uri;
          state.declared = null;
          drawn = null;
          show(null);
          redraw(
            state.waitingForTarget
              ? '<p class="kaalang-empty" role="status">Loading diagram…</p>'
              : "",
          );
        }
        if (message.text !== state.text) {
          // A list's offsets count in the text it was opened on.
          close();
          type = null;
        }
        state.text = message.text;
        state.cursor = message.cursor;
        schedule(PAUSE);
        break;
      case "target":
        if (message.uri && message.uri !== state.uri) break;
        if (state.waitingForTarget || message.cfg !== state.cfg) {
          state.waitingForTarget = false;
          state.cfg = message.cfg;
          type = null;
          close();
          schedule(0);
        }
        break;
      case "changed":
        type?.refresh(host);
        break;
      case "referrers":
        if (type?.receive(message)) schedule(0);
        break;
      case "types":
        if (message.id !== listing) {
          break;
        }
        for (const [offset, type] of Object.entries(message.types)) {
          const cell = typeCells.get(Number(offset));
          if (cell !== undefined) {
            cell.textContent = type ?? "";
          }
        }
        break;
    }
  };
}

/** Fit the clickable source ranges to the glyphs the browser actually drew. */
function measureFieldText(svg, outlines) {
  if (!svg.getClientRects().length) return false;
  const lines = svg.querySelectorAll(".field-type > tspan[x]");
  for (const spot of outlines.keys()) {
    if (!spot.fieldText) continue;
    const [index, start, end] = spot.fieldText;
    const line = lines[index];
    if (!line || end > line.getNumberOfChars()) continue;
    let left = Infinity;
    let right = -Infinity;
    for (let at = start; at < end; ++at) {
      const glyph = line.getExtentOfChar(at);
      left = Math.min(left, glyph.x);
      right = Math.max(right, glyph.x + glyph.width);
    }
    if (left < right) {
      spot.left = left;
      spot.right = right;
    }
  }
  // Finish reading layout before changing any live SVG rectangle.
  for (const [spot, rect] of outlines) {
    if (!spot.fieldText) continue;
    rect.setAttribute("x", spot.left);
    rect.setAttribute("width", spot.right - spot.left);
  }
  return true;
}
