// Integration driver only: original JSON bytes go directly into native import.
import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';
import { wasmSections } from '../../../tools/scripts/audit-components-wasm.mjs';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

const fixture = resolve('dist/artifacts/cem_ql/component-runtime');
const out = resolve(process.argv[2] || 'packages/cem_ql/dist/wasm-component');
const wasm = await readFile(`${out}/cem_ql_bg.wasm`);
const compiled = await WebAssembly.compile(wasm);
const names = WebAssembly.Module.exports(compiled).map(entry => entry.name);
assert.ok(names.includes('renderCompiledApplication'));
assert.deepEqual(names.filter(name => name !== 'memory' && !name.startsWith('__')).sort(), ['compiledDeclarationInventory', 'compileddeclarationowner_describe', 'compileddeclarationowner_dispose', 'compileddeclarationowner_new', 'compileddeclarationowner_releaseConsumer', 'compileddeclarationowner_renderJson', 'compileddeclarationowner_stylesheet', 'renderCompiledApplication'].sort(), 'only reviewed compiled application/owner entries may be public');
const bytes = await readFile(`${fixture}/cart.bin`);
const pins = await readFile(`${fixture}/cart.pins.json`, 'utf8');
const entry = `${out}/cem_ql.js`;
const legacy = await readFile(`${fixture}/legacy.bin`);
const memories = [];
for (let generation = 0; generation < 2; generation++) {
    // Distinct ESM instances give wasm-bindgen distinct lexical heap ownership.
    const api = await import(`${pathToFileURL(entry).href}?generation=${generation}`);
    const instance = api.initSync({ module: compiled });
    assert.ok(!memories.includes(instance.memory)); memories.push(instance.memory);
    for (const response of ['cart', 'updated', 'removed', 'empty', 'partial-error', 'failed']) {
        const json = await readFile(`packages/cem_ql/fixtures/component-application/shopping-cart/${response}.json`);
        const html = api.renderCompiledApplication(bytes, pins, json, `app://shopping-cart/${response}.json`, 'response');
        assert.equal(html, await readFile(`${fixture}/${response}.html`, 'utf8'), response);
    }
    const call = (b = bytes, p = pins, json = Buffer.from('{}'), uri = 'app://cart/data.json', binding = 'response') => api.renderCompiledApplication(b, p, json, uri, binding);
    assert.throws(() => call(bytes, pins, Buffer.from('{')), /native JSON import failed/);
    assert.throws(() => call(bytes, pins, Buffer.from('{}'), ''), /source URI/);
    assert.throws(() => call(bytes, pins, Buffer.alloc(1024 * 1024 + 1)), /input limit/);
    assert.throws(() => call(bytes, pins, Buffer.from('{}'), 'app://cart/data.json', 'missing'), /pinned native JSON binding/);
    const wrong = JSON.parse(pins); wrong.contentHash = '';
    assert.throws(() => call(bytes, JSON.stringify(wrong)), /hash_mismatch/);
    const corrupt = Buffer.from(bytes); corrupt[corrupt.length - 1] ^= 1;
    assert.throws(() => call(corrupt));
    assert.throws(() => call(Buffer.from('{p | source fallback}')), /compiled CSS/);
    assert.throws(() => call(legacy), /compiled CSS/);
    const denied = await readFile(`${fixture}/denied.bin`), deniedPins = await readFile(`${fixture}/denied.pins.json`, 'utf8');
    assert.throws(() => call(denied, deniedPins), error => error === 'compiled application render failed');
    // Failures must not poison the next valid call or retain a result handle.
    const plain = await readFile(`${fixture}/plain.bin`), plainPins = await readFile(`${fixture}/plain.pins.json`, 'utf8');
    assert.match(call(plain, plainPins), /document/);
    const v3 = await readFile(`${fixture}/v3.bin`), v3Pins = await readFile(`${fixture}/v3.pins.json`, 'utf8');
    assert.equal(call(v3, v3Pins), call(plain, plainPins));
}
console.log('Selected component WASM: six native JSON responses, pinned input/refusal, no full ABI, and two independent heaps passed.');

const full = await readFile('packages/cem_ql/dist/wasm/cem_ql_bg.wasm');
assert.ok(wasm.length < full.length, 'selected target must actually reduce linked bytes');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const selectedOptimization = JSON.parse(await readFile('dist/reports/wasm-optimization/cem-ql-component.json', 'utf8'));
const fullOptimization = JSON.parse(await readFile('dist/reports/wasm-optimization/cem-ql.json', 'utf8'));
assert.equal(selectedOptimization.outputSha256, hash(wasm), 'selected measurement must match its optimizer report');
assert.equal(fullOptimization.outputSha256, hash(full), 'full measurement must match its optimizer report');
assert.equal(selectedOptimization.mode, fullOptimization.mode);
assert.equal(selectedOptimization.binaryen, fullOptimization.binaryen);
const report = {
    contract: 'cem-selected-runtime-measurement/1',
    scope: 'compiled-template-json-and-passive-declaration-prototype', nativeLinkClosureVerified: false,
    selectionEvidence: 'build-command input and behavioral acceptance; no embedded registry stamp',
    fixtureRegistrySourceSha256: hash(await readFile(`${fixture}/selected_registry.rs`)),
    full: { bytes: full.length, sha256: hash(full), optimization: fullOptimization, sections: wasmSections(full) },
    selected: { bytes: wasm.length, sha256: hash(wasm), optimization: selectedOptimization, gzip9Bytes: gzipSync(wasm, {level:9}).length,
        brotli9Bytes: brotliCompressSync(wasm, {params: {[constants.BROTLI_PARAM_QUALITY]: 9}}).length,
        imports: WebAssembly.Module.imports(compiled), exports: WebAssembly.Module.exports(compiled), sections: wasmSections(wasm) },
    limits: ['Not a complete component ABI or app closure', 'Widget/provider ownership and recovery handoff remain open', 'No per-language symbol attribution or on-demand extension transport'],
};
await mkdir('dist/reports/components-wasm', {recursive:true});
await writeFile('dist/reports/components-wasm/selected-runtime.json', `${JSON.stringify(report, null, 2)}\n`);
console.log(`Selected linked WASM: ${wasm.length} bytes; full reference: ${full.length} bytes.`);
