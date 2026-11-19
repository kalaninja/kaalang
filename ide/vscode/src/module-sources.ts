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
export type Body = [number, number];
type Items = {
  modules: ExternalModule[];
  types: number[];
  bodies: Body[];
} | null;

export interface WorkspaceSources {
  files: { uri: string; text: string | null }[];
  crates: {
    packageId: string;
    name: string;
    library: boolean;
    host: boolean;
    cfg: string;
    dependencies: [string, string][];
    modules: { path: string; file: number }[];
  }[];
}

export interface Owner {
  owner: number;
  module: string;
}

export interface Matches extends Owner {
  uri: string;
  cfg: string;
  offsets: number[];
  unknown?: number[];
  focus?: boolean;
  repeated?: boolean;
}

type Job =
  | { op: "items"; args: Arguments }
  | {
      op: "references";
      snapshot?: WorkspaceSources;
      focus: string;
      offset: number;
      candidates: string[];
      place?: Owner;
    };
type Result = Items | Matches[];

let parser: ReturnType<typeof start> | undefined;
let next = 0;

function request(job: Job): Promise<Result> {
  const { worker, pending } = (parser ??= start());
  const id = ++next;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    worker.ref();
    worker.postMessage({ id, ...job });
  });
}

/** Parse off the extension host's event loop. A trap discards only this worker. */
export function moduleItems(...args: Arguments): Promise<Items> {
  return request({ op: "items", args }) as Promise<Items>;
}

/** Transfer a workspace snapshot once for every sequence of queries against it. */
export function workspaceReferences(
  snapshot: WorkspaceSources,
  focus: string,
  offset: number,
  candidates: string[],
  place?: Owner,
) {
  const state = (parser ??= start());
  const send = state.snapshot !== snapshot;
  state.snapshot = snapshot;
  return request({
    op: "references",
    snapshot: send ? snapshot : undefined,
    focus,
    offset,
    candidates,
    place,
  }) as Promise<Matches[] | null>;
}

export function stopModuleWorker(): void {
  void parser?.worker.terminate();
  parser = undefined;
}

function start() {
  const worker = new Worker(__filename, { workerData: "kaalang-module-files" });
  const pending = new Map<number, (items: Result) => void>();
  const close = () => {
    if (parser?.worker === worker) parser = undefined;
    for (const resolve of pending.values()) resolve(null);
    pending.clear();
  };
  worker.on("error", close);
  worker.on("exit", close);
  worker.on("message", ({ id, items }: { id: number; items: Result }) => {
    pending.get(id)?.(items);
    pending.delete(id);
    if (!pending.size) worker.unref();
  });
  worker.unref();
  return {
    worker,
    pending,
    snapshot: undefined as WorkspaceSources | undefined,
  };
}

async function serve(port: MessagePort): Promise<void> {
  const wasm = path.join(__dirname, "wasm");
  const bindings = await import(
    pathToFileURL(path.join(wasm, "kaalang_js.mjs")).href
  );
  bindings.initSync({
    module: await readFile(path.join(wasm, "kaalang_js_bg.wasm")),
  });
  let workspace:
    | {
        free(): void;
        references(
          focus: string,
          offset: number,
          candidates: string,
          place?: string,
        ): string;
      }
    | undefined;
  port.on("message", (job: { id: number } & Job) => {
    let items: Result;
    try {
      if (job.op === "items")
        items = JSON.parse(bindings.moduleItems(...job.args));
      else {
        if (job.snapshot) {
          workspace?.free();
          workspace = undefined;
          workspace = new bindings.Workspace(JSON.stringify(job.snapshot));
        }
        items = workspace
          ? JSON.parse(
              workspace.references(
                job.focus,
                job.offset,
                JSON.stringify(job.candidates),
                job.place && JSON.stringify(job.place),
              ),
            )
          : null;
      }
    } catch (error) {
      // A parse error leaves the instance usable. A Wasm trap ends the worker.
      if (!(error instanceof Error) || error.constructor !== Error) throw error;
      items = null;
    }
    port.postMessage({ id: job.id, items });
  });
}

if (parentPort && workerData === "kaalang-module-files") {
  void serve(parentPort).catch(() => parentPort?.close());
}
