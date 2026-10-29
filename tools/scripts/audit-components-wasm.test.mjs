import assert from 'node:assert/strict';
import test from 'node:test';
import { componentEvidence, importEvidence, capabilityRegistry, wasmSections, classifyNativeEntrypoint, behaviorAttributeReads, summarizeTraffic } from './audit-components-wasm.mjs';

const entry = { tag: 'cem-example', implementation: { source: 'example.xhtml' } };
const registry = new Map([['form-control', { source: 'form.ts' }], ['plaintext-editor', { source: 'editor.ts' }]]);

test('uses template capability precedence and retains open mode selectors and native data demand', () => {
    const row = componentEvidence(entry, `<cem-element tag="cem-example" capability="form-control"><template id="cem-example" type="text/cem-ml" capability="plaintext-editor">
{attribute @name=format | csv}{attribute @name=content-type | text/plain}
{style @type=text/css |\`\`\` :where(button) { color:red; } \`\`\`}
{data:read(value, format)}{cem-icon @image=x}{slot}
</template></cem-element>`, registry, new Set(['cem-example', 'cem-icon']));
    assert.equal(row.capability, 'plaintext-editor');
    assert.equal(row.behaviorSource, 'editor.ts');
    assert.deepEqual(row.modeSelectors, ['content-type', 'format']);
    assert.deepEqual(row.nativeCalls, ['data:read']);
    assert.deepEqual(row.componentReferences, ['cem-icon']);
    assert.equal(row.nativeDataImport, true);
    assert.match(row.boundary, /handoff/);
    assert.equal(row.dynamicPayloads, true);
});

test('refuses declaration/catalog drift, unknown capabilities and ambiguous templates', () => {
    const source = '<cem-element tag="cem-example"><template id="cem-example" type="text/cem-ml"></template></cem-element>';
    assert.throws(() => componentEvidence(entry, source.replace('tag="cem-example"', 'tag="other"'), registry), /tag/);
    assert.throws(() => componentEvidence(entry, source.replace('type="text/cem-ml"', 'type="text/cem-ml" capability="unknown"'), registry), /capability/);
    assert.throws(() => componentEvidence(entry, source.replace('</cem-element>', '<template id="cem-example" type="text/cem-ml"></template></cem-element>'), registry), /template/);
});

test('distinguishes runtime imports, erased types, workers and unresolved dynamic demands', () => {
    const result = importEvidence(`import type { A } from './types.js';
import { type B } from './types2.js'; import { C, type D } from './runtime.js';
export type { E } from './other.js'; export * from './exports.js';
const p = import('./optional.js'); const q = import(selected);
new Worker(new URL('./worker.js', import.meta.url));
const ignored = "import('./not-code.js')";`, 'fixture.ts');
    assert.deepEqual(result.edges.map(({ kind, specifier }) => [kind, specifier]), [
        ['static', './runtime.js'], ['static', './exports.js'], ['dynamic', './optional.js'], ['asset', './worker.js'],
    ]);
    assert.equal(result.unresolvedDynamicImports, 1);
});

test('resolves registry behavior imports without accepting unknown behavior symbols', () => {
    const source = `import { CONTROL } from './control.js';
export const CEM_DECLARATIVE_CAPABILITIES = Object.freeze({ 'form-control': { behavior: CONTROL, behaviorIdentity:'v1' } } as const);`;
    assert.equal(capabilityRegistry(source, 'runtime.ts').get('form-control').source, './control.js');
    assert.throws(() => capabilityRegistry(source.replace('behavior: CONTROL', 'behavior: MISSING'), 'runtime.ts'), /behavior/);
});

test('accounts for all WASM bytes without treating payload bytes as source attribution', () => {
    const bytes = Uint8Array.from([0,97,115,109,1,0,0,0, 0,2,1,120]);
    const sections = wasmSections(bytes);
    assert.deepEqual(sections, [{ id: 0, name: 'custom', payloadBytes: 2, framingBytes: 2, totalBytes: 4 }]);
    assert.equal(8 + sections.reduce((n, s) => n + s.totalBytes, 0), bytes.length);
    assert.throws(() => wasmSections(bytes.slice(0,-1)), /truncated/);
    assert.throws(() => wasmSections(Uint8Array.from([0,97,115,109,1,0,0,0, 0,128])), /LEB/);
});

test('keeps recovery and native memory/admission controls separate from optional tooling', () => {
    assert.equal(classifyNativeEntrypoint('runtimeMemoryStatus'), 'core-control');
    assert.equal(classifyNativeEntrypoint('sealCommittedCheckpoint'), 'core-control');
    assert.equal(classifyNativeEntrypoint('sourceHighlightPaintV1'), 'candidate-tooling');
    assert.equal(classifyNativeEntrypoint('retainXsltComponent'), 'candidate-language');
    assert.equal(classifyNativeEntrypoint('renderTemplateSource'), 'core-render-or-data');
});


test('records literal behavior attribute reads without pretending computed reads are resolved', () => {
    assert.deepEqual(behaviorAttributeReads(`host.getAttribute('mode'); host.hasAttribute('readonly');
        host.getAttributeNS('urn:demo', 'content-type'); host.getAttribute(selected);`, 'behavior.ts'),
        ['content-type', 'mode', 'readonly']);
});


test('separates repeat transfers from unique binary size and refuses mixed response cohorts', () => {
    const record = { path:'/core.wasm', bodyBytes:100, wireBodyBytes:100, sha256:'core' };
    assert.deepEqual(summarizeTraffic([record,record]), { requests:2, decodedBodyBytes:200,
        uniqueDecodedBodyBytes:100, wireBodyBytes:200, uniqueWasm:[{path:'/core.wasm', bytes:100, sha256:'core'}], wasmRequests:2 });
    assert.equal(summarizeTraffic([{...record, wireBodyBytes:null}]).wireBodyBytes, null);
    assert.throws(() => summarizeTraffic([record,{...record,sha256:'other'}]), /drift/);
});
