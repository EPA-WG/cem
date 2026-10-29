// Native build metadata and executable IR remain portable across separate heaps.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createCemQlWasmGeneration } from '../dist/wasm/cem_ql_factory.js';

const wasm = await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url));
const base = new URL('../../../dist/artifacts/cem_ql/declarations/worker-field', import.meta.url);
const metadataJson = await readFile(new URL(`${base.href}.json`), 'utf8');
const hash = (await readFile(new URL(`${base.href}.hash`), 'utf8')).trim();
const bytes = await readFile(new URL(`${base.href}.bin`));
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
    runtime.dispose();
    assert.throws(() => call('admitDeclarationMetadata', metadataJson, hash), /retired/);
}
console.log('Declaration metadata: native pins, intrinsic capability, corruption refusal and separate-heap disposal passed.');
