import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { bodyHash, bodyTotals, studioOfflinePlan, catalogOfflinePlan, verifyOfflineBodies } from './studio-offline-footprint.mjs';
import { publishDemoDeployment } from './demo-deployment.mjs';

async function fixture(run) {
    const root = await mkdtemp(join(tmpdir(), 'cem-offline-bytes-'));
    const json = (path, value) => writeFile(join(root, path), JSON.stringify(value));
    try {
        await mkdir(join(root, 'samples'));
        await json('build.json', { commonVersion: '0.1.0' });
        await json('cache-inventory.json', { schemaVersion: 2, commonVersion: '0.1.0', groups: [
            { id: 'shell', urls: ['./module-map.json'] }, { id: 'runtime', moduleMap: './module-map.json' },
            { id: 'samples', catalog: './samples/index.json' }] });
        await json('module-map.json', { imports: { a: { path: './a.js' }, alias: { path: './a.js' } }, resources: {} });
        await writeFile(join(root, 'a.js'), 'module bytes');
        await json('samples/index.json', { schemaVersion: 1, commonVersion: '0.1.0', cacheUrls: ['./document.cem'] });
        await writeFile(join(root, 'samples/document.cem'), 'original source bytes');
        await run(root, json);
    } finally { await rm(root, { recursive: true, force: true }); }
}

test('offline totals distinguish entries, URL copies and identical content without treating them as disk bytes', () => {
    const sha256 = bodyHash(Buffer.from('same'));
    assert.deepEqual(bodyTotals([{ path: 'a', bytes: 4, sha256 }, { path: 'a', bytes: 4, sha256 }, { path: 'b', bytes: 4, sha256 }]),
        { entries: 3, bodyBytes: 12, uniqueUrls: 2, uniqueUrlBytes: 8, uniqueContents: 1, uniqueContentBytes: 4 });
    assert.throws(() => bodyTotals([{ path: 'a', bytes: 4, sha256 }, { path: 'a', bytes: 5, sha256 }]), /inconsistent/);
    assert.throws(() => bodyTotals([{ path: 'a', bytes: 4, sha256 }, { path: 'b', bytes: 5, sha256 }]), /inconsistent size/);
});

test('plan expansion deduplicates aliases per cache but retains cross-cache copies and verifies original bytes', async () => fixture(async root => {
    const plan = await studioOfflinePlan(root);
    assert.equal(plan.groups.find(group => group.id === 'runtime').files.length, 2);
    assert.equal(plan.offlineTotals.entries, 5); assert.equal(plan.offlineTotals.uniqueUrls, 4);
    const caches = plan.groups.map(group => ({ name: 'scope:revision:' + group.id, files: structuredClone(group.files) }));
    const fetched = [...new Map(plan.groups.flatMap(group => group.files).map(file => [file.path, file])).values()];
    assert.deepEqual(verifyOfflineBodies(plan, caches, fetched), plan.offlineTotals);
    caches[0].files[0].sha256 = '0'.repeat(64);
    assert.throws(() => verifyOfflineBodies(plan, caches, fetched), /cached body differs/);
    assert.throws(() => verifyOfflineBodies(plan, [], fetched), /cache groups/);
}));

test('missing, external and escaped resources refuse instead of publishing incomplete totals', async () => {
    for (const path of ['./missing.js', '../escape.js', 'https://other.example/a.js']) await fixture(async (root, json) => {
        await json('module-map.json', { imports: { bad: { path } }, resources: {} });
        await assert.rejects(studioOfflinePlan(root), /ENOENT|scope/);
    });
});

test('optional pack files and index must agree with their original digest pins', async () => fixture(async (root, json) => {
    await mkdir(join(root, 'pack'));
    const index = Buffer.from('{"formatVersion":"cem-schema-asset-pack/1","sources":[]}');
    await writeFile(join(root, 'pack/index.json'), index);
    const indexSha256 = bodyHash(index);
    await json('build.json', { commonVersion: '0.1.0', deploymentProfile: 'schema-assets', deploymentDigest: 'a', schemaAssetIndexDigest: indexSha256 });
    await json('cache-inventory.json', { schemaVersion: 2, commonVersion: '0.1.0', deploymentProfile: 'schema-assets', deploymentDigest: 'a', groups: [
        { id: 'shell', urls: ['./module-map.json'] }, { id: 'runtime', moduleMap: './module-map.json' },
        { id: 'samples', catalog: './samples/index.json' }, { id: 'schema-assets', inventory: './pack/integrity.json' }] });
    await json('pack/integrity.json', { algorithm: 'sha256', commonVersion: '0.1.0', indexSha256,
        files: [{ path: 'index.json', bytes: index.length, sha256: indexSha256 }], offlineUrls: ['index.json', 'integrity.json'] });
    assert.equal((await studioOfflinePlan(root)).groups.length, 4);
    await writeFile(join(root, 'pack/index.json'), index.toString().replace('sources', 'changed'));
    await assert.rejects(studioOfflinePlan(root), /Assertion/);
}));

test('full catalog preload refuses an omitted declaration or drift from its original receipt', async () => fixture(async root => {
    const output = join(root, 'catalog');
    await publishDemoDeployment({ destination: output, entries: [
        { path: 'components/one.xhtml', content: '<cem-element/>' }, { path: 'core.wasm', content: 'original native bytes' }] });
    const catalog = { components: [{ tag: 'one', implementation: { source: 'components/one.xhtml' } }] };
    const plan = await catalogOfflinePlan(output, catalog);
    assert.equal(plan.groups[0].files.length, 3);
    assert.equal(plan.offlineTotals.bodyBytes, plan.deliveryTotals.bodyBytes);
    await assert.rejects(catalogOfflinePlan(output, { components: [{ implementation: { source: 'missing.xhtml' } }] }), /declaration omitted/);
    await writeFile(join(output, 'core.wasm'), 'changed native bytes');
    await assert.rejects(catalogOfflinePlan(output, catalog), /inventory drift/);
}));
