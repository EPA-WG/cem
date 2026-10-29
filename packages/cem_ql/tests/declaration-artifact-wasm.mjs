// Native build metadata and executable IR remain portable across separate heaps.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createCemQlWasmGeneration } from '../dist/wasm/cem_ql_factory.js';

const wasm = await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url));
const base = new URL('../../../dist/artifacts/cem_ql/declarations/worker-field', import.meta.url);
const metadataJson = await readFile(new URL(`${base.href}.json`), 'utf8');
const hash = (await readFile(new URL(`${base.href}.hash`), 'utf8')).trim();
const bytes = await readFile(new URL(`${base.href}.bin`));
const importsBase = new URL('../../../dist/artifacts/cem_ql/compiled-form/', import.meta.url);
const importsMarkup = await readFile(new URL('declarations.html', importsBase), 'utf8');
const importsPin = importsMarkup.split('\n').find(line => line.includes('tag="cem-compiled-form-app"')).match(/compiled-hash="([^"]+)"/)[1];
const importsJson = await readFile(new URL('cem-compiled-form-app.json', importsBase), 'utf8');
const importsBytes = await readFile(new URL('cem-compiled-form-app.bin', importsBase));
const first = createCemQlWasmGeneration(), next = createCemQlWasmGeneration();
await first.initialize(wasm); await next.initialize(wasm);
assert.notEqual(first.memory(), next.memory());
for (const runtime of [first, next]) {
    const call = (name, ...args) => runtime.call(name, args);
    const parse = (name, ...args) => JSON.parse(call(name, ...args));
    for (const [text, pin] of [[`${metadataJson} `, hash], [metadataJson, ''], ['{}', hash]]) {
        const result = parse('admitDeclarationMetadata', text, pin);
        assert.equal(result.metadata, undefined);
        assert.equal(result.diagnostics[0].code, 'cem.declaration_artifact.invalid');
    }
    const { metadata } = parse('admitDeclarationMetadata', metadataJson, hash);
    assert.equal(metadata.templateFormat, 'cem-template-artifact/2');
    assert.equal(metadata.tag, 'cem-aot-field-worker');
    assert.equal(metadata.capability, 'form-control');
    assert.equal(metadata.resourceBaseUrl, metadata.sourceUri);
    const corrupt = Uint8Array.from(bytes); corrupt[corrupt.length - 1] ^= 1;
    assert.equal(parse('importPinnedTemplateArtifact', corrupt, JSON.stringify(metadata.template)).artifactId, null);
    const retained = parse('importPinnedTemplateArtifact', bytes, JSON.stringify(metadata.template));
    assert.equal(retained.formatVersion, 'cem-template-artifact/2');
    assert.ok(retained.artifactId); assert.ok(retained.stylesheets.length);
    const css = parse('beginTemplateStylesheet', retained.artifactId, JSON.stringify({
        consumer: 'compiled-css-proof', index: 0, declarationIdentity: 'compiled-field', baseUrl: metadata.resourceBaseUrl,
        context: { identity: 'compiled-context', resolverIdentity: 'resolver:1', resourcePolicyStamp: 'policy:1',
            frames: [{ frameId: 'root', baseUrl: metadata.resourceBaseUrl, scopes: [], specifiers: { imports: {}, resources: {} } }] },
        scope: { kind: 'private', tag: metadata.tag },
    }));
    assert.equal(css.status, 'ready'); assert.ok(css.css.includes(metadata.tag));
    const again = parse('beginTemplateStylesheet', retained.artifactId, JSON.stringify({
        consumer: 'compiled-css-second', index: 0, declarationIdentity: 'compiled-field', baseUrl: metadata.resourceBaseUrl,
        context: { identity: 'compiled-context', resolverIdentity: 'resolver:1', resourcePolicyStamp: 'policy:1',
            frames: [{ frameId: 'root', baseUrl: metadata.resourceBaseUrl, scopes: [], specifiers: { imports: {}, resources: {} } }] }, scope: { kind: 'private', tag: metadata.tag },
    }));
    assert.equal(again.css, css.css); assert.deepEqual(again.identity, css.identity);

    call('disposeTemplate', retained.artifactId);
    assert.equal(parse('renderTemplate', retained.artifactId, '{}').diagnostics[0].code, 'cem.ql.wasm.unknown_artifact');
    const importedMetadata = parse('admitDeclarationMetadata', importsJson, importsPin).metadata;
    assert.equal(importedMetadata.templateFormat, 'cem-template-artifact/3');
    const imported = parse('importPinnedTemplateArtifact', importsBytes, JSON.stringify(importedMetadata.template));
    assert.equal(imported.formatVersion, 'cem-template-artifact/3');
    assert.ok(imported.artifactId);
    const options = {
        consumer: 'compiled-imports-proof', index: 0, declarationIdentity: 'compiled-imports', baseUrl: importedMetadata.resourceBaseUrl,
        context: { identity: 'compiled-imports-context', resolverIdentity: 'resolver:1', resourcePolicyStamp: 'policy:1',
            frames: [{ frameId: 'root', baseUrl: importedMetadata.resourceBaseUrl, scopes: [], specifiers: { imports: {}, resources: {} } }] },
        scope: { kind: 'private', tag: importedMetadata.tag },
    };
    const compiledImports = parse('beginTemplateStylesheet', imported.artifactId, JSON.stringify(options));
    assert.equal(compiledImports.status, 'ready'); // no CSS transport/parse request
    assert.match(compiledImports.css, /--compiled-import-proof:\s*imported/);
    assert.match(compiledImports.css, /--compiled-nested-proof:\s*nested/);
    const changedBase = { ...options, baseUrl: 'https://other.test/components/view.xhtml' };
    assert.equal(parse('beginTemplateStylesheet', imported.artifactId, JSON.stringify(changedBase)).status, 'error');
    options.context.frames[0].specifiers.resources['./styles/layout.css'] = { target: './changed.css' };
    assert.equal(parse('beginTemplateStylesheet', imported.artifactId, JSON.stringify(options)).status, 'error');
    call('disposeTemplate', imported.artifactId);
    runtime.dispose();
    assert.throws(() => call('admitDeclarationMetadata', metadataJson, hash), /retired/);
}
console.log('Declaration metadata: v2 inline and v3 imported CSS, native pins, intrinsic capability, policy refusal and separate-heap disposal passed.');
