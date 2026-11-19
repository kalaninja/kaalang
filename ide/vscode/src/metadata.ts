import * as path from "node:path";

/** The parts of `cargo metadata --format-version 1` the diagram reads. */
interface Package {
  id: string;
  name: string;
  manifest_path: string;
  source: string | null;
  dependencies: {
    name: string;
    rename: string | null;
    kind: string | null;
    path?: string;
    optional?: boolean;
    target?: string | null;
  }[];
  targets: {
    name: string;
    kind: string[];
    src_path: string;
    "required-features"?: string[];
  }[];
}

export interface Metadata {
  packages: Package[];
  workspace_root: string;
  workspace_members: string[];
  target_directory: string;
  build_directory?: string;
  resolve: {
    nodes: {
      id: string;
      features: string[];
      deps: {
        name: string;
        pkg: string;
        dep_kinds: { kind: string | null }[];
      }[];
    }[];
  } | null;
}

/** A package target under its Rust crate name, with the group its cards occupy. */
export interface CargoRoot {
  package: string;
  group: number;
  name: string;
  root: string;
  procMacro?: true;
  packageId: string;
  directory: string;
  library: boolean;
  dependencies: [string, string][];
}

/** The extern crate names Cargo makes available to each package. */
function dependencyGraph(metadata: Metadata) {
  if (metadata.resolve)
    return new Map(metadata.resolve.nodes.map((node) => [node.id, node.deps]));

  // --no-deps still reports the local packages' manifests and their path
  // dependencies. Only unconditional, non-optional edges are certain here.
  const libraries = new Map(
    metadata.packages.flatMap((pkg) => {
      const lib = pkg.targets.find((target) =>
        target.kind.some((kind) =>
          [
            "lib",
            "rlib",
            "proc-macro",
            "dylib",
            "cdylib",
            "staticlib",
          ].includes(kind),
        ),
      );
      return lib && pkg.manifest_path
        ? [[path.dirname(pkg.manifest_path), { pkg, lib }] as const]
        : [];
    }),
  );
  return new Map(
    metadata.packages.map((pkg) => [
      pkg.id,
      (pkg.dependencies ?? []).flatMap((dep) => {
        const found = dep.path && libraries.get(path.resolve(dep.path));
        if (!found || found.pkg.name !== dep.name || dep.optional || dep.target)
          return [];
        return [
          {
            name: (dep.rename ?? found.lib.name).replaceAll("-", "_"),
            pkg: found.pkg.id,
            dep_kinds: [{ kind: dep.kind }],
          },
        ];
      }),
    ]),
  );
}

/** The selected targets' source roots, including only enabled workspace targets. */
export function workspaceTargets(
  metadata: Metadata,
  graph = dependencyGraph(metadata),
): CargoRoot[] {
  const members = new Set(metadata.workspace_members);
  const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve?.nodes.map((node) => [node.id, node]));
  const roots: CargoRoot[] = [];
  for (const pkg of packages.values()) {
    if (!members.has(pkg.id)) {
      continue;
    }
    const node = nodes.get(pkg.id);
    function enabled(required: string): boolean {
      const slash = required.indexOf("/");
      if (slash === -1) {
        return node?.features.includes(required) ?? false;
      }
      // Cargo spells this with the manifest's dependency name, which may be
      // renamed or differ from its library target's name.
      const name = required.slice(0, slash);
      const feature = required.slice(slash + 1);
      return (
        node?.deps.some((edge) => {
          const dependency = packages.get(edge.pkg);
          if (!dependency || !nodes.get(edge.pkg)?.features.includes(feature)) {
            return false;
          }
          const declared = pkg.dependencies.find(
            (candidate) =>
              (candidate.rename ?? candidate.name) === name &&
              candidate.name === dependency.name,
          );
          if (!declared) {
            return false;
          }
          if (declared.rename !== null) {
            return declared.rename.replaceAll("-", "_") === edge.name;
          }
          return dependency.targets.some(
            (target) =>
              target.name.replaceAll("-", "_") === edge.name &&
              target.kind.some((kind) =>
                ["lib", "rlib", "dylib", "proc-macro"].includes(kind),
              ),
          );
        }) ?? false
      );
    }
    for (const target of pkg.targets) {
      if (
        target.kind.includes("custom-build") ||
        !(target["required-features"] ?? []).every(enabled)
      ) {
        continue;
      }
      const group =
        ["test", "example", "bench"].findIndex((kind) =>
          target.kind.includes(kind),
        ) + 1;
      roots.push({
        package: pkg.name,
        group,
        directory: path.dirname(pkg.manifest_path),
        packageId: pkg.id,
        library: group === 0 && !target.kind.includes("bin"),
        dependencies: (graph.get(pkg.id) ?? [])
          .filter((dep) =>
            dep.dep_kinds.some(
              ({ kind }) => kind === null || (kind === "dev" && group !== 0),
            ),
          )
          .map((dep) => [dep.name, dep.pkg]),
        name: target.name.replaceAll("-", "_"),
        root: target.src_path,
        ...(target.kind.includes("proc-macro") ? { procMacro: true } : {}),
      });
    }
  }
  return roots;
}

/** Workspace libraries compiled for a procedural macro, including its transitive normal dependencies. */
export function hostTargets(metadata: Metadata): CargoRoot[] {
  const graph = dependencyGraph(metadata);
  const targets = workspaceTargets(metadata, graph);
  const pending = targets
    .filter((root) => root.procMacro)
    .map((root) => root.packageId);
  const reached = new Set<string>();
  while (pending.length) {
    const id = pending.pop()!;
    if (reached.has(id)) continue;
    reached.add(id);
    for (const dependency of graph.get(id) ?? [])
      if (dependency.dep_kinds.some(({ kind }) => kind === null))
        pending.push(dependency.pkg);
  }
  return targets.filter((root) => root.library && reached.has(root.packageId));
}
