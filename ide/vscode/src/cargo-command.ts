import { execFile } from "node:child_process";
import { copyFile, mkdtemp, rm } from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";

import { cached } from "./cache";

function beforeSeparator(args: string[]): string[] {
  const end = args.indexOf("--");
  return end < 0 ? args : args.slice(0, end);
}

function configOption(arg: string, index: number, args: string[]): boolean {
  return (
    arg === "--config" ||
    arg.startsWith("--config=") ||
    args[index - 1] === "--config"
  );
}

export function configArgs(args: string[]): string[] {
  return beforeSeparator(args).filter(configOption);
}

function strict(args: string[]): boolean {
  return beforeSeparator(args).some(
    (arg) => arg === "--locked" || arg === "--frozen",
  );
}

/** Cargo keeps only the last group of --config options if they straddle a subcommand. */
export function cargoArgs(args: string[], copy?: string): string[] {
  const options = [...beforeSeparator(args)];
  const forwarded = args.slice(options.length);
  const toolchain = options[0]?.startsWith("+") ? [options.shift()!] : [];
  const guard = copy
    ? ["--config", `resolver.lockfile-path=${JSON.stringify(copy)}`]
    : strict(options)
      ? []
      : ["--locked"];
  return [
    ...toolchain,
    ...options.filter(configOption),
    ...guard,
    ...options.filter((arg, index) => !configOption(arg, index, options)),
    ...forwarded,
  ];
}

interface Lockfile {
  path: string;
  canCopy: boolean;
}

/** A Cargo command selects its lock through its executable, working directory, manifest and configuration. */
export class CargoLocks {
  readonly paths = new Set<string>();
  private locations = new Map<string, Promise<Lockfile | undefined>>();
  private copies = new Map<Lockfile, string>();

  constructor(
    private env: NodeJS.ProcessEnv,
    private log: (line: string) => void,
  ) {}

  location(
    program: string,
    args: string[],
    directory: string,
    workspace?: string,
  ) {
    const options = beforeSeparator(args);
    const toolchain = options[0]?.startsWith("+") ? [options[0]] : [];
    const config = configArgs(options);
    const manifest: string[] = [];
    const cd: string[] = [];
    let cwd = directory;
    for (let i = 0; i < options.length; i++) {
      const arg = options[i];
      if (arg === "-C" || (arg.startsWith("-C") && arg.length > 2)) {
        const value =
          arg === "-C" ? options[++i] : arg.slice(2).replace(/^=/, "");
        if (value) {
          cd.push("-C", value);
          cwd = path.resolve(cwd, value);
        }
      } else if (arg === "--manifest-path" || arg === "-m") {
        if (options[i + 1]) manifest.push(arg, options[++i]);
      } else if (arg.startsWith("--manifest-path=") || /^-m.+/.test(arg)) {
        manifest.push(arg);
      }
    }
    const key = JSON.stringify([
      program,
      directory,
      toolchain,
      cd,
      config,
      manifest,
    ]);
    return cached(this.locations, key, () => {
      const probe = (args: string[]) =>
        execute(
          program,
          [...toolchain, ...cd, ...config, "-Zunstable-options", ...args],
          directory,
          {
            ...this.env,
            __CARGO_TEST_CHANNEL_OVERRIDE_DO_NOT_USE_THIS: "nightly",
          },
        );
      return this.find(cwd, workspace, manifest, probe).catch((error) => {
        this.log(`Could not locate the Cargo lockfile: ${String(error)}`);
        return undefined;
      });
    });
  }

  private async find(
    cwd: string,
    workspace: string | undefined,
    manifest: string[],
    probe: (args: string[]) => Promise<string>,
  ): Promise<Lockfile> {
    const version = /^cargo (\d+)\.(\d+)/.exec(await probe(["-V"]));
    // Cargo 1.97 stabilized resolver.lockfile-path. Earlier Cargo must use --locked.
    const canCopy =
      !!version &&
      (Number(version[1]) > 1 ||
        (Number(version[1]) === 1 && Number(version[2]) >= 97));
    let file: string | undefined;
    if (canCopy) {
      const get = (format: string[]) =>
        probe(["config", "get", "resolver.lockfile-path", ...format]);
      try {
        const value: string = JSON.parse(await get(["--format=json-value"]));
        file = value;
        if (!path.isAbsolute(value)) {
          const [plain, annotated] = await Promise.all([
            get([]),
            get(["--show-origin"]),
          ]);
          const origin = annotated
            .slice(plain.trimEnd().length + " # ".length)
            .trimEnd();
          // A config file resolves paths from its grandparent, a CLI or environment value from cwd.
          const base = path.isAbsolute(origin)
            ? path.dirname(path.dirname(origin))
            : cwd;
          file = path.resolve(base, value);
        }
      } catch (error) {
        if (
          !String(error).includes(
            "config value `resolver.lockfile-path` is not set",
          )
        )
          throw error;
      }
    }
    if (!file) {
      const root =
        workspace ??
        path.dirname(
          (
            await probe([
              "locate-project",
              "--workspace",
              "--message-format",
              "plain",
              ...manifest,
            ])
          ).trim(),
        );
      file = path.join(root, "Cargo.lock");
    }
    this.paths.add(file);
    return { path: file, canCopy };
  }

  /** Retry only in a writable private copy, retaining existing dependency pins. */
  async copy(
    lock: Lockfile | undefined,
    args: string[],
  ): Promise<string | undefined> {
    if (!lock?.canCopy || strict(args)) return undefined;
    const previous = this.copies.get(lock);
    if (previous) return previous;
    const directory = await mkdtemp(path.join(os.tmpdir(), "kaalang-lock-"));
    const file = path.join(directory, "Cargo.lock");
    try {
      await copyFile(lock.path, file);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
        await rm(directory, { recursive: true, force: true });
        throw error;
      }
    }
    this.copies.set(lock, file);
    return file;
  }

  async dispose() {
    for (const file of this.copies.values())
      await rm(path.dirname(file), { recursive: true, force: true }).catch(
        (error) =>
          this.log(
            `Could not remove the temporary Cargo.lock: ${error.message}`,
          ),
      );
  }
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
  return execute(program, args, directory, env);
}

function execute(
  program: string,
  args: string[],
  directory: string,
  env: NodeJS.ProcessEnv,
): Promise<string> {
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
