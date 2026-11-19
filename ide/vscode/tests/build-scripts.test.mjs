import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  forgetBuildScriptRuns,
  readBuildScripts,
} from "../out/build-scripts.js";

async function project(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "kaalang-build-scripts-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const pkg = (name, kind = "lib") => ({
    id: `path+${name}#1.0.0`,
    name,
    manifest_path: path.join(root, name, "Cargo.toml"),
    source: null,
    dependencies: [],
    targets: [
      { kind: [kind], src_path: path.join(root, name, "src/lib.rs") },
      {
        kind: ["custom-build"],
        src_path: path.join(root, name, "tools/configure.rs"),
      },
    ],
  });
  const script = path.join(root, "cargo-output.mjs");
  await writeFile(
    script,
    `const {lines, code} = JSON.parse(process.argv[2]);
    for (const line of lines) console.log(line);
    if (code) process.stderr.write("error from another package");
    process.exitCode = code;`,
  );
  async function run(
    packages,
    members,
    lines,
    code = 0,
    target,
    resolve = null,
  ) {
    const logs = [];
    const args = [
      script,
      JSON.stringify({
        lines: lines.map((line) =>
          typeof line === "string" ? line : JSON.stringify(line),
        ),
        code,
      }),
    ];
    const result = await readBuildScripts(
      process.execPath,
      args,
      root,
      process.env,
      {
        workspace_members: members.map((pkg) => pkg.id),
        workspace_root: root,
        packages,
        resolve,
      },
      (line) => logs.push(line),
      target,
    );
    return { ...result, logs };
  }
  async function output(pkg, buildRoot, current, cfgs, stdout) {
    const unit = current
      ? path.join(buildRoot, "debug", "build", pkg.name, "abf1234")
      : path.join(buildRoot, "debug", "build", pkg.name + "-abf1234");
    const out = path.join(unit, "out");
    const saved = path.join(unit, current ? "run/stdout" : "output");
    await mkdir(path.dirname(saved), { recursive: true });
    await writeFile(saved, stdout);
    return {
      reason: "build-script-executed",
      package_id: pkg.id,
      cfgs,
      out_dir: out,
    };
  }
  return { root, pkg, run, output };
}

test("workspace cfgs and explicit inputs survive an unrelated failure, in either Cargo layout", async (t) => {
  const { root, pkg, run, output } = await project(t);
  const a = pkg("a"),
    b = pkg("b"),
    dependency = pkg("dependency"),
    failed = pkg("failed");
  const build = path.join(root, "intermediate files");
  const external = path.join(root, "uncreated outside.file");
  const first = await output(
    a,
    build,
    false,
    ["local", 'mode="first"'],
    [
      "cargo:rerun-if-changed=src/missing.rs",
      "cargo::rerun-if-changed=data=with space",
      `cargo::rerun-if-changed=${external}`,
      "cargo:rustc-cfg=ignored_here",
      "unrelated plain output",
    ].join("\r\n"),
  );
  const second = await output(
    b,
    build,
    true,
    ["other"],
    "cargo::rerun-if-env-changed=PROBE\n",
  );
  const unused = await output(
    dependency,
    build,
    true,
    ["unused"],
    "cargo::rerun-if-changed=private.file",
  );
  const { cfgs, inputs, logs } = await run(
    [a, b, dependency, failed],
    [a, b, failed],
    [
      "normal stdout",
      "{invalid JSON",
      null,
      { reason: "compiler-artifact" },
      first,
      second,
      unused,
    ],
    1,
  );
  assert.deepEqual(
    cfgs,
    new Map([
      [a.id, ["local", 'mode="first"']],
      [b.id, ["other"]],
    ]),
  );
  assert.deepEqual(
    new Set(inputs),
    new Set([
      a.targets[1].src_path,
      b.targets[1].src_path,
      failed.targets[1].src_path,
      path.join(root, "a/src/missing.rs"),
      path.join(root, "a/data=with space"),
      external,
    ]),
  );
  assert.match(logs.join("\n"), /error from another package/);
});

test("default and unavailable script stdout watch each package, and a missing executable only the scripts", async (t) => {
  const { root, pkg, run, output } = await project(t);
  const a = pkg("default"),
    b = pkg("custom");
  const first = await output(
    a,
    path.join(root, "build-dir"),
    false,
    [],
    "cargo::rustc-cfg=active\n",
  );
  const second = {
    reason: "build-script-executed",
    package_id: b.id,
    cfgs: ["from_override"],
    out_dir: path.join(root, "unknown-layout", "out"),
  };
  const expected = new Set([
    a.targets[1].src_path,
    b.targets[1].src_path,
    path.join(root, "default"),
    path.join(root, "custom"),
  ]);
  const { cfgs, inputs, logs } = await run([a, b], [a, b], [first, second]);
  assert.deepEqual(new Set(inputs), expected);
  assert.deepEqual(
    cfgs,
    new Map([
      [a.id, []],
      [b.id, ["from_override"]],
    ]),
  );
  assert.deepEqual(logs, []);
  const errors = [];
  const failed = await readBuildScripts(
    path.join(root, "missing-cargo"),
    [],
    root,
    process.env,
    { packages: [a, b], workspace_members: [a.id, b.id] },
    (line) => errors.push(line),
  );
  assert.deepEqual(
    new Set(failed.inputs),
    new Set([a.targets[1].src_path, b.targets[1].src_path]),
  );
  assert.equal(failed.cfgs.size, 0);
  assert.equal(errors.length, 1);
});

test("a duplicate script output chooses the selected target for a library and the host for a proc macro", async (t) => {
  const { root, pkg, run, output } = await project(t);
  const lib = pkg("build"),
    mac = pkg("derive", "proc-macro");
  mac.targets.push({
    kind: ["bin"],
    src_path: path.join(root, "derive/src/main.rs"),
  });
  const triple = "thumbv8m.main-none-eabi";
  const base = path.join(root, triple),
    target = path.join(base, triple);
  const entries = [
    await output(
      lib,
      target,
      true,
      ["library_target"],
      "cargo::rerun-if-changed=target.input",
    ),
    await output(
      mac,
      base,
      false,
      ["macro_host"],
      "cargo::rerun-if-changed=host.input",
    ),
    await output(
      lib,
      base,
      true,
      ["library_host"],
      "cargo::rerun-if-changed=host.input",
    ),
    await output(
      mac,
      target,
      false,
      ["macro_target"],
      "cargo::rerun-if-changed=target.input",
    ),
  ];
  for (const messages of [entries, [...entries].reverse()]) {
    const { cfgs, hostCfgs, inputs } = await run(
      [lib, mac],
      [lib, mac],
      messages,
      0,
      path.join(root, triple + ".json"),
    );
    assert.deepEqual(cfgs.get(lib.id), ["library_target"]);
    assert.deepEqual(cfgs.get(mac.id), ["macro_target"]);
    assert.deepEqual(hostCfgs.get(mac.id), ["macro_host"]);
    assert.ok(inputs.includes(path.join(root, "build/host.input")));
    assert.ok(inputs.includes(path.join(root, "build/target.input")));
  }
});

test("compile-time helper modules and included data remain inputs with an explicit rerun directive", async (t) => {
  const { root, pkg, run, output } = await project(t);
  const a = pkg("members/a");
  const unit = path.join(root, "compile", "custom-build-unit");
  await mkdir(unit, { recursive: true });
  const artifact = {
    reason: "compiler-artifact",
    package_id: a.id,
    target: { kind: ["custom-build"] },
    filenames: [path.join(unit, "build-script-configure")],
    fresh: true,
  };
  await writeFile(
    path.join(unit, "build_script_configure-1234.d"),
    "build_script_configure.d: members/a/tools/configure.rs members/a/tools/helper.rs members/a/tools/input\\ name.txt\n\n" +
      "members/a/tools/configure.rs:\nmembers/a/tools/helper.rs:\nmembers/a/tools/input\\ name.txt:\n\n# env-dep:OTHER=value:\n",
  );
  const executed = await output(
    a,
    path.join(root, "intermediate"),
    true,
    ["a_cfg"],
    "cargo:rerun-if-changed=runtime.txt\n",
  );
  const { inputs } = await run([a], [a], [artifact, executed]);
  assert.deepEqual(
    new Set(inputs),
    new Set(
      ["configure.rs", "helper.rs", "input name.txt"]
        .map((name) => path.join(root, "members/a/tools", name))
        .concat(path.join(root, "members/a/runtime.txt")),
    ),
  );
});

test("one invalid cfg leaves valid bare names, raw names and quoted values active", async (t) => {
  const { root, pkg, run, output } = await project(t);
  const a = pkg("local");
  const message = await output(
    a,
    path.join(root, "build"),
    false,
    [
      "active",
      "r#async",
      ' variant = "with spaces" ',
      "two-words",
      'bad="one"two"',
      "split\nline",
      42,
    ],
    "cargo:rerun-if-env-changed=CONTROL",
  );
  const { cfgs, logs } = await run([a], [a], [message]);
  assert.deepEqual(cfgs.get(a.id), [
    "active",
    "r#async",
    'variant="with spaces"',
  ]);
  assert.equal(logs.length, 4);
  assert.ok(logs.some((line) => line.includes('"two-words"')));
});

test("only local dependencies used to compile the build script invalidate its cfgs", async (t) => {
  const { root, pkg, run, output } = await project(t);
  const [a, tools, common, unused, dev, ordinary] = [
    "app",
    "tools",
    "common",
    "unused",
    "dev",
    "ordinary",
  ].map((name) => pkg(name));
  const dep = (pkg, kind) => ({
    name: pkg.name,
    path: path.dirname(pkg.manifest_path),
    kind,
  });
  a.dependencies = [
    dep(tools, "build"),
    dep(unused, "build"),
    dep(ordinary, null),
  ];
  tools.dependencies = [
    dep(common, null),
    dep(dev, "dev"),
    { name: "from-registry", kind: null },
  ];
  const message = await output(
    a,
    root,
    true,
    ["enabled"],
    "cargo::rerun-if-env-changed=VAR\n",
  );
  const resolve = {
    nodes: [
      { id: a.id, deps: [tools, ordinary].map((pkg) => ({ pkg: pkg.id })) },
      { id: tools.id, deps: [common, dev].map((pkg) => ({ pkg: pkg.id })) },
    ],
  };
  const { inputs } = await run(
    [a, tools, common, unused, dev, ordinary],
    [a],
    [message],
    0,
    undefined,
    resolve,
  );
  assert.deepEqual(
    new Set(inputs),
    new Set([
      a.targets[1].src_path,
      path.join(root, "tools"),
      path.join(root, "common"),
    ]),
  );
});

test("one build command serves both workspaces until its shared run is forgotten", async (t) => {
  const { root, pkg, output } = await project(t);
  t.after(forgetBuildScriptRuns);
  const workspaces = [pkg("first"), pkg("second")];
  const messages = await Promise.all(
    workspaces.map((member) =>
      output(
        member,
        root,
        true,
        [member.name],
        "cargo::rerun-if-changed=flag.txt",
      ),
    ),
  );
  const args = [
    "-e",
    'require("node:fs").appendFileSync("runs", "x"); console.log(process.argv[1]);',
    messages.map((message) => JSON.stringify(message)).join("\n"),
  ];
  const logs = [];
  const read = (member) =>
    readBuildScripts(
      process.execPath,
      args,
      root,
      process.env,
      {
        workspace_root: path.dirname(member.manifest_path),
        workspace_members: [member.id],
        packages: [member],
        resolve: null,
      },
      (line) => logs.push(line),
      undefined,
      root,
    );
  const results = await Promise.all(workspaces.map(read));
  for (const [index, member] of workspaces.entries()) {
    assert.deepEqual(
      results[index].cfgs,
      new Map([[member.id, [member.name]]]),
    );
    assert.deepEqual(
      new Set(results[index].inputs),
      new Set([
        member.targets[1].src_path,
        path.join(root, member.name, "flag.txt"),
      ]),
    );
  }
  const runs = path.join(root, "runs");
  await read(workspaces[0]);
  assert.equal(await readFile(runs, "utf8"), "x");
  forgetBuildScriptRuns();
  await read(workspaces[1]);
  assert.equal(await readFile(runs, "utf8"), "xx");
  assert.deepEqual(logs, []);
});
