import { realpathSync, type Stats } from "node:fs";

/** Share both an in-flight read and its result among callers. */
export function cached<K, V>(
  entries: Map<K, Promise<V>>,
  key: K,
  read: () => Promise<V>,
): Promise<V> {
  const existing = entries.get(key);
  if (existing) return existing;
  const pending = read();
  entries.set(key, pending);
  return pending;
}

/**
 * The real path of `file` as the file system resolves it, as the asynchronous
 * `realpath` does too: a symlink before the `..` after it, which plain
 * `realpathSync` would resolve first.
 */
export const realPath = realpathSync.native;

/** The real path of `file`, or `file` itself when it no longer exists. */
export function realFile(file: string): string {
  try {
    return realPath(file);
  } catch {
    return file;
  }
}

/** A missing path and a non-file are both absent from the Rust module tree. */
export function fileStamp(info?: Stats): string {
  return info?.isFile()
    ? [info.ino, info.ctimeMs, info.mtimeMs, info.size].join("/")
    : "";
}
