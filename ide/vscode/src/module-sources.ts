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

type Items = {
  modules: ExternalModule[];
  types: number[];
} | null;

/** The bindings this worker calls, each answering JSON. */
type Name = "moduleItems" | "relationshipTargets";

let parser: ReturnType<typeof start> | undefined;
let next = 0;

/** Parse off the extension host's event loop. A trap discards only this worker. */
function call(name: Name, args: unknown[]): Promise<unknown> {
  const { worker, pending } = (parser ??= start());
  const id = ++next;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    worker.ref();
    worker.postMessage({ id, name, args });
  });
}

/** The external modules and type names a file declares for `cfg`, or `null` when it does not parse. */
export function moduleItems(
  text: string,
  cfg: string,
  fileStem: string,
  ownsDirectory: boolean,
): Promise<Items> {
  return call("moduleItems", [
    text,
    cfg,
    fileStem,
    ownsDirectory,
  ]) as Promise<Items>;
}

/** The final identifier of each active relationship target, or `null` when the file does not parse. */
export function relationshipTargets(
  text: string,
  cfg: string,
): Promise<number[] | null> {
  return call("relationshipTargets", [text, cfg]) as Promise<number[] | null>;
}

export function stopModuleWorker(): void {
  void parser?.worker.terminate();
  parser = undefined;
}

function start() {
  const worker = new Worker(__filename, { workerData: "kaalang-module-files" });
  const pending = new Map<number, (answer: unknown) => void>();
  const close = () => {
    if (parser?.worker === worker) parser = undefined;
    for (const resolve of pending.values()) resolve(null);
    pending.clear();
  };
  worker.on("error", close);
  worker.on("exit", close);
  worker.on("message", ({ id, answer }: { id: number; answer: unknown }) => {
    pending.get(id)?.(answer);
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
  port.on("message", (job: { id: number; name: Name; args: unknown[] }) => {
    let answer: unknown;
    try {
      answer = JSON.parse(bindings[job.name](...job.args));
    } catch (error) {
      // A parse error leaves the instance usable. A Wasm trap ends the worker.
      if (!(error instanceof Error) || error.constructor !== Error) throw error;
      answer = null;
    }
    port.postMessage({ id: job.id, answer });
  });
}

if (parentPort && workerData === "kaalang-module-files") {
  void serve(parentPort).catch(() => parentPort?.close());
}
