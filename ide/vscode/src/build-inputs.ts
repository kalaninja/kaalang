import { statSync } from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import type { BuildInputs } from "./cargo";

function within(file: string, directory: string): boolean {
  const relative = path.relative(directory, file);
  return (
    relative === "" ||
    (relative !== ".." &&
      !relative.startsWith(".." + path.sep) &&
      !path.isAbsolute(relative))
  );
}

/** Watches Cargo's declared inputs, including files outside the open folders. */
export function watchBuildInputs(
  { paths, ignored }: BuildInputs,
  changed: () => void,
): vscode.Disposable {
  const files = new Set<string>();
  const directories: string[] = [];
  const bases = new Map<string, boolean>();
  for (const input of paths) {
    let directory = false;
    try {
      directory = statSync(input).isDirectory();
    } catch {
      // Cargo also accepts a file that has yet to be created.
    }
    if (directory) directories.push(input);
    else files.add(input);
    const parent = directory ? input : path.dirname(input);
    bases.set(parent, directory || bases.get(parent) === true);
  }
  function notify(uri: vscode.Uri) {
    if (uri.scheme !== "file") return;
    const file = path.normalize(uri.fsPath);
    const watched =
      files.has(file) ||
      directories.some((dir) => {
        if (!within(file, dir)) return false;
        const git = path.relative(dir, file).split(path.sep).includes(".git");
        const generated = ignored.some(
          (output) => within(output, dir) && within(file, output),
        );
        return !git && !generated;
      });
    if (watched) changed();
  }
  const watchers: vscode.Disposable[] = [];
  for (const [base, recursive] of bases) {
    const watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(
        vscode.Uri.file(base),
        recursive ? "**/*" : "*",
      ),
    );
    watcher.onDidCreate(notify);
    watcher.onDidChange(notify);
    watcher.onDidDelete(notify);
    watchers.push(watcher);
  }
  // VS Code can exclude paths from filesystem watching even though the editor saves them.
  watchers.push(
    vscode.workspace.onDidSaveTextDocument((doc) => notify(doc.uri)),
  );
  return vscode.Disposable.from(...watchers);
}
