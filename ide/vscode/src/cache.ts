import type { Stats } from "node:fs";

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

/** A missing path and a non-file are both absent from the Rust module tree. */
export function fileStamp(info?: Stats): string {
  return info?.isFile()
    ? [info.ino, info.ctimeMs, info.mtimeMs, info.size].join("/")
    : "";
}
