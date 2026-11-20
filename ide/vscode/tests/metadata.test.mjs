import assert from "node:assert/strict";
import test from "node:test";

import { workspaceTargets } from "../out/metadata.js";

test("workspace targets keep their groups, gated ones included, without build scripts", () => {
  const target = (kind, name, required = []) => ({
    kind: [kind],
    name,
    src_path: `/ws/${name}.rs`,
    "required-features": required,
  });
  const shop = {
    id: "shop",
    name: "shop",
    targets: [
      target("lib", "shop"),
      target("test", "check-model", ["domain-model/refs"]),
      target("example", "example", ["show"]),
      target("bench", "bench"),
      target("custom-build", "build-script"),
    ],
  };
  const model = {
    id: "model",
    name: "model",
    targets: [target("lib", "model")],
  };
  const targets = workspaceTargets({
    packages: [shop, model],
    workspace_members: ["shop"],
  });
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
});
