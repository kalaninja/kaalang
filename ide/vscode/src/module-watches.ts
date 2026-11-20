import { type Stats, unwatchFile, watch, watchFile } from "node:fs";
import { stat } from "node:fs/promises";
import * as path from "node:path";
import * as vscode from "vscode";

import { fileStamp } from "./cache";
import type { CargoRoot } from "./target";

/** Watch the local files Cargo declared, even when VS Code excludes them. */
export function moduleWatches(changed: (file: string) => void) {
  // A workspace's roots come anew with each change of a cfg set: its files stay.
  const workspaces = new Map<string, ReturnType<typeof subscribe>>();
  const key = (roots: CargoRoot[]) => roots.map(({ root }) => root).join("\n");

  function subscribe(
    files: ReadonlySet<string>,
    stamps: ReadonlyMap<string, string>,
  ) {
    let disposed = false;
    const stops: (() => void)[] = [];
    const current = {
      files,
      directoriesChanged: false,
      dispose() {
        disposed = true;
        for (const stop of stops) stop();
      },
    };
    const initial = new Map([...files].map((file) => [file, stamps.get(file)]));
    function notify(file: string) {
      if (disposed) return;
      initial.delete(file);
      changed(file);
    }
    const directories = new Set<string>();
    for (const file of files) {
      let dir = path.dirname(file);
      if (directories.has(dir)) continue;
      const root = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(dir))
        ?.uri.fsPath;
      while (!directories.has(dir)) {
        directories.add(dir);
        if (dir === root || dir === path.dirname(dir)) break;
        dir = path.dirname(dir);
      }
    }
    function descendants(dir: string) {
      const prefix = dir.endsWith(path.sep) ? dir : dir + path.sep;
      return [...files].filter((file) => file.startsWith(prefix));
    }
    function directoryChanged(dir: string) {
      current.directoriesChanged = true;
      for (const file of descendants(dir)) notify(file);
    }
    // A filesystem without native notifications, or an exhausted watch limit.
    const polled = new Set<string>();
    function poll(dir: string) {
      for (const file of descendants(dir)) {
        if (polled.has(file)) continue;
        polled.add(file);
        const listener = (after: Stats, before: Stats) => {
          if (fileStamp(after) !== fileStamp(before)) notify(file);
        };
        watchFile(file, { persistent: false }, listener);
        stops.push(() => unwatchFile(file, listener));
      }
    }
    // Watch parents first so creating or moving an entire module directory is visible.
    for (const dir of [...directories].sort((a, b) => a.length - b.length)) {
      try {
        const watcher = watch(dir, { persistent: false }, (event, name) => {
          if (!name) return directoryChanged(dir);
          // A module path can cross a symlink before `..`. Keep the watched spelling.
          const entry = dir.endsWith(path.sep)
            ? dir + name
            : dir + path.sep + name;
          if (files.has(entry)) notify(entry);
          if (event === "rename" && directories.has(entry))
            directoryChanged(entry);
        });
        watcher.on("error", () => {
          if (disposed) return;
          watcher.close();
          poll(dir);
          directoryChanged(dir);
        });
        stops.push(() => watcher.close());
      } catch (error) {
        const { code } = error as NodeJS.ErrnoException;
        // A missing directory's parent will report its creation.
        if (code !== "ENOENT" && code !== "ENOTDIR") poll(dir);
      }
    }
    // Cover edits between reading the source and installing its directory watch.
    void (async () => {
      for (const [file, seen] of initial) {
        if (disposed) return;
        if (seen === undefined) continue;
        const info = await stat(file).catch(() => undefined);
        if (initial.has(file) && fileStamp(info) !== seen) notify(file);
      }
      initial.clear();
    })();
    return current;
  }

  return {
    clear() {
      for (const current of workspaces.values()) current.dispose();
      workspaces.clear();
    },
    update(
      roots: CargoRoot[],
      files: ReadonlySet<string>,
      stamps: ReadonlyMap<string, string>,
    ) {
      const old = workspaces.get(key(roots));
      if (
        old &&
        !old.directoriesChanged &&
        old.files.size === files.size &&
        [...files].every((file) => old.files.has(file))
      )
        return;
      old?.dispose();
      if (files.size) workspaces.set(key(roots), subscribe(files, stamps));
      else workspaces.delete(key(roots));
    },
  };
}
