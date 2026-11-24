import * as vscode from "vscode";

import { realFile } from "./cache";
import { editorText } from "./offsets";

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
    const physical = realFile(uri.fsPath);
    const dirty = vscode.workspace.textDocuments.find(
      (document) =>
        document.isDirty &&
        document.uri.scheme === "file" &&
        realFile(document.uri.fsPath) === physical,
    );
    if (dirty) return dirty.getText();
  }
  return editorText(
    new TextDecoder().decode(await vscode.workspace.fs.readFile(uri)),
  );
}
