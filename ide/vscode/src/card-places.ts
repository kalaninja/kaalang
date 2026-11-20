// Workspace source files and the places of their declarations in Cargo
// targets, as the bindings walk the targets' module trees in the module worker.

import { fileURLToPath } from "node:url";
import type * as vscode from "vscode";

import { realFile } from "./cache";
import { forgetTrees, moduleTree } from "./module-sources";
import type { CargoRoot } from "./target";

/** Where a file stands in a Cargo target's module tree, as the bindings give it. */
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
  /** The places in which each physical file was read as a module, in diagram order. */
  files: Map<string, Reading[]>;
  /** Paths consulted, including a missing sibling that would make a module ambiguous, with their stamps. */
  paths: Map<string, string>;
  /** The files that may hold a relationship. */
  referring: string[];
  /** A file that failed to parse or read could declare more modules. */
  complete: boolean;
}

let trees = new WeakMap<CargoRoot[], Promise<Graph>>();
/** Every path a tree consulted or read since everything was last forgotten. */
const known = new Set<string>();

/** Edits or Cargo configuration changes invalidate the cached workspace graphs. */
export function forgetModules(file?: string): void {
  trees = new WeakMap();
  if (file === undefined) {
    watches?.clear();
    known.clear();
  }
  forgetTrees(file);
}

/** Explicit #[path] modules can have an extension VS Code opens as plain text. */
export function knownModule(uri: vscode.Uri): boolean {
  return uri.scheme === "file" && known.has(uri.fsPath);
}

/** The module trees of `roots`, read with the `unsaved` texts by real path. */
export async function workspaceTree(
  roots: CargoRoot[],
  unsaved: [string, string][],
): Promise<Graph> {
  const tree = await moduleTree(
    roots.map((target) => ({ ...target, root: fileURLToPath(target.root) })),
    unsaved,
  );
  if (!tree)
    return {
      files: new Map(),
      paths: new Map(),
      referring: [],
      complete: false,
    };
  const readings = Object.entries(tree.files).map(([file, places]) => {
    const found = places.map(({ target, place, types }) => ({
      place,
      cfg: roots[target].cfg,
      types: types && new Set(types),
    }));
    return [file, found] as const;
  });
  return {
    files: new Map(readings),
    paths: new Map(tree.stamps),
    referring: tree.referring,
    complete: tree.complete,
  };
}

export function editorTree(roots: CargoRoot[]): Promise<Graph> | undefined {
  if (!roots.length) return undefined;
  const vscode: typeof import("vscode") = require("vscode");
  if (!vscode.workspace.isTrusted) return undefined;
  const cache = trees;
  const existing = cache.get(roots);
  if (existing) return existing;
  // Cargo and the server name the real file while an editor may hold a symlink.
  const unsaved = vscode.workspace.textDocuments
    .filter((document) => document.isDirty && document.uri.scheme === "file")
    .map((document): [string, string] => [
      realFile(document.uri.fsPath),
      document.getText(),
    ]);
  const pending = workspaceTree(roots, unsaved).then((graph) => {
    if (trees === cache) {
      watches?.update(roots, new Set(graph.paths.keys()), graph.paths);
      for (const file of [...graph.paths.keys(), ...graph.files.keys()])
        known.add(file);
    }
    if (!graph.complete) cache.delete(roots);
    return graph;
  });
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
  const actual = uri.startsWith("file:") && realFile(fileURLToPath(uri));
  const readings = actual ? graph.files.get(actual) : undefined;
  const active =
    typeOffset === undefined
      ? readings
      : readings?.filter((reading) => reading.types?.has(typeOffset));
  if (!active?.length) return graph.complete ? [] : null;
  return active;
}
