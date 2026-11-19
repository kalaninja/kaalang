// Shows the shared page in a webview, in place of a Rust editor or beside it,
// and does for the page what it cannot: hands it the text and the cursor of the
// workspace's Rust document it follows, with the cfg set rust-analyzer sees it
// under, carries out a click with the editor's own actions, reads a wire's type
// and a type's module from the editor's hover, and finds the references to a
// type.

import * as path from "node:path";
import * as vscode from "vscode";

import { watchBuildInputs } from "./build-inputs";
import type { BuildInputs } from "./cargo";
import {
  type CardPlace,
  cardPlacements,
  knownModule,
  watchModuleSources,
} from "./card-places";
import { hoverModule, hoverType, moduleHovers } from "./hover";
import { stopModuleWorker } from "./module-sources";
import { navigate, type Navigation } from "./navigation";
import {
  definitionMatches,
  forgetReferences,
  referringFiles,
} from "./references";
import { affectsCfg, cargoConfig, forgetCfg, target } from "./target";

/** The panel's type, which the `when` clauses in package.json name. */
const VIEW_TYPE = "kaalang.diagram";

/** The language server that answers the editor's go-to actions and hovers on Rust. */
const RUST_ANALYZER = "rust-lang.rust-analyzer";

/** What the page asks of the host; `page.js` documents each message. */
type Request =
  | { type: "ready" }
  | Navigation
  | { type: "types"; id: number; uri: string; offsets: number[] }
  | { type: "referrers"; id: number; uri: string; offset: number }
  | {
      type: "definitions";
      id: number;
      uri: string;
      offset: number;
      places: { uri: string; offset: number }[];
    }
  | {
      type: "modules";
      id: number;
      uri: string;
      places: CardPlace[];
    }
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
  const buildWatchers = new Map<
    string,
    { inputs: BuildInputs; watcher: vscode.Disposable }
  >();

  function clearCfg() {
    forgetCfg();
    for (const { watcher } of buildWatchers.values()) watcher.dispose();
    buildWatchers.clear();
  }

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
        void target(editor.document.uri, (line) => log.appendLine(line)).then(
          ({ owned }) => {
            if (owned && question === following) followSource(editor);
          },
        );
      return;
    }
    followSource(editor);
  }

  function followSource(editor: vscode.TextEditor): void {
    const moved = editor.document !== source?.document;
    source = editor;
    send();
    if (moved) {
      watchConfig();
      void sendCfg();
    }
  }

  /** Hands the page the cfg set of the followed document. */
  async function sendCfg(): Promise<void> {
    if (panel === undefined || source === undefined) {
      return;
    }
    const question = ++asked;
    const uri = source.document.uri;
    const cursor = source.document.offsetAt(source.selection.active);
    const project = await target(
      uri,
      (line) => log.appendLine(line),
      async () => hoverModule(await hoverAt(uri.toString(), cursor)),
    ).catch((error) => {
      log.appendLine(`Could not read the source target: ${String(error)}`);
      return undefined;
    });
    if (question === asked && panel !== undefined) {
      const { workspaceRoot, buildInputs } = project ?? {};
      if (
        workspaceRoot &&
        buildWatchers.get(workspaceRoot)?.inputs !== buildInputs
      ) {
        buildWatchers.get(workspaceRoot)?.watcher.dispose();
        buildWatchers.delete(workspaceRoot);
        if (buildInputs)
          buildWatchers.set(workspaceRoot, {
            inputs: buildInputs,
            watcher: watchBuildInputs(buildInputs, reread),
          });
      }
      void panel.webview.postMessage({
        type: "target",
        uri: uri.toString(),
        cfg: project?.cfg ?? "",
        cfgs: project?.owned ? (project.cfgs ?? [project.cfg]) : undefined,
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
        const result = await referringFiles(
          request.uri,
          request.offset,
          (line) => log.appendLine(line),
          async () => hoverModule(await hoverAt(request.uri, request.offset)),
        );
        if (result !== undefined) {
          reply({ type: "referrers", id: request.id, ...result });
        }
        break;
      }
      case "definitions": {
        const matches = await definitionMatches(request, request.places);
        if (matches) reply({ type: "definitions", id: request.id, matches });
        break;
      }
      case "modules": {
        const { roots } = await target(vscode.Uri.parse(request.uri), (line) =>
          log.appendLine(line),
        );
        const placements = await cardPlacements(request.places, roots);
        const modules = await moduleHovers(request.places, placements, hoverAt);
        reply({ type: "modules", id: request.id, modules, placements });
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

  /** The files the cfg set is read from, whoever changes them. */
  const manifests = vscode.workspace.createFileSystemWatcher(
    "**/{Cargo.toml,Cargo.lock,.cargo/config.toml,.cargo/config,rust-toolchain,rust-toolchain.toml}",
  );
  let configWatcher: vscode.FileSystemWatcher | undefined;
  let watchedConfig: string | undefined;
  function watchConfig(): void {
    const file = vscode.workspace.isTrusted
      ? cargoConfig(source?.document.uri)
      : undefined;
    if (file === watchedConfig) return;
    watchedConfig = file;
    configWatcher?.dispose();
    if (file) {
      configWatcher = vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(
          vscode.Uri.file(path.dirname(file)),
          path.basename(file),
        ),
      );
      configWatcher.onDidCreate(reread);
      configWatcher.onDidChange(reread);
      configWatcher.onDidDelete(reread);
    }
  }
  let rereading: ReturnType<typeof setTimeout> | undefined;
  /** Reads the set again once changes pause: cargo unpacking crates makes hundreds. */
  function reread(): void {
    clearTimeout(rereading);
    rereading = setTimeout(() => {
      // Keep the old input watches until the new Cargo query has finished.
      forgetCfg();
      forgetReferences();
      void sendCfg();
      changed();
    }, 500);
  }

  let changing: ReturnType<typeof setTimeout> | undefined;
  let sourceCfgChanged = false;
  /**
   * Tells the page, once changes pause, that the engine may find other
   * references: it publishes diagnostics as it analyzes the workspace.
   */
  function changed(): void {
    clearTimeout(changing);
    changing = setTimeout(() => {
      if (sourceCfgChanged) {
        sourceCfgChanged = false;
        void sendCfg();
      }
      void panel?.webview.postMessage({ type: "changed" });
    }, 1000);
  }

  function sourcesChanged(source: vscode.Uri | string): void {
    forgetReferences(source);
    sourceCfgChanged = true;
    changed();
  }
  const sources = vscode.workspace.createFileSystemWatcher("**/*.rs");
  watchConfig();
  follow(vscode.window.activeTextEditor);
  context.subscriptions.push(
    log,
    { dispose: stopModuleWorker },
    watchModuleSources(sourcesChanged),
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
          sourcesChanged(document.uri);
        }
        if (document === source?.document) {
          send();
        }
      }
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (affectsCfg(event)) {
        watchConfig();
        clearCfg();
        forgetReferences();
        void sendCfg();
        changed();
      }
    }),
    vscode.languages.onDidChangeDiagnostics(changed),
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      clearCfg();
      watchConfig();
      reread();
    }),
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
      watchConfig();
      forgetReferences();
      void sendCfg();
      changed();
    }),
    manifests,
    sources,
    sources.onDidCreate(sourcesChanged),
    sources.onDidChange(sourcesChanged),
    sources.onDidDelete(sourcesChanged),
    { dispose: () => configWatcher?.dispose() },
    { dispose: clearCfg },
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
