// Workspace source files and the places of their declarations in Cargo targets.

import { realpath, stat } from "node:fs/promises";
import * as path from "node:path";
import type * as vscode from "vscode";
import { fileURLToPath, pathToFileURL } from "node:url";

import { cached, fileStamp, realFile } from "./cache";
import { type ExternalModule, moduleItems } from "./module-sources";
import type { CargoRoot } from "./target";

/** Where a file stands in a Cargo target's module tree. */
export interface Placement {
  package: string;
  group: number;
  crate: string;
  /** The Cargo target's own source path, including its final symlink. */
  root: string;
  module: string;
  /** Offsets at successive file boundaries, then the type name for a card. */
  order: number[];
}

export interface Reading {
  place: Placement;
  cfg: string;
  types: Set<number> | null;
}

export interface Graph {
  /** The places in which each physical file was read as a module. */
  files: Map<string, Reading[]>;
  /** Paths consulted, including a missing sibling that would make a module ambiguous. */
  paths: Set<string>;
  /** Only these workspace sources need a search for relationship targets. */
  referring: Map<string, string>;
  /** A file that failed to parse or read could declare more modules. */
  complete: boolean;
}

interface Parsed {
  modules: ExternalModule[];
  types: Set<number>;
}

/** Every relationship annotation contains this Rust identifier. */
export const MAY_REFER = /\brefer\b/;

/** Resolve a missing candidate against its nearest existing parent, including directory symlinks. */
function directoryPath(
  directory: string,
  parents: Map<string, Promise<string>>,
): Promise<string> {
  return cached(parents, directory, async () => {
    try {
      return await realpath(directory);
    } catch {
      const parent = path.dirname(directory);
      if (parent === directory) return directory;
      return path.join(
        await directoryPath(parent, parents),
        path.basename(directory),
      );
    }
  });
}

/** File identities and configured declarations shared between successive workspace snapshots. */
class ModuleCache {
  paths = new Map<string, Promise<string | null>>();
  stamps = new Map<string, string>();
  sources = new Map<
    string,
    {
      items: Map<string, Promise<Parsed | null>>;
      /** The text, kept only when it may contain a relationship. */
      referring?: string;
    }
  >();
  physical = new Map<string, string>();
  private aliases = new Map<string, Set<string>>();

  resolve(
    file: string,
    parents: Map<string, Promise<string>>,
  ): Promise<string | null> {
    const pending: Promise<string | null> = cached(
      this.paths,
      file,
      async () => {
        let actual: string | null = null;
        let stamp = "";
        try {
          const info = await stat(file);
          stamp = fileStamp(info);
          if (info.isFile()) actual = await realpath(file);
        } catch (error) {
          const { code } = error as NodeJS.ErrnoException;
          if (code !== "ENOENT" && code !== "ENOTDIR") {
            if (this.paths.get(file) === pending) this.paths.delete(file);
            throw error;
          }
        }
        const key =
          actual ??
          path.join(
            await directoryPath(path.dirname(file), parents),
            path.basename(file),
          );
        if (this.paths.get(file) === pending) {
          this.stamps.set(file, stamp);
          this.stamps.set(key, stamp);
          this.physical.set(file, key);
          const aliases = this.aliases.get(key) ?? new Set();
          aliases.add(file);
          this.aliases.set(key, aliases);
        }
        return actual;
      },
    );
    return pending;
  }

  prune(graph: Graph) {
    for (const file of this.paths.keys()) {
      if (graph.paths.has(file)) continue;
      this.paths.delete(file);
      const key = this.physical.get(file) ?? path.normalize(file);
      const aliases = this.aliases.get(key);
      aliases?.delete(file);
      if (!aliases?.size) this.aliases.delete(key);
      this.physical.delete(file);
    }
    for (const file of this.stamps.keys()) {
      if (!graph.paths.has(file) && !graph.files.has(file))
        this.stamps.delete(file);
    }
    for (const file of this.sources.keys()) {
      if (!graph.files.has(file)) this.sources.delete(file);
    }
  }

  forget(file: string, physical: string) {
    for (const name of [file, physical, this.physical.get(file)]) {
      if (!name) continue;
      this.paths.delete(name);
      this.stamps.delete(name);
      this.sources.delete(name);
      for (const alias of this.aliases.get(name) ?? []) {
        this.paths.delete(alias);
        this.stamps.delete(alias);
        this.physical.delete(alias);
      }
      this.aliases.delete(name);
    }
  }
}

const caches = new Map<CargoRoot[], ModuleCache>();
function moduleCache(roots: CargoRoot[]) {
  const cache = caches.get(roots) ?? new ModuleCache();
  caches.set(roots, cache);
  return cache;
}

interface Visit {
  files: { file: string; ownsDirectory: boolean }[];
  ancestors: Set<string>;
  place: Placement;
  cfg: string;
}

/** Keep `..` for the filesystem: the preceding directory may be a symlink. */
function moduleFile(parent: string, parts: string[]): string {
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

/** Every way the compiler reaches a physical file, including across shared and symlinked modules. */
export async function moduleTree(
  roots: CargoRoot[],
  read: (uri: string) => Promise<string>,
  parse: typeof moduleItems = moduleItems,
  isCurrent: () => boolean = () => true,
): Promise<Graph> {
  const cache = moduleCache(roots);
  const sources = new Map<string, Promise<string>>();
  const parents = new Map<string, Promise<string>>();
  const graph: Graph = {
    files: new Map(),
    paths: new Set(),
    referring: new Map(),
    complete: true,
  };

  async function visit({
    files,
    ancestors,
    place,
    cfg,
  }: Visit): Promise<Visit[]> {
    for (const { file } of files) graph.paths.add(file);
    const candidates = await Promise.all(
      files.map(async (choice) => {
        const actual = await cache.resolve(choice.file, parents);
        const physical = cache.physical.get(choice.file);
        if (physical) graph.paths.add(physical);
        return { ...choice, actual };
      }),
    );
    if (!isCurrent()) return [];
    const existing = candidates.filter((choice) => choice.actual !== null);
    if (existing.length > 1) {
      // rustc rejects both foo.rs and foo/mod.rs for the same declaration.
      graph.complete = false;
      return [];
    }
    const choice = existing[0];
    if (choice?.actual) {
      const { actual, file, ownsDirectory } = choice;
      if (ancestors.has(actual)) return [];
      const found = graph.files.get(actual) ?? [];
      const current: Reading = {
        place,
        cfg,
        types: null,
      };
      found.push(current);
      graph.files.set(actual, found);
      const stem = path.parse(file).name;
      const key = JSON.stringify([cfg, stem, ownsDirectory]);
      const source = cache.sources.get(actual) ?? {
        items: new Map<string, Promise<Parsed | null>>(),
      };
      cache.sources.set(actual, source);
      const parsed = source.items;
      const pending = cached(parsed, key, async () => {
        // A file URI normalizes dot segments before the editor reads it.
        const readable = path.normalize(file) === file ? file : actual;
        const text = await cached(sources, actual, () =>
          read(pathToFileURL(readable).href),
        );
        source.referring = MAY_REFER.test(text) ? text : undefined;
        const found = await parse(text, cfg, stem, ownsDirectory);
        return found ? { ...found, types: new Set(found.types) } : null;
      });
      const items = await pending.catch(() => null);
      if (source.referring !== undefined)
        graph.referring.set(pathToFileURL(actual).href, source.referring);
      if (!items) {
        if (parsed.get(key) === pending) parsed.delete(key);
        graph.complete = false;
        return [];
      }
      current.types = items.types;
      const seen = new Set(ancestors).add(actual);
      return items.modules.map((child) => ({
        files: child.files.map(({ parts, ownsDirectory }) => ({
          file: moduleFile(file, parts),
          ownsDirectory,
        })),
        ancestors: seen,
        cfg,
        place: {
          ...place,
          module: [place.module, child.module].filter(Boolean).join("::"),
          order: [...place.order, child.offset],
        },
      }));
    }
    // A declared module may have no source on disk, for example during editing.
    return [];
  }

  let pending: Visit[] = roots.map((target) => {
    const file = fileURLToPath(target.root);
    return {
      files: [{ file, ownsDirectory: true }],
      ancestors: new Set(),
      cfg: target.cfg,
      place: {
        package: target.package,
        group: target.group,
        crate: target.name,
        root: file.split(path.sep).join("/"),
        module: "",
        order: [],
      },
    };
  });
  while (pending.length) {
    const next: Visit[] = [];
    // Limit outstanding reads and worker messages on a broad module tree.
    for (let i = 0; i < pending.length; i += 8) {
      if (!isCurrent())
        return {
          files: new Map(),
          paths: new Set(),
          referring: new Map(),
          complete: false,
        };
      const children = await Promise.all(
        pending.slice(i, i + 8).map((item) =>
          visit(item).catch(() => {
            graph.complete = false;
            return [];
          }),
        ),
      );
      for (const files of children) next.push(...files);
    }
    pending = next;
  }
  if (isCurrent() && graph.complete) cache.prune(graph);
  return graph;
}

let trees = new WeakMap<CargoRoot[], Promise<Graph>>();

/** Edits or Cargo configuration changes invalidate the cached workspace graphs. */
export function forgetModules(file?: string): void {
  trees = new WeakMap();
  if (file === undefined) {
    watches?.clear();
    caches.clear();
    return;
  }
  if (!caches.size) return;
  // A dirty editor can name a symlink even when Cargo knows only its target.
  const physical = realFile(file);
  for (const cache of caches.values()) cache.forget(file, physical);
}

/** Explicit #[path] modules can have an extension VS Code opens as plain text. */
export function knownModule(uri: vscode.Uri): boolean {
  if (uri.scheme !== "file") return false;
  for (const cache of caches.values()) {
    if (cache.paths.has(uri.fsPath) || cache.sources.has(uri.fsPath))
      return true;
  }
  return false;
}

export function editorTree(roots: CargoRoot[]): Promise<Graph> | undefined {
  if (!roots.length) return undefined;
  const vscode: typeof import("vscode") = require("vscode");
  if (!vscode.workspace.isTrusted) return undefined;
  const cache = trees;
  const existing = cache.get(roots);
  if (existing) return existing;
  const { documentText }: typeof import("./documents") = require("./documents");
  const pending = moduleTree(
    roots,
    (uri) => documentText(vscode.Uri.parse(uri)),
    moduleItems,
    () => trees === cache,
  ).then(
    (graph) => {
      if (trees === cache)
        watches?.update(roots, graph.paths, moduleCache(roots).stamps);
      if (!graph.complete) cache.delete(roots);
      return graph;
    },
    (error) => {
      cache.delete(roots);
      throw error;
    },
  );
  cache.set(roots, pending);
  return pending;
}

let watches:
  | ReturnType<typeof import("./module-watches").moduleWatches>
  | undefined;

/** Tie extra file watches to the lifetime of the extension. */
export function watchModuleSources(
  changed: (file: string) => void,
): vscode.Disposable {
  const {
    moduleWatches,
  }: typeof import("./module-watches") = require("./module-watches");
  watches = moduleWatches(changed);
  return {
    dispose() {
      watches?.clear();
      watches = undefined;
    },
  };
}

/** Declaration order at successive external module boundaries. A parent precedes its children. */
function sourceOrder(a: readonly number[], b: readonly number[]): number {
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return a.length - b.length;
}

/** RFC 0009 §4.3 order: package, target group, crate name and root, then source order. */
export function placeOrder(a: Placement, b: Placement): number {
  const text = (x: string, y: string) => (x < y ? -1 : x > y ? 1 : 0);
  return (
    text(a.package, b.package) ||
    a.group - b.group ||
    text(a.crate, b.crate) ||
    text(a.root, b.root) ||
    sourceOrder(a.order, b.order)
  );
}

/**
 * Every Cargo reading of a file, in diagram order: an empty list outside every
 * workspace target, `null` when the module tree cannot tell. `typeOffset`
 * keeps the readings that declare a type there.
 */
export async function fileReadings(
  uri: string,
  roots: CargoRoot[],
  typeOffset?: number,
): Promise<Reading[] | null> {
  const graph = await editorTree(roots);
  if (!graph) return null;
  const actual =
    uri.startsWith("file:") &&
    (await realpath(fileURLToPath(uri)).catch(() => undefined));
  const readings = actual ? graph.files.get(actual) : undefined;
  const active =
    typeOffset === undefined
      ? readings
      : readings?.filter((reading) => reading.types?.has(typeOffset));
  if (!active?.length) return graph.complete ? [] : null;
  return [...active].sort((a, b) => placeOrder(a.place, b.place));
}
