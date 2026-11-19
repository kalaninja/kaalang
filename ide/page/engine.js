// The page's side of the bindings: calls into a worker, started again after a
// trap from the module compiled once.

/** The module trapped; the call's answer is lost, the next call starts anew. */
export class Crash extends Error {}

/**
 * @typedef {object} Hotspot
 * @property {'node' | 'call' | 'wire' | 'connection' | 'merge' | 'path' | 'target' | 'header'} kind
 * @property {number} left
 * @property {number} top
 * @property {number} right
 * @property {number} bottom
 * @property {number} [start] Absent on a connection and a merge.
 * @property {number} [end]
 * @property {number} [calleeStart]
 * @property {number} [calleeEnd]
 * @property {number[]} [ribbon] A connection's or a merge's ribbon, as
 *   positions in the answer's `wires`.
 * @property {number} [source] In a type diagram, the text `start` and `end`
 *   count in: 0 for the one drawn, then the files of its context.
 * @property {number[]} [fieldText] Field-type line, start and end in its UTF-16 text.
 * @property {boolean} [focus] A relationship target whose line leads to the focus.
 */

/**
 * @typedef {object} Card One card of a type diagram.
 * @property {number} source
 * @property {string} path The type's path within its text.
 * @property {number} start Where its name starts.
 * @property {number} end
 */

/**
 * @typedef {object} Wire One wire of a ribbon.
 * @property {string} name
 * @property {number} start Where its producer binds it.
 * @property {number} end
 */

/**
 * @typedef {object} Wasm The URLs of the bindings' files.
 * @property {string} worker This directory's `worker.js`.
 * @property {string} glue The JavaScript wasm-bindgen generated.
 * @property {string} module Its WebAssembly module.
 */

/**
 * @param {Wasm} wasm
 */
export function createEngine(wasm) {
  let compiled;
  /** @type {Worker | null} */
  let worker = null;
  let next = 0;
  let declarations;
  /** @type {Map<number, {resolve: (value: any) => void, reject: (error: Error) => void}>} */
  const pending = new Map();

  /** Ends the worker and fails every call it still holds. */
  function end(error) {
    worker?.terminate();
    worker = null;
    for (const call of pending.values()) {
      call.reject(error);
    }
    pending.clear();
  }

  function answer({ data }) {
    const call = pending.get(data.id);
    if (call === undefined) {
      return;
    }
    if ("crash" in data) {
      // Fails this call with every other the worker held.
      end(new Crash(data.crash));
      return;
    }
    pending.delete(data.id);
    if ("value" in data) {
      call.resolve(data.value);
    } else {
      call.reject(new Error(data.error));
    }
  }

  /**
   * @param {string} name An export of the bindings.
   * @param {unknown[]} args
   */
  async function call(name, ...args) {
    compiled ??= compile(wasm.module);
    let module;
    try {
      module = await compiled;
    } catch (error) {
      // The next call fetches the module again.
      compiled = null;
      throw new Crash(`the module did not load: ${error.message}`);
    }
    if (worker === null) {
      worker = new Worker(wasm.worker, { type: "module" });
      worker.onmessage = answer;
      worker.onerror = (event) =>
        end(new Crash(event.message || "the worker failed"));
      worker.postMessage({ glue: wasm.glue, module });
    }
    const id = next++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, name, args });
    });
  }

  return {
    /**
     * @returns {Promise<{flow: number, path: string} | {type: string, cfg: string, start: number} | null>}
     */
    async declaredAt(text, cfg, offset, cfgs) {
      if (
        declarations?.text !== text ||
        declarations.cfg !== cfg ||
        declarations.cfgs !== cfgs
      ) {
        const ranges = new Map([[cfg, null]]);
        for (const enabled of cfgs ?? []) ranges.set(enabled, null);
        declarations = { text, cfg, cfgs, ranges };
      }
      const { ranges } = declarations;
      for (const enabled of ranges.keys()) {
        if (!ranges.get(enabled))
          ranges.set(enabled, call("declaredRanges", text, enabled));
        let items;
        try {
          items = await ranges.get(enabled);
        } catch (error) {
          if (error instanceof Crash) ranges.set(enabled, null);
          throw error;
        }
        let low = 0;
        let high = items.length;
        while (low < high) {
          const middle = Math.floor((low + high) / 2);
          if (items[middle][0] <= offset) low = middle + 1;
          else high = middle;
        }
        const at = items[low - 1];
        if (!at || offset > at[1]) continue;
        if (at[2].type !== undefined)
          return { ...at[2], cfg: enabled, start: at[0] };
        if (enabled === cfg) return at[2];
      }
      return null;
    },
    /**
     * @returns {Promise<{svg: string, hotspots: Hotspot[], wires: Wire[]} | null>}
     */
    drawFlow: (text, cfg, position, collapseCycles, path) =>
      call("drawFlow", text, cfg, position, collapseCycles, path),
    /**
     * @returns {Promise<{svg: string, hotspots: Hotspot[], cards: Card[], otherPlaces?: Card[]} | null>}
     */
    drawType: (text, cfg, path, context) =>
      call("drawType", text, cfg, path, context),
    /** @returns {Promise<number[]>} The last identifiers of valid relationship targets. */
    relationshipTargets: (text, cfg) => call("relationshipTargets", text, cfg),
  };
}

/** Fetches and compiles the module at `url`. */
async function compile(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}`);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}
