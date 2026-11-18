// The cfg set rust-analyzer analyzes a file with, as `rustc --print cfg`
// prints it, so that the diagram counts the code the editor shows as active.
// rust-analyzer exposes no structured cfg set, so this one is read as it reads its own:
// rustc's cfgs for the target with cargo's configuration applied, the extra
// flags of a local crate and its settings, then the package's active features.

import { execFile } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import { fileCfgs } from "./cfgs";
import {
  type CargoRoot as CargoTarget,
  type Metadata,
  workspaceTargets,
} from "./metadata";

/** Each crate reads its module tree with its package's active features. */
export interface CargoRoot extends CargoTarget {
  cfg: string;
}

/** The settings of rust-analyzer that decide the set. */
const SECTIONS = ["rust-analyzer.cargo", "rust-analyzer.cfg"];

/** What rust-analyzer's settings ask of the set. */
interface Options {
  target: string | null;
  features: string[] | "all";
  noDefaultFeatures: boolean;
  noDeps: boolean;
  metadataArgs: string[];
  configPath?: string;
  cfgs: unknown[];
  setTest: boolean;
  extraEnv: Record<string, string | null>;
}

/** What a cargo workspace gives every file in it. */
interface Workspace {
  /** rustc's cfgs for the compilation target. */
  cfgs: string[];
  metadata: Metadata | undefined;
  roots: CargoRoot[];
}

/** The root of each source directory's cargo workspace, if it has one. */
const roots = new Map<string, Promise<string | undefined>>();
/** Each workspace read, by its root and the options it was read with. */
const workspaces = new Map<string, Promise<Workspace>>();

/** Whether Restricted Mode was logged: once is enough. */
let restricted = false;

/** Whether `event` changes a setting the set depends on. */
export function affectsCfg(event: vscode.ConfigurationChangeEvent): boolean {
  return SECTIONS.some((section) => event.affectsConfiguration(section));
}

/** Forgets what was read, after a manifest or a cargo configuration changes. */
export function forgetCfg(): void {
  roots.clear();
  workspaces.clear();
}

/** The additional Cargo configuration selected for the rust-analyzer session. */
export function cargoConfig(): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("rust-analyzer")
    .get<string | null>("cargo.configPath");
  return configured
    ? path.resolve(
        vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd(),
        configured,
      )
    : undefined;
}

/**
 * The set for `file`; whatever cannot be read is left out and `log`ged. Empty
 * in Restricted Mode: cargo and rustc run whatever binaries the workspace's
 * configuration names.
 */
export async function target(file: vscode.Uri, log: (line: string) => void) {
  if (!vscode.workspace.isTrusted) {
    if (!restricted) {
      restricted = true;
      log(
        "Restricted Mode: the diagram is drawn without the target's cfg set.",
      );
    }
    return { cfg: "", roots: [], cfgForRoot: () => "" };
  }
  const settings = vscode.workspace.getConfiguration("rust-analyzer");
  const options: Options = {
    target: settings.get<string | null>("cargo.target") ?? null,
    features: settings.get<string[] | "all">("cargo.features") ?? [],
    noDefaultFeatures:
      settings.get<boolean>("cargo.noDefaultFeatures") ?? false,
    noDeps: settings.get<boolean>("cargo.noDeps") ?? false,
    metadataArgs: settings.get<string[]>("cargo.metadataExtraArgs") ?? [],
    configPath: cargoConfig(),
    // rust-analyzer's own default, for when it is not installed.
    cfgs: settings.get<unknown[]>("cargo.cfgs") ?? ["debug_assertions", "miri"],
    setTest: settings.get<boolean>("cfg.setTest") ?? true,
    extraEnv:
      settings.get<Record<string, string | null>>("cargo.extraEnv") ?? {},
  };
  const env = { ...process.env };
  for (const [name, value] of Object.entries(options.extraEnv)) {
    if (value === null) {
      delete env[name];
    } else {
      env[name] = value;
    }
  }
  const source = workspaceFile(file);
  const root =
    source === undefined
      ? undefined
      : await once(roots, path.dirname(source), () =>
          locate(path.dirname(source), env, log),
        );
  const workspace = await once(
    workspaces,
    JSON.stringify([root ?? null, options]),
    () => read(root, options, env, log),
  );
  // shortcut: no cfg a build script sets with `cargo:rustc-cfg`, which
  // rust-analyzer learns by running build scripts; run them too if flows behind
  // such cfgs turn up.
  const cfgForRoot = (path: string | undefined) =>
    fileCfgs(workspace.cfgs, workspace.metadata, path, options, log);
  return {
    cfg: cfgForRoot(source),
    roots: workspace.roots,
    cfgForRoot,
  };
}

/** What `map` holds at `key`, made first if it holds nothing. */
function once<T>(
  map: Map<string, Promise<T>>,
  key: string,
  make: () => Promise<T>,
): Promise<T> {
  let value = map.get(key);
  if (value === undefined) {
    value = make();
    map.set(key, value);
  }
  return value;
}

/**
 * The real path of `file` when it is one of the workspace's own. Cargo runs
 * only for those: in a dependency's or the standard library's sources it would
 * write a `Cargo.lock`. Cargo names packages by their real paths.
 */
function workspaceFile(file: vscode.Uri): string | undefined {
  if (
    file.scheme !== "file" ||
    vscode.workspace.getWorkspaceFolder(file) === undefined
  ) {
    return undefined;
  }
  try {
    return fs.realpathSync(file.fsPath);
  } catch {
    return file.fsPath;
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
): Promise<Workspace> {
  const directory =
    root ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
  const config = options.configPath ? ["--config", options.configPath] : [];
  const targets = options.target
    ? [options.target]
    : root === undefined
      ? []
      : await configuredTargets(root, config, env);
  const target = targets.length > 0 ? ["--target", targets[0]] : [];
  const cfgs = await printed(root, directory, config, target, env, log);
  let metadata: Metadata | undefined;
  if (root !== undefined) {
    const features =
      options.features === "all"
        ? ["--all-features"]
        : options.features.length > 0
          ? ["--features", options.features.join(",")]
          : [];
    const noDeps = options.noDeps || options.metadataArgs.includes("--no-deps");
    const args = [
      ...config,
      "metadata",
      "--locked",
      "--format-version",
      "1",
      ...targets.flatMap((triple) => ["--filter-platform", triple]),
      ...features,
      ...(options.noDefaultFeatures ? ["--no-default-features"] : []),
      ...options.metadataArgs.filter(
        (arg) => arg !== "--locked" && arg !== "--no-deps",
      ),
    ];
    try {
      metadata = JSON.parse(
        await run("cargo", noDeps ? [...args, "--no-deps"] : args, root, env),
      );
    } catch (error) {
      log(`Could not read the features: ${(error as Error).message}`);
      // Even without a current lock file, cargo can list the workspace's own targets.
      if (!noDeps) {
        try {
          metadata = JSON.parse(
            await run("cargo", [...args, "--no-deps"], root, env),
          );
        } catch {
          /* The full metadata error above already names the workspace. */
        }
      }
    }
  }
  const packageCfg = new Map<string, string>();
  for (const pkg of metadata?.packages ?? []) {
    if (metadata?.workspace_members.includes(pkg.id)) {
      packageCfg.set(
        pkg.name,
        fileCfgs(cfgs, metadata, pkg.manifest_path, options, log),
      );
    }
  }
  const roots = metadata
    ? workspaceTargets(metadata).map((target) => ({
        ...target,
        cfg: packageCfg.get(target.package)!,
        root: vscode.Uri.file(target.root).toString(),
      }))
    : [];
  return { cfgs, metadata, roots };
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
    // Cargo reports an error when the setting is absent.
    const version = await run("rustc", ["-vV"], root, env).catch(() => "");
    const host = /^host: (.+)$/m.exec(version)?.[1];
    return host ? [host] : [];
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

/**
 * Runs `command` in `directory` and answers what it printed: the rustc or the
 * cargo `RUSTC` or `CARGO` names, as cargo itself runs, else the one on `PATH`.
 */
function run(
  command: "rustc" | "cargo",
  args: string[],
  directory: string,
  env: NodeJS.ProcessEnv,
): Promise<string> {
  const program = (command === "rustc" ? env.RUSTC : env.CARGO) || command;
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
