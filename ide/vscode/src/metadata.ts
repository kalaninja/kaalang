/** The parts of `cargo metadata --format-version 1` the diagram reads. */
interface Package {
  id: string;
  name: string;
  manifest_path: string;
  source: string | null;
  dependencies: { name: string; rename: string | null }[];
  targets: {
    name: string;
    kind: string[];
    src_path: string;
    "required-features"?: string[];
  }[];
}

export interface Metadata {
  packages: Package[];
  workspace_members: string[];
  resolve: {
    nodes: {
      id: string;
      features: string[];
      deps: { name: string; pkg: string }[];
    }[];
  } | null;
}

/** A package target under its Rust crate name, with the group its cards occupy. */
export interface CargoRoot {
  package: string;
  group: number;
  name: string;
  root: string;
}

/** The selected targets' source roots, including only enabled workspace targets. */
export function workspaceTargets(metadata: Metadata): CargoRoot[] {
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
        name: target.name.replaceAll("-", "_"),
        root: target.src_path,
      });
    }
  }
  return roots;
}
