import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { relative, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';

export async function participantFiles(root, directory = root) {
    const files = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = resolve(directory, entry.name);
        assert.ok(!entry.isSymbolicLink(), 'Participant kit must contain original files, not symlinks');
        if (entry.isDirectory()) files.push(...await participantFiles(root, path));
        else files.push(relative(root, path).split(sep).join('/'));
    }
    return files.sort();
}

export async function verifyParticipantFiles(root) {
    root = resolve(root);
    const manifest = JSON.parse(await readFile(resolve(root, 'kit-manifest.json'), 'utf8'));
    assert.equal(manifest.schemaVersion, 1);
    assert.deepEqual((await participantFiles(root)).filter(path => path !== 'kit-manifest.json'),
        manifest.files.map(file => file.path).sort(), 'Participant kit file inventory changed');
    for (const file of manifest.files) {
        const path = resolve(root, file.path);
        assert.ok(path.startsWith(`${root}${sep}`), 'Participant kit path escapes its directory');
        const bytes = await readFile(path);
        assert.equal(bytes.length, file.bytes, `Participant kit size changed: ${file.path}`);
        assert.equal(createHash('sha256').update(bytes).digest('hex'), file.sha256, `Participant kit bytes changed: ${file.path}`);
    }
    return manifest;
}

export async function startParticipantKit(root, port = 8080) {
    await verifyParticipantFiles(root);
    assert.ok(Number.isInteger(port) && port >= 0 && port <= 65535, 'Use a valid TCP port');
    const server = createServer(createCemStudioPreviewHandler({ root: resolve(root, 'site'), versionedAssets: false }));
    await new Promise((ready, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', ready); });
    return server;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
    const server = await startParticipantKit(import.meta.dirname, Number(process.env.CEM_STUDIO_KIT_PORT ?? '8080'));
    console.log(`CEM Studio participant kit: http://127.0.0.1:${server.address().port}/`);
}
