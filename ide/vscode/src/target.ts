// The cfg set rust-analyzer analyzes a file with, as `rustc --print cfg`
// prints it, so that the diagram counts the code the editor shows as active.
// rust-analyzer exposes no structured cfg set, so this one is read as it reads its own:
// rustc's cfgs for the target with cargo's configuration applied, the extra
// flags of a local crate and its settings, then its features and build-script cfgs.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import { cached } from "./cache";
import { forgetBuildScriptRuns } from "./build-scripts";
import { fileOwners } from "./card-places";
import { type CargoOptions, readCargoProject } from "./cargo";
import { configArgs, run } from "./cargo-command";
import { fileCfgs, packageCfgs } from "./cfgs";
import {
  type CargoRoot as CargoTarget,
  type Metadata,
  hostTargets,
  workspaceTargets,
} from "./metadata";

/** Each crate reads its module tree with its package's active features. */
export interface CargoRoot extends CargoTarget {
  cfg: string;
  /** An additional Cargo compilation for the host in a cross-target editor. */
  host?: true;
  /** The actual compilation of a procedural macro shown with rust-analyzer's target cfg. */
  semanticOwner?: number;
}

/** The settings of rust-analyzer that decide the set. */
const SECTIONS = [
  "rust-analyzer.cargo",
  "rust-analyzer.cfg",
  "rust-analyzer.procMacro.enable",
];

/** What rust-analyzer's settings ask of the set. */
interface Options extends CargoOptions {
  configPath?: string;
  cfgs: unknown[];
  setTest: boolean;
  extraEnv: Record<string, string | null>;
}

function cargoCache() {
  return {
    roots: new Map<string, Promise<string | undefined>>(),
    workspaces: new Map<string, ReturnType<typeof read>>(),
    projects: new Map<string, vscode.Uri>(),
  };
}
let cache = cargoCache();

/** Whether Restricted Mode was logged: once is enough. */
let restricted = false;

/** Whether `event` changes a setting the set depends on. */
export function affectsCfg(event: vscode.ConfigurationChangeEvent): boolean {
  return SECTIONS.some((section) => event.affectsConfiguration(section));
}

/** Forgets what was read, after a manifest or a cargo configuration changes. */
export function forgetCfg(): void {
  cache = cargoCache();
  forgetBuildScriptRuns();
}

/** The additional Cargo configuration selected for the rust-analyzer session. */
export function cargoConfig(file?: vscode.Uri): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("rust-analyzer", file)
    .get<string | null>("cargo.configPath");
  return configured
    ? path.resolve(
        (file && vscode.workspace.getWorkspaceFolder(file)?.uri.fsPath) ??
          vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ??
          process.cwd(),
        configured,
      )
    : undefined;
}

/**
 * The set for `file`; whatever cannot be read is left out and `log`ged. Empty
 * in Restricted Mode: cargo and rustc run whatever binaries the workspace's
 * configuration names.
 */
export async function target(
  file: vscode.Uri,
  log: (line: string) => void,
  activeModule?: () => Promise<string | null>,
  typeOffset?: number,
) {
  if (!vscode.workspace.isTrusted) {
    if (!restricted) {
      restricted = true;
      log(
        "Restricted Mode: the diagram is drawn without the target's cfg set.",
      );
    }
    return {
      cfg: "",
      roots: [],
      cfgForRoot: () => "",
      buildInputs: undefined,
      workspaceRoot: undefined,
      owned: false,
      selectedOwner: undefined,
      displayOwner: undefined,
      cfgs: undefined,
    };
  }
  const revision = cache;
  const folder = vscode.workspace.getWorkspaceFolder(file);
  const source = file.scheme === "file" ? realFile(file.fsPath) : undefined;
  const directory = source && folder ? path.dirname(source) : undefined;
  let primary = directory
    ? await configuredWorkspace(revision, file, directory, log)
    : undefined;
  const checked = new Set<CargoRoot[]>();

  async function from(workspace: Awaited<ReturnType<typeof read>>) {
    if (!source || !workspace.roots.length || checked.has(workspace.roots))
      return;
    checked.add(workspace.roots);
    function select(rank: number, module: string, cfgs?: string[]) {
      const root = workspace.roots[rank];
      return {
        ...workspace,
        cfg: root.cfg,
        owned: true,
        selectedOwner: { owner: root.semanticOwner ?? rank, module },
        displayOwner: { owner: rank, module },
        cfgs,
      };
    }
    const direct = workspace.roots.findIndex(
      (root) => root.root === file.toString() && !root.host,
    );
    // A crate root may also be an external module of another target. Its
    // Cargo cfg can even equal that target's while the declaration is a different type.
    if (
      direct >= 0 &&
      ((!activeModule && typeOffset === undefined) ||
        workspace.roots.length === 1)
    )
      return select(direct, "");
    const [readings] = await fileOwners(
      [file.toString()],
      workspace.roots,
      typeOffset,
    );
    if (!readings?.length) return direct >= 0 ? select(direct, "") : undefined;
    const displayed = readings.filter(
      ({ rank }) => !workspace.roots[rank].host,
    );
    const owners = displayed.length ? displayed : readings;
    let owner = owners[0];
    if (activeModule && owners.length > 1) {
      const module = await activeModule();
      const matching =
        module &&
        owners
          .filter(
            (other) =>
              module === other.module || module.startsWith(other.module + "::"),
          )
          .sort((a, b) => b.module.length - a.module.length)[0];
      if (matching) owner = matching;
    }
    return select(
      owner.rank,
      owner.path,
      owners.length > 1
        ? [...new Set(owners.map(({ cfg }) => cfg))]
        : undefined,
    );
  }

  if (primary) {
    const selected = await from(primary);
    if (selected) return selected;
  }
  // A configured target or #[path] module can live outside its package or folder.
  // Only start Cargo in directories that belong to a trusted editor workspace.
  if (source) {
    const others = new Map(revision.projects);
    for (const candidate of vscode.workspace.workspaceFolders ?? [])
      others.set(candidate.uri.fsPath, candidate.uri);
    for (const [directory, resource] of others) {
      if (directory === primary?.workspaceRoot) continue;
      const candidate = await configuredWorkspace(
        revision,
        resource,
        directory,
        log,
      );
      const selected = await from(candidate);
      if (selected) return selected;
    }
  }
  primary ??= await configuredWorkspace(revision, file, undefined, log);
  return {
    ...primary,
    cfg: primary.cfgForRoot(folder ? source : undefined),
    owned: false,
    selectedOwner: undefined,
    displayOwner: undefined,
    cfgs: undefined,
  };
}

async function configuredWorkspace(
  revision: typeof cache,
  file: vscode.Uri,
  directory: string | undefined,
  log: (line: string) => void,
) {
  const settings = vscode.workspace.getConfiguration("rust-analyzer", file);
  const options: Options = {
    target: settings.get<string | null>("cargo.target") ?? null,
    features: settings.get<string[] | "all">("cargo.features") ?? [],
    noDefaultFeatures:
      settings.get<boolean>("cargo.noDefaultFeatures") ?? false,
    noDeps: settings.get<boolean>("cargo.noDeps") ?? false,
    metadataArgs: settings.get<string[]>("cargo.metadataExtraArgs") ?? [],
    configPath: cargoConfig(file),
    // rust-analyzer's own default, for when it is not installed.
    cfgs: settings.get<unknown[]>("cargo.cfgs") ?? ["debug_assertions", "miri"],
    setTest: settings.get<boolean>("cfg.setTest") ?? true,
    extraEnv:
      settings.get<Record<string, string | null>>("cargo.extraEnv") ?? {},
    buildScripts:
      (settings.get<boolean>("cargo.buildScripts.enable") ?? true) ||
      (settings.get<boolean>("procMacro.enable") ?? true),
    rebuildOnSave:
      settings.get<boolean>("cargo.buildScripts.rebuildOnSave") ?? true,
    buildCommand:
      settings.get<string[] | null>("cargo.buildScripts.overrideCommand") ??
      null,
    buildDirectory:
      settings.get<string>("cargo.buildScripts.invocationStrategy") === "once"
        ? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
        : undefined,
    extraArgs: settings.get<string[]>("cargo.extraArgs") ?? [],
    targetDir: settings.get<boolean | string | null>("cargo.targetDir") ?? null,
  };
  const env = { ...process.env };
  for (const [name, value] of Object.entries(options.extraEnv)) {
    if (value === null) {
      delete env[name];
    } else {
      env[name] = value;
    }
  }
  const root =
    directory === undefined
      ? undefined
      : await cached(
          revision.roots,
          JSON.stringify([directory, options.extraEnv]),
          () => locate(directory, env, log),
        );
  if (root) revision.projects.set(root, file);
  const workspace = await cached(
    revision.workspaces,
    JSON.stringify([root ?? null, options]),
    () => read(root, options, env, log),
  );
  return workspace;
}

function realFile(file: string): string {
  try {
    return fs.realpathSync(file);
  } catch {
    return file;
  }
}

/** The root of the cargo workspace `directory` belongs to, as rust-analyzer runs cargo there. */
async function locate(
  directory: string,
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
): Promise<string | undefined> {
  try {
    const manifest = await run(
      "cargo",
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

async function read(
  root: string | undefined,
  options: Options,
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
) {
  const directory =
    root ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
  const config = options.configPath ? ["--config", options.configPath] : [];
  const cargoConfig = [...config, ...configArgs(options.extraArgs)];
  const targets = options.target
    ? [options.target]
    : root === undefined
      ? []
      : await configuredTargets(root, cargoConfig, env);
  const target = targets.length > 0 ? ["--target", targets[0]] : [];
  const cfgs = await printed(root, directory, cargoConfig, target, env, log);
  const { metadata, hostMetadata, buildCfgs, hostCfgs, buildInputs } =
    root === undefined
      ? {}
      : await readCargoProject(root, targets, config, options, env, log);
  const cfgForRoot = (file: string | undefined) =>
    fileCfgs(cfgs, metadata, file, options, log, buildCfgs);
  function configured(
    targets: CargoTarget[],
    metadata: Metadata,
    printed: string[],
    host: boolean,
  ): CargoRoot[] {
    const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
    const perPackage = new Map<string, string>();
    return targets.map((target) => {
      const key = JSON.stringify([target.packageId, !!target.procMacro]);
      let cfg = perPackage.get(key);
      if (cfg === undefined) {
        cfg = packageCfgs(
          host && target.procMacro ? [...printed, "proc_macro"] : printed,
          metadata,
          packages.get(target.packageId),
          options,
          log,
          host || target.procMacro ? hostCfgs : buildCfgs,
        );
        perPackage.set(key, cfg);
      }
      return {
        ...target,
        cfg,
        ...(host ? { host: true as const } : {}),
        root: vscode.Uri.file(target.root).toString(),
      };
    });
  }
  const editorTargets = metadata ? workspaceTargets(metadata) : [];
  const roots = metadata
    ? configured(editorTargets, metadata, cfgs, false)
    : [];
  const macros = new Map(
    roots
      .filter((target) => target.procMacro)
      .map((target) => [target.packageId, target]),
  );
  const actual = hostMetadata ?? (!targets.length ? metadata : undefined);
  if (actual && macros.size) {
    // An explicit compilation target separates Cargo's host flags from RUSTFLAGS,
    // even when the triples coincide. A native macro keeps RUSTFLAGS and still
    // receives proc_macro, independently of its visible rust-analyzer cfg.
    const host = hostMetadata
      ? await run("rustc", ["--print", "cfg", "-O"], directory, env).then(
          (text) => text.split(/\r?\n/).filter(Boolean),
          (error) => {
            log(`Could not read the host's cfgs: ${String(error)}`);
            return [];
          },
        )
      : cfgs;
    const compiled = hostMetadata
      ? hostTargets(actual)
      : editorTargets.filter((target) => target.procMacro);
    if (host.length) {
      for (const target of configured(compiled, actual, host, true)) {
        const editor = target.procMacro && macros.get(target.packageId);
        if (editor) {
          if (!hostMetadata && editor.cfg === target.cfg) continue;
          editor.semanticOwner = roots.length;
        }
        roots.push(target);
      }
    }
  }
  return { cfgForRoot, roots, buildInputs, workspaceRoot: root };
}

/** Cargo may select several targets. rust-analyzer draws cfgs from the first
 * and resolves dependencies for all of them. */
async function configuredTargets(
  root: string,
  config: string[],
  env: NodeJS.ProcessEnv,
): Promise<string[]> {
  try {
    const text = await run(
      "cargo",
      [
        ...config,
        "-Z",
        "unstable-options",
        "config",
        "get",
        "build.target",
        "--format",
        "json",
      ],
      root,
      { ...env, __CARGO_TEST_CHANNEL_OVERRIDE_DO_NOT_USE_THIS: "nightly" },
    );
    const configured: string | string[] = JSON.parse(text).build.target;
    return typeof configured === "string" ? [configured] : configured;
  } catch {
    return [];
  }
}

/**
 * rustc's cfgs as rust-analyzer asks for them: through cargo, which applies its
 * configuration's target and flags, else from rustc itself.
 */
async function printed(
  root: string | undefined,
  directory: string,
  config: string[],
  target: string[],
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
): Promise<string[]> {
  const lines = (text: string) => text.split(/\r?\n/).filter(Boolean);
  if (root !== undefined) {
    try {
      return lines(
        await run(
          "cargo",
          [
            ...config,
            "rustc",
            "--locked",
            "-Z",
            "unstable-options",
            "--print",
            "cfg",
            ...target,
            "--",
            "-O",
          ],
          root,
          // How rust-analyzer reaches cargo's unstable `--print`.
          { ...env, __CARGO_TEST_CHANNEL_OVERRIDE_DO_NOT_USE_THIS: "nightly" },
        ),
      );
    } catch {
      // A cargo that refuses the unstable flag, or a `Cargo.lock` out of date:
      // rustc answers, as for rust-analyzer.
    }
  }
  // The flags cargo would pass rustc, which may set cfgs of their own.
  const flags =
    env.CARGO_ENCODED_RUSTFLAGS?.split("\x1f") ??
    env.RUSTFLAGS?.split(/\s+/) ??
    [];
  try {
    return lines(
      await run(
        "rustc",
        ["--print", "cfg", "-O", ...target, ...flags.filter(Boolean)],
        directory,
        env,
      ),
    );
  } catch (error) {
    log(`Could not read the target's cfgs: ${(error as Error).message}`);
    return [];
  }
}
