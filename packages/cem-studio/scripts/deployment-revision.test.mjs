import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { computeStudioDeploymentRevision, deploymentRevisionModule } from './deployment-revision.mjs';

test('deployment revision is deterministic and covers shell, modules and sample bytes', async () => {
    const root = await mkdtemp(join(tmpdir(), 'cem-deployment-revision-'));
    try {
        await mkdir(join(root, 'samples'));
        for (const [path, text] of [['index.html', '<main/>'], ['module.js', 'export default 1;'],
            ['samples/readme.md', 'First'], ['deployment-revision.js', deploymentRevisionModule('')]]) {
            await writeFile(join(root, path), text);
        }
        const first = await computeStudioDeploymentRevision(root);
        assert.match(first.revision, /^[a-f0-9]{64}$/);
        assert.deepEqual(await computeStudioDeploymentRevision(root), first);
        for (const [path, changed, original] of [['index.html', '<article/>', '<main/>'],
            ['module.js', 'export default 2;', 'export default 1;'], ['samples/readme.md', 'Second', 'First']]) {
            await writeFile(join(root, path), changed);
            assert.notEqual((await computeStudioDeploymentRevision(root)).revision, first.revision, path);
            await writeFile(join(root, path), original);
        }
        assert.deepEqual(await computeStudioDeploymentRevision(root), first);
        assert.deepEqual(first.files.map(file => file.path), ['deployment-revision.js', 'index.html', 'module.js', 'samples/readme.md']);
        await writeFile(join(root, 'extra.css'), 'body {}');
        assert.notEqual((await computeStudioDeploymentRevision(root)).revision, first.revision);
    } finally { await rm(root, { recursive: true, force: true }); }
});

test('revision module admits only a digest or the provisional build input', () => {
    assert.equal(deploymentRevisionModule('f'.repeat(64)), `export const deploymentRevision = '${'f'.repeat(64)}';\n`);
    for (const revision of ['not-a-digest', 'A'.repeat(64), 'f'.repeat(65), "'; execute(); //"]) {
        assert.throws(() => deploymentRevisionModule(revision), /deployment revision/);
    }
});
