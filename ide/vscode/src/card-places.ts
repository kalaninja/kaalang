// Workspace source files and the places of their declarations in Cargo targets.

import { createHash } from "node:crypto";
import { realpath, stat } from "node:fs/promises";
import * as path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { type ExternalModule, moduleItems } from "./module-sources";
import type { CargoRoot } from "./target";

interface Place {
  uri: string;
  offset: number;
  /** The type's path within its source. Absent when asking about a file alone. */
  path?: string;
}

interface Placement {
  package: string;
  group: number;
  crate: string;
  /** The Cargo target's own source path, including its final symlink. */
  root: string;
  module: string;
  /** Offsets at successive file boundaries, then the type name for a card. */
  order: number[];
}

interface Graph {
  /** The places in which each physical file was read as a module. */
  files: Map<string, { place: Placement; types: Set<number> | null }[]>;
  /** A file that failed to parse or read could declare more modules. */
  complete: boolean;
}

interface Parsed {
  modules: ExternalModule[];
  types: Set<number>;
}

interface CachedItems {
  hash: string;
  items: Parsed;
}

/** Parsed declarations survive an edit while their file still has the same contents and cfg. */
const parsed = new WeakMap<CargoRoot[], Map<string, CachedItems>>();

/** Pending and completed work is shared even by visits to different Cargo roots. */
function cached<T>(
  values: Map<string, Promise<T>>,
  key: string,
  read: () => Promise<T>,
): Promise<T> {
  const existing = values.get(key);
  if (existing) return existing;
  const pending = read();
  values.set(key, pending);
  return pending;
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
    const tail = part.slice(root.length);
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
  const physical = new Map<string, Promise<string | null>>();
  const sources = new Map<string, Promise<{ text: string; hash: string }>>();
  const modules = new Map<string, Promise<Parsed | null>>();
  const previous = parsed.get(roots) ?? new Map<string, CachedItems>();
  parsed.set(roots, previous);
  const graph: Graph = { files: new Map(), complete: true };

  async function visit({
    files,
    ancestors,
    place,
    cfg,
  }: Visit): Promise<Visit[]> {
    for (const { file, ownsDirectory } of files) {
      const actual = await cached(physical, file, async () => {
        try {
          return (await stat(file)).isFile() ? await realpath(file) : null;
        } catch (error) {
          const { code } = error as NodeJS.ErrnoException;
          if (code === "ENOENT" || code === "ENOTDIR") return null;
          throw error;
        }
      });
      if (actual === null) continue;
      if (ancestors.has(actual)) return [];
      const found = graph.files.get(actual) ?? [];
      const current: { place: Placement; types: Set<number> | null } = {
        place,
        types: null,
      };
      found.push(current);
      graph.files.set(actual, found);
      const stem = path.parse(file).name;
      const key = JSON.stringify([actual, cfg, stem, ownsDirectory]);
      const items = await cached(modules, key, async () => {
        // A file URI normalizes dot segments before the editor reads it.
        const readable = path.normalize(file) === file ? file : actual;
        const { text, hash } = await cached(sources, actual, async () => {
          const text = await read(pathToFileURL(readable).href);
          return {
            text,
            hash: createHash("sha256").update(text, "utf16le").digest("hex"),
          };
        });
        const known = previous.get(key);
        if (known?.hash === hash) return known.items;
        const found = await parse(text, cfg, stem, ownsDirectory);
        if (!found) return null;
        const items = { modules: found.modules, types: new Set(found.types) };
        if (isCurrent()) previous.set(key, { hash, items });
        return items;
      });
      if (!items) {
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
      if (!isCurrent()) return { files: new Map(), complete: false };
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
  if (graph.complete && isCurrent()) {
    for (const key of previous.keys()) {
      if (!modules.has(key)) previous.delete(key);
    }
  }
  return graph;
}

/** The declaration's inline path and offset extend each of its file's places. */
export async function placements(
  graph: Graph,
  cards: Place[],
): Promise<(Placement[] | null)[]> {
  const files = new Map<string, Promise<string>>();
  return Promise.all(
    cards.map(async ({ uri, path, offset }) => {
      try {
        const actual = await cached(files, uri, () =>
          realpath(fileURLToPath(uri)),
        );
        const positions = graph.files.get(actual);
        const active = positions?.filter(
          ({ types }) => path === undefined || types?.has(offset),
        );
        if (!active?.length) return graph.complete ? [] : null;
        const inline = path?.split("::").slice(0, -1).join("::");
        return active.map(({ place: position }) => ({
          ...position,
          module: [position.module, inline].filter(Boolean).join("::"),
          order: [...position.order, offset],
        }));
      } catch {
        return null;
      }
    }),
  );
}

let trees = new WeakMap<CargoRoot[], Promise<Graph>>();

/** Edits or Cargo configuration changes invalidate the cached workspace graphs. */
export function forgetModules(): void {
  trees = new WeakMap();
}

function editorTree(roots: CargoRoot[]): Promise<Graph> | undefined {
  const vscode: typeof import("vscode") = require("vscode");
  if (!vscode.workspace.isTrusted || !roots.length) return undefined;
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

/** A declaration's places, an empty list outside the workspace, or null when unknown. */
export async function cardPlacements(places: Place[], roots: CargoRoot[]) {
  const graph = await editorTree(roots);
  return graph ? placements(graph, places) : places.map(() => null);
}

/** The workspace sources, with any uncertainty from files still being edited. */
export async function moduleFiles(roots: CargoRoot[]) {
  const graph = await editorTree(roots);
  return graph
    ? {
        uris: [...graph.files.keys()].map((file) => pathToFileURL(file).href),
        complete: graph.complete,
      }
    : null;
}
