import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';

const project = resolve(import.meta.dirname, '..');
const workspace = resolve(project, '../..');
const generated = resolve(project, 'generated/feature-tour');
const ordinary = resolve(project, 'dist/static');
const optional = resolve(project, 'dist/schema-assets');
const cli = resolve(workspace, 'dist/target/cem_ml_cli/debug/cem-ml');

function run(command, args) {
    const result = spawnSync(command, args, { cwd: project, encoding: 'utf8' });
    if (result.error) throw result.error;
    assert.equal(result.status, 0, `${command} ${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
}

async function hashes(root, prefix = '') {
    const result = [];
    for (const entry of (await readdir(resolve(root, prefix), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
        const path = prefix ? `${prefix}/${entry.name}` : entry.name;
        if (entry.isDirectory()) result.push(...await hashes(root, path));
        else result.push([path, createHash('sha256').update(await readFile(resolve(root, path))).digest('hex')]);
    }
    return result;
}

test('default → optional → default keeps exact generated inputs and native sample output', async () => {
    run(process.execPath, ['scripts/generate-feature-tour.mjs', '--check']);
    const sourceBefore = await hashes(generated);
    const ordinaryBefore = await hashes(ordinary);
    const emitDefault = () => run(cli, ['transform', '--config', resolve(generated, 'feature-tour.cem')]);
    emitDefault();
    assert.deepEqual(await hashes(ordinary), ordinaryBefore);

    run(process.execPath, ['scripts/build-schema-assets-profile.mjs']);
    // Check before regeneration: rewriting the default tree would hide pollution.
    run(process.execPath, ['scripts/generate-feature-tour.mjs', '--check']);
    assert.deepEqual(await hashes(generated), sourceBefore);
    assert.deepEqual(await hashes(ordinary), ordinaryBefore);
    assert.deepEqual(await hashes(resolve(optional, 'samples')), await hashes(resolve(ordinary, 'samples')));
    const optionalBefore = await hashes(optional);

    run(process.execPath, ['scripts/generate-feature-tour.mjs']);
    emitDefault();
    run(process.execPath, ['scripts/generate-feature-tour.mjs', '--check']);
    assert.deepEqual(await hashes(generated), sourceBefore);
    assert.deepEqual(await hashes(ordinary), ordinaryBefore);
    assert.deepEqual(await hashes(optional), optionalBefore);
});

test('the default check continues to reject an extra generated file', async () => {
    const extra = resolve(generated, 'sample-profile-isolation.unexpected.cem');
    const before = await hashes(generated);
    await writeFile(extra, '{run}', { flag: 'wx' });
    try {
        const result = spawnSync(process.execPath, ['scripts/generate-feature-tour.mjs', '--check'], {
            cwd: project, encoding: 'utf8',
        });
        if (result.error) throw result.error;
        assert.notEqual(result.status, 0);
        assert.match(result.stderr, /generated Feature Tour file set drifted/);
        assert.match(result.stderr, /sample-profile-isolation\.unexpected\.cem/);
    } finally { await rm(extra, { force: true }); }
    assert.deepEqual(await hashes(generated), before);
    run(process.execPath, ['scripts/generate-feature-tour.mjs', '--check']);
});
