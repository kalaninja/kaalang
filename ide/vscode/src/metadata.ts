/** The parts of `cargo metadata --format-version 1 --no-deps` the diagram reads. */
interface Package {
  id: string;
  name: string;
  manifest_path: string;
  targets: {
    name: string;
    kind: string[];
    src_path: string;
  }[];
}

export interface Metadata {
  packages: Package[];
  workspace_members: string[];
  workspace_root: string;
  target_directory: string;
}

/**
 * A package target as `cargo metadata` lists it: the bindings tell from its
 * kinds and name which row its cards start and what its crate is named.
 */
export interface CargoTarget {
  package: string;
  kind: string[];
  name: string;
  root: string;
}

/**
 * The workspace's targets, those with required features that are off
 * included: rust-analyzer reads them too.
 */
export function workspaceTargets(metadata: Metadata): CargoTarget[] {
  const members = new Set(metadata.workspace_members);
  return metadata.packages
    .filter((pkg) => members.has(pkg.id))
    .flatMap((pkg) =>
      pkg.targets.map((target) => ({
        package: pkg.name,
        kind: target.kind,
        name: target.name,
        root: target.src_path,
      })),
    );
}
