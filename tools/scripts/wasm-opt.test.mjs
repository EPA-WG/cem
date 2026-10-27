import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import binaryen from 'binaryen';
import { optimizeWasm, WASM_OPT_VERSION } from './wasm-opt.mjs';

function fixture(t) {
    const root = mkdtempSync(join(tmpdir(), 'cem-wasm-opt-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const module = binaryen.parseText(`(module
        (import "host" "offset" (func $offset (result i32)))
        (memory (export "memory") 1)
        (global (export "constant") i32 (i32.const 7))
        (func (export "answer") (param i32) (result i32)
            (i32.add (i32.add (local.get 0) (i32.const 0)) (call $offset)))
        ${Array.from({ length: 20 }, (_, n) => `(func $unused${n} (result i32) (i32.const ${n}))`).join('\n')}
    )`);
    const input = Buffer.from(module.emitBinary()); module.dispose();
    const path = join(root, 'module.wasm'); writeFileSync(path, input);
    return { root, path, input };
}
for (const mode of ['Os', 'Oz']) test(`-${mode} preserves imports, exports, memory, globals and function behavior`, async t => {
    const { root, path, input } = fixture(t);
    const result = optimizeWasm(path, { mode });
    assert.equal(result.binaryen, WASM_OPT_VERSION);
    assert.equal(result.inputBytes, input.length);
    assert.ok(result.outputBytes < input.length);
    const { instance } = await WebAssembly.instantiate(readFileSync(path), { host: { offset: () => 5 } });
    assert.equal(instance.exports.answer(37), 42);
    assert.equal(instance.exports.constant.value, 7);
    assert.equal(instance.exports.memory.buffer.byteLength, 65536);
    assert.deepEqual(readdirSync(root), ['module.wasm']);
});
test('oversized optimized output is refused without replacing input or retaining temporary files', t => {
    const { root, path, input } = fixture(t);
    assert.throws(() => optimizeWasm(path, { fileLimit: 1 }), /must be below 1/);
    assert.deepEqual(readFileSync(path), input);
    assert.deepEqual(readdirSync(root), ['module.wasm']);
});
test('invalid modes, limits and binaries leave the source untouched', t => {
    const { root, path, input } = fixture(t);
    assert.throws(() => optimizeWasm(path, { mode: 'O3' }), /Unsupported/);
    assert.throws(() => optimizeWasm(path, { fileLimit: 0 }), /Invalid/);
    assert.deepEqual(readFileSync(path), input);
    writeFileSync(path, 'invalid wasm');
    assert.throws(() => optimizeWasm(path), WebAssembly.CompileError);
    assert.equal(readFileSync(path, 'utf8'), 'invalid wasm');
    assert.deepEqual(readdirSync(root), ['module.wasm']);
});
