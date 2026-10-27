#!/usr/bin/env node

import { cemQlWasmFactory, cemQlWasmFactoryTypes } from './cem-ql-wasm-factory.mjs';
import { optimizeWasm } from './wasm-opt.mjs';
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const requiredWasmBindgen = '0.2.122';
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const targetDir = 'dist/target/cem_ql';
const wasmInput = `${targetDir}/wasm32-unknown-unknown/release/cem_ql.wasm`;
const outDir = 'packages/cem_ql/dist/wasm';

run('cargo', [
    'build',
    '--locked',
    '--package',
    'cem-ql',
    '--lib',
    '--release',
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
    '--remove-name-section',
    '--remove-producers-section',
]);

const optimization = optimizeWasm(resolve(repoRoot, outDir, 'cem_ql_bg.wasm'));
const reportRoot = resolve(repoRoot, 'dist/reports/wasm-optimization');
mkdirSync(reportRoot, { recursive: true });
writeFileSync(resolve(reportRoot, 'cem-ql.json'), `${JSON.stringify(optimization, null, 2)}\n`);

// Owned wasm-bindgen classes expose Symbol.dispose; keep ES2022 consumers
// compatible by declaring its ambient library in the generated artifact.
const declarations = resolve(repoRoot, outDir, 'cem_ql.d.ts');
writeFileSync(declarations, `/// <reference lib="esnext.disposable" />\n${readFileSync(declarations, 'utf8')}`);

// A separate lexical factory leaves the legacy module ABI unchanged.
writeFileSync(resolve(repoRoot, outDir, 'cem_ql_factory.js'), cemQlWasmFactory(readFileSync(resolve(repoRoot, outDir, 'cem_ql.js'), 'utf8')));
writeFileSync(resolve(repoRoot, outDir, 'cem_ql_factory.d.ts'), cemQlWasmFactoryTypes);

for (const file of ['native-generation.js', 'native-generation.d.ts', 'schema-asset-protocol.js', 'schema-asset-protocol.d.ts']) {
    copyFileSync(resolve(repoRoot, 'packages/cem_ql/runtime', file.replace('.d.ts', '.d.mts').replace('.js', '.mjs')), resolve(repoRoot, outDir, file));
}

// wasm-bindgen `--target web` emits ESM, but the out dir has no package.json, so
// Node/TypeScript `nodenext` resolution would infer CommonJS from the repo root and
// the default `init` import becomes non-callable. Mark the artifact as ESM.
writeFileSync(resolve(repoRoot, outDir, 'package.json'), `${JSON.stringify({ type: 'module' }, null, 2)}\n`);
copyFileSync(
    resolve(repoRoot, 'packages/cem_ql/src/suggestions/UNICODE-LICENSE.txt'),
    resolve(repoRoot, outDir, 'UNICODE-LICENSE.txt'),
);

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
