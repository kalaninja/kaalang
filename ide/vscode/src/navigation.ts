import * as vscode from "vscode";

import { openDocument } from "./documents";

/** The provider that finds the targets of each go-to request. */
const PROVIDERS = {
  definition: "vscode.executeDefinitionProvider",
  typeDefinition: "vscode.executeTypeDefinitionProvider",
};

export type Navigation =
  | { type: "reveal"; uri: string; start: number; end: number; text?: string }
  | {
      type: keyof typeof PROVIDERS;
      uri: string;
      offset: number;
      text?: string;
    };

/** Opens a click's targets from the source editor, so going back returns there. */
export async function navigate(
  source: vscode.TextEditor,
  request: Navigation,
  current: () => boolean,
): Promise<void> {
  const document = await openDocument(request.uri);
  if (!current() || document === undefined) {
    return;
  }
  if (request.text !== undefined && document.getText() !== request.text) {
    return;
  }
  if (
    source.document.isClosed &&
    source.document.uri.toString() === request.uri &&
    source.document.getText() !== document.getText()
  ) {
    await vscode.window.showTextDocument(document, {
      viewColumn: source.viewColumn,
    });
    return;
  }
  const { version } = document;
  let locations: vscode.Location[];
  const last = request.type === "reveal" ? request.end : request.offset;
  if (
    document.offsetAt(document.positionAt(last)) !== last ||
    (request.type === "reveal" && (request.start < 0 || request.start > last))
  ) {
    return;
  }
  if (request.type === "reveal") {
    locations = [
      new vscode.Location(
        document.uri,
        new vscode.Range(
          document.positionAt(request.start),
          document.positionAt(request.end),
        ),
      ),
    ];
  } else {
    const found = await vscode.commands
      .executeCommand<
        (vscode.Location | vscode.LocationLink)[] | undefined
      >(PROVIDERS[request.type], document.uri, document.positionAt(request.offset))
      .then(
        (found) => found ?? [],
        () => [],
      );
    if (!current() || document.version !== version) {
      return;
    }
    if (found.length === 0) {
      void vscode.window.showInformationMessage(
        request.type === "definition"
          ? "No definition found."
          : "No type definition found.",
      );
      return;
    }
    locations = found.map((target) =>
      "targetUri" in target
        ? new vscode.Location(
            target.targetUri,
            target.targetSelectionRange ?? target.targetRange,
          )
        : target,
    );
  }
  // A hidden or closed source editor reopens in its own group.
  const editor = await vscode.window.showTextDocument(source.document.uri, {
    viewColumn: source.viewColumn,
  });
  if (!current() || document.version !== version) {
    return;
  }
  await vscode.commands.executeCommand(
    "editor.action.goToLocations",
    editor.document.uri,
    editor.selection.active,
    locations,
    "peek",
    "",
  );
  // The jump leaves the cursor at the start. Show the whole requested range.
  const active = vscode.window.activeTextEditor;
  if (
    request.type === "reveal" &&
    active?.document === document &&
    document.version === version
  ) {
    const { range } = locations[0];
    active.selection = new vscode.Selection(range.start, range.end);
  }
}
