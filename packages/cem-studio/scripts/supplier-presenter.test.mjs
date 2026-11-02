import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { readSupplierPresenter, supplierPresenterRoot, verifySupplierPresenter } from './supplier-presenter.mjs';

async function altered(change, check) {
    const directory = await mkdtemp(resolve(tmpdir(), 'cem-supplier-freeze-test-'));
    const root = resolve(directory, 'freeze');
    try {
        await cp(supplierPresenterRoot, root, { recursive: true });
        const manifest = JSON.parse(await readFile(resolve(root, 'manifest.json'), 'utf8'));
        await change(manifest, root);
        await writeFile(resolve(root, 'manifest.json'), JSON.stringify(manifest));
        await check(root);
    } finally { await rm(directory, { recursive: true, force: true }); }
}
test('freezes all Supplier configurations, independent XML repair and three stock variants', async () => {
    const { manifest, bundle } = await readSupplierPresenter();
    assert.equal(bundle.project.entries.length, 6);
    assert.equal(manifest.checkpoints.length, 10);
    const failed = manifest.checkpoints.find(item => item.scenario.id === 'availability-invalid-stock');
    assert.equal(failed.exitCode, 1); assert.deepEqual(failed.artifacts, []);
    const repaired = manifest.checkpoints.find(item => item.scenario.id === 'availability-restored-stock');
    assert.equal(repaired.exitCode, 0); assert.equal(repaired.artifacts.length, 3);
});
for (const [name, change, pattern] of [
    ['missing YAML output', m => { m.checkpoints.find(item => item.scenario.id === 'availability').artifacts.splice(1, 1); }, /output set changed/],
    ['stale output on failed run', m => { m.checkpoints.find(item => item.scenario.stock === 'invalid').artifacts = m.checkpoints.find(item => item.scenario.id === 'availability').artifacts; }, /output set changed/],
    ['changed stock scenario', m => { m.checkpoints.find(item => item.scenario.stock === 'invalid').scenario.stock = 'zero'; }, /checkpoint coverage changed/],
    ['weakened privacy', m => { m.privacy.automaticRecovery = true; }, /privacy policy changed/],
    ['changed archive', async (m, root) => { await writeFile(resolve(root, m.archive.file), 'altered'); }, /Frozen Supplier bytes changed/],
]) test(`refuses ${name}`, async () => altered(change, root => assert.rejects(readSupplierPresenter(root), pattern)));

test('native replay refuses altered YAML even when the snapshot checksum is renewed', async () => altered(async (manifest, root) => {
    const artifact = manifest.checkpoints.find(item => item.scenario.id === 'availability').artifacts.find(item => item.path.endsWith('.yaml'));
    const bytes = Buffer.concat([await readFile(resolve(root, artifact.file)), Buffer.from('\n')]);
    await writeFile(resolve(root, artifact.file), bytes);
    artifact.sha256 = createHash('sha256').update(bytes).digest('hex');
}, root => assert.rejects(verifySupplierPresenter(root), /availability.yaml.*drifted/)));
