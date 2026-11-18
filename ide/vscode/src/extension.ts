// Shows the shared page in a webview, in place of a Rust editor or beside it,
// and does for the page what it cannot: hands it the text and the cursor of the
// workspace's Rust document it follows, with the cfg set rust-analyzer sees it
// under, carries out a click with the editor's own actions, reads a wire's type
// and a type's module from the editor's hover, and finds the references to a
// type.

import * as vscode from "vscode";

import { hoverModule, hoverType } from "./hover";
import { offsets } from "./offsets";
import { affectsCfg, cfg, forgetCfg } from "./target";

/** The panel's type, which the `when` clauses in package.json name. */
const VIEW_TYPE = "kaalang.diagram";

/** The language server that answers the editor's go-to actions and hovers on Rust. */
const RUST_ANALYZER = "rust-lang.rust-analyzer";

/** The provider that finds the targets of each go-to request. */
const PROVIDERS = {
  definition: "vscode.executeDefinitionProvider",
  typeDefinition: "vscode.executeTypeDefinitionProvider",
};

/** What the page asks of the host; `page.js` documents each message. */
type Request =
  | { type: "ready" }
  | { type: "reveal"; start: number; end: number; uri?: string }
  | { type: keyof typeof PROVIDERS; offset: number; uri?: string }
  | { type: "types"; id: number; offsets: number[] }
  | { type: "referrers"; id: number; uri: string; offset: number }
  | { type: "modules"; id: number; places: { uri: string; offset: number }[] }
  | { type: "back" | "forward" };

export function activate(context: vscode.ExtensionContext): void {
  const out = vscode.Uri.joinPath(context.extensionUri, "out");
  let panel: vscode.WebviewPanel | undefined;
  /**
   * The last Rust editor active, which the diagram follows while another has
   * focus. Its column stays current as groups close; hidden, it keeps its
   * document, cursor, and column.
   */
  let source: vscode.TextEditor | undefined;
  /** Counts the questions for a cfg set; only the latest answer is sent. */
  let asked = 0;
  const log = vscode.window.createOutputChannel("kaalang");

  /** Follows `editor`, unless it shows a document outside the workspace. */
  function follow(editor: vscode.TextEditor | undefined): void {
    if (
      editor?.document.languageId !== "rust" ||
      (vscode.workspace.workspaceFolders?.length &&
        vscode.workspace.getWorkspaceFolder(editor.document.uri) === undefined)
    ) {
      return;
    }
    const moved = editor.document !== source?.document;
    source = editor;
    send();
    if (moved) {
      void sendCfg();
    }
  }

  /** Hands the page the cfg set of the followed document. */
  async function sendCfg(): Promise<void> {
    if (panel === undefined || source === undefined) {
      return;
    }
    const question = ++asked;
    const set = await cfg(source.document.uri, (line) => log.appendLine(line));
    if (question === asked && panel !== undefined) {
      void panel.webview.postMessage({ type: "target", cfg: set });
    }
  }

  /** Hands the page the document, as on every change. */
  function send(): void {
    if (panel === undefined || source === undefined) {
      return;
    }
    const { document } = source;
    panel.title = `Diagram ${document.uri.path.slice(document.uri.path.lastIndexOf("/") + 1)}`;
    void panel.webview.postMessage({
      type: "document",
      uri: document.uri.toString(),
      text: document.getText(),
      // By line and character, as the editor keeps it while text before it changes.
      cursor: document.offsetAt(source.selection.active),
    });
  }

  async function receive(request: Request): Promise<void> {
    // An answer goes to the page that asked, not to one opened since, which
    // numbers its requests anew.
    const asker = panel;
    const reply = (message: object): void => {
      if (panel === asker) {
        void asker?.webview.postMessage(message);
      }
    };
    switch (request.type) {
      case "ready":
        send();
        void sendCfg();
        break;
      case "reveal":
        await reveal(request.start, request.end, request.uri);
        break;
      case "definition":
      case "typeDefinition":
        void offerRustAnalyzer();
        await goTo(request.type, request.offset, request.uri);
        break;
      case "types":
        void offerRustAnalyzer();
        sendTypes(request.id, request.offsets, reply);
        break;
      case "referrers":
        void sendReferrers(request, reply);
        break;
      case "modules":
        void sendModules(request.id, request.places, reply);
        break;
      case "back":
        await vscode.commands.executeCommand("workbench.action.navigateBack");
        break;
      case "forward":
        await vscode.commands.executeCommand(
          "workbench.action.navigateForward",
        );
        break;
    }
  }

  /**
   * Runs the go-to request `type` at `offset` of the followed document, or of
   * the one at `uri`, and opens what it finds as the editor's own go-to does,
   * without first moving the cursor to the place asked about: the diagram
   * follows only the target. None found is said.
   */
  async function goTo(
    type: keyof typeof PROVIDERS,
    offset: number,
    uri?: string,
  ): Promise<void> {
    if (source === undefined) {
      return;
    }
    const document = uri === undefined ? source.document : await open(uri);
    const found = await vscode.commands
      .executeCommand<
        (vscode.Location | vscode.LocationLink)[] | undefined
      >(PROVIDERS[type], document.uri, document.positionAt(offset))
      .then(
        (found) => found ?? [],
        () => [],
      );
    if (found.length === 0) {
      void vscode.window.showInformationMessage(
        type === "definition"
          ? "No definition found."
          : "No type definition found.",
      );
      return;
    }
    await jump(
      found.map((target) =>
        "targetUri" in target
          ? new vscode.Location(
              target.targetUri,
              target.targetSelectionRange ?? target.targetRange,
            )
          : target,
      ),
    );
  }

  /**
   * Opens `locations` as the editor's own go-to does, from where the followed
   * editor's cursor stands, so that going back returns there: a single one at
   * once, several in a peek.
   */
  async function jump(locations: vscode.Location[]): Promise<void> {
    if (source === undefined) {
      return;
    }
    // The go-to runs from the editor that has focus; a closed one reopens.
    const editor = await vscode.window.showTextDocument(source.document, {
      viewColumn: source.viewColumn,
    });
    await vscode.commands.executeCommand(
      "editor.action.goToLocations",
      editor.document.uri,
      editor.selection.active,
      locations,
      "peek",
      "",
    );
  }

  /**
   * Answers the page's request for the references to the type named at an
   * offset of a document: each document whose text holds a `refer`, where
   * alone they can draw a card, with its text, its cfg set, and where each
   * reference starts. Documents not open are read from disk, so that the
   * engine is not told of them and analyzes nothing anew. A search that the
   * document's next edit cancels is not answered: the page asks again.
   */
  async function sendReferrers(
    request: { id: number; uri: string; offset: number },
    reply: (message: object) => void,
  ): Promise<void> {
    const document = await open(request.uri).catch(() => undefined);
    if (document === undefined) {
      return;
    }
    const { version } = document;
    const locations = await vscode.commands
      .executeCommand<
        vscode.Location[] | undefined
      >("vscode.executeReferenceProvider", document.uri, document.positionAt(request.offset))
      .then(
        (found) => found ?? [],
        () => [],
      );
    const found = new Map<string, vscode.Range[]>();
    for (const { uri, range } of locations) {
      found.set(uri.toString(), [...(found.get(uri.toString()) ?? []), range]);
    }
    const files = await Promise.all(
      [...found].map(async ([uri, ranges]) => {
        try {
          const opened = vscode.workspace.textDocuments.find(
            (open) => open.uri.toString() === uri,
          );
          const text =
            opened?.getText() ??
            new TextDecoder().decode(
              await vscode.workspace.fs.readFile(vscode.Uri.parse(uri)),
            );
          if (!text.includes("refer")) {
            return null;
          }
          const at = offsets(text);
          return {
            uri,
            text,
            cfg: await cfg(vscode.Uri.parse(uri), (line) =>
              log.appendLine(line),
            ),
            offsets: ranges.map((range) =>
              at(range.start.line, range.start.character),
            ),
          };
        } catch {
          return null;
        }
      }),
    );
    if (document.version === version) {
      reply({
        type: "referrers",
        id: request.id,
        files: files.filter((file) => file !== null),
      });
    }
  }

  /**
   * Answers the page's request `id` with the module path of the type named at
   * each place, as the editor's hover gives it, or `null`.
   */
  async function sendModules(
    id: number,
    places: { uri: string; offset: number }[],
    reply: (message: object) => void,
  ): Promise<void> {
    const modules = await Promise.all(
      places.map(async ({ uri, offset }) => {
        try {
          const document = await open(uri);
          return hoverModule(await hoverAt(document, offset));
        } catch {
          return null;
        }
      }),
    );
    reply({ type: "modules", id, modules });
  }

  /** The followed document when `uri` names it, or the document at `uri`. */
  async function open(uri: string): Promise<vscode.TextDocument> {
    return source?.document.uri.toString() === uri
      ? source.document
      : vscode.workspace.openTextDocument(vscode.Uri.parse(uri));
  }

  /**
   * Answers the page's request `id` with the type at each offset of the
   * document it follows, each as soon as it is known.
   */
  function sendTypes(
    id: number,
    offsets: number[],
    reply: (message: object) => void,
  ): void {
    if (source === undefined) {
      return;
    }
    const { document } = source;
    for (const offset of offsets) {
      void typeAt(document, offset).then((type) => {
        reply({ type: "types", id, types: { [offset]: type } });
      });
    }
  }

  /**
   * Selects a range of the followed document, or of the one at `uri`, as a
   * jump going back returns from.
   */
  async function reveal(
    start: number,
    end: number,
    uri?: string,
  ): Promise<void> {
    if (source === undefined) {
      return;
    }
    // The text the page counted the offsets in; a closed document stays readable.
    const document = uri === undefined ? source.document : await open(uri);
    const range = new vscode.Range(
      document.positionAt(start),
      document.positionAt(end),
    );
    await jump([new vscode.Location(document.uri, range)]);
    // The jump leaves the cursor at the start; the range shows what was asked.
    const editor = vscode.window.activeTextEditor;
    if (editor?.document === document) {
      editor.selection = new vscode.Selection(range.start, range.end);
    }
  }

  function show(column: vscode.ViewColumn): void {
    if (panel !== undefined) {
      panel.reveal(column);
      return;
    }
    panel = vscode.window.createWebviewPanel(VIEW_TYPE, "Diagram", column, {
      enableScripts: true,
      // Else every return to the diagram's tab compiles the module again.
      retainContextWhenHidden: true,
      localResourceRoots: [out],
    });
    panel.webview.html = html(panel.webview, out);
    panel.webview.onDidReceiveMessage(receive);
    panel.onDidDispose(() => {
      panel = undefined;
    });
  }

  /** The files the cfg set is read from, whoever changes them. */
  const manifests = vscode.workspace.createFileSystemWatcher(
    "**/{Cargo.toml,Cargo.lock,.cargo/config.toml,.cargo/config}",
  );
  let rereading: ReturnType<typeof setTimeout> | undefined;
  /** Reads the set again once changes pause: cargo unpacking crates makes hundreds. */
  function reread(): void {
    clearTimeout(rereading);
    rereading = setTimeout(() => {
      forgetCfg();
      void sendCfg();
    }, 500);
  }

  let changing: ReturnType<typeof setTimeout> | undefined;
  /**
   * Tells the page, once changes pause, that the engine may find other
   * references: it publishes diagnostics as it analyzes the workspace.
   */
  function changed(): void {
    clearTimeout(changing);
    changing = setTimeout(() => {
      void panel?.webview.postMessage({ type: "changed" });
    }, 1000);
  }

  follow(vscode.window.activeTextEditor);
  context.subscriptions.push(
    log,
    vscode.commands.registerCommand("kaalang.showDiagram", () =>
      show(vscode.ViewColumn.Active),
    ),
    vscode.commands.registerCommand("kaalang.showDiagramToSide", () =>
      show(vscode.ViewColumn.Beside),
    ),
    vscode.commands.registerCommand("kaalang.showSource", () => {
      if (source !== undefined) {
        void vscode.window.showTextDocument(source.document, {
          viewColumn: source.viewColumn,
        });
      }
    }),
    vscode.window.onDidChangeActiveTextEditor(follow),
    // VS Code reports a selection moved by an edit in every editor of the
    // document, so another editor's would take the diagram over.
    vscode.window.onDidChangeTextEditorSelection(({ textEditor }) => {
      if (textEditor === source) {
        follow(textEditor);
      }
    }),
    // A save changes no text but reports a change too.
    vscode.workspace.onDidChangeTextDocument(({ document, contentChanges }) => {
      if (document === source?.document && contentChanges.length > 0) {
        send();
      }
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (affectsCfg(event)) {
        void sendCfg();
      }
    }),
    vscode.languages.onDidChangeDiagnostics(changed),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (document.languageId === "rust") {
        changed();
      }
    }),
    vscode.workspace.onDidGrantWorkspaceTrust(() => {
      offered = false;
      void sendCfg();
    }),
    manifests,
    manifests.onDidCreate(reread),
    manifests.onDidChange(reread),
    manifests.onDidDelete(reread),
  );
}

/**
 * The webview's page. A worker must come from the page's own origin and the
 * extension's files come from another, so `webview.js` loads the worker and the
 * bindings' glue as blob URLs; the SVG brings its own `<style>`.
 */
function html(webview: vscode.Webview, out: vscode.Uri): string {
  const asset = (path: string) =>
    webview.asWebviewUri(vscode.Uri.joinPath(out, path)).toString();
  const csp = [
    "default-src 'none'",
    `style-src ${webview.cspSource} 'unsafe-inline'`,
    `script-src ${webview.cspSource} blob: 'wasm-unsafe-eval'`,
    "worker-src blob:",
    `connect-src ${webview.cspSource}`,
  ].join("; ");
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta http-equiv="Content-Security-Policy" content="${csp}" />
    <link rel="stylesheet" href="${asset("page/page.css")}" />
    <style>
      html, body { height: 100%; margin: 0; padding: 0; }
    </style>
  </head>
  <body>
    <main
      id="page"
      data-worker="${asset("page/worker.js")}"
      data-glue="${asset("wasm/kaalang_js.js")}"
      data-module="${asset("wasm/kaalang_js_bg.wasm")}"
    ></main>
    <script type="module" src="${asset("webview.js")}"></script>
  </body>
</html>`;
}

/** The type the editor's hover gives the value bound at `offset`, or `null`. */
async function typeAt(
  document: vscode.TextDocument,
  offset: number,
): Promise<string | null> {
  if (document.isClosed) {
    return null;
  }
  try {
    return hoverType(await hoverAt(document, offset));
  } catch {
    return null;
  }
}

/** The text of the editor's hover at `offset`, as Markdown. */
async function hoverAt(
  document: vscode.TextDocument,
  offset: number,
): Promise<string> {
  const hovers =
    (await vscode.commands.executeCommand<vscode.Hover[] | undefined>(
      "vscode.executeHoverProvider",
      document.uri,
      document.positionAt(offset),
    )) ?? [];
  return hovers
    .flatMap((hover) => hover.contents)
    .map((part) => (typeof part === "string" ? part : part.value))
    .join("\n");
}

/**
 * Whether rust-analyzer, or the trust it needs, was offered since activation or
 * since trust was granted: once is enough.
 */
let offered = false;

/**
 * Offers rust-analyzer, which types and go-to actions need, when it is missing,
 * or the trust it needs to run in Restricted Mode, unless its settings let it
 * run there.
 */
async function offerRustAnalyzer(): Promise<void> {
  const trusted = vscode.workspace.isTrusted;
  const extension = vscode.extensions.getExtension(RUST_ANALYZER);
  if (
    offered ||
    (trusted ? extension !== undefined : extension?.isActive === true)
  ) {
    return;
  }
  offered = true;
  if (!trusted) {
    const trust = "Manage Workspace Trust";
    const choice = await vscode.window.showInformationMessage(
      "Types and go-to actions in the diagram need rust-analyzer, which does not run in Restricted Mode.",
      trust,
    );
    if (choice === trust) {
      await vscode.commands.executeCommand("workbench.trust.manage");
    }
    return;
  }
  const install = "Install";
  const choice = await vscode.window.showInformationMessage(
    "Types and go-to actions in the diagram need rust-analyzer.",
    install,
  );
  if (choice === install) {
    await vscode.commands.executeCommand(
      "workbench.extensions.installExtension",
      RUST_ANALYZER,
    );
  }
}
