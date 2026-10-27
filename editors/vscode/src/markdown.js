'use strict';

const { renderSvg } = require('./render');

const LANGUAGES = new Set(['puml', 'plantuml', 'uml']);

/**
 * Replaces ```puml fences with the rendered SVG, so the built-in Markdown preview shows diagrams.
 * Returns the markdown-it plugin; VS Code calls `extendMarkdownIt` on the extension export.
 */
function markdownItPlugin(md) {
  const fallback = md.renderer.rules.fence;
  md.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const info = (token.info || '').trim().split(/\s+/)[0].toLowerCase();
    if (!LANGUAGES.has(info)) {
      return fallback ? fallback(tokens, idx, options, env, self) : self.renderToken(tokens, idx, options);
    }
    const { svg } = renderSvg(token.content);
    return `<div class="plantuml-rs-block">${svg}</div>`;
  };
  return md;
}

/** VS Code entry point for markdown-it plugins (`markdown.markdownItPlugins`). */
function extendMarkdownIt(md) {
  return markdownItPlugin(md);
}

module.exports = { markdownItPlugin, extendMarkdownIt };
