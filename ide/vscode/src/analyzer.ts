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

/** The language server that answers the editor's go-to actions and hovers on Rust. */
export const RUST_ANALYZER = "rust-lang.rust-analyzer";

/** The running language client, when the workspace may use it. */
export function analyzerClient(): Client | undefined {
  const vscode: typeof import("vscode") = require("vscode");
  if (!vscode.workspace.isTrusted) return undefined;
  const extension = vscode.extensions.getExtension<{ client?: Client }>(
    RUST_ANALYZER,
  );
  return extension?.isActive ? extension.exports?.client : undefined;
}

/** A crate rust-analyzer reads a file in. */
export interface AnalyzedCrate {
  name: string;
  package: string;
  /** Its cfg set, as `rustc --print cfg` prints it. */
  cfg: string;
}

/**
 * Each crate rust-analyzer reads a file in, from its status report: the first
 * is the one whose context it shows. Empty while the file belongs to no crate
 * it has loaded.
 */
export function statusCrates(status: string): AnalyzedCrate[] {
  const section = status.slice(status.indexOf("Crates for file"));
  return section
    .split(/^(?=Crate: )/m)
    .slice(1)
    .flatMap((block) => {
      const name = /^Crate: (.+?)\(Crate\(/.exec(block)?.[1];
      const atoms = /^\s*Enabled cfgs: CfgOptions\((\[.*\])\)$/m.exec(
        block,
      )?.[1];
      if (name === undefined || atoms === undefined) return [];
      const cfg = (JSON.parse(atoms) as string[])
        .map((atom) => {
          const equals = atom.indexOf("=");
          return equals < 0
            ? atom
            : `${atom.slice(0, equals)}=${JSON.stringify(atom.slice(equals + 1))}`;
        })
        .join("\n");
      const pkg = /"CARGO_PKG_NAME": "([^"]*)"/.exec(block)?.[1] ?? "";
      return [{ name, package: pkg, cfg }];
    });
}

/** Where rust-analyzer's definition request leads from `position`. */
export async function locations(
  client: Client,
  uri: string,
  position: Position,
) {
  const found = await client.sendRequest<Location | Location[] | null>(
    "textDocument/definition",
    { textDocument: { uri }, position },
  );
  return (found === null ? [] : Array.isArray(found) ? found : [found]).map(
    (link) =>
      "targetUri" in link
        ? { uri: link.targetUri, position: link.targetSelectionRange.start }
        : { uri: link.uri, position: link.range.start },
  );
}
