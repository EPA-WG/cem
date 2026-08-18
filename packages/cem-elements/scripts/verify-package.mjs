#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(packageRoot, '../..');
const wasmSourceRoot = resolve(workspaceRoot, 'packages/cem_ql/dist/wasm');
const vendorRelativeRoot = 'dist/lib/internal/runtime-support/vendor';
const vendorRoot = resolve(packageRoot, vendorRelativeRoot);
const packageJson = JSON.parse(readFileSync(resolve(packageRoot, 'package.json'), 'utf8'));
const runtimeFiles = ['cem_ql.js', 'cem_ql.d.ts', 'cem_ql_bg.wasm'];

if (!packageJson.files?.includes('dist')) {
    throw new Error('cem-elements package.json files must include dist');
}

for (const file of runtimeFiles) {
    const source = readFileSync(resolve(wasmSourceRoot, file));
    const packaged = readFileSync(resolve(vendorRoot, file));
    if (!source.equals(packaged)) {
        throw new Error(`packaged ${file} must be byte-identical to the cem_ql:build:wasm output`);
    }
}

for (const file of ['cem-ql-query.js', 'cem-ql-render.js']) {
    const output = readFileSync(resolve(packageRoot, 'dist/lib/internal/runtime-support', file), 'utf8');
    if (!output.includes("from './vendor/cem_ql.js'")) {
        throw new Error(`built ${file} must import the package-owned CEM-QL runtime`);
    }
    if (output.includes('cem_ql/dist/wasm')) {
        throw new Error(`built ${file} must not retain a workspace-relative CEM-QL import`);
    }
}

const verificationRoot = await mkdtemp(join(tmpdir(), 'cem-elements-npm-pack-'));
const npmCache = resolve(verificationRoot, 'npm-cache');
const consumerRoot = resolve(verificationRoot, 'consumer');

try {
    const packOutput = execFileSync('npm', ['pack', '--json', '--cache', npmCache, '--pack-destination', verificationRoot], {
        cwd: packageRoot,
        encoding: 'utf8',
        env: { ...process.env, npm_config_update_notifier: 'false' },
    });
    const packResults = JSON.parse(packOutput);
    const packedFiles = packResults[0]?.files?.map(({ path }) => path) ?? [];
    const requiredFiles = [
        'dist/lib/internal/runtime-support/processing-engine.js',
        'dist/lib/internal/runtime-support/processing-host.js',
        'dist/lib/internal/runtime-support/processing-host-runtime.js',
        'dist/lib/internal/runtime-support/processing-worker.js',
        'dist/lib/legacy-xslt/template-language.js',
        ...runtimeFiles.map((file) => `${vendorRelativeRoot}/${file}`),
    ];

    for (const file of requiredFiles) {
        if (!packedFiles.includes(file)) {
            throw new Error(`npm pack must contain ${file}`);
        }
    }

    const buildInfoFiles = packedFiles.filter((path) => path.endsWith('.tsbuildinfo'));
    if (buildInfoFiles.length > 0) {
        throw new Error(`npm pack must exclude tsbuildinfo files: ${buildInfoFiles.join(', ')}`);
    }

    if (packedFiles.some((path) => path.startsWith('src/') || path.startsWith('scripts/'))) {
        throw new Error('npm pack must not publish cem-elements source or build scripts');
    }

    const archiveName = packResults[0]?.filename;
    if (!archiveName) {
        throw new Error('npm pack did not report an archive filename');
    }
    await mkdir(consumerRoot, { recursive: true });
    await writeFile(
        resolve(consumerRoot, 'package.json'),
        `${JSON.stringify({ name: 'cem-elements-package-probe', private: true, type: 'module' }, null, 2)}\n`,
    );
    execFileSync(
        'npm',
        ['install', '--ignore-scripts', '--no-audit', '--no-fund', '--cache', npmCache, resolve(verificationRoot, archiveName)],
        { cwd: consumerRoot, encoding: 'utf8', env: { ...process.env, npm_config_update_notifier: 'false' } },
    );
    execFileSync(
        process.execPath,
        [
            '--input-type=module',
            '--eval',
            "const runtime = await import('@epa-wg/cem-elements'); if (runtime.cemElements() !== '@epa-wg/cem-elements') process.exit(1);",
        ],
        { cwd: consumerRoot, encoding: 'utf8' },
    );

    const wasmBytes = readFileSync(resolve(vendorRoot, 'cem_ql_bg.wasm')).byteLength;
    console.log(
        `cem-elements package verified (${packedFiles.length} files, ${wasmBytes} WASM bytes, clean package import).`,
    );
} finally {
    await rm(verificationRoot, { force: true, recursive: true });
}
