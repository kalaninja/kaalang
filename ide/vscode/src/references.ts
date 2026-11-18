import { realpath } from "node:fs/promises";
import * as path from "node:path";
import * as vscode from "vscode";

import { analyzerClient, locations } from "./analyzer";
import { cardPlacements, forgetModules, moduleFiles } from "./card-places";
import { documentText, openDocument } from "./documents";
import { coordinates, offsets } from "./offsets";
import { type CargoRoot, target } from "./target";

interface Place {
  uri: string;
  offset: number;
}

/** Every relationship annotation contains this Rust identifier. */
const MAY_REFER = /\brefer\b/;

type Definition = {
  uri: string;
  position: { line: number; character: number };
};

interface Candidates {
  texts: Map<string, string>;
  complete: boolean;
}

/** One revision of the sources and Cargo configuration. Diagnostics can repeat within it. */
function sourceRevision() {
  return {
    candidates: new WeakMap<CargoRoot[], Promise<Candidates | null>>(),
    resolved: new Map<string, Promise<Definition[]>>(),
    indexes: new Map<string, Promise<ReturnType<typeof coordinates>>>(),
    paths: new Map<string, Promise<string>>(),
    drawings: new Set<string>(),
  };
}

let current = sourceRevision();

/** A source, configuration or workspace change can move declarations and their references. */
export function forgetReferences(): void {
  current = sourceRevision();
  forgetModules();
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

async function candidateTexts(roots: CargoRoot[]): Promise<Candidates | null> {
  const cache = current.candidates;
  const cached = cache.get(roots);
  if (cached) return cached;
  const pending = moduleFiles(roots)
    .then(async (files) => {
      if (files === null) return null;
      const texts = new Map<string, string>();
      let complete = files.complete;
      await batch(files.uris, async (key) => {
        const uri = vscode.Uri.parse(key);
        try {
          const text = await documentText(uri);
          if (MAY_REFER.test(text)) texts.set(uri.toString(), text);
        } catch {
          complete = false;
        }
      });
      return { texts, complete };
    })
    .catch(() => null);
  cache.set(roots, pending);
  const texts = await pending;
  // Retry when the language server finishes indexing or a file becomes readable.
  if (!texts?.complete) cache.delete(roots);
  return texts;
}

/**
 * Workspace texts that can contain relationships and the references the editor
 * already resolved in each. The compiler identifies any targets still to check.
 */
export async function referringFiles(
  source: string,
  offset: number,
  log: (line: string) => void,
) {
  const revision = current;
  const document = await openDocument(source);
  if (document === undefined) return [];
  const { version } = document;
  const [project, references] = await Promise.all([
    target(document.uri, log),
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
  const owners = await cardPlacements(
    keys.map((uri) => ({ uri, offset: 0 })),
    project.roots,
  );
  const primary = new Map<string, string>();
  const cfgs = new Map<string, string>();
  const cfgByRoot = new Map(
    project.roots.map(({ root, cfg }) => [vscode.Uri.parse(root).fsPath, cfg]),
  );
  let missing = false;
  await batch([...keys.entries()], async ([index, key]) => {
    const owner = owners[index];
    const uri = vscode.Uri.parse(key);
    if (owner?.length) {
      const cfg = cfgByRoot.get(path.normalize(owner[0].root));
      if (cfg !== undefined) cfgs.set(key, cfg);
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
  const supplemental = secondary ? await candidateTexts(project.roots) : null;
  const texts = new Map<string, string>();
  for (const [uri, text] of supplemental?.texts ?? []) {
    const key = aliases.get(await physical(uri, revision)) ?? uri;
    texts.set(key, primary.get(key) ?? text);
    if (!found.has(key)) found.set(key, []);
  }
  if (document.version !== version || revision !== current) return undefined;
  // Supplementary sources are already confirmed members of the module graph,
  // including #[path] declarations reaching outside the editor's folder.
  const external = [...texts.keys()].filter(
    (key) =>
      !cfgs.has(key) &&
      !vscode.workspace.getWorkspaceFolder(vscode.Uri.parse(key)),
  );
  const places = await cardPlacements(
    external.map((uri) => ({ uri, offset: 0 })),
    project.roots,
  );
  external.forEach((key, index) => {
    const owner = places[index]?.[0];
    if (owner)
      cfgs.set(
        key,
        cfgByRoot.get(path.normalize(owner.root)) ??
          project.cfgForRoot(owner.root),
      );
  });
  const files = await batch([...found], async ([key, positions]) => {
    try {
      const uri = vscode.Uri.parse(key);
      const text =
        texts.get(key) ?? primary.get(key) ?? (await documentText(uri));
      if (!MAY_REFER.test(text)) return null;
      const at = offsets(text);
      return {
        uri: key,
        text,
        cfg:
          cfgs.get(key) ??
          (project.roots.length
            ? project.cfgForRoot(uri.fsPath)
            : (await target(uri, log)).cfg),
        offsets: positions.map((position) =>
          at(position.line, position.character),
        ),
      };
    } catch {
      return null;
    }
  });
  if (document.version !== version || revision !== current) return undefined;
  revision.drawings.add(JSON.stringify([source, offset]));
  return files.filter((file) => file !== null);
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
