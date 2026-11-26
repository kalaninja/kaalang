// A stub host for trying the page in a browser: a text field stands for the
// editor, a log shows what the page asks of the host, a search for a
// declaration or a `refer` by name in the same text stands for the engine, and
// a binding's name for its type.

import { start } from "../page.js";

const editor = document.querySelector("#editor");
const log = document.querySelector("#log");
const built = new URL("/target/wasm/web/", location.href);
const sample = document.querySelector("#sample");
let uri = "";

const page = document.querySelector("#page");
const receive = start(page, {
  send(message) {
    log.textContent += `${JSON.stringify(message)}\n`;
    log.scrollTop = log.scrollHeight;
    switch (message.type) {
      case "ready":
        report();
        break;
      case "reveal":
        select(message.start, message.end);
        break;
      case "definition": {
        const [start, end] = word(message.offset);
        const name = editor.value.slice(start, end);
        const found = new RegExp(
          `\\b(?:fn|struct|enum|union|type)\\s+${name}\\b`,
        ).exec(editor.value);
        if (found === null) {
          // A callee declared elsewhere: show the name the engine would resolve.
          select(start, end);
        } else {
          select(found.index, found.index);
        }
        break;
      }
      case "typeDefinition":
        // No engine knows the type here: show the binding it would ask about.
        select(...word(message.offset));
        break;
      case "types": {
        // No engine knows the types here: name each after its binding, late
        // as an engine answers.
        const types = message.offsets.map((offset) => [
          offset,
          `type of ${editor.value.slice(...word(offset))}`,
        ]);
        const { id } = message;
        setTimeout(
          () =>
            receive({ type: "types", id, types: Object.fromEntries(types) }),
          300,
        );
        break;
      }
      case "referrers": {
        // Every `refer` whose path ends in the type's name, late as an engine
        // answers.
        const name = editor.value.slice(...word(message.offset));
        const offsets = [];
        for (const found of editor.value.matchAll(/refer\(\s*([\w:]+)/g)) {
          if (found[1].split("::").at(-1) === name) {
            offsets.push(found.index + found[0].length - name.length);
          }
        }
        const { id } = message;
        // Every type at the crate root.
        const place = {
          package: "bench",
          group: 0,
          crate: "bench",
          root: "bench.rs",
          module: "",
          order: [],
        };
        const file = { uri, text: editor.value, cfg: "", offsets, place };
        setTimeout(
          () =>
            receive({
              type: "referrers",
              id,
              focus: { place },
              files: [file],
            }),
          300,
        );
        break;
      }
    }
  },
  wasm: {
    worker: new URL("../worker.js", import.meta.url).href,
    glue: new URL("kaalang_js.js", built).href,
    module: new URL("kaalang_js_bg.wasm", built).href,
  },
});

/** Hands the page the text and the cursor, as a host does on every change. */
function report() {
  receive({
    type: "document",
    uri,
    text: editor.value,
    cursor: editor.selectionStart,
  });
}

/** Moves the editor's selection, as a host reveals a place. */
function select(start, end) {
  editor.focus();
  editor.setSelectionRange(start, end);
  report();
}

/** The identifier around `offset`. */
function word(offset) {
  const text = editor.value;
  let start = offset;
  let end = offset;
  while (start > 0 && /\w/.test(text[start - 1])) {
    start -= 1;
  }
  while (end < text.length && /\w/.test(text[end])) {
    end += 1;
  }
  return [start, end];
}

for (const event of ["input", "keyup", "click", "select"]) {
  editor.addEventListener(event, report);
}

document.querySelector("#dark").addEventListener("change", ({ target }) => {
  page.classList.toggle("dark", target.checked);
});
document.querySelector("#outlines").addEventListener("change", ({ target }) => {
  page.classList.toggle("kaalang-show-hotspots", target.checked);
});

/** Shows `text` as the file `name`, the cursor on its first declaration. */
function open(name, text) {
  uri = name;
  editor.value = text;
  const first = Math.max(editor.value.indexOf("#[kaa"), 0);
  editor.setSelectionRange(first, first);
  report();
}

/** Opens the sample workspace's file at `name`, unless another was chosen meanwhile. */
async function openSample(name) {
  const text = await (await fetch(`/ide/sample/${name}`)).text();
  if (sample.value === name) open(name, text);
}

document
  .querySelector("#file")
  .addEventListener("change", async ({ target }) => {
    const [file] = target.files;
    sample.value = "";
    open(file.name, await file.text());
  });
sample.addEventListener("change", () => openSample(sample.value));

for (const name of await (await fetch("sample.json")).json()) {
  sample.add(new Option(name, name));
}
sample.value = "shop/src/lib.rs";
await openSample(sample.value);
