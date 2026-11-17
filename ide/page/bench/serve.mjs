// Serves the repository read-only for the test bench, whose page loads the
// bindings from `target/wasm/web` and its default text from the gallery.
// Usage: node serve.mjs <root> <port>

import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";

const root = path.resolve(process.argv[2]);
const port = Number(process.argv[3]);
// A page elsewhere can reach this port through DNS rebinding, under its own host.
const hosts = [`127.0.0.1:${port}`, `localhost:${port}`];
const types = {
  ".css": "text/css",
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".rs": "text/plain; charset=utf-8",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
};

http
  .createServer(async (request, response) => {
    try {
      const { pathname } = new URL(request.url, "http://localhost");
      const file = path.join(root, decodeURIComponent(pathname));
      if (
        request.method !== "GET" ||
        !hosts.includes(request.headers.host) ||
        !(file === root || file.startsWith(root + path.sep))
      ) {
        response.writeHead(403).end();
        return;
      }
      let served = file;
      if ((await fs.stat(file)).isDirectory()) {
        if (!pathname.endsWith("/")) {
          // Relative, so that a path like `//site` cannot lead off the bench.
          const location = `${path.posix.basename(pathname)}/`;
          response.writeHead(301, { location }).end();
          return;
        }
        served = path.join(file, "index.html");
      }
      const body = await fs.readFile(served);
      const type = types[path.extname(served)] ?? "application/octet-stream";
      response
        .writeHead(200, { "content-type": type, "cache-control": "no-store" })
        .end(body);
    } catch {
      response.writeHead(404).end();
    }
  })
  .listen(port, () => {
    const bench = `http://127.0.0.1:${port}/ide/page/bench/`;
    console.log(`Bench: ${bench}\nRace checks: ${bench}checks.html`);
  });
