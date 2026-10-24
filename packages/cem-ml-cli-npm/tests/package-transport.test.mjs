import assert from 'node:assert/strict';
import test from 'node:test';
import { parseBrowserCommandPackageRequest } from '../dist/protocol.js';

const index = new Uint8Array([1]), pin = 'a'.repeat(64);
const source = { packageId: 'csv', version: '1.0.0', path: 'schema-packages/csv/v1/package.cem', bytes: new Uint8Array([2]) };
const packet = operation => ({ type: 'cem-command-package', actionId: 1, ...operation });

test('worker package protocol carries only bounded original bytes and named controls', () => {
    for (const operation of [{ operation: 'bootstrap' }, { operation: 'release' },
        { operation: 'summary', index, pin }, { operation: 'import', index, pin, source },
        { operation: 'activate', index, pin, sources: [source], selection: '[{"packageId":"csv","version":"1.0.0"}]' }]) {
        const message = packet(operation);
        assert.equal(parseBrowserCommandPackageRequest(message), message);
        for (const extra of ['owner', 'grants', 'hostConfigurationJson', 'runtimeId', 'replacementAllowed']) {
            assert.throws(() => parseBrowserCommandPackageRequest({ ...message, [extra]: true }));
        }
    }
    for (const operation of [{ operation: 'unknown' }, { operation: 'close' }, { operation: 'summary', index, pin: 'wrong' },
        { operation: 'summary', index: new Uint8Array(), pin }, { operation: 'import', index, pin, source: { ...source, owner: 1 } },
        { operation: 'activate', index, pin, sources: [], selection: '[]' },
        { operation: 'activate', index, pin, sources: [source], selection: 'x'.repeat(32 * 1024 + 1) }]) {
        assert.throws(() => parseBrowserCommandPackageRequest(packet(operation)));
    }
    assert.throws(() => parseBrowserCommandPackageRequest({ ...packet({ operation: 'bootstrap' }), actionId: 0 }));
    assert.throws(() => parseBrowserCommandPackageRequest({ ...packet({ operation: 'release' }), type: 'other' }));
});

test('worker package copies are bounded before posting and cannot share mutable memory', () => {
    const large = { ...source, bytes: new Uint8Array(16 * 1024 * 1024) };
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'activate', index, pin, sources: [large, large], selection: '[]' })));
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'summary', index: new Uint8Array(4 * 1024 * 1024 + 1), pin })));
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'import', index, pin,
        source: { ...source, bytes: new Uint8Array(new SharedArrayBuffer(1)) } })));
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'summary', index: new Uint8Array(new SharedArrayBuffer(1)), pin })));
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'activate', index, pin, sources: new Array(1025).fill(source), selection: '[]' })));
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'activate', index, pin, sources: new Array(1), selection: '[]' })));
    // Structured clone copies a view's entire backing buffer, including bytes
    // outside its visible range. Bound actual storage before posting it.
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'summary', pin,
        index: new Uint8Array(4 * 1024 * 1024 + 1).subarray(0, 1) })));
    assert.throws(() => parseBrowserCommandPackageRequest(packet({ operation: 'import', index, pin,
        source: { ...source, bytes: new Uint8Array(16 * 1024 * 1024 + 1).subarray(0, 1) } })));
});
