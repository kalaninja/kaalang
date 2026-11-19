import * as path from "node:path";

import { readBuildScripts } from "./build-scripts";
import { CargoLocks, cargoArgs, configArgs, run } from "./cargo-command";
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
    ...configArgs(options.extraArgs),
    "metadata",
    "--format-version",
    "1",
    ...features,
    ...options.extraArgs.filter(
      (arg) =>
        ["--offline", "--frozen", "--locked"].includes(arg) &&
        !options.metadataArgs.includes(arg),
    ),
    ...options.metadataArgs.filter((arg) => arg !== "--no-deps"),
  ];
  const platforms = targets.length ? targets : await hostPlatform(root, env);
  let metadataCopy: string | undefined;
  const readMetadata = async (
    skipDeps: boolean,
    selected = platforms,
  ): Promise<Metadata> =>
    JSON.parse(
      await run(
        "cargo",
        cargoArgs(
          [
            ...metadataArgs,
            ...selected.flatMap((triple) => ["--filter-platform", triple]),
            ...(skipDeps ? ["--no-deps"] : []),
          ],
          metadataCopy,
        ),
        root,
        env,
      ),
    );
  let metadata: Metadata;
  let metadataError: string | undefined;
  try {
    metadata = await readMetadata(noDeps);
  } catch (error) {
    metadataError = `Could not read the package features: ${(error as Error).message}`;
    // Cargo can list local targets without resolving dependencies or updating the lock.
    const local = noDeps
      ? undefined
      : await readMetadata(true).catch(() => undefined);
    if (!local) {
      log(metadataError);
      return {};
    }
    metadata = local;
  }
  const project: CargoProject = { metadata };
  const members = new Set(metadata.workspace_members);
  const scripts =
    options.buildScripts &&
    metadata.packages.some(
      (pkg) =>
        members.has(pkg.id) &&
        pkg.targets.some((target) => target.kind.includes("custom-build")),
    );
  const cargoProgram = env.CARGO || "cargo";
  const locks = new CargoLocks(env, log);
  const lockedMetadata = !!metadata.resolve;
  let inputs: string[] = [];
  let ignored: string[] = [];
  try {
    const metaLock = await locks.location(
      cargoProgram,
      metadataArgs,
      root,
      metadata.workspace_root,
    );
    if (!metadata.resolve && !noDeps) {
      metadataCopy = await locks.copy(metaLock, metadataArgs);
      if (metadataCopy) {
        metadata = await readMetadata(false).catch((error) => {
          metadataError = String(error);
          return metadata;
        });
        project.metadata = metadata;
      }
    }
    if (!metadata.resolve && metadataError) log(metadataError);
    if (!scripts) return project;

    const targetDir =
      options.targetDir === true
        ? path.join(metadata.target_directory, "rust-analyzer")
        : options.targetDir
          ? path.resolve(root, options.targetDir)
          : undefined;
    ignored = [
      metadata.target_directory,
      metadata.build_directory,
      targetDir,
    ].filter((dir): dir is string => !!dir);
    const custom = options.buildCommand?.length
      ? options.buildCommand
      : undefined;
    // Compile only what runs at build time, so unfinished application code can still be drawn.
    const args = custom?.slice(1) ?? [
      ...config,
      "check",
      "--quiet",
      "--workspace",
      "--message-format=json",
      ...options.extraArgs,
      "--compile-time-deps",
      "-Zunstable-options",
      "--all-targets",
      "--keep-going",
      ...(options.target ? ["--target", options.target] : []),
      ...features,
      ...(targetDir ? ["--target-dir", targetDir] : []),
    ];
    const cargo =
      !custom ||
      /^(?:cargo|cargo\.exe)$/i.test(path.basename(custom[0])) ||
      custom[0] === env.CARGO;
    const program = custom?.[0] ?? cargoProgram;
    const directory = custom ? (options.buildDirectory ?? root) : root;
    let copy: string | undefined;
    if (cargo) {
      const buildLock = await locks.location(program, args, directory);
      // A successful locked metadata read already validated this authored lock.
      copy =
        lockedMetadata && metaLock === buildLock
          ? undefined
          : await locks.copy(buildLock, args);
    } else {
      // An arbitrary override takes its own arguments. Nested Cargo can still
      // use its normal configuration and a private lock through the environment.
      const wrapperLock = await locks.location(cargoProgram, [], directory);
      copy = await locks.copy(wrapperLock ?? metaLock, []);
    }
    const build = await readBuildScripts(
      program,
      cargo ? cargoArgs(args, copy) : args,
      directory,
      {
        ...env,
        ...(copy ? { CARGO_RESOLVER_LOCKFILE_PATH: copy } : {}),
        ...(!custom
          ? { __CARGO_TEST_CHANNEL_OVERRIDE_DO_NOT_USE_THIS: "nightly" }
          : {}),
      },
      metadata,
      log,
      targets[0],
      custom && options.buildDirectory
        ? JSON.stringify([options.buildDirectory, custom, env])
        : undefined,
    );
    project.buildCfgs = build.cfgs;
    if (options.rebuildOnSave) inputs = build.inputs;
  } catch (error) {
    log(`Could not read build-script cfgs: ${(error as Error).message}`);
  } finally {
    await locks.dispose();
    // An alternate lock can live beyond the Cargo.toml/Cargo.lock workspace watcher.
    if (locks.paths.size || inputs.length)
      project.buildInputs = {
        paths: [...new Set([...locks.paths, ...inputs])],
        ignored,
      };
  }
  return project;
}

async function hostPlatform(
  root: string,
  env: NodeJS.ProcessEnv,
): Promise<string[]> {
  const version = await run("rustc", ["-vV"], root, env).catch(() => "");
  const host = /^host: (.+)$/m.exec(version)?.[1];
  return host ? [host] : [];
}
