import assert from "node:assert/strict";
import { EventEmitter, once } from "node:events";
import {
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rename,
  rm,
  stat,
  symlink,
  writeFile,
} from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { compileFunction } from "node:vm";
import { fileStamp } from "../out/cache.js";
import { forgetModules, moduleTree } from "../out/card-places.js";

const uri = (fsPath) => ({ scheme: "file", fsPath });

async function editor(t, modules = {}) {
  const directory = await mkdtemp(
    path.join(os.tmpdir(), "kaalang-build-inputs-"),
  );
  t.after(() => rm(directory, { recursive: true, force: true }));
  const watchers = new Map();
  const saves = new EventEmitter();
  const vscode = {
    Uri: { file: uri },
    RelativePattern: class {
      constructor(baseUri, pattern) {
        this.baseUri = baseUri;
        this.pattern = pattern;
      }
    },
    Disposable: {
      from: (...items) => ({
        dispose: () => items.forEach((item) => item.dispose()),
      }),
    },
    workspace: {
      getWorkspaceFolder: (file) => {
        const root = path.join(directory, "workspace");
        return file.fsPath === root || file.fsPath.startsWith(root + path.sep)
          ? { uri: uri(root) }
          : undefined;
      },
      createFileSystemWatcher({ baseUri, pattern }) {
        const events = new EventEmitter();
        watchers.set(baseUri.fsPath, { pattern, events });
        return {
          onDidCreate: (fn) => events.on("create", fn),
          onDidChange: (fn) => events.on("change", fn),
          onDidDelete: (fn) => events.on("delete", fn),
          dispose: () => events.removeAllListeners(),
        };
      },
      onDidSaveTextDocument(fn) {
        saves.on("save", fn);
        return { dispose: () => saves.off("save", fn) };
      },
    },
  };
  async function load(name) {
    const file = new URL("../out/" + name + ".js", import.meta.url);
    const require = createRequire(file);
    const host = {};
    compileFunction(await readFile(file, "utf8"), ["exports", "require"], {
      filename: fileURLToPath(file),
    })(host, (key) =>
      key === "vscode" ? vscode : (modules[key] ?? require(key)),
    );
    return host;
  }
  return {
    directory,
    watchers,
    watch: (await load("build-inputs")).watchBuildInputs,
    moduleWatches: (await load("module-watches")).moduleWatches,
    save: (file) => saves.emit("save", { uri: uri(file) }),
  };
}

test("declared files, including missing and external inputs, refresh on change or editor save", async (t) => {
  const { directory, watchers, watch, save } = await editor(t);
  const workspace = path.join(directory, "workspace");
  const external = path.join(directory, "external");
  await Promise.all([mkdir(workspace), mkdir(external)]);
  const script = path.join(workspace, "flags.rs");
  const missing = path.join(workspace, "later.txt");
  const settings = path.join(external, "flags.toml");
  await Promise.all([writeFile(script, ""), writeFile(settings, "")]);

  let changes = 0;
  const subscription = watch(
    { paths: [script, missing, settings], ignored: [] },
    () => ++changes,
  );
  const local = watchers.get(workspace);
  assert.equal(local.pattern, "*");
  local.events.emit("change", uri(path.join(workspace, "another.rs")));
  assert.equal(changes, 0);
  save(script);
  assert.equal(changes, 1);
  await writeFile(missing, "ready");
  local.events.emit("create", uri(missing));
  assert.equal(changes, 2);
  await rm(settings);
  const outside = watchers.get(external);
  outside.events.emit("delete", uri(settings));
  assert.equal(changes, 3);

  subscription.dispose();
  save(script);
  local.events.emit("change", uri(script));
  outside.events.emit("create", uri(settings));
  assert.equal(changes, 3);
});

test("a directory input includes nested files while ignoring Git and Cargo output", async (t) => {
  const { directory, watchers, watch, save } = await editor(t);
  const root = path.join(directory, "workspace");
  const output = path.join(root, "build");
  const nested = path.join(root, "schema", "v1");
  const similarlyNamed = path.join(root, "build-inputs");
  await Promise.all(
    [output, nested, similarlyNamed].map((dir) =>
      mkdir(dir, { recursive: true }),
    ),
  );
  const source = path.join(nested, "schema.json");
  await writeFile(source, "{}");
  let changes = 0;
  const subscription = watch(
    { paths: [root], ignored: [output] },
    () => ++changes,
  );
  const workspace = watchers.get(root);
  assert.equal(workspace.pattern, "**/*");
  workspace.events.emit("change", uri(source));
  assert.equal(changes, 1);
  workspace.events.emit("change", uri(path.join(root, ".git", "index")));
  save(path.join(nested, ".git", "HEAD"));
  workspace.events.emit("create", uri(path.join(output, "debug", "artifact")));
  save(path.join(output, "generated.rs"));
  assert.equal(changes, 1);
  workspace.events.emit(
    "create",
    uri(path.join(similarlyNamed, "options.json")),
  );
  assert.equal(changes, 2);
  subscription.dispose();
});

test(
  "module watches follow excluded Rust files, missing paths, and directory renames",
  { timeout: 8000 },
  async (t) => {
    const { directory, moduleWatches } = await editor(t);
    const inside = path.join(directory, "workspace"),
      outside = path.join(directory, "elsewhere");
    await Promise.all([mkdir(inside), mkdir(outside)]);
    const root = path.join(inside, "lib.rs"),
      custom = path.join(inside, "page.in");
    const old = path.join(outside, "types.rs"),
      future = path.join(outside, "nested", "mod.rs");
    const updates = new EventEmitter();
    const extra = moduleWatches((file) => updates.emit(file));
    t.after(() => extra.clear());
    const roots = [];
    const files = [root, custom, old, future];
    const update = () => extra.update(roots, new Set(files), new Map());
    async function expect(paths, change) {
      const observed = Promise.all(paths.map((file) => once(updates, file)));
      await change();
      await observed;
    }
    await Promise.all(
      [root, custom, old].map((file) => writeFile(file, "original")),
    );
    update();
    await expect([root, custom, old], () =>
      Promise.all([root, custom, old].map((file) => writeFile(file, "edited"))),
    );
    await expect([future], () => mkdir(path.dirname(future)));
    update();
    await expect([future], () => writeFile(future, "created"));
    await expect([old, future], () => rename(outside, outside + "-moved"));
    extra.update(roots, new Set([root]), new Map());
    let removed = false;
    updates.on(custom, () => {
      removed = true;
    });
    await writeFile(custom, "no longer watched");
    await expect([root], () => writeFile(root, "still watched"));
    assert.equal(removed, false);
  },
);

test("a write during registration of an external module watch refreshes exactly that file", async (t) => {
  const { directory, moduleWatches } = await editor(t);
  const external = path.join(directory, "external.rs"),
    still = path.join(directory, "unchanged.rs");
  await Promise.all(
    [external, still].map((file) => writeFile(file, "pub struct Old;")),
  );
  let notify;
  const changed = new Promise((resolve) => {
    notify = resolve;
  });
  const watch = moduleWatches((file) => notify(file));
  t.after(() => watch.clear());
  const stamps = new Map(
    await Promise.all(
      [external, still].map(async (file) => [
        file,
        fileStamp(await stat(file)),
      ]),
    ),
  );
  await writeFile(external, "pub struct Replaced;");
  watch.update([], new Set([external, still]), stamps);
  assert.equal(
    await Promise.race([
      changed,
      new Promise((_, reject) => {
        const timer = setTimeout(
          () => reject(new Error("module watch missed its initial change")),
          3000,
        );
        timer.unref();
      }),
    ]),
    external,
  );
});

test(
  "retargeting an external directory link refreshes the cached module through dot segments",
  { timeout: 8000 },
  async (t) => {
    const { directory, moduleWatches } = await editor(t);
    const src = path.join(directory, "workspace", "src");
    const generated = path.join(directory, "build");
    const current = path.join(generated, "current");
    const main = path.join(src, "lib.rs");
    const first = path.join(generated, "v1", "model.in");
    const second = path.join(generated, "v2", "model.in");
    await Promise.all(
      [src, path.dirname(first), path.dirname(second)].map((p) =>
        mkdir(p, { recursive: true }),
      ),
    );
    const modulePath = "../../build/current/model.in";
    const source = `#[path="${modulePath}"] mod model;`;
    await Promise.all([
      writeFile(main, source),
      writeFile(first, "struct First;"),
      writeFile(second, "struct Second;"),
    ]);
    try {
      await symlink(path.dirname(first), current, "dir");
      await symlink(path.dirname(second), current + "-next", "dir");
    } catch (error) {
      if (error.code !== "EPERM") throw error;
      return t.skip(
        "Creating a directory symlink needs this platform's permission",
      );
    }
    const roots = [
      {
        root: pathToFileURL(main).href,
        package: "app",
        group: 0,
        name: "app",
        cfg: "",
      },
    ];
    const tree = () =>
      moduleTree(
        roots,
        (uri) => readFile(fileURLToPath(uri), "utf8"),
        async (text) => ({
          modules:
            text === source
              ? [
                  {
                    module: "model",
                    offset: source.indexOf("model;"),
                    files: [{ parts: [modulePath], ownsDirectory: true }],
                  },
                ]
              : [],
          types: text === source ? [] : [7],
          bodies: [],
        }),
      );
    const graph = await tree();
    const firstPath = await realpath(first);
    assert.equal(graph.texts.get(firstPath), "struct First;");
    const events = new EventEmitter();
    const watch = moduleWatches((file) => {
      forgetModules(file);
      events.emit("changed");
    });
    t.after(() => watch.clear());
    watch.update(roots, graph.paths, new Map());
    const changed = once(events, "changed");
    if (process.platform === "win32") await rm(current);
    await rename(current + "-next", current);
    await changed;
    const after = await tree();
    assert.equal(after.texts.get(await realpath(second)), "struct Second;");
    assert.equal(after.texts.has(firstPath), false);
  },
);

test("module files still report changes when native directory watches are exhausted", async (t) => {
  const listeners = new Map();
  const { directory, moduleWatches } = await editor(t, {
    "node:fs": {
      watch() {
        throw Object.assign(new Error("watch limit"), { code: "ENOSPC" });
      },
      watchFile(file, _options, listener) {
        listeners.set(file, listener);
      },
      unwatchFile(file) {
        listeners.delete(file);
      },
    },
  });
  const file = path.join(directory, "file.rs");
  await writeFile(file, "before");
  const changes = [];
  const watch = moduleWatches((file) => changes.push(file));
  watch.update([], new Set([file]), new Map());
  const before = await stat(file);
  await writeFile(file, "changed");
  listeners.get(file)(await stat(file), before);
  assert.deepEqual(changes, [file]);
  watch.clear();
  assert.equal(listeners.size, 0);
});
