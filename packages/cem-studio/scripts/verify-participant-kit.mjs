import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { workspaceRoot } from './login-presenter.mjs';
import { verifyParticipantFiles } from './participant-kit-launcher.mjs';

const archive = resolve(workspaceRoot, 'dist/packages/cem-studio-login-kit.tgz');
const directory = await mkdtemp(resolve(tmpdir(), 'cem-login-kit-consumer-'));
let server;
try {
    execFileSync('tar', ['-xzf', archive, '-C', directory]);
    const root = resolve(directory, 'cem-studio-login-kit');
    const manifest = await verifyParticipantFiles(root);
    server = spawn(process.execPath, ['start.mjs'], { cwd: root, env: { ...process.env, CEM_STUDIO_KIT_PORT: '0' }, stdio: ['ignore', 'pipe', 'pipe'] });
    let stderr = '';
    server.stderr.on('data', bytes => { stderr += bytes; });
    const url = await new Promise((ready, reject) => {
        const timeout = setTimeout(() => reject(new Error(`Participant launcher timed out: ${stderr}`)), 20_000);
        let stdout = '';
        const finish = value => { clearTimeout(timeout); ready(value); };
        server.stdout.on('data', bytes => {
            stdout += bytes;
            const match = stdout.match(/http:\/\/127\.0\.0\.1:\d+/);
            if (match) finish(match[0]);
        });
        server.once('error', error => { clearTimeout(timeout); reject(error); });
        server.once('exit', code => { clearTimeout(timeout); reject(new Error(`Participant launcher exited ${code}: ${stderr}`)); });
    });
    const index = await fetch(`${url}/index.html`);
    assert.equal(index.status, 200);
    assert.ok(!(await index.text()).includes('/__cem_preview__/'), 'Participant hosting must keep production service-worker scope');
    execFileSync(process.execPath, [resolve(import.meta.dirname, 'verify-static-worker.mjs'), '--presentation-only'], {
        cwd: resolve(workspaceRoot, 'packages/cem-studio'), env: { ...process.env, CEM_STUDIO_ACCEPTANCE_URL: url }, stdio: 'inherit',
    });
    assert.deepEqual(await verifyParticipantFiles(root), manifest, 'Browser use changed the kit deployment');
    const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
    const browserVersion = browser.version(); await browser.close();
    const report = { schemaVersion: 1, sourceRevision: manifest.sourceRevision, frozenSourceRevision: manifest.frozenSourceRevision,
        supplierVersion: manifest.supplierVersion, frozenSupplierRevision: manifest.frozenSupplierRevision,
        archiveSha256: createHash('sha256').update(await readFile(archive)).digest('hex'),
        node: process.version, platform: process.platform, arch: process.arch, browser: browserVersion,
        extractedOutsideWorkspace: true, launcher: 'node-builtins', installedApplicationDependencies: false,
        originalProductionIndex: true, fileCount: manifest.files.length + 1,
        loginConfigurations: 20, supplierConfigurations: 3, offline: true, draftPreservation: true, independentCopies: true,
        advertisedAmsterdamRoutes: ['login', 'supplier-pipeline'],
        credentialPolicy: manifest.privacy, publicAvailability: 'unverified' };
    const reports = resolve(workspaceRoot, 'dist/reports/cem-studio'); await mkdir(reports, { recursive: true });
    await writeFile(resolve(reports, 'participant-kit.json'), `${JSON.stringify(report, null, 2)}\n`);
    console.log('Extracted participant kit passed local production hosting and complete offline Login/Supplier acceptance.');
} finally { server?.kill(); await rm(directory, { recursive: true, force: true }); }
