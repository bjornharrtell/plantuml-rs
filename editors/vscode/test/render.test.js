'use strict';

const assert = require('node:assert');
const { renderSvg } = require('../src/render');
const { extendMarkdownIt } = require('../src/markdown');

let failures = 0;
function check(name, fn) {
  try {
    fn();
    console.log(`ok   ${name}`);
  } catch (e) {
    failures += 1;
    console.log(`FAIL ${name}: ${e.message}`);
  }
}

check('renders a sequence diagram', () => {
  const { ok, svg } = renderSvg('@startuml\nAlice -> Bob: hi\n@enduml');
  assert.ok(ok, 'renders');
  assert.match(svg, /<svg/);
  assert.match(svg, />Alice<\/text>/);
});

check('renders a class diagram', () => {
  const { ok, svg } = renderSvg('@startuml\nclass A {\n +int x\n}\nclass B\nA --> B\n@enduml');
  assert.ok(ok, 'renders');
  assert.match(svg, /class="entity"/);
  assert.match(svg, />int x<\/text>/);
});

check('renders C4 through the bundled stdlib', () => {
  const { ok, svg } = renderSvg('@startuml\n!include <C4/C4_Context>\nPerson(u, "User")\nSystem(s, "Sys")\nRel(u, s, "Uses")\n@enduml');
  assert.ok(ok, 'renders');
  assert.match(svg, />Sys<\/text>/);
});

check('reports an error instead of throwing', () => {
  const { ok, svg } = renderSvg('@startuml\nclass\n@enduml');
  assert.strictEqual(ok, false);
  assert.match(svg, /<svg/);
});

check('renders ```puml fences in markdown', () => {
  const md = extendMarkdownIt(require('markdown-it')({ html: true }));
  const html = md.render('before\n\n```puml\nA -> B\n```\n\nafter');
  assert.match(html, /plantuml-rs-block/);
  assert.match(html, /<svg/);
  assert.match(html, /before/);
});

check('leaves other fences alone', () => {
  const md = extendMarkdownIt(require('markdown-it')({ html: true }));
  const html = md.render('```js\nlet a = 1;\n```');
  assert.match(html, /language-js/);
  assert.doesNotMatch(html, /plantuml-rs-block/);
});

if (failures > 0) {
  process.exit(1);
}
