#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const requiredWasmBindgen = '0.2.122';
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const targetDir = 'dist/target/cem_ql';
const wasmInput = `${targetDir}/wasm32-unknown-unknown/debug/cem_ql.wasm`;
const outDir = 'packages/cem_ql/dist/wasm';

run('cargo', [
    'build',
    '--package',
    'cem-ql',
    '--lib',
    '--target',
    'wasm32-unknown-unknown',
    '--target-dir',
    targetDir,
]);

const version = spawnSync('wasm-bindgen', ['--version'], {
    cwd: repoRoot,
    encoding: 'utf8',
});
if (version.status !== 0) {
    fail(
        `wasm-bindgen ${requiredWasmBindgen} is required. Install it with: ` +
            `cargo install wasm-bindgen-cli --version ${requiredWasmBindgen} --locked`
    );
}
const actualVersion = version.stdout.trim().split(/\s+/).at(-1);
if (actualVersion !== requiredWasmBindgen) {
    fail(`wasm-bindgen ${requiredWasmBindgen} is required, found ${actualVersion || 'unknown'}.`);
}

rmSync(resolve(repoRoot, outDir), { recursive: true, force: true });
mkdirSync(resolve(repoRoot, outDir), { recursive: true });
run('wasm-bindgen', [
    wasmInput,
    '--target',
    'web',
    '--out-dir',
    outDir,
    '--out-name',
    'cem_ql',
]);

function run(command, args) {
    const result = spawnSync(command, args, {
        cwd: repoRoot,
        stdio: 'inherit',
    });
    if (result.status !== 0) {
        process.exit(result.status ?? 1);
    }
}

function fail(message) {
    console.error(message);
    process.exit(1);
}
