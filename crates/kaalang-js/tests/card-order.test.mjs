import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  drawType,
  initSync,
  relationshipTargets,
} from "../../../target/wasm/web/kaalang_js.js";

const target = path.resolve(import.meta.dirname, "../../../target");
initSync({
  module: fs.readFileSync(path.join(target, "wasm/web/kaalang_js_bg.wasm")),
});

test("the editor and CLI draw the same cards, paths, and rows", (t) => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "kaalang-order-"));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  function write(file, text) {
    const name = path.join(directory, file);
    fs.mkdirSync(path.dirname(name), { recursive: true });
    fs.writeFileSync(name, text);
    return text;
  }
  write(
    "Cargo.toml",
    '[workspace]\nmembers = ["app", "other"]\nresolver = "3"\n',
  );
  const manifest = (name) =>
    `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n`;
  write(
    "app/Cargo.toml",
    manifest("app") +
      '[features]\ngated = []\ninvoices = []\n[[bin]]\nname = "gated"\npath = "tools/gated.rs"\nrequired-features = ["gated"]\n',
  );
  write(
    "other/Cargo.toml",
    manifest("other") +
      '[dependencies]\napp = {path = "../app"}\n[features]\ndefault = ["invoices"]\ninvoices = []\n',
  );
  const refers = (name, member) =>
    `#[kaa] pub struct ${name} { #[kaa::refer(${member}, "Member")] id: u8 }\n`;
  const lib = write(
    "app/src/lib.rs",
    "pub struct Member;\npub mod names;\npub mod other_names;\nmod zebra;\n" +
      refers("Middle", "Member") +
      'mod alpha;\n#[path="../../shared.rs"] mod shared;\n',
  );
  write("app/src/names.rs", "pub use crate::Member as Person;\n");
  write("app/src/other_names.rs", "pub use crate::names::Person as Author;\n");
  const binary = write(
    "app/src/main.rs",
    "use app::Member;\n" + refers("FromBin", "Member") + "fn main() {}\n",
  );
  const zebra = write("app/src/zebra.rs", refers("Before", "crate::Member"));
  const alpha = write(
    "app/src/alpha.rs",
    refers("After", "crate::other_names::Author"),
  );
  const check = write("app/tests/check.rs", refers("FromTest", "app::Member"));
  const other = write(
    "other/src/lib.rs",
    'pub use app::Member;\n#[path="../../shared.rs"] mod shared;\n' +
      refers("FromOther", "app::Member"),
  );
  // Both packages compile this file, each keeping another Exclusive. A
  // declaration both kept would give the CLI a card per copy and the editor,
  // whose rust-analyzer reads the file in one module, a single one.
  const shared = write(
    "shared.rs",
    '#[cfg(not(feature="invoices"))]\n' +
      refers("Exclusive", "crate::Member") +
      '#[cfg(feature="invoices")]\n' +
      refers("Exclusive", "crate::Member"),
  );
  write(
    "app/tools/gated.rs",
    refers("Gated", "app::Member") + "fn main() {}\n",
  );
  const cli = path.join(
    target,
    "debug",
    process.platform === "win32" ? "cargo-kaalang.exe" : "cargo-kaalang",
  );
  execFileSync(cli, ["diagram", "--type", "Member", "-p", "app"], {
    cwd: directory,
  });
  const expected = fs.readFileSync(path.join(directory, "Member.svg"), "utf8");

  const place = (pkg, group, crate, root, module, ...order) => ({
    package: pkg,
    group,
    crate,
    root,
    module,
    order,
  });
  const inLib = (module, ...order) =>
    place("app", 0, "app", "app/src/lib.rs", module, ...order);
  const inBin = (module, ...order) =>
    place("app", 0, "app", "app/src/main.rs", module, ...order);
  const references = (text, place) => ({
    text,
    cfg: "",
    // Each of these single-relationship files refers to Member. A definition
    // can confirm a target even when its last identifier is a re-export.
    offsets: JSON.parse(relationshipTargets(text, "")),
    place,
  });
  // The host can discover files in either order. The library and binary have
  // the same crate name. The gated binary is outside the enabled targets.
  const files = [
    ...[false, true].map((enabled) => {
      const cfg = enabled ? 'feature="invoices"' : "";
      return {
        text: shared,
        cfg,
        offsets: JSON.parse(relationshipTargets(shared, cfg)),
        place: enabled
          ? place(
              "other",
              0,
              "other",
              "other/src/lib.rs",
              "shared",
              other.indexOf("mod shared"),
            )
          : inLib("shared", lib.indexOf("mod shared")),
      };
    }),
    references(other, place("other", 0, "other", "other/src/lib.rs", "")),
    references(check, place("app", 1, "check", "app/tests/check.rs", "")),
    references(binary, inBin("")),
    references(alpha, inLib("alpha", lib.indexOf("mod alpha"))),
    references(zebra, inLib("zebra", lib.indexOf("mod zebra"))),
  ];
  const focus = references(lib, inLib(""));
  for (const found of [files, files.toReversed()]) {
    const context = {
      offsets: focus.offsets,
      place: focus.place,
      files: found,
    };
    const diagram = JSON.parse(
      drawType(lib, "", "Member", JSON.stringify(context)),
    );
    assert.equal(diagram.svg, expected);
    for (const card of diagram.cards) {
      const text = card.source === 0 ? lib : found[card.source - 1].text;
      assert.equal(text.slice(card.start, card.end), card.path);
    }
  }
});
