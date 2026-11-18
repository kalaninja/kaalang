// Serves the test bench, its bindings and its default gallery example.
// Usage: node serve.mjs <root> <port>

import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";

const bench = "/ide/page/bench/";
const types = {
  ".css": "text/css",
  ".html": "text/html",
  ".js": "text/javascript",
  ".rs": "text/plain; charset=utf-8",
  ".wasm": "application/wasm",
};

/** The published asset at `pathname` under the canonical `root`, if any. */
export async function asset(root, pathname) {
  const file = path.join(
    root,
    pathname === bench ? `${bench}index.html` : pathname,
  );
  const relative = path.relative(root, file).split(path.sep).join("/");
  const directory = path.posix.dirname(relative);
  const ext = path.extname(file);
  const published =
    (directory === "ide/page" && [".js", ".css"].includes(ext)) ||
    (directory === "ide/page/bench" && [".js", ".html"].includes(ext)) ||
    relative === "target/wasm/web/kaalang_js.js" ||
    relative === "target/wasm/web/kaalang_js_bg.wasm" ||
    relative === "crates/kaalang/examples/gallery/sorting/quick_sort.rs";
  return published && (await fs.realpath(file)) === file ? file : null;
}

if (import.meta.main) {
  const root = await fs.realpath(process.argv[2]);
  const port = Number(process.argv[3]);
  // A page elsewhere can reach this port through DNS rebinding, under its own host.
  const hosts = [`127.0.0.1:${port}`, `localhost:${port}`];
  http
    .createServer(async (request, response) => {
      try {
        if (request.method !== "GET" || !hosts.includes(request.headers.host)) {
          response.writeHead(403).end();
          return;
        }
        const { pathname } = new URL(request.url, "http://localhost");
        if (pathname === bench.slice(0, -1)) {
          response.writeHead(301, { location: bench }).end();
          return;
        }
        const served = await asset(root, decodeURIComponent(pathname));
        if (served === null) {
          response.writeHead(403).end();
          return;
        }
        const body = await fs.readFile(served);
        const type = types[path.extname(served)];
        response
          .writeHead(200, { "content-type": type, "cache-control": "no-store" })
          .end(body);
      } catch {
        response.writeHead(404).end();
      }
    })
    .listen(port, () => {
      const url = `http://127.0.0.1:${port}${bench}`;
      console.log(`Bench: ${url}\nRace checks: ${url}checks.html`);
    });
}
