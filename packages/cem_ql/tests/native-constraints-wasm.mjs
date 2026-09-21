// CEMT-NAMED-CONSTRAINTS: native schema contracts survive workers and saved pipelines.
// Rust owns fixture graphs; JavaScript transports binary artifacts and control metadata.
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import * as wasm from '../dist/wasm/cem_ql.js';

const limits = JSON.stringify({ maxBytes: 16 * 1024 * 1024, maxValues: 100000, maxDepth: 128 });
const ready = async () => wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });

function consume(fixtures) {
    for (const { name, bytes, invalid } of fixtures) {
        for (const artifact of invalid) {
            assert.throws(() => wasm.importNativeValueArtifact(artifact, limits), `${name}: invalid inherited/local constraint`);
        }
        for (const version of [1, 2, 4]) {
            const changed = bytes.slice();
            changed[4] = version;
            assert.throws(() => wasm.importNativeValueArtifact(changed, limits), `${name}: unsupported contract version`);
        }
        const id = wasm.importNativeValueArtifact(bytes, limits);
        const source = name === 'count'
            ? '{attribute @name=count @type=integer @required=true}{p | {$count + 1}}'
            : '{attribute @name=code @type=string @required=true}{p | {$code}}';
        const template = JSON.parse(wasm.compileTemplate(source, JSON.stringify([name])));
        try {
            const binding = JSON.stringify([{ name, index: 0, artifactId: id }]);
            const plan = JSON.parse(wasm.renderTemplateWithNativeValues(template.artifactId, 0, '{}', '[]', binding, limits));
            assert.deepEqual(plan.diagnostics, []);
            assert.equal(plan.nodes[0].children[0].text, name === 'count' ? '4' : 'ABC');
            if (plan.nativeValueArtifactId) wasm.takeRenderValueArtifact(plan.nativeValueArtifactId);
        } finally {
            wasm.disposeTemplate(template.artifactId);
            assert.equal(wasm.disposeNativeValueArtifact(id), true);
        }
    }
    return true;
}

if (isMainThread) {
    const directory = await mkdtemp(join(tmpdir(), 'cem-native-constraints-'));
    try {
        await promisify(execFile)('cargo', ['test', '-p', 'cem-ql', '--target-dir', 'dist/target/cem_ql', '--test', 'named_attribute_constraints', 'portable_contract_retains_every_restriction_and_rejects_tampering'], {
            cwd: fileURLToPath(new URL('../../../', import.meta.url)),
            env: { ...process.env, CEM_NATIVE_CONSTRAINT_FIXTURES: directory },
        });
        const fixtures = await Promise.all(['count', 'code'].map(async name => ({
            name,
            bytes: new Uint8Array(await readFile(join(directory, `${name}-valid.cemv`))),
            invalid: await Promise.all([0, 1].map(async index => new Uint8Array(await readFile(join(directory, `${name}-invalid-${index}.cemv`))))),
        })));
        for (let index = 0; index < 2; index += 1) {
            const worker = new Worker(new URL(import.meta.url), { workerData: fixtures });
            try {
                assert.equal(await new Promise((resolve, reject) => {
                    worker.once('message', resolve);
                    worker.once('error', reject);
                    worker.once('exit', code => { if (code) reject(new Error(`worker exited ${code}`)); });
                }), true);
            } finally { await worker.terminate(); }
        }
        await ready();
        assert.equal(consume(fixtures), true);
        console.log('Composed native constraints: saved artifacts, separate workers, fallback and rejection passed.');
    } finally { await rm(directory, { recursive: true, force: true }); }
} else {
    await ready();
    parentPort.postMessage(consume(workerData));
}
