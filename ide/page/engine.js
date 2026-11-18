// The page's side of the bindings: calls into a worker, started again after a
// trap from the module compiled once.

/** The module trapped; the call's answer is lost, the next call starts anew. */
export class Crash extends Error {}

/**
 * @typedef {object} Hotspot
 * @property {'node' | 'call' | 'wire' | 'connection' | 'merge'} kind
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
  /** Started at once, so that compiling overlaps the first pause. */
  let compiled = compile(wasm.module);
  /** @type {Worker | null} */
  let worker = null;
  let next = 0;
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
     * @returns {Promise<{flow: number} | {type: string} | null>}
     */
    declaredAt: (text, cfg, offset) => call("declaredAt", text, cfg, offset),
    /**
     * @returns {Promise<{svg: string, hotspots: Hotspot[], wires: Wire[]} | null>}
     */
    drawFlow: (text, cfg, position, collapseCycles) =>
      call("drawFlow", text, cfg, position, collapseCycles),
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
