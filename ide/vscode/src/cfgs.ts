// The parts of the cfg set that run no process, apart from VS Code so that
// tests reach them: what rust-analyzer's settings add to rustc's cfgs, and what
// a file's package adds.

import * as path from "node:path";

/** An entry of `rust-analyzer.cargo.cfgs`: `name` or `name=value`, `!` before one to leave it out. */
const EXTRA_CFG = /^(!?)([A-Za-z_][A-Za-z0-9_]*)(?:=([^"]*))?$/;

/** The part of `cargo metadata`'s answer the set needs. */
export interface Metadata {
  packages: { id: string; manifest_path: string }[];
  workspace_members: string[];
  resolve: { nodes: { id: string; features: string[] }[] } | null;
}

/**
 * Applies the entries of `rust-analyzer.cargo.cfgs` to `lines`, as rust-analyzer
 * does before it adds features. An entry that names no cfg is left out and
 * `log`ged: the set's parser would reject the whole set for it.
 */
export function applyExtraCfgs(
  lines: Set<string>,
  entries: unknown[],
  log: (line: string) => void,
): void {
  for (const entry of entries) {
    const match = typeof entry === "string" ? EXTRA_CFG.exec(entry) : null;
    if (match === null) {
      log(
        `Left out ${JSON.stringify(entry)} of rust-analyzer.cargo.cfgs, which names no cfg.`,
      );
      continue;
    }
    const [, remove, name, value] = match;
    const line = value === undefined ? name : `${name}="${value}"`;
    if (remove) {
      lines.delete(line);
    } else {
      lines.add(line);
    }
  }
}

/**
 * Adds to `lines` what the package holding `file`, a real path, gives it: the
 * features cargo enables for it, and for a workspace crate `rust_analyzer`, as
 * rust-analyzer sets it, and `test` when `setTest` asks.
 */
export function addPackageCfgs(
  lines: Set<string>,
  metadata: Metadata,
  file: string,
  setTest: boolean,
): void {
  // The package whose directory holds the file most closely.
  const owner = metadata.packages
    .filter((candidate) =>
      file.startsWith(path.dirname(candidate.manifest_path) + path.sep),
    )
    .sort((a, b) => b.manifest_path.length - a.manifest_path.length)[0];
  if (owner === undefined) {
    return;
  }
  metadata.resolve?.nodes
    .find((node) => node.id === owner.id)
    ?.features.forEach((feature) => lines.add(`feature="${feature}"`));
  if (metadata.workspace_members.includes(owner.id)) {
    lines.add("rust_analyzer");
    if (setTest) {
      lines.add("test");
    }
  }
}
