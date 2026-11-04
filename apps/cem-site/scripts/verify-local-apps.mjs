import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';

// The same assertions run on local workerd and the owner's deployed Site.
export async function verifyLocalApps(origin, securityHeaders) {
    const target = new URL(origin);
    // Studio's production CSP upgrades redirected HTTP requests. Exercise it
    // over local TLS; only loopback previews may use a development certificate.
    const localTls = target.protocol === 'https:' && ['127.0.0.1', 'localhost', '[::1]'].includes(target.hostname);
    const browser = await chromium.launch({ args: localTls ? ['--ignore-certificate-errors'] : [] });
    const report = { origin, status: 'started' };
    const errors = [], failures = [], legacyRequests = [], workerErrors = [];
    let stage = 'navigation';
    let page;
    try {
        const context = await browser.newContext({ serviceWorkers: 'allow', ignoreHTTPSErrors: localTls });
        page = await context.newPage();
        page.setDefaultTimeout(30_000);
        context.on('serviceworker', worker => worker.on('console', message => {
            if (message.type() === 'error') workerErrors.push(message.text());
        }));
        page.on('pageerror', error => errors.push(error.message));
        context.on('response', response => {
            if (response.url().startsWith(origin + '/') && response.status() >= 400
                && !response.url().endsWith('/favicon.ico')) failures.push(`${response.status()} ${response.url()}`);
        });
        context.on('request', request => {
            if (new URL(request.url()).hostname === 'green-morning-fd6e.trans-acf.workers.dev') legacyRequests.push(request.url());
        });
        const guide = origin + '/guides/declarative-amsterdam-2026/';
        await page.goto(guide);
        const navigation = page.locator('nav[aria-label=Primary]');
        const studio = navigation.getByRole('link', { name: 'Studio', exact: true });
        const playground = navigation.getByRole('link', { name: 'Playground', exact: true });
        assert.equal(await studio.getAttribute('href'), '/studio/');
        assert.equal(await playground.getAttribute('href'), '/packages/cem-demo-element/demo/playground');
        await studio.click();
        stage = 'Studio startup';
        assert.equal(new URL(page.url()).origin, origin);
        assert.equal(new URL(page.url()).pathname, '/studio/');
        const response = await context.request.get(origin + '/studio/');
        for (const [name, value] of Object.entries(securityHeaders)) assert.equal(response.headers()[name.toLowerCase()], value, `Studio header ${name}`);
        const ready = async () => {
            await page.waitForFunction(() => globalThis.__cemStudioApplication?.workbenchView);
            await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
        };
        await ready();
        console.info('Local applications: Studio ready');
        stage = 'Studio service worker activation';
        await page.waitForFunction(() => navigator.serviceWorker.controller);
        const scope = await page.evaluate(async () => (await navigator.serviceWorker.ready).scope);
        assert.equal(scope, origin + '/studio/');
        stage = 'Studio online build';
        await page.evaluate(() => globalThis.__cemStudioApplication.workbench.openEntity('collection=content-types-documents-ui&sample=scss-stylesheet&build=build-html'));
        await ready();
        await page.getByRole('button', { name: 'Clone selected', exact: true }).click();
        await page.getByRole('dialog').getByRole('textbox').fill('Presentation Site verification');
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click();
        await ready();
        const build = async () => {
            await page.getByRole('button', { name: 'Build project', exact: true }).click();
            await ready();
            const result = await page.evaluate(() => {
                const preview = globalThis.__cemStudioApplication.workbench.snapshot().projectPreview;
                return { status: preview.attempt?.status, text: preview.projection?.output.text };
            });
            assert.equal(result.status, 'succeeded');
            await page.waitForFunction(() => {
                const doc = document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument;
                const card = doc?.querySelector('.demo-card');
                return card && doc.defaultView.getComputedStyle(card).paddingTop === '12px';
            });
            return result.text;
        };
        const online = await build();
        console.info('Local applications: Studio online build passed');
        const workspace = resolve(import.meta.dirname, '../../..');
        const definition = JSON.parse(await readFile(resolve(workspace, 'examples/projects/scss-stylesheet/example.json'), 'utf8'));
        const graph = definition.configurations.find(item => item.id === 'build-html');
        const expected = definition.sources.find(item => item.id === graph.expectedGraphOutputs[graph.entryArtifact]);
        assert.equal(online, await readFile(resolve(workspace, expected.source), 'utf8'));
        await context.setOffline(true);
        stage = 'Studio offline rebuild';
        await page.reload(); await ready();
        assert.equal(await build(), online);
        await context.setOffline(false);
        report.studio = { route: '/studio/', scope, securityHeaders: 'matched', savedCopy: true, nativeBuild: true, offlineRebuild: true };
        console.info('Local applications: Studio offline rebuild passed');
        stage = 'Playground render';
        await page.goto(guide);
        assert.equal(await page.evaluate(() => navigator.serviceWorker.controller), null, 'Studio worker must not control Site documents');
        await playground.click();
        assert.equal(new URL(page.url()).origin, origin);
        assert.equal(new URL(page.url()).pathname, '/packages/cem-demo-element/demo/playground');
        await page.waitForFunction(() => document.querySelector('.playground-status')?.getAttribute('data-state') === 'ready');
        await page.getByRole('textbox', { name: 'Source code', exact: true }).fill('<article><h2>Presentation Site render</h2><p>Local source</p></article>');
        await page.getByRole('combobox', { name: 'Input format', exact: true }).selectOption('html');
        await page.getByRole('button', { name: 'Render', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('.playground-status')?.getAttribute('data-state') === 'ready');
        assert.equal(await page.locator('.playground-preview h2').textContent(), 'Presentation Site render');
        assert.equal(await page.locator('.playground-preview p').textContent(), 'Local source');
        report.playground = { route: '/packages/cem-demo-element/demo/playground', editedSourceRender: true };
        assert.deepEqual(errors, [], 'Same-origin application browser errors');
        assert.deepEqual(failures, [], 'Same-origin application HTTP failures');
        assert.deepEqual(legacyRequests, [], 'Applications must not request the former demo host');
        assert.deepEqual(workerErrors, [], 'Studio service worker errors');
        report.legacyRequests = legacyRequests;
        report.status = 'passed';
        await context.close();
        return report;
    } catch (error) {
        const workers = await page?.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).map(registration => ({
            scope: registration.scope, installing: registration.installing?.state, waiting: registration.waiting?.state, active: registration.active?.state,
        }))).catch(() => undefined);
        throw new Error(`${stage}: ${error.message}\n${JSON.stringify({ errors, failures, workerErrors, legacyRequests, workers })}`, { cause: error });
    } finally { await browser.close(); }
}
