// Explicit query/render control exports; document parsing and URI semantics stay native.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const library = `@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
  {function @name=test.uris @visibility=public @returns=any |
    {param @name=node @type=any @required=true}
    {body | {xpath @context=node @sequence-type="item()*" |
      {expression | (count(document-uri(.)), string(base-uri(.)))}}}}
  {function @name=test.parsed @visibility=public @returns=any |
    {body | {xpath @sequence-type="item()*" |
      {expression | (count(document-uri(parse-xml('<r/>'))), string(base-uri(parse-xml('<r/>'))))}}}}
}`;
const base = 'https://example.test/base/';
const libraryUri = 'https://example.test/functions.cemt';

async function verify(runtime, label) {
    const evaluate = source => {
        const result = JSON.parse(runtime.evaluateQuerySource(source, '{}'));
        assert.equal(result.error, null, JSON.stringify(result));
        assert.deepEqual(result.diagnostics, []);
        return result.items;
    };
    const companion = JSON.parse(runtime.retainCemtXPathFunctions(library, libraryUri));
    assert.ok(companion.companionId > 0);
    try {
        for (const [format, source] of [
            ['xml', '<r>Café 🍒</r>'], ['json', '{"name":"Café 🍒"}'],
            ['csv', 'name\nCafé 🍒\n'], ['yaml', 'name: Café 🍒\n'],
        ]) for (const explicitBase of [false, true]) {
            const parse = `data:parse(${JSON.stringify(source)}, "${format}"${explicitBase ? `, {"base-uri": "${base}"}` : ''})`;
            assert.deepEqual(evaluate(`data:document_uri(${parse})`), [], `${label}: ${format}`);
            assert.deepEqual(evaluate(`data:base_uri(${parse})`), explicitBase
                ? [{ kind: 'atomic', type: 'any-uri', value: base }] : []);
            assert.deepEqual(evaluate(`data:line_number(${parse})`), [{ kind: 'atomic', type: 'integer', value: 1 }]);
            const artifact = JSON.parse(runtime.compileTemplate(`{cem:variable @name=d @select='${parse}'}{cem:variable @name=u @select='native:call("test.uris", d)'}{p | {$seq:first(u)}|{$seq:last(u)}}`, '[]'));
            assert.deepEqual(artifact.diagnostics, []);
            try {
                const rendered = JSON.parse(runtime.renderTemplateWithXPathFunctions(artifact.artifactId, companion.companionId, '{}'));
                assert.deepEqual(rendered.diagnostics, []);
                assert.equal(rendered.nodes[0].children.map(node => node.text).join(''), `0|${explicitBase ? base : ''}`);
            } finally { runtime.disposeTemplate(artifact.artifactId); }
        }
        const artifact = JSON.parse(runtime.compileTemplate('{cem:variable @name=u @select=\'native:call("test.parsed")\'}{p | {$seq:first(u)}|{$seq:last(u)}}', '[]'));
        try {
            const rendered = JSON.parse(runtime.renderTemplateWithXPathFunctions(artifact.artifactId, companion.companionId, '{}'));
            assert.deepEqual(rendered.diagnostics, []);
            assert.equal(rendered.nodes[0].children.map(node => node.text).join(''), `0|${libraryUri}`);
        } finally { runtime.disposeTemplate(artifact.artifactId); }
        const rejected = JSON.parse(runtime.evaluateQuerySource('data:parse("<r>", "xml")', '{}'));
        assert.ok(rejected.error);
        assert.ok(rejected.diagnostics.some(diagnostic => diagnostic.code === 'cem.ql.import_malformed'
            && diagnostic.details.importDiagnostics.some(item => item.uri === 'data:parse/source')));
        console.log(`${label}: constructed document/base/provenance URI separation passed.`);
    } finally { runtime.disposeCemtXPathFunctions(companion.companionId); }
}

const query = await import('../dist/wasm/cem_ql.js');
await query.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
await verify(query, 'CEM-QL WASM');
if (process.argv.includes('--with-common-runtime')) {
    await verify(await import('../../cem-ml-npm/dist/wasm/node/cem_ml.js'), 'CEM-ML Node WASM');
    const browser = await import('../../cem-ml-npm/dist/wasm/browser/cem_ml.js');
    await browser.default({ module_or_path: await readFile(new URL('../../cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    await verify(browser, 'CEM-ML browser WASM');
}
