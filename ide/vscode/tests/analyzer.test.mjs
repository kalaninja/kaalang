import assert from "node:assert/strict";
import test from "node:test";

import { statusCrates } from "../out/analyzer.js";

// rust-analyzer 0.3.3121's status for a file that a library and a test target
// include, cut to the lines that matter.
const status = `Workspaces:
Loaded 28 packages across 1 workspace.
Analysis:
Crates for file 0:
Crate: app(Crate(Id(1d14)))
    Root module file id: 11
    Edition: 2024
    Enabled cfgs: CfgOptions(["debug_assertions", "feature=shown", "from_build", "target_abi=", "target_os=linux", "test"])
    Potential cfgs: Some(CfgOptions(["feature=hidden", "feature=shown"]))
    Env: Env { entries: {"CARGO_CRATE_NAME": "app", "CARGO_PKG_NAME": "app-core"} }
    Is a proc macro crate: false
Crate: it(Crate(Id(1d18)))
    Root module file id: 11
    Enabled cfgs: CfgOptions(["test", "unix"])
    Env: Env { entries: {"CARGO_PKG_NAME": "app-core"} }
Version:
0.3.3121-standalone
Configuration:
Config {
    cfg: CfgOptions(["ignored"]),
}`;

test("rust-analyzer's status gives each crate with its set in rustc's form", () => {
  assert.deepEqual(statusCrates(status), [
    {
      name: "app",
      package: "app-core",
      cfg: 'debug_assertions\nfeature="shown"\nfrom_build\ntarget_abi=""\ntarget_os="linux"\ntest',
    },
    { name: "it", package: "app-core", cfg: "test\nunix" },
  ]);
});

test("a file rust-analyzer has not loaded gives no crate", () => {
  assert.deepEqual(
    statusCrates(
      "Analysis:\nCrates for file 3:\nDoes not belong to any crate\n",
    ),
    [],
  );
});
