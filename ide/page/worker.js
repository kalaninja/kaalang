// One instance of the bindings, answering the page's calls in the order they
// come. A trap leaves the instance broken, so the worker reports it and closes;
// the engine starts another.

/** The message of the last panic, which the bindings write before a trap. */
let panic = null;
const report = console.error;
console.error = (...parts) => {
  panic = parts.join(" ");
  report(...parts);
};

/** @type {Promise<Record<string, (...args: unknown[]) => string>>} */
let bindings;

self.onmessage = async ({ data }) => {
  if (data.module) {
    bindings = import(data.glue).then((glue) => {
      glue.initSync({ module: data.module });
      return glue;
    });
    return;
  }
  const { id, name, args } = data;
  panic = null;
  try {
    const glue = await bindings;
    self.postMessage({ id, value: JSON.parse(glue[name](...args)) });
  } catch (error) {
    // The bindings throw a plain `Error` for text they cannot draw; anything
    // else is a failed load or a trap, a `RuntimeError` or a `RangeError` for
    // the stack.
    if (Object.getPrototypeOf(error) === Error.prototype) {
      self.postMessage({ id, error: error.message });
    } else {
      self.postMessage({ id, crash: panic ?? String(error) });
      self.close();
    }
  }
};
