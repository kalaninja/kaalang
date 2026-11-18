import { realpath } from "node:fs/promises";
import * as vscode from "vscode";

const paths = new WeakMap<vscode.TextDocument, Promise<string | undefined>>();

/** Reopening a URI returns the current document even after its editor closed. */
export async function openDocument(
  uri: string,
): Promise<vscode.TextDocument | undefined> {
  try {
    return await vscode.workspace.openTextDocument(vscode.Uri.parse(uri));
  } catch {
    return undefined;
  }
}

/** Read unsaved text when open, and avoid opening closed files in the language server. */
export async function documentText(uri: vscode.Uri): Promise<string> {
  const key = uri.toString();
  const open = vscode.workspace.textDocuments.find(
    (document) => document.uri.toString() === key,
  );
  if (open) return open.getText();
  // Cargo and the server can name the canonical file while an unsaved editor
  // holds the same file under a symlink.
  if (uri.scheme === "file") {
    const dirty = vscode.workspace.textDocuments.filter(
      (document) => document.isDirty && document.uri.scheme === "file",
    );
    if (dirty.length) {
      const physical = await realpath(uri.fsPath).catch(() => undefined);
      for (const document of dirty) {
        let path = paths.get(document);
        if (!path) {
          path = realpath(document.uri.fsPath).catch(() => undefined);
          paths.set(document, path);
        }
        if (physical && (await path) === physical) return document.getText();
      }
    }
  }
  return new TextDecoder().decode(await vscode.workspace.fs.readFile(uri));
}
