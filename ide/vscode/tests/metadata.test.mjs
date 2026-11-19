import assert from "node:assert/strict";
import test from "node:test";

import { hostTargets, workspaceTargets } from "../out/metadata.js";

test("workspace targets keep their groups and require enabled local or dependency features", () => {
  const target = (kind, name, required = []) => ({
    kind: [kind],
    name,
    src_path: `/ws/${name}.rs`,
    "required-features": required,
  });
  const shop = {
    id: "shop",
    name: "shop",
    manifest_path: "/ws/shop/Cargo.toml",
    dependencies: [
      { name: "model", rename: "domain-model" },
      { name: "utils", rename: null },
    ],
    targets: [
      target("lib", "shop"),
      target("test", "check-model", ["domain-model/refs"]),
      target("example", "example", ["show", "utils/extra"]),
      target("bench", "bench"),
      target("bin", "disabled", ["model/refs"]),
      target("bin", "missing", ["domain-model/other"]),
      target("custom-build", "build-script"),
    ],
  };
  const model = {
    id: "model",
    name: "model",
    targets: [target("lib", "model")],
  };
  const utils = {
    id: "utils",
    name: "utils",
    targets: [target("lib", "other_name")],
  };
  const metadata = {
    packages: [shop, model, utils],
    workspace_members: ["shop"],
    resolve: {
      nodes: [
        {
          id: "shop",
          features: ["show"],
          deps: [
            {
              name: "domain_model",
              pkg: "model",
              dep_kinds: [{ kind: "dev" }],
            },
            { name: "other_name", pkg: "utils", dep_kinds: [{ kind: null }] },
          ],
        },
        { id: "model", features: ["refs"], deps: [] },
        { id: "utils", features: ["extra"], deps: [] },
      ],
    },
  };
  const targets = workspaceTargets(metadata);
  assert.deepEqual(targets[0].dependencies, [["other_name", "utils"]]);
  assert.deepEqual(targets[1].dependencies, [
    ["domain_model", "model"],
    ["other_name", "utils"],
  ]);
  assert.deepEqual(
    targets.map(({ package: pkg, group, name, root }) => ({
      package: pkg,
      group,
      name,
      root,
    })),
    [
      { package: "shop", name: "shop", group: 0, root: "/ws/shop.rs" },
      {
        package: "shop",
        name: "check_model",
        group: 1,
        root: "/ws/check-model.rs",
      },
      { package: "shop", name: "example", group: 2, root: "/ws/example.rs" },
      { package: "shop", name: "bench", group: 3, root: "/ws/bench.rs" },
    ],
  );
  assert.deepEqual(
    workspaceTargets({ ...metadata, resolve: null }).map(({ name }) => name),
    ["shop", "bench"],
  );
});

test("no-deps keeps certain local imports, including renamed libraries and a proc macro's dependencies", () => {
  const member = (name, kind, lib = name) => ({
    id: name,
    name,
    manifest_path: `/ws/${name}/Cargo.toml`,
    dependencies: [],
    targets: [{ name: lib, kind: [kind], src_path: `/ws/${name}/src/lib.rs` }],
  });
  const macros = member("macros", "proc-macro"),
    bridge = member("bridge", "lib", "bridge_api"),
    model = member("model", "lib"),
    other = member("other", "lib");
  const dep = (name, extra = {}) => ({
    name,
    path: `/ws/${name}`,
    rename: null,
    kind: null,
    ...extra,
  });
  macros.dependencies.push(dep("bridge"), dep("other", { kind: "build" }));
  bridge.targets.push({
    name: "test",
    kind: ["test"],
    src_path: "/ws/bridge/tests/test.rs",
  });
  bridge.dependencies.push(
    dep("model", { rename: "shared-model" }),
    dep("other", { kind: "dev" }),
    dep("other", { rename: "feature", optional: true }),
    dep("other", { rename: "platform", target: "cfg(windows)" }),
  );
  const packages = [macros, bridge, model, other],
    data = {
      packages,
      workspace_members: packages.map((pkg) => pkg.id),
      resolve: null,
    };
  const imports = new Map(
    workspaceTargets(data).map((root) => [root.name, root.dependencies]),
  );
  assert.deepEqual(imports.get("macros"), [["bridge_api", "bridge"]]);
  assert.deepEqual(imports.get("bridge_api"), [["shared_model", "model"]]);
  assert.deepEqual(imports.get("test"), [
    ["shared_model", "model"],
    ["other", "other"],
  ]);
  assert.deepEqual(
    hostTargets(data).map((root) => root.name),
    ["macros", "bridge_api", "model"],
  );
});
