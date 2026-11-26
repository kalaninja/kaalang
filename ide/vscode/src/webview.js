// Starts the shared page in VS Code's webview. The extension carries messages
// both ways; the worker and the bindings' glue load as blob URLs, since a
// worker must come from the page's origin and the extension's files from
// another.

import { start } from "./page/page.js";

const vscode = acquireVsCodeApi();
const root = document.querySelector("#page");
const { worker, glue, module } = root.dataset;

async function blob(url) {
  const text = await (await fetch(url)).text();
  return URL.createObjectURL(new Blob([text], { type: "text/javascript" }));
}

const receive = start(root, {
  send: (message) => vscode.postMessage(message),
  wasm: { worker: await blob(worker), glue: await blob(glue), module },
});
window.addEventListener("message", ({ data }) => receive(data));
