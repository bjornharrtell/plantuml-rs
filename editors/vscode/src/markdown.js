'use strict';

const { renderSvg } = require('./render');
const { log } = require('./log');

const LANGUAGES = new Set(['puml', 'plantuml', 'uml']);

/**
 * Replaces ```puml fences with the rendered SVG, so the built-in Markdown preview shows diagrams.
 * VS Code calls the exported `extendMarkdownIt(md)` of an extension that contributes
 * `markdown.markdownItPlugins`.
 */
function markdownItPlugin(md) {
  log('installing the markdown-it fence renderer');
  const fallback = md.renderer.rules.fence;
  md.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const info = (token.info || '').trim().split(/\s+/)[0].toLowerCase();
    if (!LANGUAGES.has(info)) {
      return fallback
        ? fallback(tokens, idx, options, env, self)
        : self.renderToken(tokens, idx, options);
    }
    try {
      const { svg } = renderSvg(token.content);
      log(`rendered a ${info} block (${token.content.length} chars)`);
      return `<div class="plantuml-rs-block">${svg}</div>`;
    } catch (e) {
      log(`markdown block failed: ${String(e)}`);
      return `<pre class="plantuml-rs-error">${String(e)}</pre>`;
    }
  };
  return md;
}

/** VS Code entry point for markdown-it plugins (`markdown.markdownItPlugins`). */
function extendMarkdownIt(md) {
  return markdownItPlugin(md);
}

module.exports = { markdownItPlugin, extendMarkdownIt };
