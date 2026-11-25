import { pathToFileURL } from "node:url";
import * as vscode from "vscode";

import { analyzerClient, locations } from "./analyzer";
import { cached, realFile } from "./cache";
import { editorTree, forgetModules } from "./card-places";
import { documentText, openDocument } from "./documents";
import { relationshipTargets } from "./module-sources";
import { coordinates, offsets } from "./offsets";
import { analyzedPath, located } from "./target";

/** Every relationship annotation contains this Rust identifier. */
const MAY_REFER = /\brefer\b/;

/** One revision of the sources and Cargo configuration. Diagnostics can repeat within it. */
function sourceRevision() {
  return {
    resolved: new Map<string, ReturnType<typeof locations>>(),
  };
}

let current = sourceRevision();

/** A source, configuration or workspace change can move declarations and their references. */
export function forgetReferences(source?: vscode.Uri | string): void {
  current = sourceRevision();
  if (typeof source === "string") forgetModules(source);
  else if (!source || source.scheme === "file") forgetModules(source?.fsPath);
}

/** A batch preserves its input order without flooding the language server or file system. */
async function batch<T, U>(
  items: T[],
  read: (item: T) => Promise<U>,
): Promise<U[]> {
  const answers: U[] = [];
  let next = 0;
  async function run() {
    while (next < items.length) {
      const index = next++;
      answers[index] = await read(items[index]);
    }
  }
  await Promise.all(Array.from({ length: Math.min(items.length, 8) }, run));
  return answers;
}

/**
 * The texts of the workspace that refer to the type named at `offset` of
 * `source`, each read once, in the crate rust-analyzer reads it in, with the
 * offsets of the relationship targets that name the type, and where each
 * stands in the module tree. `undefined` when the source or the workspace
 * changed meanwhile.
 */
export async function referringFiles(
  source: string,
  offset: number,
  log: (line: string) => void,
) {
  const revision = current;
  const document = await openDocument(source);
  if (document === undefined) return { files: [] };
  const { version } = document;
  const name = document.positionAt(offset);
  const [focus, references] = await Promise.all([
    located(document.uri, log, offset),
    vscode.commands
      .executeCommand<
        vscode.Location[] | undefined
      >("vscode.executeReferenceProvider", document.uri, name)
      .then(
        (found) => found ?? [],
        () => [],
      ),
  ]);
  // Keep an open symlink's URI for the page even when the provider or Cargo
  // names the same physical file through its canonical path.
  const aliases = new Map([[physical(source), source]]);
  const found = new Map<string, vscode.Position[]>();
  for (const { uri, range } of references) {
    const file = physical(uri.toString());
    const key = aliases.get(file) ?? uri.toString();
    aliases.set(file, key);
    found.set(key, [...(found.get(key) ?? []), range.start]);
  }
  const texts = new Map<string, string>();
  await batch([...found.keys()], async (key) => {
    const text = await documentText(vscode.Uri.parse(key)).catch(
      () => undefined,
    );
    if (text !== undefined) texts.set(key, text);
  });
  // A reference search stops at a renamed import: every relationship of the
  // workspace is then checked through its target's definition.
  const renamed = [...found].some(([key, positions]) =>
    renames(texts.get(key), positions),
  );
  if (renamed) {
    const graph = await editorTree(focus.roots);
    for (const file of graph?.referring ?? []) {
      const uri = pathToFileURL(analyzedPath(file)).href;
      const key = aliases.get(physical(uri)) ?? uri;
      if (!found.has(key)) found.set(key, []);
    }
  }
  if (document.version !== version || revision !== current) return undefined;
  const files = await batch([...found], async ([key, positions]) => {
    try {
      const uri = vscode.Uri.parse(key);
      const text = texts.get(key) ?? (await documentText(uri));
      if (!MAY_REFER.test(text)) return [];
      const { cfg, place } = key === source ? focus : await located(uri, log);
      if (!place && key !== source && !vscode.workspace.getWorkspaceFolder(uri))
        return [];
      const at = offsets(text);
      const targets = positions.map(({ line, character }) =>
        at(line, character),
      );
      if (renamed) {
        const named = await namingFocus(
          { uri: source, name },
          key,
          text,
          cfg,
          new Set(targets),
          revision,
        );
        targets.push(...named);
      }
      if (!targets.length) return [];
      return [{ uri: key, text, cfg, offsets: targets, place }];
    } catch {
      return [];
    }
  });
  if (document.version !== version || revision !== current) return undefined;
  return {
    focus: { place: focus.place },
    files: files.flat(),
  };
}

/**
 * Whether a reference among `positions` of `text` is a `use` that renames the
 * type: the name, `as`, the new name, then what ends it in a `use`, which
 * leaves out `<Focus as Trait>`.
 */
function renames(
  text: string | undefined,
  positions: vscode.Position[],
): boolean {
  if (text === undefined) return false;
  const at = offsets(text);
  const tokens = [
    /(?:r#)?\p{ID_Continue}+/uy,
    /as(?!\p{ID_Continue})/uy,
    /(?:r#)?\p{ID_Continue}+/uy,
    /[;,}]/y,
  ];
  return positions.some(({ line, character }) => {
    let offset = at(line, character);
    return tokens.every((token, index) => {
      token.lastIndex = index ? afterGap(text, offset) : offset;
      if (!token.test(text)) return false;
      offset = token.lastIndex;
      return true;
    });
  });
}

/** The offset past the whitespace and comments at `offset`, which Rust allows between tokens. */
function afterGap(text: string, offset: number): number {
  // Block comments nest.
  let depth = 0;
  while (offset < text.length) {
    if (text.startsWith("/*", offset)) {
      depth++;
      offset += 2;
    } else if (depth && text.startsWith("*/", offset)) {
      depth--;
      offset += 2;
    } else if (depth || /\s/u.test(text[offset])) {
      offset++;
    } else if (text.startsWith("//", offset)) {
      const end = text.indexOf("\n", offset);
      offset = end < 0 ? text.length : end;
    } else {
      break;
    }
  }
  return offset;
}

/** A reference and its definition can use a symlink and its real path. */
function physical(uri: string): string {
  const file = vscode.Uri.parse(uri);
  return file.scheme === "file" ? realFile(file.fsPath) : file.toString();
}

/**
 * The relationship targets of `text`, other than `known`, whose definition is
 * the focus's name: those behind a renamed import, which the reference search
 * leaves out.
 */
async function namingFocus(
  focus: { uri: string; name: vscode.Position },
  uri: string,
  text: string,
  cfg: string,
  known: Set<number>,
  revision: typeof current,
): Promise<number[]> {
  const client = analyzerClient();
  const targets = await relationshipTargets(text, cfg);
  if (!client || !targets) return [];
  const focusPath = physical(focus.uri);
  const { resolved } = revision;
  const at = coordinates(text);
  const named = await batch(
    targets.filter((offset) => !known.has(offset)),
    async (offset) => {
      const key = JSON.stringify([uri, offset]);
      try {
        const matches = await cached(resolved, key, () =>
          locations(client, uri, at.positionAt(offset)).catch(() => []),
        );
        // An empty response may mean rust-analyzer is still loading its workspace.
        if (!matches.length) resolved.delete(key);
        for (const location of matches) {
          if (
            location.position.line === focus.name.line &&
            location.position.character === focus.name.character &&
            physical(location.uri) === focusPath
          )
            return [offset];
        }
      } catch {
        resolved.delete(key);
      }
      return [];
    },
  );
  return named.flat();
}
