import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { publicationOrigin, verifyPublishedAssets } from './publication-host.mjs';
import { verifyComponentGalleries } from './verify-galleries.mjs';
import { readPublicationInventory } from './publication-host-galleries.mjs';
import { verifyCodeSnippets } from './verify-snippets.mjs';
import { verifyLocalApps } from './verify-local-apps.mjs';
import { browserTransport } from './publication-browser.mjs';
import { verifyMarkdownTables } from './verify-markdown-tables.mjs';

const workspace = resolve(import.meta.dirname, '../../..');
const origin = publicationOrigin(process.env.CEM_SITE_ACCEPTANCE_URL);
assert.ok(origin, 'Set CEM_SITE_ACCEPTANCE_URL to the actual HTTPS origin printed by Wrangler');
const reports = resolve(workspace, 'dist/reports/cem-site');
const file = resolve(reports, 'public-deployment.json');
const report = { schemaVersion: 1, checkedAt: new Date().toISOString(), origin, browserTransport, status: 'started', suites: [] };
const save = async () => { await mkdir(reports, { recursive: true }); await writeFile(file, JSON.stringify(report, null, 2) + '\n'); };
await save();
try {
    for (const suite of ['production', 'interactive', 'search']) await rm(resolve(reports, `public-${suite}.json`), { force: true });
    const native = JSON.parse(await readFile(resolve(reports, 'determinism.json'), 'utf8'));
    const prepared = JSON.parse(await readFile(resolve(reports, 'cloudflare-preparation.json'), 'utf8'));
    assert.equal(native.aggregateSha256, prepared.nativeAggregateSha256, 'Public verification must use the verified prepared release; run cem-site:prepare:cloudflare first');
    const inventory = await readPublicationInventory(prepared, native);
    report.preparedArchiveSha256 = prepared.archive.sha256;
    report.nativeAggregateSha256 = native.aggregateSha256;
    for (const entry of prepared.hostingControls ?? []) {
        const bytes = await readFile(resolve(prepared.output, entry.path));
        assert.ok(bytes.length === entry.bytes && createHash('sha256').update(bytes).digest('hex') === entry.sha256,
            `Prepared hosting control changed: ${entry.path}`);
    }
    const nativePaths = new Set(native.files.map(entry => entry.path));
    for (const entry of inventory.files) {
        const bytes = await readFile(resolve(nativePaths.has(entry.path) ? resolve(workspace, 'dist/apps/cem-site') : prepared.output, entry.path));
        assert.ok(bytes.length === entry.bytes && createHash('sha256').update(bytes).digest('hex') === entry.sha256,
            `Prepared Site differs from deterministic build: ${entry.path}`);
    }
    const routes = JSON.parse(await readFile(resolve(workspace, 'apps/cem-site/site.routes.json'), 'utf8'));
    assert.ok(prepared.markdownTables, 'Prepare the Markdown-table Site update before public verification');
    const markdownBytes = await readFile(prepared.markdownTables.path);
    assert.equal(createHash('sha256').update(markdownBytes).digest('hex'), prepared.markdownTables.sha256, 'Prepared Markdown audit changed');
    report.assets = await verifyPublishedAssets(origin, inventory, { routes: [...routes.entries, ...inventory.galleryRoutes] });
    await save();
    // These are the existing browser suites, with their complete original
    // assertions and deadlines. Execute sequentially against fresh contexts.
    for (const suite of ['production', 'interactive', 'search']) {
        const output = resolve(reports, `public-${suite}.json`);
        const result = spawnSync(process.execPath, [resolve(import.meta.dirname, `verify-${suite}.mjs`)], {
            cwd: workspace, env: process.env, stdio: 'inherit',
        });
        report.suites.push({ suite, exitCode: result.status, error: result.error?.message });
        await save();
        assert.equal(result.status, 0, `Public Site ${suite} acceptance failed`);
        report.suites.at(-1).receipt = JSON.parse(await readFile(output, 'utf8'));
    }
    const galleries = JSON.parse(await readFile(resolve(prepared.output, prepared.galleries.manifestPath), 'utf8'));
    report.galleries = await verifyComponentGalleries(origin, galleries);
    report.snippets = await verifyCodeSnippets(origin, routes.entries);
    report.markdownTables = await verifyMarkdownTables(origin, routes.entries, JSON.parse(markdownBytes));
    const security = JSON.parse(await readFile(resolve(prepared.output, 'studio/security-headers.json'), 'utf8'));
    report.localApps = await verifyLocalApps(origin, security.headers);
    report.status = 'passed';
} catch (error) {
    report.status = 'failed'; report.error = error.message;
    if (error.cause?.files) report.assetFailures = error.cause.files;
    if (error.startupCapture) report.studioStartup = error.startupCapture;
    throw error;
}
finally { report.finishedAt = new Date().toISOString(); await save(); }
