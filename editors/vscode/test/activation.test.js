'use strict';

// Exercise the activation return value used by VS Code, not the module's helper exports.
const assert = require('node:assert/strict');
const Module = require('node:module');
const commands = new Map();
const disposable = { dispose() {} };
let panel;
let changed;
const document = {
  fileName: 'example.puml',
  uri: { toString: () => 'file:///example.puml' },
  getText: () => '@startuml\nAlice -> Bob: hello\n@enduml',
};
const vscode = {
  version: 'test',
  ViewColumn: { Beside: 2 },
  commands: {
    registerCommand(id, callback) { commands.set(id, callback); return disposable; },
  },
  window: {
    activeTextEditor: { document },
    createOutputChannel() { return { ...disposable, appendLine() {}, show() {} }; },
    createWebviewPanel() {
      panel = { ...disposable, webview: { html: '' }, onDidDispose() {} };
      return panel;
    },
    showInformationMessage() {},
  },
  workspace: {
    onDidChangeTextDocument(callback) { changed = callback; return disposable; },
  },
};
const load = Module._load;
Module._load = function (id, ...args) {
  return id === 'vscode' ? vscode : load.call(this, id, ...args);
};
const extension = require('../src/extension');
Module._load = load;
const api = extension.activate({ subscriptions: [] });
assert.equal(typeof api?.extendMarkdownIt, 'function', 'activate must return the Markdown API');
for (const language of ['puml', 'plantuml', 'uml']) {
  const md = api.extendMarkdownIt(require('markdown-it')());
  const html = md.render('```' + language + '\nAlice -> Bob: hello\n```');
  assert.match(html, />Alice<\/text>/);
  assert.doesNotMatch(html, /#fff5f5/);
}
(async () => {
  await commands.get('plantuml-rs.openPreview')();
  assert.match(panel.webview.html, />Alice<\/text>/);
  document.getText = () => '@startuml\nCarol -> Dave: update\n@enduml';
  changed({ document });
  assert.match(panel.webview.html, />Carol<\/text>/);
  assert.doesNotMatch(panel.webview.html, />Alice<\/text>/);
  commands.get('plantuml-rs.showLog')();
  console.log('ok   activation API, all Markdown fence aliases, preview command and live updates');
})().catch((error) => { console.error(error); process.exitCode = 1; });
