/** The parts of `cargo metadata --format-version 1 --no-deps` the diagram reads. */
interface Package {
  id: string;
  name: string;
  targets: {
    name: string;
    kind: string[];
    src_path: string;
  }[];
}

export interface Metadata {
  packages: Package[];
  workspace_members: string[];
}

/** A package target under its Rust crate name, with the group its cards occupy. */
export interface CargoRoot {
  package: string;
  group: number;
  name: string;
  root: string;
  packageId: string;
}

/**
 * The workspace's targets, those with required features that are off
 * included: rust-analyzer reads them too.
 */
export function workspaceTargets(metadata: Metadata): CargoRoot[] {
  const members = new Set(metadata.workspace_members);
  return metadata.packages
    .filter((pkg) => members.has(pkg.id))
    .flatMap((pkg) =>
      pkg.targets
        .filter((target) => !target.kind.includes("custom-build"))
        .map((target) => ({
          package: pkg.name,
          group:
            ["test", "example", "bench"].findIndex((kind) =>
              target.kind.includes(kind),
            ) + 1,
          packageId: pkg.id,
          name: target.name.replaceAll("-", "_"),
          root: target.src_path,
        })),
    );
}
