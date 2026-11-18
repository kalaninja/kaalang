// Shows the shared page in a webview, in place of a Rust editor or beside it,
// and does for the page what it cannot: hands it the text and the cursor of the
// Rust document it follows, with the cfg set rust-analyzer sees it under,
// carries out a click with the editor's own actions, and reads a wire's type
// from the editor's hover.

import * as vscode from "vscode";

import { hoverType } from "./hover";
import { affectsCfg, cfg, forgetCfg } from "./target";

/** The panel's type, which the `when` clauses in package.json name. */
const VIEW_TYPE = "kaalang.diagram";

/** The language server that answers the editor's go-to actions and hovers on Rust. */
const RUST_ANALYZER = "rust-lang.rust-analyzer";

/** The editor action each of the page's go-to requests runs. */
const ACTIONS = {
  definition: "editor.action.revealDefinition",
  typeDefinition: "editor.action.goToTypeDefinition",
};

/** What the page asks of the host; `page.js` documents each message. */
type Request =
  | { type: "ready" }
  | { type: "reveal"; start: number; end: number }
  | { type: keyof typeof ACTIONS; offset: number }
  | { type: "types"; id: number; offsets: number[] };

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

  function follow(editor: vscode.TextEditor | undefined): void {
    if (editor?.document.languageId !== "rust") {
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
    switch (request.type) {
      case "ready":
        send();
        void sendCfg();
        break;
      case "reveal":
        await select(request.start, request.end);
        break;
      case "definition":
      case "typeDefinition":
        void offerRustAnalyzer();
        await select(request.offset, request.offset);
        await vscode.commands.executeCommand(ACTIONS[request.type]);
        break;
      case "types":
        void offerRustAnalyzer();
        sendTypes(request.id, request.offsets);
        break;
    }
  }

  /**
   * Answers the page's request `id` with the type at each offset of the
   * document it follows, each as soon as it is known.
   */
  function sendTypes(id: number, offsets: number[]): void {
    if (source === undefined) {
      return;
    }
    const { document } = source;
    for (const offset of offsets) {
      void typeAt(document, offset).then((type) => {
        void panel?.webview.postMessage({
          type: "types",
          id,
          types: { [offset]: type },
        });
      });
    }
  }

  /** Selects a range of the followed document in its editor, which comes to the front. */
  async function select(start: number, end: number): Promise<void> {
    if (source === undefined) {
      return;
    }
    // The text the page counted the offsets in; a closed document stays readable.
    const { document, viewColumn } = source;
    await vscode.window.showTextDocument(document, {
      viewColumn,
      selection: new vscode.Range(
        document.positionAt(start),
        document.positionAt(end),
      ),
    });
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
    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
      "vscode.executeHoverProvider",
      document.uri,
      document.positionAt(offset),
    );
    const parts = hovers.flatMap((hover) => hover.contents);
    return hoverType(
      parts
        .map((part) => (typeof part === "string" ? part : part.value))
        .join("\n"),
    );
  } catch {
    return null;
  }
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
