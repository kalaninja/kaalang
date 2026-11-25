// Shows the shared page in a webview, in place of a Rust editor or beside it,
// and does for the page what it cannot: hands it the text and the cursor of the
// workspace's Rust document it follows, with the cfg set rust-analyzer sees it
// under, carries out a click with the editor's own actions, reads a wire's type
// from the editor's hover, and finds the references to a type.

import * as path from "node:path";
import * as vscode from "vscode";

import { RUST_ANALYZER } from "./analyzer";
import { knownModule, watchModuleSources } from "./card-places";
import { hoverType } from "./hover";
import { stopModuleWorker } from "./module-sources";
import { navigate, type Navigation } from "./navigation";
import { forgetReferences, referringFiles } from "./references";
import {
  affectsCfg,
  analyzedCfg,
  forgetAnalyzedCfg,
  forgetCfg,
  forgetTargets,
  inBuildDirectory,
  located,
  watchOutsideManifests,
} from "./target";

/** The panel's type, which the `when` clauses in package.json name. */
const VIEW_TYPE = "kaalang.diagram";

/** What the page asks of the host; `page.js` documents each message. */
type Request =
  | { type: "ready" }
  | Navigation
  | { type: "types"; id: number; uri: string; offsets: number[] }
  | { type: "referrers"; id: number; uri: string; offset: number }
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
  /** A newer click or a switch to another source overtakes a pending go-to. */
  let navigation = 0;
  let following = 0;
  const edited = new WeakSet<vscode.TextDocument>();
  const log = vscode.window.createOutputChannel("kaalang");

  /** Follow Rust sources in an open folder or declared by one of its Cargo targets. */
  function follow(editor: vscode.TextEditor | undefined): void {
    if (
      editor &&
      editor.document.uri.toString() !== source?.document.uri.toString()
    )
      ++navigation;
    if (editor?.document.languageId !== "rust") return;
    const question = ++following;
    if (
      editor.document !== source?.document &&
      vscode.workspace.workspaceFolders?.length &&
      !vscode.workspace.getWorkspaceFolder(editor.document.uri)
    ) {
      if (panel && vscode.workspace.isTrusted)
        void located(editor.document.uri, (line) => log.appendLine(line)).then(
          ({ place }) => {
            if (place && question === following) followSource(editor);
          },
          (error) =>
            log.appendLine(`Could not read the source target: ${error}`),
        );
      return;
    }
    followSource(editor);
  }

  function followSource(editor: vscode.TextEditor): void {
    const moved = editor.document !== source?.document;
    source = editor;
    send();
    if (moved) void sendCfg();
  }

  async function sendCfg(): Promise<void> {
    if (panel === undefined || source === undefined) {
      return;
    }
    const question = ++asked;
    const uri = source.document.uri;
    const cfg =
      uri.scheme === "file"
        ? await analyzedCfg(uri.fsPath, (line) => log.appendLine(line))
        : "";
    if (question === asked && panel !== undefined) {
      void panel.webview.postMessage({
        type: "target",
        uri: uri.toString(),
        cfg,
      });
    }
  }

  /** Hands the page the document, as on every change. */
  function send(): void {
    if (panel === undefined || source === undefined) {
      return;
    }
    const { document } = source;
    panel.title = `Diagram ${path.posix.basename(document.uri.path)}`;
    void panel.webview.postMessage({
      type: "document",
      uri: document.uri.toString(),
      text: document.getText(),
      waitForTarget:
        vscode.workspace.isTrusted &&
        document.uri.scheme === "file" &&
        !!vscode.workspace.workspaceFolders?.length,
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
      case "definition":
      case "typeDefinition":
      case "reveal": {
        if (source !== undefined) {
          if (request.type !== "reveal") {
            void offerRustAnalyzer();
          }
          const id = ++navigation;
          await navigate(source, request, () => navigation === id);
        }
        break;
      }
      case "types":
        void offerRustAnalyzer();
        for (const offset of request.offsets) {
          void hoverAt(request.uri, offset).then((text) =>
            reply({
              type: "types",
              id: request.id,
              types: { [offset]: hoverType(text) },
            }),
          );
        }
        break;
      case "referrers": {
        // A failed search draws the focus alone rather than none at all.
        const result = await referringFiles(
          request.uri,
          request.offset,
          (line) => log.appendLine(line),
        ).catch((error: unknown) => {
          log.appendLine(`Could not find the referring types: ${error}`);
          return { files: [] };
        });
        if (result !== undefined) {
          reply({ type: "referrers", id: request.id, ...result });
        }
        break;
      }
      case "back":
        ++navigation;
        await vscode.commands.executeCommand("workbench.action.navigateBack");
        break;
      case "forward":
        ++navigation;
        await vscode.commands.executeCommand(
          "workbench.action.navigateForward",
        );
        break;
    }
  }

  function show(column: vscode.ViewColumn): void {
    if (panel !== undefined) {
      panel.reveal(column);
      return;
    }
    const editor = vscode.window.activeTextEditor;
    panel = vscode.window.createWebviewPanel(VIEW_TYPE, "Diagram", column, {
      enableScripts: true,
      // Else every return to the diagram's tab compiles the module again.
      retainContextWhenHidden: true,
      localResourceRoots: [out],
    });
    panel.webview.html = html(panel.webview, out);
    if (editor && editor !== source) follow(editor);
    panel.webview.onDidReceiveMessage(receive);
    panel.onDidDispose(() => {
      ++following;
      ++navigation;
      panel = undefined;
    });
  }

  /** The manifests that list the workspace's targets. */
  const manifests = vscode.workspace.createFileSystemWatcher("**/Cargo.toml");
  /** Reads the targets and the set again. */
  function reconfigure(): void {
    forgetCfg();
    forgetReferences();
    void sendCfg();
    changed();
  }

  let rereading: ReturnType<typeof setTimeout> | undefined;
  /** Reconfigures once manifest changes pause: cargo unpacking crates makes hundreds. */
  function reread(): void {
    clearTimeout(rereading);
    rereading = setTimeout(reconfigure, 500);
  }

  let changing: ReturnType<typeof setTimeout> | undefined;
  let announce = false;
  /**
   * Once changes pause, asks rust-analyzer for the set again, which it changes
   * as it loads the workspace and reruns build scripts, and tells the page
   * that the engine may find other references: it publishes diagnostics as it
   * analyzes the workspace. The page asks again by itself after an edit of the
   * text it follows.
   */
  function changed(page = true): void {
    announce ||= page;
    clearTimeout(changing);
    changing = setTimeout(() => {
      forgetAnalyzedCfg();
      void sendCfg();
      if (announce) {
        announce = false;
        void panel?.webview.postMessage({ type: "changed" });
      }
    }, 1000);
  }

  function sourcesChanged(source: vscode.Uri | string, page = true): void {
    forgetReferences(source);
    changed(page);
  }
  /** A source file appeared or disappeared, which may add or remove a Cargo target. */
  function sourceSetChanged(source: vscode.Uri): void {
    forgetTargets();
    sourcesChanged(source);
  }
  const sources = vscode.workspace.createFileSystemWatcher("**/*.rs");
  /** Where Cargo finds targets: a folder removed or renamed is reported alone, without its files. */
  const folders = vscode.workspace.createFileSystemWatcher(
    "**/{tests,examples,benches,bin}{,/*}",
    false,
    true,
  );
  function targetFolderChanged(folder: vscode.Uri): void {
    // The source watcher reports a file, and a build fills `examples` in its
    // build directory: a workspace not read yet has it where Cargo puts it by default.
    const relative = vscode.workspace.asRelativePath(folder, false);
    if (
      !/\.rs$|(^|\/)target\//.test(relative) &&
      !inBuildDirectory(folder.fsPath)
    )
      reread();
  }
  follow(vscode.window.activeTextEditor);
  context.subscriptions.push(
    log,
    { dispose: stopModuleWorker },
    watchModuleSources(sourcesChanged),
    watchOutsideManifests(reread),
    vscode.commands.registerCommand("kaalang.showDiagram", () =>
      show(vscode.ViewColumn.Active),
    ),
    vscode.commands.registerCommand("kaalang.showDiagramToSide", () =>
      show(vscode.ViewColumn.Beside),
    ),
    vscode.commands.registerCommand("kaalang.showSource", () => {
      if (source !== undefined) {
        return vscode.window.showTextDocument(source.document.uri, {
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
      if (contentChanges.length > 0) {
        if (document.languageId === "rust" || knownModule(document.uri)) {
          edited.add(document);
          sourcesChanged(document.uri, document !== source?.document);
        }
        if (document === source?.document) {
          send();
        }
      }
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (affectsCfg(event)) reconfigure();
    }),
    // Other extensions publish diagnostics too, for files the engine does not read.
    vscode.languages.onDidChangeDiagnostics(({ uris }) => {
      if (uris.some((uri) => uri.path.endsWith(".rs") || knownModule(uri)))
        changed();
    }),
    vscode.workspace.onDidChangeWorkspaceFolders(reconfigure),
    vscode.workspace.onDidCloseTextDocument((document) => {
      if (edited.has(document)) sourcesChanged(document.uri);
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (document.languageId === "rust" || knownModule(document.uri)) {
        edited.delete(document);
        sourcesChanged(document.uri);
      }
    }),
    vscode.workspace.onDidGrantWorkspaceTrust(() => {
      offered = false;
      reconfigure();
    }),
    manifests,
    sources,
    sources.onDidCreate(sourceSetChanged),
    sources.onDidChange(sourcesChanged),
    sources.onDidDelete(sourceSetChanged),
    folders,
    folders.onDidCreate(targetFolderChanged),
    folders.onDidDelete(targetFolderChanged),
    manifests.onDidCreate(reread),
    manifests.onDidChange(reread),
    manifests.onDidDelete(reread),
  );
}

/**
 * The webview's page. Its policy admits the blob URLs `webview.js` loads the
 * worker and the bindings' glue as, and inline styles: the SVG brings its own
 * `<style>`.
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
      data-glue="${asset("wasm/kaalang_js.mjs")}"
      data-module="${asset("wasm/kaalang_js_bg.wasm")}"
    ></main>
    <script type="module" src="${asset("webview.js")}"></script>
  </body>
</html>`;
}

/** The Markdown of the editor's hover at this place, empty when unavailable. */
async function hoverAt(uri: string, offset: number): Promise<string> {
  try {
    const document = await vscode.workspace.openTextDocument(
      vscode.Uri.parse(uri),
    );
    const position = document.positionAt(offset);
    if (document.offsetAt(position) !== offset) {
      return "";
    }
    const hovers =
      (await vscode.commands.executeCommand<vscode.Hover[] | undefined>(
        "vscode.executeHoverProvider",
        document.uri,
        position,
      )) ?? [];
    return hovers
      .flatMap((hover) => hover.contents)
      .map((part) => (typeof part === "string" ? part : part.value))
      .join("\n");
  } catch {
    return "";
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
