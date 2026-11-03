import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { studioAcceptanceUrl, amsterdamStudioUrl } from './studio-acceptance-url.mjs';
import { verifyParticipantAcceptance } from './participant-kit-acceptance.mjs';
import { participantFiles, startParticipantKit, verifyParticipantFiles } from './participant-kit-launcher.mjs';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';

async function fixture(run) {
    const root = await mkdtemp(resolve(tmpdir(), 'cem-kit-test-'));
    try {
        await mkdir(resolve(root, 'site'));
        await writeFile(resolve(root, 'site/index.html'), '<base href="./" data-cem-studio-scope><h1>Studio</h1>');
        await writeFile(resolve(root, 'site/security-headers.json'), JSON.stringify({ headers: { 'content-security-policy': "default-src 'self'" } }));
        await writeFile(resolve(root, 'site/runtime.wasm'), new Uint8Array([0, 97, 115, 109]));
        const files = [];
        for (const path of await participantFiles(root)) {
            const bytes = await readFile(resolve(root, path));
            files.push({ path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
        }
        await writeFile(resolve(root, 'kit-manifest.json'), JSON.stringify({ schemaVersion: 1, files }));
        await run(root);
    } finally { await rm(root, { recursive: true, force: true }); }
}

test('portable production hosting preserves original index, security headers and WASM MIME without a build report', async () => fixture(async root => {
    const server = await startParticipantKit(root, 0);
    try {
        const url = `http://127.0.0.1:${server.address().port}`;
        const index = await fetch(url);
        assert.equal(await index.text(), await readFile(resolve(root, 'site/index.html'), 'utf8'));
        assert.equal(index.headers.get('content-security-policy'), "default-src 'self'");
        assert.equal(index.headers.get('service-worker-allowed'), '/');
        assert.equal((await fetch(`${url}/runtime.wasm`)).headers.get('content-type'), 'application/wasm');
        assert.equal((await fetch(`${url}/%2e%2e%2fkit-manifest.json`)).status, 403);
    } finally { await new Promise(resolveClose => server.close(resolveClose)); }
}));

test('refuses a same-size changed deployment file before serving', async () => fixture(async root => {
    await writeFile(resolve(root, 'site/runtime.wasm'), new Uint8Array([1, 97, 115, 109]));
    await assert.rejects(startParticipantKit(root, 0), /bytes changed/);
}));
test('development preview still versions assets by default', async () => fixture(async root => {
    const report = resolve(root, 'build.json');
    await writeFile(report, JSON.stringify({ hashScheme: 'sha256', hash: 'a'.repeat(64), assets: [] }));
    const server = createServer(createCemStudioPreviewHandler({ root: resolve(root, 'site'), buildReport: report }));
    await new Promise(ready => server.listen(0, '127.0.0.1', ready));
    try {
        const response = await fetch(`http://127.0.0.1:${server.address().port}/`);
        assert.match(await response.text(), /<base href="\/__cem_preview__\/[a-f0-9]{16}\//);
    } finally { await new Promise(resolveClose => server.close(resolveClose)); }
}));
test('refuses missing files', async () => fixture(async root => {
    await unlink(resolve(root, 'site/index.html'));
    await assert.rejects(verifyParticipantFiles(root), /inventory changed/);
}));
test('refuses unexpected files', async () => fixture(async root => {
    await writeFile(resolve(root, 'unexpected.txt'), 'unreviewed');
    await assert.rejects(verifyParticipantFiles(root), /inventory changed/);
}));


const receipt = JSON.parse(await readFile(new URL('../../../docs/declarative-amsterdam-2026/participant-kit-readback.json', import.meta.url), 'utf8'));
const kitClaim = { archiveSha256: '2b8cec8f3cebe74c3a705313f9fc8b815056e614a1bd707a336c9b550ccfdf53',
    sourceRevision: 'ee3731757e2864334ef2bb18f67c92ea4ed37a3b', node: 'v24.16.0',
    platform: 'linux', arch: 'x64', browser: '148.0.7778.96' };

test('dated exercise evidence accepts only the exact local kit and tested host', () => {
    const result = verifyParticipantAcceptance(receipt, kitClaim);
    assert.equal(result.route, 'local-transfer');
    for (const field of ['publicAvailability', 'freshInstallation', 'otherPlatforms']) assert.equal(result[field], 'unverified');
});
for (const [name, mutate] of [
    ['HTTP reachability', () => ({ schemaVersion: 1, httpStatus: 200 })],
    ['CLI version smoke', () => ({ schemaVersion: 1, node: receipt.node, cliVersion: '0.1.1' })],
    ['different archive', data => ({ ...data, archiveSha256: 'a'.repeat(64) })],
    ['different source', data => ({ ...data, sourceRevision: 'a'.repeat(40) })],
    ['another OS', data => ({ ...data, platform: 'win32' })],
    ['another Node', data => ({ ...data, node: 'v22.12.0' })],
    ['another browser', data => ({ ...data, browser: 'unverified' })],
    ['no offline run', data => ({ ...data, offline: false })],
    ['lost drafts', data => ({ ...data, draftPreservation: false })],
    ['shared copies', data => ({ ...data, independentCopies: false })],
    ['incomplete configurations', data => ({ ...data, loginConfigurations: 19 })],
    ['wrong advertised routes', data => ({ ...data, advertisedAmsterdamRoutes: ['login'] })],
    ['credential export', data => ({ ...data, credentialPolicy: { ...data.credentialPolicy, enteredCredentials: 'export' } })],
    ['public promotion', data => ({ ...data, publicAvailability: 'verified' })],
]) test(`release claims refuse ${name}`, () => {
    assert.throws(() => verifyParticipantAcceptance(mutate(structuredClone(receipt)), kitClaim));
});


test('full exercise acceptance remains loopback-only by default and normalizes trailing slashes', () => {
    assert.equal(studioAcceptanceUrl(), undefined);
    for (const host of ['localhost', '127.0.0.1', '[::1]']) {
        assert.equal(studioAcceptanceUrl(`http://${host}:8080/studio/`), `http://${host}:8080/studio`);
    }
    assert.throws(() => studioAcceptanceUrl(amsterdamStudioUrl));
    assert.equal(studioAcceptanceUrl(amsterdamStudioUrl, {publicProfile:'amsterdam-2026',presentationOnly:true}), amsterdamStudioUrl.slice(0,-1));
});
test('public full exercise acceptance requires its named profile, exact target and selected suite', () => {
    const options = { publicProfile:'amsterdam-2026', presentationOnly:true };
    for (const target of [
        'http://green-morning-fd6e.trans-acf.workers.dev/studio/',
        'https://example.test/studio/',
        'https://green-morning-fd6e.trans-acf.workers.dev/',
        'https://green-morning-fd6e.trans-acf.workers.dev/studio/other/',
        'https://green-morning-fd6e.trans-acf.workers.dev:8443/studio/',
        'https://user:password@green-morning-fd6e.trans-acf.workers.dev/studio/',
        `${amsterdamStudioUrl}?project=existing`, `${amsterdamStudioUrl}#existing`,
    ]) assert.throws(() => studioAcceptanceUrl(target,options));
    assert.throws(() => studioAcceptanceUrl(amsterdamStudioUrl, {...options,publicProfile:'amsterdam'}));
    assert.throws(() => studioAcceptanceUrl(amsterdamStudioUrl, {...options,presentationOnly:false}));
    assert.throws(() => studioAcceptanceUrl(undefined, options));
});
