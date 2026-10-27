import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, renameSync, rmSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

export const WASM_OPT_VERSION = '132.0.0';
export const WASM_FILE_LIMIT = 25 * 1024 * 1024;
const require = createRequire(import.meta.url);
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

function interfaceOf(bytes) {
    const module = new WebAssembly.Module(bytes);
    const sorted = entries => entries.sort((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b)));
    return { imports: sorted(WebAssembly.Module.imports(module)), exports: sorted(WebAssembly.Module.exports(module)) };
}

/** Optimize post-bindgen bytes before any integrity records or factory pins are emitted. */
export function optimizeWasm(path, { mode = 'Oz', fileLimit = WASM_FILE_LIMIT } = {}) {
    if (!['Os', 'Oz'].includes(mode)) throw new Error(`Unsupported WASM optimization mode: ${mode}`);
    if (!Number.isSafeInteger(fileLimit) || fileLimit < 1) throw new Error('Invalid WASM file size limit.');
    const binaryenRoot = dirname(require.resolve('binaryen/package.json'));
    const version = JSON.parse(readFileSync(join(binaryenRoot, 'package.json'), 'utf8')).version;
    if (version !== WASM_OPT_VERSION) throw new Error(`Expected Binaryen ${WASM_OPT_VERSION}, found ${version}.`);
    const input = readFileSync(path);
    const originalInterface = interfaceOf(input);
    const temporary = mkdtempSync(join(dirname(path), '.wasm-opt-'));
    const outputPath = join(temporary, 'optimized.wasm');
    const started = performance.now();
    try {
        // No feature expansion, closed-world assumptions, or relaxed numeric semantics.
        const result = spawnSync(process.execPath, [join(binaryenRoot, 'bin/wasm-opt'), path, `-${mode}`, '-o', outputPath], {
            encoding: 'utf8', timeout: 30 * 60 * 1000, maxBuffer: 4 * 1024 * 1024,
        });
        if (result.error || result.status !== 0) throw new Error(`wasm-opt failed: ${result.error?.message || result.stderr || `exit ${result.status}, signal ${result.signal}`}`);
        const output = readFileSync(outputPath);
        assert.deepEqual(interfaceOf(output), originalInterface, 'wasm-opt changed the public import/export descriptors.');
        if (output.length >= fileLimit) throw new Error(`Optimized WASM has ${output.length} bytes; must be below ${fileLimit}: ${path}`);
        if (output.length > input.length) throw new Error(`wasm-opt increased the binary size: ${path}`);
        if (digest(readFileSync(path)) !== digest(input)) throw new Error(`WASM input changed during optimization: ${path}`);
        const report = {
            binaryen: version, mode, inputBytes: input.length, outputBytes: output.length,
            inputSha256: digest(input), outputSha256: digest(output),
            imports: originalInterface.imports.length, exports: originalInterface.exports.length,
            milliseconds: Math.round(performance.now() - started),
        };
        renameSync(outputPath, path);
        console.log(`wasm-opt -${mode}: ${input.length} -> ${output.length} bytes (${path})`);
        return report;
    } finally { rmSync(temporary, { recursive: true, force: true }); }
}
