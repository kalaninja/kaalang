import { realpath } from "node:fs/promises";
import * as vscode from "vscode";

import { analyzerClient, locations } from "./analyzer";
import { fileOwners, forgetModules, MAY_REFER } from "./card-places";
import { documentText, openDocument } from "./documents";
import type { Matches } from "./module-sources";
import { coordinates, offsets } from "./offsets";
import { referringSources } from "./referring-sources";
import { target } from "./target";

interface Place {
  uri: string;
  offset: number;
}

type Definition = {
  uri: string;
  position: { line: number; character: number };
};

/** One revision of the sources and Cargo configuration. Diagnostics can repeat within it. */
function sourceRevision() {
  return {
    resolved: new Map<string, Promise<Definition[]>>(),
    indexes: new Map<string, Promise<ReturnType<typeof coordinates>>>(),
    paths: new Map<string, Promise<string>>(),
    drawings: new Set<string>(),
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
 * Workspace texts that can contain relationships and the references the editor
 * already resolved in each. The compiler identifies any targets still to check.
 */
export async function referringFiles(
  source: string,
  offset: number,
  log: (line: string) => void,
  activeModule?: () => Promise<string | null>,
) {
  const revision = current;
  const document = await openDocument(source);
  if (document === undefined) return { files: [] };
  const { version } = document;
  const [project, references] = await Promise.all([
    target(document.uri, log, activeModule, offset),
    vscode.commands
      .executeCommand<
        vscode.Location[] | undefined
      >("vscode.executeReferenceProvider", document.uri, document.positionAt(offset))
      .then(
        (found) => found ?? [],
        () => [],
      ),
  ]);
  // Keep an open symlink's URI for the page even when the provider or Cargo
  // names the same physical file through its canonical path.
  const aliases = new Map([[await physical(source, revision), source]]);
  const found = new Map<string, vscode.Position[]>();
  const referencesByFile = new Map<string, vscode.Position[]>();
  for (const { uri, range } of references) {
    const key = uri.toString();
    const positions = referencesByFile.get(key) ?? [];
    positions.push(range.start);
    referencesByFile.set(key, positions);
  }
  for (const [uri, positions] of referencesByFile) {
    const file = await physical(uri, revision);
    const key = aliases.get(file) ?? uri;
    aliases.set(file, key);
    found.set(key, (found.get(key) ?? []).concat(positions));
  }

  const keys = [...new Set([source, ...found.keys()])];
  const owners = await fileOwners(keys, project.roots);
  const primary = new Map<string, string>();
  const cfgs = new Map<string, string[]>();
  let missing = false;
  await batch([...keys.entries()], async ([index, key]) => {
    const owner = owners[index];
    const uri = vscode.Uri.parse(key);
    if (owner?.length) {
      cfgs.set(key, [...new Set(owner.map(({ cfg }) => cfg))]);
    } else if (
      key !== source &&
      (owner?.length === 0 || !vscode.workspace.getWorkspaceFolder(uri))
    ) {
      missing = true;
      found.delete(key);
      return;
    }
    if (found.has(key)) {
      const text = await documentText(uri).catch(() => undefined);
      if (text !== undefined) primary.set(key, text);
    }
  });
  // A reference search stops at a renamed import and can see only one
  // interpretation of a file shared between several crates or workspaces.
  const secondary =
    project.roots.length > 0 &&
    ((vscode.workspace.workspaceFolders?.length ?? 0) > 1 ||
      missing ||
      owners[0]?.length !== 1 ||
      primary.size !== found.size ||
      [...primary.values()].some((text) => /\bas\b/.test(text)));
  const supplemental = await referringSources(
    project.roots,
    secondary,
    source,
    offset,
    project.selectedOwner,
  );
  const resolved = new Map<string, Matches[]>();
  const texts = new Map<string, string>();
  for (const [uri, { text, cfgs: active, matches }] of supplemental) {
    const key = aliases.get(await physical(uri, revision)) ?? uri;
    texts.set(key, primary.get(key) ?? text);
    cfgs.set(key, active);
    if (matches) resolved.set(key, matches);
    if (!found.has(key)) found.set(key, []);
  }
  if (document.version !== version || revision !== current) return undefined;
  const files = await batch([...found], async ([key, positions]) => {
    try {
      const uri = vscode.Uri.parse(key);
      const text =
        texts.get(key) ?? primary.get(key) ?? (await documentText(uri));
      if (!MAY_REFER.test(text)) return [];
      const owners = resolved.get(key);
      if (owners)
        return owners.map((match) => ({
          ...match,
          uri: key,
          text,
          resolved: true,
        }));
      const at = offsets(text);
      const references = positions.map((position) =>
        at(position.line, position.character),
      );
      const active = cfgs.get(key) ?? [
        project.roots.length
          ? project.cfgForRoot(uri.fsPath)
          : (await target(uri, log)).cfg,
      ];
      return active.map((cfg) => ({
        uri: key,
        text,
        cfg,
        offsets: references,
      }));
    } catch {
      return [];
    }
  });
  if (document.version !== version || revision !== current) return undefined;
  revision.drawings.add(JSON.stringify([source, offset]));
  // The diagram needs the selected cfg and mount even when the focus has no refer of its own.
  return {
    files: files.flat(),
    focus: project.displayOwner && {
      ...project.displayOwner,
      cfg: project.cfg,
    },
  };
}

/** A reference and its definition can use a symlink and its real path. */
function physical(uri: string, revision: typeof current): Promise<string> {
  const cached = revision.paths.get(uri);
  if (cached) return cached;
  const file = vscode.Uri.parse(uri);
  const pending =
    file.scheme === "file"
      ? realpath(file.fsPath).catch(() => file.fsPath)
      : Promise.resolve(file.toString());
  revision.paths.set(uri, pending);
  return pending;
}

/** Which target names go to the focus, including uses hidden behind renamed imports. */
export async function definitionMatches(
  focus: Place,
  places: Place[],
): Promise<boolean[] | undefined> {
  const revision = current;
  if (!revision.drawings.has(JSON.stringify([focus.uri, focus.offset])))
    return undefined;
  const client = analyzerClient();
  const document = await openDocument(focus.uri);
  if (!client || !document) return places.map(() => false);
  const name = document.positionAt(focus.offset);
  const focusPath = await physical(focus.uri, revision);
  const { resolved, indexes } = revision;
  const matches = await batch(places, async ({ uri, offset }) => {
    const key = JSON.stringify([uri, offset]);
    try {
      let answer = resolved.get(key);
      if (!answer) {
        let index = indexes.get(uri);
        if (!index) {
          index = documentText(vscode.Uri.parse(uri))
            .then(coordinates)
            .catch((error) => {
              indexes.delete(uri);
              throw error;
            });
          indexes.set(uri, index);
        }
        answer = index.then(async (at) => {
          const position = at.positionAt(offset);
          const request = () =>
            locations(client, "textDocument/definition", uri, position);
          const first = await request().catch(() => []);
          // Another loaded workspace can omit a closed integration target
          // from the server's VFS. Opening its text model loads it without
          // revealing an editor. The next diagnostics can finish this lookup.
          if (
            !first.length &&
            !vscode.workspace.textDocuments.some(
              (doc) => doc.uri.toString() === uri,
            )
          ) {
            if (await openDocument(uri)) return request();
          }
          return first;
        });
        resolved.set(key, answer);
      }
      const matches = await answer;
      // An empty response may mean rust-analyzer is still loading its workspace.
      if (!matches.length) resolved.delete(key);
      for (const location of matches) {
        if (
          location.position.line === name.line &&
          location.position.character === name.character &&
          (await physical(location.uri, revision)) === focusPath
        )
          return true;
      }
    } catch {
      resolved.delete(key);
    }
    return false;
  });
  return revision === current ? matches : undefined;
}
