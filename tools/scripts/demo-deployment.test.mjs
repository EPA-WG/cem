import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, writeFile, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { DEMO_INVENTORY, listDemoFiles, publishDemoDeployment } from './demo-deployment.mjs';

const entry = (path, content) => ({ path, content });
async function fixture(t) {
    const parent = await mkdtemp(join(tmpdir(), 'cem-deployment-'));
    const destination = join(parent, 'site'); await mkdir(destination);
    t.after(() => rm(parent, { recursive: true, force: true }));
    return { parent, destination };
}
test('deterministic inventory records exact binary bytes and includes itself in the count', async t => {
    const { destination, parent } = await fixture(t);
    const source = join(parent, 'module.wasm'); await writeFile(source, Buffer.from([0, 97, 115, 109]));
    const entries = [entry('index.html', 'home'), { path: 'assets/module.wasm', source }];
    const first = await publishDemoDeployment({ destination, entries });
    const bytes = await readFile(join(destination, DEMO_INVENTORY));
    assert.equal(first.totalFileCount, 3); assert.equal(first.inventory.fileCount, 3);
    assert.equal(first.inventory.files[0].sha256, createHash('sha256').update(await readFile(source)).digest('hex'));
    await publishDemoDeployment({ destination, entries: [...entries].reverse() });
    assert.deepEqual(await readFile(join(destination, DEMO_INVENTORY)), bytes);
});
test('first migration deletes only hash-proven legacy outputs and preserves unknown or modified files', async t => {
    const { destination } = await fixture(t);
    await writeFile(join(destination, 'old.html'), 'owned');
    await writeFile(join(destination, 'modified.js'), 'user change');
    await writeFile(join(destination, 'notes.txt'), 'unrelated');
    const result = await publishDemoDeployment({ destination, entries: [entry('index.html', 'new')],
        legacyEntries: [entry('old.html', 'owned'), entry('modified.js', 'original')] });
    assert.deepEqual(result.removed, ['old.html']); assert.deepEqual(result.preserved, ['modified.js']);
    assert.equal(await readFile(join(destination, 'modified.js'), 'utf8'), 'user change');
    assert.equal(await readFile(join(destination, 'notes.txt'), 'utf8'), 'unrelated');
    assert.equal(result.totalFileCount, 4);
});
test('later publications retire stale owned files and leave modified stale files untouched', async t => {
    const { destination } = await fixture(t);
    await publishDemoDeployment({ destination, entries: [entry('old.js', 'old'), entry('edited.js', 'old')] });
    await writeFile(join(destination, 'edited.js'), 'keep');
    const result = await publishDemoDeployment({ destination, entries: [entry('index.html', 'new')] });
    assert.deepEqual(result.removed, ['old.js']); assert.deepEqual(result.preserved, ['edited.js']);
    assert.deepEqual(await listDemoFiles(destination), [DEMO_INVENTORY, 'edited.js', 'index.html']);
});
test('file-count overflow refuses before writing or removing anything, including unrelated files', async t => {
    const { destination } = await fixture(t);
    await publishDemoDeployment({ destination, entries: [entry('old.html', 'original')] });
    await writeFile(join(destination, 'personal.txt'), 'keep');
    const previous = await readFile(join(destination, DEMO_INVENTORY));
    await assert.rejects(publishDemoDeployment({ destination, fileLimit: 3,
        entries: [entry('a.html', 'a'), entry('b.html', 'b')] }), /4 files/);
    assert.deepEqual(await readFile(join(destination, DEMO_INVENTORY)), previous);
    assert.equal(await readFile(join(destination, 'old.html'), 'utf8'), 'original');
    assert.equal(await readFile(join(destination, 'personal.txt'), 'utf8'), 'keep');
    assert.equal((await listDemoFiles(destination)).includes('a.html'), false);
});
test('rejects malformed, duplicate and escaping inventory records before publication', async t => {
    const { destination } = await fixture(t);
    const original = 'preserved'; await writeFile(join(destination, 'index.html'), original);
    const record = { path: 'old.js', bytes: 0, sha256: '0'.repeat(64) };
    for (const files of [[{ ...record, path: '../secret' }], [record, record], [{ ...record, path: 'C:/secret' }], [{ ...record, sha256: 'invalid' }]]) {
        await writeFile(join(destination, DEMO_INVENTORY), JSON.stringify({ schemaVersion: 1, producer: 'cem-demo-site', fileCount: files.length + 1, files }));
        await assert.rejects(publishDemoDeployment({ destination, entries: [entry('index.html', 'replacement')] }), /Invalid/);
        assert.equal(await readFile(join(destination, 'index.html'), 'utf8'), original);
    }
    await writeFile(join(destination, DEMO_INVENTORY), '{');
    await assert.rejects(publishDemoDeployment({ destination, entries: [entry('index.html', 'replacement')] }));
});
test('rejects symlinked managed paths and inventory files without changing outside bytes', async t => {
    const { destination, parent } = await fixture(t);
    const outside = join(parent, 'outside'); await mkdir(outside); await writeFile(join(outside, 'secret'), 'keep');
    await symlink(outside, join(destination, 'assets'));
    await assert.rejects(publishDemoDeployment({ destination, entries: [entry('assets/secret', 'bad')] }), /symbolic link/);
    await symlink(join(outside, 'secret'), join(destination, DEMO_INVENTORY));
    await assert.rejects(publishDemoDeployment({ destination, entries: [entry('index.html', 'bad')] }), /symbolic link/);
    assert.equal(await readFile(join(outside, 'secret'), 'utf8'), 'keep');
});
test('size limit is strict for generated and source bytes and refuses before mutation', async t => {
    const { destination, parent } = await fixture(t);
    await publishDemoDeployment({ destination, entries: [entry('old.html', 'keep')] });
    const previous = await readFile(join(destination, DEMO_INVENTORY));
    const source = join(parent, 'large.wasm'); await writeFile(source, Buffer.alloc(2048));
    for (const oversized of [entry('large.wasm', Buffer.alloc(2048)), { path: 'large.wasm', source }]) {
        await assert.rejects(publishDemoDeployment({ destination, fileSizeLimit: 2048,
            entries: [entry('index.html', 'new'), oversized] }), /must be below 2048/);
        assert.deepEqual(await readFile(join(destination, DEMO_INVENTORY)), previous);
        assert.equal(await readFile(join(destination, 'old.html'), 'utf8'), 'keep');
        assert.equal((await listDemoFiles(destination)).includes('index.html'), false);
    }
    const result = await publishDemoDeployment({ destination, fileSizeLimit: 2048, entries: [entry('fits.wasm', Buffer.alloc(2047))] });
    assert.equal(result.inventory.files[0].bytes, 2047);
});
test('retained unrelated or edited oversized files block publication without being removed', async t => {
    for (const path of ['personal.bin', 'edited.js']) {
        const { destination } = await fixture(t);
        await publishDemoDeployment({ destination, entries: [entry('edited.js', 'old')] });
        await writeFile(join(destination, path), Buffer.alloc(2048));
        const previous = await readFile(join(destination, DEMO_INVENTORY));
        await assert.rejects(publishDemoDeployment({ destination, fileSizeLimit: 2048, entries: [entry('new.js', 'new')] }), /must be below 2048/);
        assert.deepEqual(await readFile(join(destination, DEMO_INVENTORY)), previous);
        assert.equal((await readFile(join(destination, path))).length, 2048);
    }
});
test('inventory size and invalid size limits are checked before publication', async t => {
    const { destination } = await fixture(t);
    await assert.rejects(publishDemoDeployment({ destination, fileSizeLimit: 64, entries: [entry('a', 'a')] }), /demo-deployment.json/);
    for (const fileSizeLimit of [0, -1, NaN, 1.5]) await assert.rejects(publishDemoDeployment({ destination, fileSizeLimit, entries: [] }), /Invalid/);
    assert.deepEqual(await listDemoFiles(destination), []);
});
test('the default hosting policy refuses a file of exactly 25 MiB', async t => {
    const { destination, parent } = await fixture(t);
    const source = join(parent, 'limit.wasm'); await writeFile(source, Buffer.alloc(25 * 1024 * 1024));
    await assert.rejects(publishDemoDeployment({ destination, entries: [{ path: 'limit.wasm', source }] }), /must be below 26214400/);
    assert.deepEqual(await listDemoFiles(destination), []);
});
test('a compliant deployment may replace oversized owned files and retire unchanged oversized stale files', async t => {
    const { destination } = await fixture(t);
    await publishDemoDeployment({ destination, entries: [entry('runtime.wasm', Buffer.alloc(2048)), entry('stale.wasm', Buffer.alloc(2048))] });
    const result = await publishDemoDeployment({ destination, fileSizeLimit: 2048, entries: [entry('runtime.wasm', Buffer.alloc(2047))] });
    assert.deepEqual(result.removed, ['stale.wasm']);
    assert.equal((await readFile(join(destination, 'runtime.wasm'))).length, 2047);
    assert.deepEqual(await listDemoFiles(destination), [DEMO_INVENTORY, 'runtime.wasm']);
});
