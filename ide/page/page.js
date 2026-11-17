// The page every supported IDE shows: the flow at the editor's cursor, drawn
// again as the text is typed. A host gives the text and the cursor and carries out what a
// click asks for; it never touches the module.

import { Crash, createEngine } from "./engine.js";
import { innermost, request } from "./hotspots.js";

/** How long typing pauses before the page draws again, in milliseconds. */
const PAUSE = 100;

/** What the diagram shows before a flow is drawn. */
const EMPTY = '<p class="kaalang-empty">Put the cursor in a flow.</p>';

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
 *   prints it, empty until a host sends one.
 *
 * To the host:
 * - `{type: "ready"}` once started;
 * - `{type: "reveal", start, end}` to select a node's code;
 * - `{type: "definition", offset}` and `{type: "typeDefinition", offset}` to
 *   run the editor's own go-to action there. The editor's cursor moves to the
 *   target, and the diagram follows it.
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
    <div class="kaalang-diagram">${EMPTY}</div>`;
  const collapse = root.querySelector(".kaalang-collapse");
  const banner = root.querySelector(".kaalang-banner");
  const diagram = root.querySelector(".kaalang-diagram");
  const engine = createEngine(host.wasm);

  const state = {
    uri: "",
    text: "",
    cursor: 0,
    cfg: "",
    /** The flow drawn, by its position in the text; kept while the cursor is outside every flow. */
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
      const { position } = state;
      if (position === null) {
        show(null);
        return;
      }
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
      diagram.innerHTML = result.svg;
      outline(diagram.querySelector("svg"), result.hotspots);
      drawn = {
        text,
        cfg,
        position,
        collapseCycles,
        hotspots: result.hotspots,
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
   * The hotspot under a pointer event, while the diagram on screen shows the
   * current text: the ranges of an older text would lead astray.
   */
  function target(event) {
    const svg = diagram.querySelector("svg");
    if (svg === null || drawn?.text !== state.text) {
      return null;
    }
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(
      svg.getScreenCTM().inverse(),
    );
    return innermost(drawn.hotspots, point.x, point.y);
  }

  /**
   * Adds an outline for each hotspot to `svg`, in the hotspots' order. Each
   * stays hidden until the pointer is on it under the modifier, as an editor
   * underlines a link.
   */
  function outline(svg, hotspots) {
    const group = document.createElementNS(svg.namespaceURI, "g");
    group.setAttribute("class", "kaalang-hotspots");
    for (const hotspot of hotspots) {
      const rect = document.createElementNS(svg.namespaceURI, "rect");
      rect.setAttribute("x", hotspot.left);
      rect.setAttribute("y", hotspot.top);
      rect.setAttribute("width", hotspot.right - hotspot.left);
      rect.setAttribute("height", hotspot.bottom - hotspot.top);
      group.append(rect);
    }
    svg.append(group);
  }

  /** Shows the outline of `hotspot`, or of none. */
  function highlight(hotspot) {
    diagram
      .querySelector(".kaalang-hovered")
      ?.classList.remove("kaalang-hovered");
    diagram.classList.toggle("kaalang-linking", hotspot !== null);
    if (hotspot !== null) {
      const outlines = diagram.querySelectorAll(".kaalang-hotspots > rect");
      outlines[drawn.hotspots.indexOf(hotspot)].classList.add(
        "kaalang-hovered",
      );
    }
  }

  /** Ctrl, or Cmd on macOS, as an editor goes to a definition. */
  const navigating = (event) => event.ctrlKey || event.metaKey;
  /** The last pointer move over the diagram, for a modifier pressed in place. */
  let pointer = null;
  diagram.addEventListener("pointermove", (event) => {
    pointer = event;
    highlight(navigating(event) ? target(event) : null);
  });
  diagram.addEventListener("pointerleave", () => {
    pointer = null;
    highlight(null);
  });
  for (const name of ["keydown", "keyup"]) {
    window.addEventListener(name, (event) => {
      if (pointer !== null) {
        highlight(navigating(event) ? target(pointer) : null);
      }
    });
  }
  diagram.addEventListener("click", (event) => {
    const hotspot = navigating(event) ? target(event) : null;
    if (hotspot !== null) {
      event.preventDefault();
      host.send(request(hotspot));
    }
  });

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
          diagram.innerHTML = EMPTY;
        }
        state.text = message.text;
        state.cursor = message.cursor;
        schedule(PAUSE);
        break;
      case "target":
        state.cfg = message.cfg;
        schedule(0);
        break;
    }
  };
}
