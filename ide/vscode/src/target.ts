// The cfg set rust-analyzer analyzes a file with, as `rustc --print cfg`
// prints it, so that the diagram counts the code the editor shows as active.
// rust-analyzer exposes no such set, so this one is read as it reads its own:
// rustc's cfgs for the target with cargo's configuration applied, the extra
// cfgs of its settings, then the features cargo enables for the file's package
// and, for a workspace crate, `rust_analyzer` and `test`.

import { execFile } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import { addPackageCfgs, applyExtraCfgs, type Metadata } from "./cfgs";

/** The settings of rust-analyzer that decide the set. */
const SECTIONS = ["rust-analyzer.cargo", "rust-analyzer.cfg"];

/** What rust-analyzer's settings ask of the set. */
interface Options {
  target: string | null;
  features: string[] | "all";
  noDefaultFeatures: boolean;
  cfgs: unknown[];
  setTest: boolean;
  extraEnv: Record<string, string | null>;
}

/** What a cargo workspace gives every file in it. */
interface Workspace {
  /** rustc's cfgs with the extra ones applied. */
  cfgs: string[];
  metadata: Metadata | undefined;
}

/** The root of each source directory's cargo workspace, if it has one. */
const roots = new Map<string, Promise<string | undefined>>();
/** Each workspace read, by its root and the options it was read with. */
const workspaces = new Map<string, Promise<Workspace>>();

/** Whether `event` changes a setting the set depends on. */
export function affectsCfg(event: vscode.ConfigurationChangeEvent): boolean {
  return SECTIONS.some((section) => event.affectsConfiguration(section));
}

/** Forgets what was read, after a manifest or a cargo configuration changes. */
export function forgetCfg(): void {
  roots.clear();
  workspaces.clear();
}

/** The set for `file`; whatever cannot be read is left out and `log`ged. */
export async function cfg(
  file: vscode.Uri,
  log: (line: string) => void,
): Promise<string> {
  const settings = vscode.workspace.getConfiguration("rust-analyzer", file);
  const options: Options = {
    target: settings.get<string | null>("cargo.target") ?? null,
    features: settings.get<string[] | "all">("cargo.features") ?? [],
    noDefaultFeatures:
      settings.get<boolean>("cargo.noDefaultFeatures") ?? false,
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
  const lines = new Set(workspace.cfgs);
  // shortcut: no cfg a build script sets with `cargo:rustc-cfg`, which
  // rust-analyzer learns by running build scripts; run them too if flows behind
  // such cfgs turn up.
  if (source !== undefined && workspace.metadata !== undefined) {
    addPackageCfgs(lines, workspace.metadata, source, options.setTest);
  }
  return [...lines].join("\n");
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
  const target = options.target ? ["--target", options.target] : [];
  const lines = new Set(await printed(root, directory, target, env, log));
  applyExtraCfgs(lines, options.cfgs, log);
  let metadata: Metadata | undefined;
  if (root !== undefined) {
    try {
      const triple =
        options.target ??
        (await run("rustc", ["-vV"], directory, env)).match(
          /^host: (.+)$/m,
        )?.[1] ??
        "";
      const features =
        options.features === "all"
          ? ["--all-features"]
          : options.features.length > 0
            ? ["--features", options.features.join(",")]
            : [];
      metadata = JSON.parse(
        await run(
          "cargo",
          [
            "metadata",
            // Cargo writes no lock file into a crate it only reads.
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            triple,
            ...features,
            ...(options.noDefaultFeatures ? ["--no-default-features"] : []),
          ],
          root,
          env,
        ),
      ) as Metadata;
    } catch (error) {
      log(`Could not read the features: ${(error as Error).message}`);
    }
  }
  return { cfgs: [...lines], metadata };
}

/**
 * rustc's cfgs as rust-analyzer asks for them: through cargo, which applies its
 * configuration's target and flags, else from rustc itself.
 */
async function printed(
  root: string | undefined,
  directory: string,
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
