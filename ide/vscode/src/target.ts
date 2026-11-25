// The cfg set rust-analyzer analyzes a file with, as `rustc --print cfg`
// prints it, so that the diagram counts the code the editor shows as active.
// rust-analyzer's status report states the set of each crate, so it is asked
// rather than recomputed; Cargo only lists the targets whose module trees hold
// the cards.

import { execFile } from "node:child_process";
import * as path from "node:path";
import * as vscode from "vscode";

import { type AnalyzedCrate, analyzerClient, statusCrates } from "./analyzer";
import { cached, realFile } from "./cache";
import { fileReadings, type Placement } from "./card-places";
import { type CargoTarget, type Metadata, workspaceTargets } from "./metadata";

/** Each crate reads its module tree with the set rust-analyzer gives its root. */
export interface CargoRoot extends CargoTarget {
  cfg: string;
}

interface Workspace {
  roots: CargoRoot[];
  workspaceRoot: string | undefined;
}

/** The settings of rust-analyzer that decide the targets or the set. */
const SECTIONS = [
  "rust-analyzer.cargo",
  "rust-analyzer.cfg",
  "rust-analyzer.procMacro.enable",
];

function cargoCache() {
  return {
    roots: new Map<string, Promise<string | undefined>>(),
    metadata: new Map<string, Promise<Metadata | undefined>>(),
    projects: new Map<string, vscode.Uri>(),
  };
}
let cache = cargoCache();
/** rust-analyzer's crates by file; their sets change as it loads the workspace and reruns build scripts. */
let analyzed = new Map<string, Promise<AnalyzedCrate[]>>();
/** Its last answer by file, which stands while it restarts or reloads the workspace and gives none. */
const answered = new Map<string, AnalyzedCrate[]>();
/** One array per distinct list of roots, so that module trees stay cached while it holds. */
let shapes = new Map<string, CargoRoot[]>();

/**
 * The watches of the manifests outside every workspace folder, which the
 * editor's own watcher leaves out. Each stays until the extension stops, so
 * that no change falls between two readings of the targets.
 */
let outside = new Map<string, vscode.Disposable>();
let manifestChanged: (() => void) | undefined;

/**
 * The build directories of the workspaces read so far. One stays after a
 * setting moves it: what a build left there is no target either.
 */
const buildDirectories = new Set<string>();

/** Whether `file` lies in the build directory of a workspace read so far. */
export function inBuildDirectory(file: string): boolean {
  return [...buildDirectories].some((directory) =>
    file.startsWith(directory + path.sep),
  );
}

/** Whether Restricted Mode was logged: once is enough. */
let restricted = false;
/** Whether a missing answer of rust-analyzer was logged since its last answer. */
let unanalyzed = false;

export function affectsCfg(event: vscode.ConfigurationChangeEvent): boolean {
  return SECTIONS.some((section) => event.affectsConfiguration(section));
}

/** Forgets the targets and the sets, after a manifest or a setting changes. */
export function forgetCfg(): void {
  cache = cargoCache();
  shapes = new Map();
  forgetAnalyzedCfg();
}

/**
 * Reports a change of a manifest outside every workspace folder: a folder
 * opened below its Cargo workspace's root has the root's manifest, and other
 * members', outside it.
 */
export function watchOutsideManifests(changed: () => void): vscode.Disposable {
  manifestChanged = changed;
  return {
    dispose() {
      manifestChanged = undefined;
      for (const watch of outside.values()) watch.dispose();
      outside = new Map();
    },
  };
}

function watchManifests(metadata: Metadata): void {
  const changed = manifestChanged;
  if (!changed) return;
  const members = new Set(metadata.workspace_members);
  const manifests = [
    path.join(metadata.workspace_root, "Cargo.toml"),
    ...metadata.packages
      .filter(({ id }) => members.has(id))
      .map(({ manifest_path }) => manifest_path),
  ];
  for (const manifest of manifests) {
    if (
      outside.has(manifest) ||
      vscode.workspace.getWorkspaceFolder(vscode.Uri.file(manifest))
    )
      continue;
    const watch = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(
        vscode.Uri.file(path.dirname(manifest)),
        "Cargo.toml",
      ),
    );
    watch.onDidCreate(changed);
    watch.onDidChange(changed);
    watch.onDidDelete(changed);
    outside.set(manifest, watch);
  }
}

/** Forgets the targets, after a source file appears or disappears: Cargo finds them by their files. */
export function forgetTargets(): void {
  cache.metadata.clear();
}

/** Asks rust-analyzer again, which may have loaded the workspace or rerun build scripts since. */
export function forgetAnalyzedCfg(): void {
  analyzed = new Map();
}

/** Where a file stands, as `located` finds it. */
export interface Located {
  /** The set rust-analyzer reads the file with. */
  cfg: string;
  /** Its place in the crate rust-analyzer reads it in; none outside every workspace target. */
  place?: Placement;
  /** The targets of the file's Cargo workspace. */
  roots: CargoRoot[];
}

/**
 * Where `file` stands in the module trees of its Cargo workspace's targets,
 * in the crate rust-analyzer reads it in: a file several targets include gets
 * no other place, since rust-analyzer reads it in that one only. `typeOffset`
 * keeps the places whose reading declares a type there. Whatever cannot be
 * read is left out and `log`ged; Restricted Mode, where neither Cargo nor
 * rust-analyzer runs, gives nothing.
 */
export async function located(
  file: vscode.Uri,
  log: (line: string) => void,
  typeOffset?: number,
): Promise<Located> {
  if (!trusted(log) || file.scheme !== "file") return { cfg: "", roots: [] };
  const revision = cache;
  const folder = vscode.workspace.getWorkspaceFolder(file);
  const source = realFile(file.fsPath);
  const [crate] = await analyzedCrates(source, log);
  const primary = folder
    ? await configuredWorkspace(revision, file, path.dirname(source), log)
    : undefined;
  const candidates: (() => Promise<Workspace>)[] = [];
  if (primary) candidates.push(async () => primary);
  // A configured target or #[path] module can live outside its package or folder.
  // Only start Cargo in directories that belong to a trusted editor workspace.
  const others = new Map(revision.projects);
  for (const candidate of vscode.workspace.workspaceFolders ?? [])
    others.set(candidate.uri.fsPath, candidate.uri);
  for (const [directory, resource] of others) {
    if (directory !== primary?.workspaceRoot)
      candidates.push(() =>
        configuredWorkspace(revision, resource, directory, log),
      );
  }
  const checked = new Set<CargoRoot[]>();
  for (const candidate of candidates) {
    const { roots } = await candidate();
    if (!roots.length || checked.has(roots)) continue;
    checked.add(roots);
    const readings = await fileReadings(source, roots, typeOffset);
    if (!readings?.length) continue;
    const reading =
      readings.find(
        ({ place }) =>
          place.package === crate?.package && place.crate === crate?.name,
      ) ?? readings[0];
    return { cfg: crate?.cfg ?? reading.cfg, place: reading.place, roots };
  }
  return { cfg: crate?.cfg ?? "", roots: primary?.roots ?? [] };
}

/**
 * `file` as rust-analyzer knows it: below the path its workspace folder was
 * opened by, which a symlink makes differ from the real one Cargo names.
 */
export function analyzedPath(file: string): string {
  for (const { uri } of vscode.workspace.workspaceFolders ?? []) {
    if (uri.scheme !== "file") continue;
    const real = realFile(uri.fsPath);
    if (real !== uri.fsPath && (file + path.sep).startsWith(real + path.sep))
      return uri.fsPath + file.slice(real.length);
  }
  return file;
}

/** Each crate rust-analyzer reads `file` in, the one it shows first; none until it can tell. */
async function analyzedCrates(
  file: string,
  log: (line: string) => void,
): Promise<AnalyzedCrate[]> {
  const crates = await cached(analyzed, file, async () => {
    const client = analyzerClient();
    if (!client) return [];
    try {
      const status = await client.sendRequest<string>(
        "rust-analyzer/analyzerStatus",
        {
          textDocument: { uri: vscode.Uri.file(analyzedPath(file)).toString() },
        },
      );
      return statusCrates(status);
    } catch (error) {
      log(
        `Could not read rust-analyzer's cfg set: ${(error as Error).message}`,
      );
      return [];
    }
  });
  if (crates.length) {
    unanalyzed = false;
    answered.set(file, crates);
    return crates;
  }
  // rust-analyzer may still be loading the workspace: ask again next time.
  analyzed.delete(file);
  const last = answered.get(file);
  if (last) return last;
  if (!unanalyzed) {
    unanalyzed = true;
    log(
      "rust-analyzer gives no cfg set yet: the diagram is drawn without one.",
    );
  }
  return [];
}

/** The set rust-analyzer reads `file` with in the crate it shows; empty until it can tell. */
export async function analyzedCfg(
  file: string,
  log: (line: string) => void,
): Promise<string> {
  return trusted(log) ? ((await analyzedCrates(file, log))[0]?.cfg ?? "") : "";
}

function trusted(log: (line: string) => void): boolean {
  if (vscode.workspace.isTrusted) return true;
  if (!restricted) {
    restricted = true;
    log("Restricted Mode: the diagram is drawn without the target's cfg set.");
  }
  return false;
}

async function configuredWorkspace(
  revision: typeof cache,
  file: vscode.Uri,
  directory: string,
  log: (line: string) => void,
): Promise<Workspace> {
  const extraEnv =
    vscode.workspace
      .getConfiguration("rust-analyzer", file)
      .get<Record<string, string | null>>("cargo.extraEnv") ?? {};
  const env = { ...process.env };
  for (const [name, value] of Object.entries(extraEnv)) {
    if (value === null) {
      delete env[name];
    } else {
      env[name] = value;
    }
  }
  const root = await cached(
    revision.roots,
    JSON.stringify([directory, extraEnv]),
    () => locate(directory, env, log),
  );
  if (!root) return { roots: [], workspaceRoot: undefined };
  revision.projects.set(root, file);
  const metadata = await cached(
    revision.metadata,
    JSON.stringify([root, extraEnv]),
    async () => {
      const metadata = await readMetadata(root, env, log);
      if (metadata) {
        watchManifests(metadata);
        // Cargo names the real path, a watcher the opened one.
        buildDirectories.add(analyzedPath(metadata.target_directory));
      }
      return metadata;
    },
  );
  return {
    roots: metadata ? await configured(metadata, log) : [],
    workspaceRoot: root,
  };
}

/** The root of the cargo workspace `directory` belongs to, as rust-analyzer runs cargo there. */
async function locate(
  directory: string,
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
): Promise<string | undefined> {
  try {
    const manifest = await cargo(
      ["locate-project", "--workspace", "--message-format", "plain"],
      directory,
      env,
    );
    return path.dirname(manifest.trim());
  } catch (error) {
    log(`No cargo workspace holds ${directory}: ${(error as Error).message}`);
    return undefined;
  }
}

/** The workspace's targets. Without dependencies Cargo resolves nothing, so it leaves the lockfile alone. */
async function readMetadata(
  root: string,
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
): Promise<Metadata | undefined> {
  try {
    return JSON.parse(
      await cargo(
        ["metadata", "--format-version", "1", "--no-deps"],
        root,
        env,
      ),
    );
  } catch (error) {
    log(`Could not read the workspace's targets: ${(error as Error).message}`);
    return undefined;
  }
}

/** The workspace's targets, each with the set rust-analyzer reads its root with. */
async function configured(
  metadata: Metadata,
  log: (line: string) => void,
): Promise<CargoRoot[]> {
  const roots = await Promise.all(
    workspaceTargets(metadata).map(async (target) => ({
      ...target,
      cfg: await analyzedCfg(target.root, log),
    })),
  );
  const key = JSON.stringify(roots);
  const known = shapes.get(key);
  if (known) return known;
  shapes.set(key, roots);
  return roots;
}

/** Runs Cargo, the one `CARGO` names as Cargo runs itself, and answers what it printed. */
function cargo(
  args: string[],
  directory: string,
  env: NodeJS.ProcessEnv,
): Promise<string> {
  const program = env.CARGO || "cargo";
  return new Promise((resolve, reject) =>
    execFile(
      program,
      args,
      { cwd: directory, env, maxBuffer: 256 * 1024 * 1024 },
      (error, stdout, stderr) =>
        error
          ? reject(
              new Error(
                `\`${program} ${args.join(" ")}\`: ${stderr.trim() || error.message}`,
              ),
            )
          : resolve(stdout),
    ),
  );
}
