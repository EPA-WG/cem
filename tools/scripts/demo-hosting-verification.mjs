import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { DEMO_INVENTORY } from './demo-deployment.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
export function demoHostingMode(value = 'local-preview') {
    assert.ok(['local-preview', 'production'].includes(value), 'CEM_DEMO_HOSTING must be local-preview or production');
    return value;
}
export function assertDemoStudioPath(path, mode) {
    demoHostingMode(mode);
    if (mode === 'production') assert.equal(path, '/studio/', 'Production Studio must retain its exact subtree');
    else assert.match(path, /^\/studio\/__cem_preview__\/[a-f0-9]{16}\/$/, 'Local Studio must retain its revision subtree');
}

/** Inventory identity is a freshness check, not proof of remote asset bytes or interaction. */
export async function verifyRemoteDemoAssets({ destination, url, request = fetch }) {
    const base = new URL(url);
    assert.ok(['http:', 'https:'].includes(base.protocol));
    assert.equal(base.pathname, '/', 'Demo verification requires the deployment root');
    assert.equal(base.search, ''); assert.equal(base.hash, '');
    const local = await readFile(join(destination, DEMO_INVENTORY));
    const receipts = [];
    async function verify(path, expected) {
        const response = await request(new URL(path, base), { cache: 'no-store', signal: AbortSignal.timeout(30_000) });
        assert.equal(response.status, 200, `Public demo HTTP failure: ${path}`);
        const bytes = Buffer.from(await response.arrayBuffer());
        assert.equal(bytes.length, expected.bytes, `Stale public demo byte length: ${path}. Refresh the configured deployment.`);
        const sha256 = hash(bytes);
        assert.equal(sha256, expected.sha256, `Stale public demo hash: ${path}. Refresh the configured deployment.`);
        receipts.push({ path, url: response.url, bytes: bytes.length, sha256 });
    }
    await verify(DEMO_INVENTORY, { bytes: local.length, sha256: hash(local) });
    const inventory = JSON.parse(local);
    const critical = [
        'packages/cem-demo-element/demo/playground.html',
        'studio/module-map.json', 'studio/cache-inventory.json',
        'studio/samples/examples/login.cem-studio.json',
        'studio/samples/examples/supplier-pipeline.cem-studio.json',
        'packages/cem-ml-npm/dist/wasm/browser/cem_ml_factory.js',
        'packages/cem_ql/dist/wasm/cem_ql_factory.js',
    ];
    const paths = [...critical, ...inventory.files.filter(entry => entry.path.endsWith('.wasm')).map(entry => entry.path)];
    for (const path of paths) {
        const entry = inventory.files.find(entry => entry.path === path);
        assert.ok(entry, `Missing critical deployment asset: ${path}`);
        await verify(path, entry);
    }
    return { hosting: 'production', inventorySha256: hash(local), fileCount: inventory.fileCount,
        checkedRemoteFiles: receipts, interactionAcceptance: 'not-established-by-byte-checks' };
}
