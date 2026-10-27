// Optional editor gate: see ../README.md for the isolated dependency setup.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { Registry, parseRawGrammar } = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');

(async () => {
  const wasm = fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm'));
  await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
  const file = path.join(__dirname, '../cem-ml.tmLanguage.json');
  const registry = new Registry({
    onigLib: Promise.resolve({
      createOnigScanner: patterns => new oniguruma.OnigScanner(patterns),
      createOnigString: text => new oniguruma.OnigString(text),
    }),
    loadGrammar: async () => parseRawGrammar(fs.readFileSync(file, 'utf8'), file),
  });
  const grammar = await registry.loadGrammar('source.cem-ml');
  const queries = [
    '#nodes["}"]', '#nodes[\'it\'\'s}\']', '#nodes["a\\"}b"]',
    '#nodes["say""}"]', '#nodes (: } (: { :) still } :)',
    '#choose({nested: {value: "}"}}, nodes)', '#nodes\n (: } :)',
  ];
  for (const query of queries) {
    for (const slot of [`{${query}}`, `@target={${query}}`]) {
      const source = `{section @quoted='{#nodes}' ${slot} | {after}}`;
      let stack;
      let offset = 0;
      const tokens = [];
      for (const line of source.split('\n')) {
        const result = grammar.tokenizeLine(line, stack);
        tokens.push(...result.tokens.map(token => ({
          start: offset + token.startIndex, end: offset + token.endIndex, scopes: token.scopes,
        })));
        stack = result.ruleStack;
        offset += line.length + 1;
      }
      const scopesAt = index => tokens.find(token => token.start <= index && token.end > index)?.scopes || [];
      const queryStart = source.indexOf(query);
      for (let i = 0; i < query.length; i++) {
        if (query[i] === '}') {
          const scopes = scopesAt(queryStart + i);
          assert(!scopes.includes('punctuation.section.node.end.cem-ml'), source);
          assert(!scopes.includes('punctuation.section.cem-ql.end.cem-ml'), source);
          assert(scopes.some(scope => scope.endsWith('.cem-ql')), source);
        }
      }
      const literalScopes = scopesAt(source.indexOf("'{#nodes}'") + 2);
      assert(literalScopes.includes('string.quoted.single.cem-ml'), source);
      assert(!literalScopes.includes('meta.expression-node.cem-ml'), source);
      assert(scopesAt(source.lastIndexOf('after')).includes('entity.name.tag.cem-ml'), source);
    }
  }
  for (const eol of ['\n', '\r\n']) {
    const source = `{row @value=\`\`\`\`${eol}can't "quote" \`\`\` {#never} {$never()}${eol}\`\`\`\` @after=ok | {after}}`;
    let stack;
    const lines = source.split('\n');
    const tokenized = lines.map(line => {
      const result = grammar.tokenizeLine(line, stack);
      stack = result.ruleStack;
      return result.tokens;
    });
    const scopesAt = (line, index) => tokenized[line].find(token => token.startIndex <= index && token.endIndex > index)?.scopes || [];
    const payload = scopesAt(1, lines[1].indexOf('{#never}'));
    assert(payload.includes('string.quoted.other.fenced.cem-ml'), source);
    assert(!payload.includes('meta.expression-node.cem-ml'), source);
    assert(scopesAt(2, lines[2].lastIndexOf('after')).includes('entity.name.tag.cem-ml'), source);
  }
  registry.dispose();
  console.log(`TextMate parity: ${queries.length * 2} expression/attribute cases and 2 fenced literal cases passed`);
})().catch(error => { console.error(error); process.exitCode = 1; });
