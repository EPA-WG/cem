import assert from 'node:assert/strict';
import { verifySiteThemes } from './verify-themes.mjs';
import { verifyDeployedXhtml } from './verify-xhtml.mjs';
import { createReadStream } from 'node:fs';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join, normalize, relative, resolve, sep } from 'node:path';
import { chromium } from 'playwright';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const searchPath = '/search/';
const manifest = JSON.parse(await readFile(resolve(workspaceRoot, 'apps/cem-site/site.routes.json'), 'utf8'));
const expectedDocumentCount = manifest.searchDocuments.length;

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(requestUrl.pathname);
        const relativePath = pathname.endsWith('/') ? `${pathname}index.html` : pathname;
        const filePath = normalize(join(outputRoot, relativePath));
        if (!filePath.startsWith(outputRoot + sep)) {
            response.writeHead(403);
            response.end('Forbidden');
            return;
        }
        const fileStat = await stat(filePath);
        if (!fileStat.isFile()) {
            throw new Error('not a file');
        }
        response.writeHead(200, { 'content-type': contentType(filePath) });
        createReadStream(filePath).pipe(response);
    } catch {
        response.writeHead(404);
        response.end('Not found');
    }
});

await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
const address = server.address();
const port = typeof address === 'object' && address ? address.port : 0;
const browser = await chromium.launch({ headless: true });

try {
    const page = await browser.newPage();
    const browserErrors = [];
    page.on('pageerror', (error) => browserErrors.push(error.message));
    page.on('console', (message) => {
        if (message.type() === 'error') {
            browserErrors.push(message.text());
        }
    });
    page.on('requestfailed', (request) => {
        browserErrors.push(`${request.url()}: ${request.failure()?.errorText ?? 'request failed'}`);
    });

    await page.goto(`http://127.0.0.1:${port}${searchPath}?q=Graph%20Semantics`, {
        waitUntil: 'networkidle',
        timeout: 120_000,
    });
    try {
        await page.waitForFunction(() => globalThis.__cemSiteSearch?.done === true);
    } catch (cause) {
        const runtimeState = await page.evaluate(() => ({
            search: globalThis.__cemSiteSearch,
            components: globalThis.__cemSiteComponents,
            scripts: [...document.scripts].map(({ src, type }) => ({ src, type })),
        }));
        throw new Error(
            `search readiness timed out: ${JSON.stringify({ browserErrors, runtimeState })}`,
            { cause },
        );
    }
    const initialRuntime = await page.evaluate(() => globalThis.__cemSiteSearch);
    if (browserErrors.length > 0 || initialRuntime.errors.length > 0) {
        throw new Error(`search runtime failed: ${[...browserErrors, ...initialRuntime.errors].join('; ')}`);
    }
    if (initialRuntime.documentCount !== expectedDocumentCount || initialRuntime.resultCount !== 1) {
        throw new Error(`search query contract drifted: ${JSON.stringify(initialRuntime)}`);
    }

    const xhtml = await verifyDeployedXhtml(page);
    if (browserErrors.length) throw new Error(`XHTML deployment failed: ${browserErrors.join("; ")}`);

    const deepLink = '/reference/cem-ml/transform-config/#heading-13';
    const link = page.locator(`a[href="${deepLink}"]`).first();
    if ((await link.textContent())?.trim() !== 'CEM-ML transform graph reference') {
        throw new Error('search did not route Graph Semantics to the transform reference fragment');
    }
    await link.click();
    await page.waitForLoadState('networkidle');
    const deepLinkContract = await page.evaluate(() => ({
        hash: globalThis.location.hash,
        heading: document.querySelector(globalThis.location.hash)?.textContent?.replace(/\s+/g, ' ').trim(),
    }));
    if (deepLinkContract.hash !== '#heading-13' || deepLinkContract.heading !== 'Graph Semantics') {
        throw new Error(`stable deep link contract drifted: ${JSON.stringify(deepLinkContract)}`);
    }

    await page.goto(`http://127.0.0.1:${port}${searchPath}`, { waitUntil: 'networkidle' });
    await page.waitForFunction(() => globalThis.__cemSiteSearch?.done === true);
    await page.locator('[data-search-field] input').fill('native output');
    await page.waitForFunction(() => globalThis.__cemSiteSearch?.query === 'native output');
    await page.locator('[data-search-action] button').click();
    await page.waitForFunction(() => globalThis.__cemSiteSearch?.done === true);
    const liveRuntime = await page.evaluate(() => globalThis.__cemSiteSearch);
    const liveLink = page.locator('a[href="/examples/interactive/#native-output"]').first();
    if (liveRuntime.resultCount !== 1 || (await liveLink.textContent())?.trim() !== 'Interactive CEM examples') {
        throw new Error(`live search contract drifted: ${JSON.stringify(liveRuntime)}`);
    }
    const browserContract = await page.evaluate(() => ({
        importMap: JSON.parse(document.querySelector('script[type="importmap"]')?.textContent ?? '{}'),
        fieldLightDom: Boolean(document.querySelector('[data-search-field] input[part~=control]')),
        fieldShadow: Boolean(document.querySelector('[data-search-field]')?.shadowRoot),
        actionLightDom: Boolean(document.querySelector('[data-search-action] > button')),
        actionShadow: Boolean(document.querySelector('[data-search-action]')?.shadowRoot),
        componentRuntime: globalThis.__cemSiteComponents,
        styleSheets: [...document.styleSheets].map(({ href }) => href).filter(Boolean),
    }));
    const expectedImports = [
        '@epa-wg/cem-elements',
        '@epa-wg/cem-site/components-runtime',
        '@epa-wg/cem-site/runtime',
        '@epa-wg/cem-site/search',
    ];
    if (
        JSON.stringify(Object.keys(browserContract.importMap.imports ?? {}).sort()) !== JSON.stringify(expectedImports)
    ) {
        throw new Error(`search import map drifted: ${JSON.stringify(browserContract.importMap)}`);
    }
    if (
        !browserContract.fieldLightDom ||
        browserContract.fieldShadow ||
        !browserContract.actionLightDom ||
        browserContract.actionShadow ||
        browserContract.componentRuntime?.errors?.length !== 0 ||
        browserContract.styleSheets.length !== 2 ||
        !browserContract.styleSheets.every((href) => href.includes('/search/assets/'))
    ) {
        throw new Error(`search CEM component contract drifted: ${JSON.stringify(browserContract)}`);
    }

    const field = page.locator('[data-search-field] input');
    const fieldOwner = await field.elementHandle();
    await page.evaluate(() => history.replaceState({ retained: true }, '', '?keep=yes&q=native%20output#site-search'));
    for (const query of ['CEM', '<img onerror="attack()">', '  Graph Semantics  ', '']) {
        await field.fill(query);
        await page.waitForFunction(query => globalThis.__cemSiteSearch?.query === query.trim(), query);
        const result = await page.evaluate(() => ({
            count: document.querySelectorAll('[data-search-document]').length,
            status: document.querySelector('[data-search-status]').textContent,
            runtime: globalThis.__cemSiteSearch, url: location.href, history: history.state,
            injected: !!document.querySelector('[data-search-status] img'),
        }));
        assert.deepEqual(result.runtime.errors, []);
        assert.equal(result.count, result.runtime.resultCount);
        assert.equal(result.status, query.trim() ? `${result.count} result${result.count === 1 ? '' : 's'} for “${query.trim()}”` : `${expectedDocumentCount} searchable documents`);
        if (query === 'CEM') assert.ok(result.count > 1);
        if (query.startsWith('<')) assert.equal(result.count, 0);
        if (!query) assert.equal(result.count, expectedDocumentCount);
        assert.equal(result.injected, false);
        const url = new URL(result.url);
        assert.equal(url.searchParams.get('keep'), 'yes');
        assert.equal(url.searchParams.get('q'), query.trim() || null);
        assert.equal(url.hash, '#site-search');
        assert.deepEqual(result.history, { retained: true });
        assert.ok(await fieldOwner.evaluate(node => node === document.activeElement));
        assert.equal(await field.inputValue(), query);
    }
    await field.fill('Graph Semantics');
    await page.waitForFunction(() => globalThis.__cemSiteSearch?.query === 'Graph Semantics');
    const marker = await page.evaluate(() => (globalThis.__siteSubmitProbe = crypto.randomUUID()));
    await field.press('Enter');
    await page.waitForFunction(() => globalThis.__cemSiteSearch?.query === 'Graph Semantics');
    assert.equal(await page.evaluate(() => globalThis.__siteSubmitProbe), marker, 'native form must invoke the host command without navigating');
    assert.equal(await page.locator('[data-search-results] > li > a').getAttribute('href'), deepLink);

    const staticPage = await browser.newPage({ javaScriptEnabled: false });
    await staticPage.goto(`http://127.0.0.1:${port}${searchPath}?q=Graph%20Semantics`);
    assert.equal(await staticPage.locator('[data-search-results] > li').count(), expectedDocumentCount);
    assert.equal(await staticPage.locator('[data-search-results] > li > a').count(), expectedDocumentCount);
    await staticPage.close();

    const themes = await verifySiteThemes(page);
    assert.deepEqual(browserErrors, []);

    const report = {
        themes,
        version: 1,
        xhtml,
        route: searchPath,
        documentCount: initialRuntime.documentCount,
        graphSemanticsResults: initialRuntime.resultCount,
        liveNativeOutputResults: liveRuntime.resultCount,
        deepLink,
        componentLightDom: { field: true, action: true },
        importSpecifiers: expectedImports,
        stylesheetCount: browserContract.styleSheets.length,
        runtimeErrors: [],
    };
    await mkdir(reportRoot, { recursive: true });
    const reportPath = join(reportRoot, 'search.json');
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log(`CEM Site search verification passed: ${relative(workspaceRoot, reportPath)}`);
} finally {
    await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.xhtml':
            return 'application/xhtml+xml; charset=utf-8';
        case '.html':
            return 'text/html; charset=utf-8';
        case '.css':
            return 'text/css; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.js':
            return 'text/javascript; charset=utf-8';
        case '.json':
            return 'application/json; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
