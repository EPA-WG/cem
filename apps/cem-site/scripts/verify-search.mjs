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
    await page.waitForFunction(() => globalThis.__cemSiteSearch?.done === true);
    const initialRuntime = await page.evaluate(() => globalThis.__cemSiteSearch);
    if (browserErrors.length > 0 || initialRuntime.errors.length > 0) {
        throw new Error(`search runtime failed: ${[...browserErrors, ...initialRuntime.errors].join('; ')}`);
    }
    if (initialRuntime.documentCount !== expectedDocumentCount || initialRuntime.resultCount !== 1) {
        throw new Error(`search query contract drifted: ${JSON.stringify(initialRuntime)}`);
    }

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
    const liveRuntime = await page.evaluate(() => globalThis.__cemSiteSearch);
    const liveLink = page.locator('a[href="/examples/interactive/#native-output"]').first();
    if (liveRuntime.resultCount !== 1 || (await liveLink.textContent())?.trim() !== 'Interactive CEM examples') {
        throw new Error(`live search contract drifted: ${JSON.stringify(liveRuntime)}`);
    }
    const browserContract = await page.evaluate(() => ({
        importMap: JSON.parse(document.querySelector('script[type="importmap"]')?.textContent ?? '{}'),
        fieldLightDom: Boolean(document.querySelector('[data-search-field] > div > label > input')),
        fieldShadow: Boolean(document.querySelector('[data-search-field]')?.shadowRoot),
        actionLightDom: Boolean(document.querySelector('[data-search-action] > button')),
        actionShadow: Boolean(document.querySelector('[data-search-action]')?.shadowRoot),
        componentRuntime: globalThis.__cemSiteComponents,
        styleSheets: [...document.styleSheets].map(({ href }) => href).filter(Boolean),
    }));
    const expectedImports = [
        '@epa-wg/cem-components/primitives',
        '@epa-wg/cem-site/components-runtime',
        '@epa-wg/cem-site/runtime',
        '@epa-wg/cem-site/search',
        '@epa-wg/custom-element',
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

    const report = {
        version: 1,
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
        case '.html':
            return 'text/html; charset=utf-8';
        case '.js':
            return 'text/javascript; charset=utf-8';
        case '.json':
            return 'application/json; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
