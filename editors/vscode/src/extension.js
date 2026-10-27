'use strict';

const vscode = require('vscode');
const { renderSvg, wasmStatus } = require('./render');
const { extendMarkdownIt } = require('./markdown');
const { log, output } = require('./log');

function activate(context) {
  log(`activated (vscode ${vscode.version}); wasm ${JSON.stringify(wasmStatus())}`);
  context.subscriptions.push(output());
  context.subscriptions.push(
    vscode.commands.registerCommand('plantuml-rs.openPreview', () => openPreview(context)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('plantuml-rs.showLog', () => output().show(true)),
  );
  return { extendMarkdownIt };
}

/** Opens a preview panel for the active PlantUML document and keeps it in sync. */
async function openPreview(context) {
  const editor = vscode.window.activeTextEditor;
  log(`openPreview for ${editor ? editor.document.fileName : 'no editor'}`);
  if (!editor) {
    vscode.window.showInformationMessage('Open a .puml file first.');
    return;
  }
  const document = editor.document;
  const panel = vscode.window.createWebviewPanel(
    'plantuml-rs.preview',
    `Preview ${document.fileName.split(/[\\/]/).pop()}`,
    vscode.ViewColumn.Beside,
    { enableScripts: false, retainContextWhenHidden: true },
  );
  const update = () => {
    panel.webview.html = previewHtml(document.getText());
  };
  update();
  const changed = vscode.workspace.onDidChangeTextDocument((e) => {
    if (e.document.uri.toString() === document.uri.toString()) {
      update();
    }
  });
  panel.onDidDispose(() => changed.dispose());
  context.subscriptions.push(panel);
}

function previewHtml(source) {
  const { svg } = renderSvg(source);
  return `<!doctype html><html><head><meta charset="utf-8">
<style>
  body { margin: 0; padding: 8px; background: var(--vscode-editor-background); }
  svg { max-width: 100%; height: auto; }
</style></head><body>${svg}</body></html>`;
}

module.exports = { activate, extendMarkdownIt };
