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
import { fileURLToPath } from "node:url";
import { compileFunction } from "node:vm";
import { fileStamp } from "../out/cache.js";
import { forgetFound, treeHost } from "../out/module-sources.js";

const uri = (fsPath) => ({ scheme: "file", fsPath });

async function editor(t, modules = {}) {
  const directory = await mkdtemp(
    path.join(os.tmpdir(), "kaalang-module-watches-"),
  );
  t.after(() => rm(directory, { recursive: true, force: true }));
  const watchers = new Map();
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
    moduleWatches: (await load("module-watches")).moduleWatches,
  };
}

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

test("a workspace's watches move to its new roots when a cfg set changes", async (t) => {
  const { directory, moduleWatches } = await editor(t);
  const first = path.join(directory, "first.rs"),
    second = path.join(directory, "second.rs");
  await Promise.all([first, second].map((file) => writeFile(file, "original")));
  const updates = new EventEmitter();
  const watch = moduleWatches((file) => updates.emit(file));
  t.after(() => watch.clear());
  const roots = (cfg) => [{ root: "/lib.rs", cfg }];
  watch.update(roots(""), new Set([first]), new Map());
  watch.update(roots("test"), new Set([second]), new Map());
  let stale = false;
  updates.on(first, () => {
    stale = true;
  });
  await writeFile(first, "edited");
  const changed = once(updates, second);
  await writeFile(second, "edited");
  await changed;
  assert.equal(stale, false);
});

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
    const found = new Map();
    const consulted = new Map();
    const host = treeHost(found, new Map(), consulted);
    const model = host.join(main, [modulePath]);
    assert.equal(host.canonical(model), await realpath(first));
    const events = new EventEmitter();
    const watch = moduleWatches((file) => {
      forgetFound(found, file);
      events.emit("changed");
    });
    t.after(() => watch.clear());
    watch.update([], new Set(consulted.keys()), consulted);
    const changed = once(events, "changed");
    if (process.platform === "win32") await rm(current);
    await rename(current + "-next", current);
    await changed;
    assert.equal(host.canonical(model), await realpath(second));
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
