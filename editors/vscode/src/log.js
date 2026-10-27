'use strict';

// `vscode` only exists inside the editor, so the module stays loadable from a plain Node test.
let vscode = null;
try {
  vscode = require('vscode');
} catch (e) {
  vscode = null;
}

let channel = null;

/** The 'PlantUML' output channel, or null outside the editor. */
function output() {
  if (vscode === null) {
    return null;
  }
  if (channel === null) {
    channel = vscode.window.createOutputChannel('PlantUML');
  }
  return channel;
}

function log(message) {
  const line = `${new Date().toISOString()} ${message}`;
  try {
    const out = output();
    if (out) {
      out.appendLine(line);
    }
  } catch (e) {
    // Logging must never be the reason rendering fails.
  }
  console.log(`[plantuml-rs] ${line}`);
}

module.exports = { log, output };
