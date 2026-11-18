// The navigation requests sent to the editor's running rust-analyzer client.
interface Position {
  line: number;
  character: number;
}

type Location =
  | { uri: string; range: { start: Position } }
  | { targetUri: string; targetSelectionRange: { start: Position } };

interface Client {
  sendRequest<T>(method: string, params: object): Promise<T>;
}

/** The running language client, when the workspace may use it. */
export function analyzerClient(): Client | undefined {
  const vscode: typeof import("vscode") = require("vscode");
  if (!vscode.workspace.isTrusted) return undefined;
  const extension = vscode.extensions.getExtension<{ client?: Client }>(
    "rust-lang.rust-analyzer",
  );
  return extension?.isActive ? extension.exports?.client : undefined;
}

/** The selected destinations of a language-server navigation request. */
export async function locations(
  client: Client,
  method: string,
  uri: string,
  position: Position,
) {
  const found = await client.sendRequest<Location | Location[] | null>(method, {
    textDocument: { uri },
    position,
  });
  return (found === null ? [] : Array.isArray(found) ? found : [found]).map(
    (link) =>
      "targetUri" in link
        ? { uri: link.targetUri, position: link.targetSelectionRange.start }
        : { uri: link.uri, position: link.range.start },
  );
}
