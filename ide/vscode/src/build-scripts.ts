// Reads the per-package cfgs and change inputs of Cargo's build scripts.

import { type ChildProcess, spawn } from "node:child_process";
import { open, readdir } from "node:fs/promises";
import * as path from "node:path";
import { createInterface } from "node:readline";

import type { Metadata } from "./metadata";

interface BuildScripts {
  /** Each package's target compilation and its compilation for a procedural macro on the host. */
  cfgs: Map<string, string[]>;
  hostCfgs: Map<string, string[]>;
  /** Absolute input files and directories, including paths that do not exist yet. */
  inputs: string[];
}

interface Output {
  cfgs: string[];
  outDir: string;
}

const CFG = /^ *((?:r#)?[a-zA-Z_][a-zA-Z0-9_]*)(?: *= *("[^"\r\n]*"))? *$/;

interface CargoMessages {
  outputs: Map<string, Output[]>;
  units: Map<string, Set<string>>;
}

/** An override with invocationStrategy=once supplies all projects in the window. */
const sharedRuns = new Map<string, Promise<CargoMessages>>();
const running = new Set<ChildProcess>();

/** A new revision reads its own: an older run would only hold Cargo's lock. */
export function forgetBuildScriptRuns(): void {
  sharedRuns.clear();
  for (const cargo of running) cargo.kill();
  running.clear();
}

/** Cargo reports the cfgs even for fresh scripts. Its saved stdout also lists their change inputs. */
export async function readBuildScripts(
  program: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv,
  metadata: Metadata,
  log: (line: string) => void,
  selectedTarget?: string,
  sharedRunKey?: string,
): Promise<BuildScripts> {
  const members = new Set(metadata.workspace_members);
  let pending = sharedRunKey ? sharedRuns.get(sharedRunKey) : undefined;
  if (!pending) {
    pending = buildMessages(
      program,
      args,
      cwd,
      env,
      log,
      sharedRunKey ? undefined : members,
    );
    if (sharedRunKey) sharedRuns.set(sharedRunKey, pending);
  }
  const { outputs, units } = await pending;
  const inputs = new Set<string>();
  const cfgs = new Map<string, string[]>();
  const hostCfgs = new Map<string, string[]>();
  localDependencies(metadata, members, inputs);
  for (const pkg of metadata.packages) {
    if (!members.has(pkg.id)) continue;
    const scripts = pkg.targets.filter((target) =>
      target.kind.includes("custom-build"),
    );
    for (const unit of units.get(pkg.id) ?? []) {
      await compileInputs(unit, metadata.workspace_root ?? cwd, inputs);
    }
    for (const script of scripts) inputs.add(path.resolve(script.src_path));
    const directory = path.dirname(pkg.manifest_path);
    const found = outputs.get(pkg.id);
    // A script that did not run declared nothing; its source is already an input.
    if (!found) continue;
    cfgs.set(pkg.id, selectOutput(found, false, selectedTarget).cfgs);
    hostCfgs.set(pkg.id, selectOutput(found, true, selectedTarget).cfgs);
    for (const output of found)
      await readInputs(output.outDir, directory, inputs);
  }
  return { cfgs, hostCfgs, inputs: [...inputs] };
}

/** Only local dependencies that compile into a workspace build script can change its flags. */
function localDependencies(
  metadata: Metadata,
  members: Set<string>,
  inputs: Set<string>,
) {
  const directories = new Map(
    metadata.packages
      .filter((pkg) => pkg.source === null)
      .map((pkg) => [path.dirname(pkg.manifest_path), pkg]),
  );
  const resolved = new Map(
    metadata.resolve?.nodes.map((node) => [
      node.id,
      new Set(node.deps.map((dep) => dep.pkg)),
    ]),
  );
  const dependencies = new Set<string>();
  function watchDependencies(
    pkg: Metadata["packages"][number],
    buildOnly: boolean,
  ) {
    for (const dep of pkg.dependencies) {
      if (!dep.path || (buildOnly ? dep.kind !== "build" : dep.kind === "dev"))
        continue;
      const directory = path.resolve(dep.path);
      const dependency = directories.get(directory);
      if (
        dependencies.has(directory) ||
        (dependency && resolved.get(pkg.id)?.has(dependency.id) === false)
      )
        continue;
      dependencies.add(directory);
      inputs.add(directory);
      if (dependency) watchDependencies(dependency, false);
    }
  }
  for (const pkg of metadata.packages) {
    if (
      members.has(pkg.id) &&
      pkg.targets.some((target) => target.kind.includes("custom-build"))
    ) {
      watchDependencies(pkg, true);
    }
  }
}

async function buildMessages(
  program: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv,
  log: (line: string) => void,
  members?: Set<string>,
): Promise<CargoMessages> {
  const outputs = new Map<string, Output[]>();
  const units = new Map<string, Set<string>>();
  let tail = "";
  let spawned: ChildProcess | undefined;
  try {
    const cargo = spawn(program, args, {
      cwd,
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    spawned = cargo;
    running.add(cargo);
    cargo.stderr.setEncoding("utf8");
    cargo.stderr.on("data", (chunk: string) => {
      tail = (tail + chunk).slice(-4096);
    });
    const lines = createInterface({ input: cargo.stdout, crlfDelay: Infinity });
    lines.on("line", (line: string) => {
      if (!line.startsWith("{")) return;
      let message;
      try {
        message = JSON.parse(line);
      } catch {
        return;
      }
      if (
        message?.reason === "compiler-message" &&
        message.message?.level === "error" &&
        typeof message.message.message === "string"
      ) {
        tail = (tail + message.message.message + "\n").slice(-4096);
        return;
      }
      const id = message?.package_id;
      if (typeof id !== "string" || (members && !members.has(id))) return;
      if (
        message.reason === "compiler-artifact" &&
        Array.isArray(message.target?.kind) &&
        message.target.kind.includes("custom-build") &&
        Array.isArray(message.filenames)
      ) {
        const directories = units.get(id) ?? new Set<string>();
        for (const file of message.filenames) {
          if (typeof file === "string" && path.isAbsolute(file))
            directories.add(path.dirname(file));
        }
        units.set(id, directories);
        return;
      }
      if (
        message.reason !== "build-script-executed" ||
        !Array.isArray(message.cfgs)
      )
        return;
      const flags: string[] = [];
      for (const cfg of message.cfgs) {
        const atom = typeof cfg === "string" && CFG.exec(cfg);
        if (atom)
          flags.push(atom[2] === undefined ? atom[1] : `${atom[1]}=${atom[2]}`);
        else
          log(
            `Left out invalid build-script cfg ${JSON.stringify(cfg)} for ${message.package_id}.`,
          );
      }
      const found = outputs.get(message.package_id) ?? [];
      found.push({
        cfgs: flags,
        outDir: typeof message.out_dir === "string" ? message.out_dir : "",
      });
      outputs.set(message.package_id, found);
    });
    await new Promise<void>((resolve, reject) => {
      cargo.once("error", reject);
      cargo.once("close", (code, signal) => {
        if (code === 0) resolve();
        else
          reject(
            new Error(
              tail.trim() || `${program} stopped with ${signal ?? code}`,
            ),
          );
      });
    });
  } catch (error) {
    // Cargo can finish local scripts before an unrelated package fails.
    if (!spawned?.killed)
      log(
        `Could not finish reading build-script cfgs: ${(error as Error).message}`,
      );
  } finally {
    if (spawned) running.delete(spawned);
  }
  return { outputs, units };
}

async function compileInputs(
  unit: string,
  workspace: string,
  inputs: Set<string>,
) {
  // Each script compiles in its own unit directory. rustc writes one empty
  // make target for each source it read, including mod and include! inputs.
  const files = await readdir(unit).catch(() => []);
  for (const file of files.filter((file) => file.endsWith(".d"))) {
    await linesIn(path.join(unit, file), (line) => {
      if (
        line.endsWith(":") &&
        !line.startsWith("# ") &&
        !line.includes(": ")
      ) {
        inputs.add(
          path.resolve(workspace, line.slice(0, -1).replaceAll("\\ ", " ")),
        );
      }
    });
  }
}

/** Each Cargo layout puts a profile and its build/ directory below the host or target build root. */
function buildRoot(outDir: string): string | undefined {
  const unit = path.dirname(outDir);
  const parent = path.dirname(unit);
  // In the newer layout the unit is build/<package>/<hash>, in the older one build/<package>-<hash>.
  const build = /^[a-f0-9]+$/i.test(path.basename(unit))
    ? path.dirname(parent)
    : parent;
  return path.basename(build) === "build"
    ? path.dirname(path.dirname(build))
    : undefined;
}

function selectOutput(
  outputs: Output[],
  procMacro: boolean,
  target?: string,
): Output {
  if (outputs.length > 1 && target) {
    const triple = path.basename(target, ".json");
    const roots = outputs.map((output) => buildRoot(output.outDir));
    const targets = roots.filter(
      (root): root is string =>
        root !== undefined && path.basename(root) === triple,
    );
    const targetRoot =
      targets.find((root) => roots.includes(path.dirname(root))) ?? targets[0];
    if (targetRoot) {
      const index = roots.indexOf(
        procMacro ? path.dirname(targetRoot) : targetRoot,
      );
      if (index >= 0) return outputs[index];
    }
  }
  return outputs[outputs.length - 1];
}

async function readInputs(
  outDir: string,
  directory: string,
  inputs: Set<string>,
): Promise<void> {
  if (path.isAbsolute(outDir)) {
    // The raw stdout moved in Cargo's build-unit layout. Both paths are
    // relative to the out_dir of this exact run, also with a custom build-dir.
    for (const name of ["output", path.join("run", "stdout")]) {
      let narrowed = false;
      if (
        await linesIn(path.join(path.dirname(outDir), name), (line) => {
          const rerun = /^cargo::?rerun-if-(changed|env-changed)=(.*)$/.exec(
            line.trimEnd(),
          );
          if (!rerun) return;
          narrowed = true;
          if (rerun[1] === "changed")
            inputs.add(path.resolve(directory, rerun[2]));
        })
      ) {
        if (!narrowed) inputs.add(directory);
        return;
      }
    }
  }
  inputs.add(directory);
}

/** Reads the text records of an optional Cargo artifact. */
async function linesIn(
  file: string,
  visit: (line: string) => void,
): Promise<boolean> {
  try {
    const handle = await open(file, "r");
    try {
      for await (const line of handle.readLines()) visit(line);
      return true;
    } finally {
      await handle.close();
    }
  } catch {
    return false;
  }
}
