import { execFile } from "node:child_process";
import { copyFile, mkdtemp, rm } from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";

import { readBuildScripts } from "./build-scripts";
import type { Metadata } from "./metadata";

export interface CargoOptions {
  target: string | null;
  features: string[] | "all";
  noDefaultFeatures: boolean;
  noDeps: boolean;
  metadataArgs: string[];
  buildScripts: boolean;
  rebuildOnSave: boolean;
  buildCommand: string[] | null;
  /** Working directory of a build-script override configured to run once. */
  buildDirectory?: string;
  extraArgs: string[];
  targetDir: boolean | string | null;
}

/** The disk inputs Cargo uses and its own output directories. */
export interface BuildInputs {
  paths: string[];
  ignored: string[];
}

interface CargoProject {
  metadata?: Metadata;
  buildCfgs?: Map<string, string[]>;
  buildInputs?: BuildInputs;
}

/** Read package membership, features and build-script outputs for the same Cargo configuration. */
export async function readCargoProject(
  root: string,
  targets: string[],
  config: string[],
  options: CargoOptions,
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
): Promise<CargoProject> {
  const features: string[] = [];
  if (options.features === "all") features.push("--all-features");
  else if (options.features.length)
    features.push("--features", options.features.join(","));
  if (options.noDefaultFeatures) features.push("--no-default-features");
  const noDeps = options.noDeps || options.metadataArgs.includes("--no-deps");
  const metadataArgs = [
    ...config,
    "metadata",
    "--format-version",
    "1",
    ...targets.flatMap((triple) => ["--filter-platform", triple]),
    ...features,
    ...options.extraArgs.filter(
      (arg) => arg === "--offline" || arg === "--frozen",
    ),
    ...options.metadataArgs.filter(
      (arg) => arg !== "--no-deps" && arg !== "--locked",
    ),
  ];
  let locked = ["--locked"];
  let cargoEnv = env;
  const readMetadata = async (skipDeps: boolean): Promise<Metadata> =>
    JSON.parse(
      await run(
        "cargo",
        [...metadataArgs, ...locked, ...(skipDeps ? ["--no-deps"] : [])],
        root,
        cargoEnv,
      ),
    );
  let metadata: Metadata;
  try {
    metadata = await readMetadata(noDeps);
  } catch (error) {
    log(`Could not read the package features: ${(error as Error).message}`);
    // Cargo can list local targets without resolving dependencies or updating the lock.
    if (noDeps) return {};
    const local = await readMetadata(true).catch(() => undefined);
    if (!local) return {};
    metadata = local;
  }
  const project: CargoProject = { metadata };
  const members = new Set(metadata.workspace_members);
  if (
    !options.buildScripts ||
    !metadata.packages.some(
      (pkg) =>
        members.has(pkg.id) &&
        pkg.targets.some((target) => target.kind.includes("custom-build")),
    )
  )
    return project;

  let temp: string | undefined;
  try {
    const requireLock = [...options.extraArgs, ...options.metadataArgs].some(
      (arg) => arg === "--locked" || arg === "--frozen",
    );
    if (!metadata.resolve && !requireLock) {
      temp = await privateLockDirectory(root, env);
      if (temp) {
        locked = [];
        cargoEnv = {
          ...env,
          CARGO_RESOLVER_LOCKFILE_PATH: path.join(temp, "Cargo.lock"),
        };
        if (!noDeps) {
          metadata = await readMetadata(false).catch(() => metadata);
          project.metadata = metadata;
        }
      }
    }
    const targetDir =
      options.targetDir === true
        ? path.join(metadata.target_directory, "rust-analyzer")
        : options.targetDir
          ? path.resolve(root, options.targetDir)
          : undefined;
    const custom = options.buildCommand?.length
      ? options.buildCommand
      : undefined;
    // Compile only what runs at build time, so unfinished application code can still be drawn.
    const args = custom?.slice(1) ?? [
      ...config,
      "check",
      ...locked,
      "--quiet",
      "--workspace",
      "--message-format=json",
      ...options.extraArgs.filter((arg) => arg !== "--locked"),
      "--compile-time-deps",
      "-Zunstable-options",
      "--all-targets",
      "--keep-going",
      ...(options.target ? ["--target", options.target] : []),
      ...features,
      ...(targetDir ? ["--target-dir", targetDir] : []),
    ];
    const { cfgs, inputs } = await readBuildScripts(
      custom?.[0] ?? (env.CARGO || "cargo"),
      args,
      custom ? (options.buildDirectory ?? root) : root,
      custom
        ? cargoEnv
        : {
            ...cargoEnv,
            __CARGO_TEST_CHANNEL_OVERRIDE_DO_NOT_USE_THIS: "nightly",
          },
      metadata,
      log,
      targets[0],
      custom && options.buildDirectory
        ? JSON.stringify([options.buildDirectory, custom, env])
        : undefined,
    );
    project.buildCfgs = cfgs;
    if (options.rebuildOnSave)
      project.buildInputs = {
        paths: inputs,
        ignored: [
          metadata.target_directory,
          metadata.build_directory,
          targetDir,
        ].filter((dir): dir is string => !!dir),
      };
  } catch (error) {
    log(`Could not read build-script cfgs: ${(error as Error).message}`);
  } finally {
    if (temp)
      await rm(temp, { recursive: true, force: true }).catch((error) =>
        log(`Could not remove the temporary Cargo.lock: ${error.message}`),
      );
  }
  return project;
}

/** A private copy lets Cargo resolve a new or outdated lock without writing the developer's file. */
async function privateLockDirectory(
  root: string,
  env: NodeJS.ProcessEnv,
): Promise<string | undefined> {
  const version = await run("cargo", ["-V"], root, env);
  const match = /^cargo (\d+)\.(\d+)/.exec(version);
  // resolver.lockfile-path was stabilized in Cargo 1.97. Older Cargo keeps --locked.
  if (!match || (Number(match[1]) === 1 && Number(match[2]) < 97))
    return undefined;
  const directory = await mkdtemp(path.join(os.tmpdir(), "kaalang-lock-"));
  try {
    await copyFile(
      path.resolve(root, env.CARGO_RESOLVER_LOCKFILE_PATH ?? "Cargo.lock"),
      path.join(directory, "Cargo.lock"),
    );
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
      await rm(directory, { recursive: true, force: true });
      throw error;
    }
  }
  return directory;
}

/**
 * Runs `command` in `directory` and answers what it printed: the rustc or the
 * cargo `RUSTC` or `CARGO` names, as cargo itself runs, else the one on `PATH`.
 */
export function run(
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
