#!/usr/bin/env node

import {
    copyFileSync,
    mkdirSync,
    readFileSync,
    rmSync,
    writeFileSync,
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(packageRoot, '../..');
const wasmSourceRoot = resolve(workspaceRoot, 'packages/cem_ql/dist/wasm');
const runtimeSupportRoot = resolve(packageRoot, 'dist/lib/internal/runtime-support');
const vendorRoot = resolve(runtimeSupportRoot, 'vendor');
const runtimeFiles = ['cem_ql.js', 'cem_ql.d.ts', 'cem_ql_factory.js', 'cem_ql_factory.d.ts', 'cem_ql_bg.wasm', 'UNICODE-LICENSE.txt', 'native-generation.js', 'native-generation.d.ts'];
const workspaceImport = '../../../../../cem_ql/dist/wasm/cem_ql.js';
const packageImport = './vendor/cem_ql.js';

rmSync(vendorRoot, { force: true, recursive: true });
mkdirSync(vendorRoot, { recursive: true });

for (const file of runtimeFiles) {
    copyFileSync(resolve(wasmSourceRoot, file), resolve(vendorRoot, file));
}

for (const file of ['native-runtime.js', 'native-runtime.d.ts', 'native-generation.js', 'native-generation.d.ts']) {
    const outputPath = resolve(runtimeSupportRoot, file);
    const output = readFileSync(outputPath, 'utf8');
    const packaged = output.replaceAll(workspaceImport, packageImport).replaceAll('../../../../../cem_ql/dist/wasm/cem_ql_factory.js', './vendor/cem_ql_factory.js').replaceAll('../../../../../cem_ql/dist/wasm/native-generation.js', './vendor/native-generation.js').replaceAll('../../../../../cem_ql/dist/wasm/cem_ql_bg.wasm', './vendor/cem_ql_bg.wasm');
    if (!packaged.includes(file.startsWith('native-generation') ? './vendor/native-generation.js' : packageImport)) {
        throw new Error(`${file} must import the package-owned CEM-QL runtime`);
    }
    writeFileSync(outputPath, packaged);
}

console.log(`Packaged ${runtimeFiles.length} CEM-QL runtime assets under cem-elements/dist.`);
