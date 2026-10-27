'use strict';

const vscode = require('vscode');
const { renderSvg } = require('./render');
const { extendMarkdownIt } = require('./markdown');

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand('plantuml-rs.openPreview', () => openPreview(context)),
  );
}

/** Opens (or reuses) a preview panel for the active PlantUML document. */
async function openPreview(context) {
  const editor = vscode.window.activeTextEditor;
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
  const subscription = vscode.workspace.onDidChangeTextDocument((e) => {
    if (e.document.uri.toString() === document.uri.toString()) {
      update();
    }
  });
  const saved = vscode.workspace.onDidSaveTextDocument((d) => {
    if (d.uri.toString() === document.uri.toString()) {
      update();
    }
  });
  panel.onDidDispose(() => {
    subscription.dispose();
    saved.dispose();
  });
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
