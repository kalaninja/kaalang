// The parts of the cfg set that run no process, apart from VS Code so that
// tests reach them: what rust-analyzer's settings add to rustc's cfgs, and what
// a file's package adds.

import * as path from "node:path";

import type { Metadata } from "./metadata";

/** An entry of `rust-analyzer.cargo.cfgs`: `name` or `name=value`, `!` before one to leave it out. */
const EXTRA_CFG = /^(!?)([A-Za-z_][A-Za-z0-9_]*)(?:=([^"\r\n]*))?$/;

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
  const enabled = new Set<string>();
  const disabled = new Set<string>();
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
    (remove ? disabled : enabled).add(line);
  }
  // rust-analyzer cancels an override that is both enabled and disabled.
  for (const line of enabled) {
    if (!disabled.has(line)) {
      lines.add(line);
    }
  }
  for (const line of disabled) {
    if (!enabled.has(line)) {
      lines.delete(line);
    }
  }
}

/**
 * rust-analyzer sets the local crate's flags, applies the configured
 * overrides, then adds the package's features and build-script cfgs from Cargo.
 */
export function fileCfgs(
  rustc: string[],
  metadata: Metadata | undefined,
  file: string | undefined,
  options: { cfgs: unknown[]; setTest: boolean },
  log: (line: string) => void,
  buildCfgs?: ReadonlyMap<string, string[]>,
): string {
  const lines = new Set(rustc);
  // The package whose directory holds the file most closely.
  const source = file === undefined ? undefined : path.normalize(file);
  const owner = metadata?.packages
    .filter((candidate) =>
      source?.startsWith(
        path.join(path.dirname(candidate.manifest_path), path.sep),
      ),
    )
    .sort((a, b) => b.manifest_path.length - a.manifest_path.length)[0];
  if (owner?.source === null) {
    lines.add("rust_analyzer");
    if (options.setTest) {
      lines.add("test");
    }
  }
  applyExtraCfgs(lines, options.cfgs, log);
  metadata?.resolve?.nodes
    .find((node) => node.id === owner?.id)
    ?.features.forEach((feature) => lines.add(`feature="${feature}"`));
  if (owner) {
    for (const cfg of buildCfgs?.get(owner.id) ?? []) lines.add(cfg);
  }
  return [...lines].join("\n");
}
