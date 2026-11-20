import { readFileSync, statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import * as path from "node:path";
import { pathToFileURL } from "node:url";
import {
  type MessagePort,
  parentPort,
  Worker,
  workerData,
} from "node:worker_threads";

import { fileStamp, realFile, realPath } from "./cache";
import type { Placement } from "./card-places";

/** A Cargo target, as `cargo metadata` lists it, whose module tree the bindings read from the file `root`. */
export interface TreeRoot {
  package: string;
  kind: string[];
  name: string;
  root: string;
  cfg: string;
}

/**
 * The bindings' module trees, with each path the walk consulted, a missing
 * candidate's physical path included, and its stamp when it was consulted.
 */
export interface Tree {
  files: Record<
    string,
    { target: number; place: Placement; types: number[] | null }[]
  >;
  referring: string[];
  complete: boolean;
  stamps: [string, string][];
}

/** The jobs this worker runs. */
type Name = "moduleTree" | "forget" | "relationshipTargets";

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

/**
 * The module trees of `roots`, reading the `unsaved` texts, by real path,
 * before the disk; `null` when the worker failed.
 */
export function moduleTree(
  roots: TreeRoot[],
  unsaved: [string, string][],
): Promise<Tree | null> {
  // A string crosses threads faster than the objects it describes.
  return (call("moduleTree", [roots, unsaved]) as Promise<string | null>).then(
    (tree) => tree && JSON.parse(tree),
  );
}

/** Forgets what the trees read of `file`, or of every file. */
export function forgetTrees(file?: string): void {
  if (parser) void call("forget", [file]);
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

/** Keep `..` for the filesystem: the preceding directory may be a symlink. */
export function moduleFile(parent: string, parts: string[]): string {
  let file = path.dirname(parent);
  for (const part of parts) {
    const root = path.parse(part).root;
    if (root) file = path.resolve(file, root);
    const tail = part
      .slice(root.length)
      .replace(path.sep === "\\" ? /[\\/]+/g : /\/+/g, path.sep);
    if (tail) file += (file.endsWith(path.sep) ? "" : path.sep) + tail;
  }
  return file;
}

/** Resolve a missing candidate against its nearest existing parent, including directory symlinks. */
function directoryPath(directory: string): string {
  try {
    return realPath(directory);
  } catch {
    const parent = path.dirname(directory);
    if (parent === directory) return directory;
    return path.join(directoryPath(parent), path.basename(directory));
  }
}

/** What a path consulted holds: a file's real path, and the physical path to watch. */
interface Found {
  canonical?: string;
  physical: string;
  stamp: string;
}

/**
 * The file system as the bindings' walk asks it. What a path holds is kept
 * until a change forgets it; each path consulted is recorded with its stamp.
 */
export function treeHost(
  found: Map<string, Found>,
  unsaved: Map<string, string>,
  consulted: Map<string, string>,
) {
  return {
    join: moduleFile,
    canonical(file: string): string | undefined {
      let entry = found.get(file);
      if (!entry) {
        let canonical: string | undefined;
        let stamp = "";
        try {
          const info = statSync(file);
          stamp = fileStamp(info);
          if (info.isFile()) canonical = realPath(file);
        } catch (error) {
          const { code } = error as NodeJS.ErrnoException;
          if (code !== "ENOENT" && code !== "ENOTDIR") throw error;
        }
        const physical =
          canonical ??
          path.join(directoryPath(path.dirname(file)), path.basename(file));
        entry = { canonical, physical, stamp };
        found.set(file, entry);
      }
      consulted.set(file, entry.stamp);
      consulted.set(entry.physical, entry.stamp);
      return entry.canonical;
    },
    read: (file: string) => unsaved.get(file) ?? readFileSync(file, "utf8"),
  };
}

/** Forgets what `found` holds of `file` under any spelling, or everything; `names` are its physical paths. */
export function forgetFound(
  found: Map<string, Found>,
  file?: string,
): string[] {
  if (file === undefined) {
    found.clear();
    return [];
  }
  const names = new Set([file, realFile(file)]);
  const physical = found.get(file)?.physical;
  if (physical) names.add(physical);
  for (const [key, entry] of found) {
    if (names.has(key) || names.has(entry.physical)) found.delete(key);
  }
  return [...names];
}

async function serve(port: MessagePort): Promise<void> {
  const wasm = path.join(__dirname, "wasm");
  const bindings = await import(
    pathToFileURL(path.join(wasm, "kaalang_js.mjs")).href
  );
  bindings.initSync({
    module: await readFile(path.join(wasm, "kaalang_js_bg.wasm")),
  });
  const found = new Map<string, Found>();
  const jobs = {
    moduleTree(roots: TreeRoot[], unsaved: [string, string][]): string {
      const consulted = new Map<string, string>();
      const host = treeHost(found, new Map(unsaved), consulted);
      const tree = bindings.moduleTree(JSON.stringify(roots), host);
      return `${tree.slice(0, -1)},"stamps":${JSON.stringify([...consulted])}}`;
    },
    forget(file?: string): null {
      const names = forgetFound(found, file);
      if (file === undefined) bindings.forgetModules();
      for (const name of names) bindings.forgetModules(name);
      return null;
    },
    relationshipTargets: (text: string, cfg: string): number[] =>
      JSON.parse(bindings.relationshipTargets(text, cfg)),
  };
  port.on("message", (job: { id: number; name: Name; args: unknown[] }) => {
    let answer: unknown;
    try {
      answer = (jobs[job.name] as (...args: unknown[]) => unknown)(...job.args);
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
