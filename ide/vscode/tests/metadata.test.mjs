import assert from "node:assert/strict";
import test from "node:test";

import { workspaceTargets } from "../out/metadata.js";

test("workspace targets keep their Cargo kinds and names, gated ones included", () => {
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
  const listed = (kind, name) => ({
    package: "shop",
    kind: [kind],
    name,
    root: `/ws/${name}.rs`,
  });
  assert.deepEqual(targets, [
    listed("lib", "shop"),
    listed("test", "check-model"),
    listed("example", "example"),
    listed("bench", "bench"),
    listed("custom-build", "build-script"),
  ]);
});
