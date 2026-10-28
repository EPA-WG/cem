// Host transports native source bytes and scalar options; no document AST handoff.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const rootUri = 'https://example.test/root.xsl', childUri = 'https://example.test/child.xsl';
const root = '<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:controls" version="3.0"><xsl:import href="./child.xsl"/></xsl:stylesheet>';
const child = `<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:controls" version="3.0">
<xsl:template match="/"><xsl:call-template name="view"/></xsl:template>
<xsl:template name="view"><xsl:param name="p:label" select="'Default'"/><p><xsl:value-of select="$p:label"/></p></xsl:template></xsl:stylesheet>`;
const selection = JSON.stringify([{ packageId: 'xslt', version: '1.0.0' }]);
const host = JSON.stringify({ rootQueueSize: 1024, schemaPackageReplacementGrants: [{ packageId: 'xslt',
    expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'schema-packages/xslt/v1/package.cem' }] });
const options = { entrypoint: 'view', parameters: [{ name: 'p:label', select: '"Fresh Café 🍒"' }] };

function missing(call) {
    for (const [method, args] of [
        ['xsltStylesheetImports', [root, rootUri]], ['compileXsltBundle', [child, childUri]],
        ['retainXsltComponent', [child, childUri, JSON.stringify(options), '[]']],
    ]) assert.throws(() => call(method, args), error => String(error).includes('cem.schema_asset.missing'), method);
}

function exercise(call) {
    assert.deepEqual(JSON.parse(call('xsltStylesheetImports', [root, rootUri])), ['./child.xsl']);
    assert.ok(call('compileXsltBundle', [child, childUri]).length > 0);
    const bundle = JSON.parse(call('retainXsltStylesheet', [child, childUri]));
    let component, document;
    try {
        assert.equal(bundle.sourceClosure[0].source.uri, childUri);
        assert.equal(bundle.generatedSource.uri, `${childUri}#generated-cemt`);
        const mapped = { ...options, modules: [{ parentUri: rootUri, href: './child.xsl', uri: childUri,
            source: child, contentHash: bundle.rootSourceHash }] };
        component = JSON.parse(call('retainXsltComponent', [root, rootUri, JSON.stringify(mapped), '[]']));
        document = call('retainCemDocument', [new TextEncoder().encode('<r/>'), 'application/xml', 'memory:input.xml']);
        const rendered = JSON.parse(call('renderXsltComponent', [component.artifactId, '{}', '[]', document]));
        assert.deepEqual(rendered.diagnostics, []);
        assert.equal(rendered.nodes[0].tag, 'p');
        assert.equal(rendered.nodes[0].children[0].text, 'Fresh Café 🍒');
        assert.throws(() => call('retainXsltComponent', [root, rootUri,
            JSON.stringify({ ...mapped, modules: [{ ...mapped.modules[0], contentHash: bundle.contentHash }] }), '[]']));
    } finally {
        if (document) call('disposeCemDocument', [document]);
        if (component) call('disposeXsltComponent', [component.artifactId]);
        call('disposeXsltBundle', [bundle.bundleId]);
    }
}

const full = await import('../dist/wasm/browser/cem_ml_factory.js');
const ordinary = full.createCemMlWasmGeneration();
await ordinary.initialize(await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)));
try { exercise((method, args) => ordinary.call(method, args)); }
catch (error) { console.error('Complete XSLT exercise failed:', error); throw error; }
finally { ordinary.dispose(); }
console.log('Complete WASM: imports, namespaced controls, module hashes and native XSLT output passed.');
if (process.argv.includes('--complete-only')) process.exit(0);

const optional = await import('../dist-schema-assets-wasm/wasm/browser/cem_ml_factory.js');
const binary = await readFile(new URL('../dist-schema-assets-wasm/wasm/browser/cem_ml_bg.wasm', import.meta.url));
const indexBytes = await readFile(new URL('../dist-schema-assets/index.json', import.meta.url));
const index = JSON.parse(indexBytes);
const originals = await Promise.all(index.sources.filter(source => source.packageId === 'xslt').map(async source => ({
    ...source, bytes: await readFile(new URL(`../dist-schema-assets/sources/${source.digest}`, import.meta.url)),
})));
let previousMemory;
for (const label of ['first', 'fresh']) {
    const generation = optional.createCemMlWasmGeneration();
    await generation.initialize(binary);
    assert.notEqual(generation.memory(), previousMemory);
    previousMemory = generation.memory();
    const call = (method, args) => generation.call(method, args);
    let owner;
    try {
        missing(call);
        const pin = JSON.parse(call('schemaAssetPackBootstrapV1', []));
        owner = call('schemaAssetOwnerOpenV1', [indexBytes, pin.indexDigest, host]);
        assert.throws(() => call('schemaAssetOwnerActivateV1', [owner, selection]));
        missing(call);
        for (const source of originals) call('schemaAssetOwnerAdmitV1', [owner, 'xslt', '1.0.0', source.path, source.bytes]);
        assert.equal(JSON.parse(call('schemaAssetOwnerActivateV1', [owner, selection])).ready, true);
        exercise(call);
        assert.equal(call('schemaAssetOwnerDisposeV1', [owner]), true); owner = undefined;
        missing(call);
        console.log(`Optional WASM ${label}: missing/incomplete/disposed authority refusal and original-source replay passed.`);
    } finally {
        if (owner) call('schemaAssetOwnerDisposeV1', [owner]);
        generation.dispose();
    }
}
