// The page every supported IDE shows: the flow at the editor's cursor, drawn
// again as the text is typed, and the ribbon of a clicked connection or merge.
// A host gives the text, the cursor and the wires' types, and carries out what
// a click asks for; it never touches the module.

import { Crash, createEngine } from "./engine.js";
import { carryingAt, innermost, request, wires } from "./hotspots.js";

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
 * - `{type: "document", uri, text, cursor}` on opening a document, on every
 *   edit, and on every cursor move;
 * - `{type: "target", cfg}` with the target's cfg set as `rustc --print cfg`
 *   prints it, empty until a host sends one;
 * - `{type: "types", id, types}` in answer to `types`, at once or in parts:
 *   each offset answered maps to the type's text, or to `null` when the host
 *   learns none.
 *
 * To the host:
 * - `{type: "ready"}` once started;
 * - `{type: "reveal", start, end}` to select a node's code;
 * - `{type: "definition", offset}` and `{type: "typeDefinition", offset}` to
 *   run the editor's own go-to action there. The editor's cursor moves to the
 *   target, and the diagram follows it;
 * - `{type: "types", id, offsets}` for the types of the values bound at these
 *   offsets, as the editor's hover gives them; each answer repeats `id`.
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
  const collapse = root.querySelector(".kaalang-collapse");
  const banner = root.querySelector(".kaalang-banner");
  const diagram = root.querySelector(".kaalang-diagram");
  const engine = createEngine(host.wasm);

  const state = {
    uri: "",
    text: "",
    cursor: 0,
    cfg: "",
    /** The flow the cursor was last in, by its position in the text; until then the first is drawn. */
    position: null,
    collapseCycles: false,
  };
  /** What the diagram on screen was drawn from. */
  let drawn = null;
  /** Counts updates, so that an answer an update has overtaken is dropped. */
  let generation = 0;
  let timer;

  function schedule(delay) {
    clearTimeout(timer);
    timer = setTimeout(update, delay);
  }

  async function update() {
    const current = ++generation;
    const { text, cfg, cursor, collapseCycles } = state;
    try {
      const declared = await engine.declaredAt(text, cfg, cursor);
      if (current !== generation) {
        return;
      }
      if (declared?.flow !== undefined) {
        state.position = declared.flow;
      }
      const position = state.position ?? 0;
      if (
        drawn?.text === text &&
        drawn.cfg === cfg &&
        drawn.position === position &&
        drawn.collapseCycles === collapseCycles
      ) {
        show(null);
        return;
      }
      const result = await engine.drawFlow(text, cfg, position, collapseCycles);
      if (current !== generation) {
        return;
      }
      if (result === null && position !== 0) {
        // The flow the cursor was last in is gone: the first one stands in.
        state.position = null;
        schedule(0);
        return;
      }
      let outlines = new Map();
      if (result === null) {
        redraw('<p class="kaalang-empty">No flow to draw.</p>');
      } else {
        redraw(result.svg);
        outlines = outline(diagram.querySelector("svg"), result.hotspots);
      }
      drawn = {
        text,
        cfg,
        position,
        collapseCycles,
        hotspots: result?.hotspots ?? [],
        wires: result?.wires ?? [],
        outlines,
      };
      show(null);
    } catch (error) {
      if (current !== generation) {
        return;
      }
      show(
        error instanceof Crash
          ? `Internal error: ${error.message}. Edit the text to try again.`
          : error.message,
      );
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
    if (svg === null || drawn?.text !== state.text) {
      return [];
    }
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
    }
    group.append(...outlines.values());
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
        host.send({ type: "typeDefinition", offset: wire.start });
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
      host.send(request(found[0]));
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

  // After `start` returns, so that the host holds the receiver by then.
  queueMicrotask(() => host.send({ type: "ready" }));

  return (message) => {
    switch (message.type) {
      case "document":
        if (message.uri !== state.uri) {
          // An answer about the last document must not reach this one.
          ++generation;
          state.uri = message.uri;
          state.position = null;
          drawn = null;
          redraw("");
        }
        if (message.text !== state.text) {
          // A list's offsets count in the text it was opened on.
          close();
        }
        state.text = message.text;
        state.cursor = message.cursor;
        schedule(PAUSE);
        break;
      case "target":
        state.cfg = message.cfg;
        schedule(0);
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
