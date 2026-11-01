import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { presenterRoot, readLoginPresenter, verifyLoginPresenter } from './login-presenter.mjs';

async function altered(change, check) {
    const directory = await mkdtemp(resolve(tmpdir(), 'cem-login-freeze-test-'));
    const root = resolve(directory, 'freeze');
    try {
        await cp(presenterRoot, root, { recursive: true });
        const manifest = JSON.parse(await readFile(resolve(root, 'manifest.json'), 'utf8'));
        await change(manifest, root);
        await writeFile(resolve(root, 'manifest.json'), JSON.stringify(manifest));
        await check(root);
    } finally { await rm(directory, { recursive: true, force: true }); }
}

test('frozen ordinary Login project covers all configurations and its saved repair', async () => {
    const { manifest, bundle, definition } = await readLoginPresenter();
    assert.equal(definition.configurations.length, 20);
    assert.equal(manifest.checkpoints.length, 21);
    assert.equal(bundle.project.entries.length, 20);
    assert.ok(manifest.checkpoints.some(item => item.id === 'validation-repair-repaired' && item.exitCode === 0));
});

for (const [name, change, message] of [
    ['changed input bytes', async (m, root) => {
        await writeFile(resolve(root, m.archive.file), `${await readFile(resolve(root, m.archive.file), 'utf8')}\n`);
    }, /Frozen presenter bytes changed/],
    ['missing checkpoint', m => { m.checkpoints.pop(); }, /checkpoint coverage changed/],
    ['altered command', m => { m.checkpoints[0].argv.push('--unreviewed'); }, /deep-equal/],
    ['foreign output path', m => { m.checkpoints[0].output.file = '../outside'; }, /escapes its directory/],
    ['relaxed credential blocking', m => { m.privacy.enteredCredentials = 'capture'; }, /privacy policy changed/],
    ['automatic recovery enabled', m => { m.privacy.automaticRecovery = true; }, /privacy policy changed/],
    ['larger recovery budget', m => { m.privacy.aggregateCheckpointBytes *= 2; }, /privacy policy changed/],
    ['longer handoff deadline', m => { m.privacy.handoffTimeoutMs *= 2; }, /privacy policy changed/],
    ['implicit source revision', m => { m.sourceRevision = 'latest'; }, /source revision must be explicit/],
]) test(`refuses ${name}`, async () => altered(change,
    root => assert.rejects(readLoginPresenter(root), message)));

for (const boundary of ['output', 'diagnostics']) test(`native replay refuses changed ${boundary} even with a renewed file hash`, async () => {
    await altered(async (manifest, root) => {
        const descriptor = manifest.checkpoints[0][boundary];
        const bytes = Buffer.concat([await readFile(resolve(root, descriptor.file)), Buffer.from('\n')]);
        await writeFile(resolve(root, descriptor.file), bytes);
        descriptor.sha256 = createHash('sha256').update(bytes).digest('hex');
    }, root => assert.rejects(verifyLoginPresenter(root), new RegExp(`native ${boundary} drifted`)));
});
