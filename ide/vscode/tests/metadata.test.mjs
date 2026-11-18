import assert from "node:assert/strict";
import test from "node:test";

import { workspaceTargets } from "../out/metadata.js";

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
            { name: "domain_model", pkg: "model" },
            { name: "other_name", pkg: "utils" },
          ],
        },
        { id: "model", features: ["refs"], deps: [] },
        { id: "utils", features: ["extra"], deps: [] },
      ],
    },
  };
  assert.deepEqual(workspaceTargets(metadata), [
    { package: "shop", name: "shop", group: 0, root: "/ws/shop.rs" },
    {
      package: "shop",
      name: "check_model",
      group: 1,
      root: "/ws/check-model.rs",
    },
    { package: "shop", name: "example", group: 2, root: "/ws/example.rs" },
    { package: "shop", name: "bench", group: 3, root: "/ws/bench.rs" },
  ]);
  assert.deepEqual(
    workspaceTargets({ ...metadata, resolve: null }).map(({ name }) => name),
    ["shop", "bench"],
  );
});
