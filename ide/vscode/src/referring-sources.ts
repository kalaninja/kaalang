// Select workspace relationship candidates and resolve alternate compilations of a source.
import { realpath } from "node:fs/promises";
import * as path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  type Graph,
  type Reading,
  editorTree,
  sourceOrder,
} from "./card-places";
import {
  type Body,
  type Matches,
  type Owner,
  type WorkspaceSources,
  workspaceReferences,
} from "./module-sources";
import type { CargoRoot } from "./target";

function snapshot(graph: Graph, roots: CargoRoot[]): WorkspaceSources {
  const files: WorkspaceSources["files"] = [];
  const crates: WorkspaceSources["crates"] = roots.map(
    ({ packageId, name, library, semanticOwner, host, cfg, dependencies }) => ({
      packageId,
      name,
      library: library && semanticOwner === undefined,
      host: !!host,
      cfg,
      dependencies,
      modules: [],
    }),
  );
  const modules = crates.map(
    () => [] as { path: string; file: number; order: number[] }[],
  );
  for (const [path, readings] of graph.files) {
    const file = files.length;
    files.push({
      uri: pathToFileURL(path).href,
      text: graph.texts.get(path) ?? null,
    });
    for (const { rank, place } of readings)
      if (roots[rank].semanticOwner === undefined)
        modules[rank].push({ path: place.module, file, order: place.order });
  }
  // The root precedes its children even when another target discovered a shared file first.
  for (const [rank, readings] of modules.entries()) {
    readings.sort((a, b) => sourceOrder(a.order, b.order));
    crates[rank].modules = readings.map(({ path, file }) => ({ path, file }));
  }
  return { files, crates };
}

/** Equal declarations and imports, ignoring the interiors the Rust resolver never reads. */
function sameOutsideBodies(
  a: string,
  previous: Body[],
  b: string,
  current: Body[],
): boolean {
  if (previous.length !== current.length) return false;
  let left = 0,
    right = 0;
  for (let i = 0; i < previous.length; i++) {
    if (a.slice(left, previous[i][0]) !== b.slice(right, current[i][0]))
      return false;
    left = previous[i][1];
    right = current[i][1];
  }
  return a.slice(left) === b.slice(right);
}

/** Parallel file reads can discover the same Cargo mounts in a different order. */
function byOwner(readings: Reading[]): Reading[] {
  return readings.length < 2
    ? readings
    : [...readings].sort(
        (a, b) =>
          a.rank - b.rank || a.place.module.localeCompare(b.place.module),
      );
}

function sameWorkspace(a: Graph, b: Graph): boolean {
  if (!a.complete || !b.complete || a.files.size !== b.files.size) return false;
  for (const [file, before] of a.files) {
    const after = b.files.get(file);
    if (!after || before.length !== after.length) return false;
    const left = a.texts.get(file),
      right = b.texts.get(file);
    if (left == null || right == null) return false;
    const oldReadings = byOwner(before),
      newReadings = byOwner(after);
    for (let i = 0; i < oldReadings.length; i++) {
      const previous = oldReadings[i],
        current = newReadings[i];
      if (
        previous.rank !== current.rank ||
        previous.place.module !== current.place.module ||
        previous.cfg !== current.cfg
      )
        return false;
      if (left === right) continue;
      if (
        !previous.types ||
        !current.types ||
        previous.types.size !== current.types.size
      )
        return false;
      for (const offset of previous.types)
        if (!current.types.has(offset)) return false;
      if (
        !previous.bodies ||
        !current.bodies ||
        !sameOutsideBodies(left, previous.bodies, right, current.bodies)
      )
        return false;
    }
  }
  return true;
}

interface Snapshot {
  graph: Graph;
  sources: WorkspaceSources;
  answer?: { key: string; pending: ReturnType<typeof workspaceReferences> };
}

const snapshots = new WeakMap<CargoRoot[], Snapshot>();

function workspace(roots: CargoRoot[], graph: Graph) {
  const previous = snapshots.get(roots);
  if (previous?.graph === graph) return previous;
  const sources =
    previous && sameWorkspace(previous.graph, graph)
      ? previous.sources
      : snapshot(graph, roots);
  const current: Snapshot = { graph, sources };
  snapshots.set(roots, current);
  return current;
}

/** Workspaces with a shared or detached source need each owner's name resolution. */
export async function referringSources(
  roots: CargoRoot[],
  all: boolean,
  focus: string,
  offset: number,
  place?: Owner,
) {
  const graph = await editorTree(roots);
  const candidates = new Map<
    string,
    { text: string; cfgs: string[]; matches?: Matches[] }
  >();
  if (!graph?.referring.size) return candidates;
  const actual = focus.startsWith("file:")
    ? await realpath(fileURLToPath(focus)).catch(() => undefined)
    : undefined;
  function needsResolver(file: string) {
    const owners = graph!.files.get(file);
    return (
      owners &&
      (owners.length > 1 ||
        owners.some(({ rank }) => {
          if (roots[rank].host || roots[rank].semanticOwner !== undefined)
            return true;
          const relative = path.relative(roots[rank].directory, file);
          return (
            relative.startsWith(".." + path.sep) || path.isAbsolute(relative)
          );
        }))
    );
  }
  const resolveAll = actual && needsResolver(actual);
  const chosen: string[] = [];
  for (const [uri, text] of graph.referring) {
    const file = fileURLToPath(uri);
    const owners = graph.files.get(file)!;
    if (resolveAll || needsResolver(file)) chosen.push(uri);
    else if (!all) continue;
    candidates.set(uri, {
      text,
      cfgs: [...new Set(owners.map(({ cfg }) => cfg))],
    });
  }
  if (chosen.length && actual) {
    const cache = workspace(roots, graph);
    const key = JSON.stringify([actual, offset, place]);
    if (cache.answer?.key !== key) {
      cache.answer = {
        key,
        pending: workspaceReferences(
          cache.sources,
          pathToFileURL(actual).href,
          offset,
          chosen,
          place,
        ),
      };
    }
    const answer = cache.answer;
    let result: Matches[] | null = null;
    try {
      result = await answer.pending;
    } finally {
      if (!result && cache.answer === answer) cache.answer = undefined;
    }
    for (const match of result ?? []) {
      const item = candidates.get(match.uri);
      if (item) (item.matches ??= []).push(match);
    }
    // A module compiled only for rust-analyzer's target-side view of a macro
    // has no incoming relationships in its real Cargo compilation.
    if (result)
      for (const uri of chosen)
        if (
          graph.files
            .get(fileURLToPath(uri))!
            .every(({ rank }) => roots[rank].semanticOwner !== undefined)
        )
          candidates.get(uri)!.matches = [];
  }
  return candidates;
}
