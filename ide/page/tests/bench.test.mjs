import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { asset } from "../bench/serve.mjs";

test("the bench publishes its own assets directly", async (t) => {
  const root = await fs.realpath(
    await fs.mkdtemp(path.join(os.tmpdir(), "kaalang-bench-")),
  );
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const page = path.join(root, "ide/page");
  await fs.mkdir(page, { recursive: true });
  const script = path.join(page, "page.js");
  await fs.writeFile(script, "");
  await fs.symlink(script, path.join(page, "alias.js"));

  assert.equal(await asset(root, "/ide/page/page.js"), script);
  assert.equal(await asset(root, "/ide/page/alias.js"), null);
  assert.equal(await asset(root, "/Cargo.toml"), null);
});
