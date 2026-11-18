import { readFile } from "node:fs/promises";
import * as path from "node:path";
import { pathToFileURL } from "node:url";
import {
  type MessagePort,
  parentPort,
  Worker,
  workerData,
} from "node:worker_threads";

export interface ExternalModule {
  module: string;
  offset: number;
  files: { parts: string[]; ownsDirectory: boolean }[];
}

type Arguments = [
  text: string,
  cfg: string,
  fileStem: string,
  ownsDirectory: boolean,
];
type Items = { modules: ExternalModule[]; types: number[] } | null;

let parser: ReturnType<typeof start> | undefined;
let next = 0;

/** Parse off the extension host's event loop. A trap discards only this worker. */
export function moduleItems(...args: Arguments): Promise<Items> {
  const { worker, pending } = (parser ??= start());
  const id = ++next;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    worker.ref();
    worker.postMessage({ id, args });
  });
}

export function stopModuleWorker(): void {
  void parser?.worker.terminate();
  parser = undefined;
}

function start() {
  const worker = new Worker(__filename, { workerData: "kaalang-module-files" });
  const pending = new Map<number, (items: Items) => void>();
  const close = () => {
    if (parser?.worker === worker) parser = undefined;
    for (const resolve of pending.values()) resolve(null);
    pending.clear();
  };
  worker.on("error", close);
  worker.on("exit", close);
  worker.on("message", ({ id, items }: { id: number; items: Items }) => {
    pending.get(id)?.(items);
    pending.delete(id);
    if (!pending.size) worker.unref();
  });
  worker.unref();
  return { worker, pending };
}

async function serve(port: MessagePort): Promise<void> {
  const wasm = path.join(__dirname, "wasm");
  const bindings = await import(
    pathToFileURL(path.join(wasm, "kaalang_js.mjs")).href
  );
  bindings.initSync({
    module: await readFile(path.join(wasm, "kaalang_js_bg.wasm")),
  });
  port.on("message", ({ id, args }: { id: number; args: Arguments }) => {
    let items: Items;
    try {
      items = JSON.parse(bindings.moduleItems(...args));
    } catch (error) {
      // A parse error leaves the instance usable. A Wasm trap ends the worker.
      if (!(error instanceof Error) || error.constructor !== Error) throw error;
      items = null;
    }
    port.postMessage({ id, items });
  });
}

if (parentPort && workerData === "kaalang-module-files") {
  void serve(parentPort).catch(() => parentPort?.close());
}
