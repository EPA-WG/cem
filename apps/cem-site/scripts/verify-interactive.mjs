import { createReadStream } from 'node:fs';
import { mkdir, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join, normalize, relative, resolve, sep } from 'node:path';
import { chromium } from 'playwright';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const fixturePath = '/examples/interactive/';

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

    await page.goto(`http://127.0.0.1:${port}${fixturePath}`, {
        waitUntil: 'networkidle',
        timeout: 120_000,
    });
    await page.waitForFunction(() => globalThis.__cemSiteInteractive?.done === true, null, {
        timeout: 120_000,
    });

    const runtime = await page.evaluate(() => globalThis.__cemSiteInteractive);
    const runtimeErrors = [...browserErrors, ...(runtime.errors ?? [])];
    if (runtimeErrors.length > 0) {
        throw new Error(
            `interactive fixture runtime failed:\n${runtimeErrors.map((error) => `- ${error}`).join('\n')}`,
        );
    }

    await page.locator('[data-token-filter]').fill('comfort');
    const visibleTokens = await page.locator('[data-token-example]:visible').count();
    const tokenStatus = await page.locator('[data-token-status]').textContent();
    if (visibleTokens !== 1 || tokenStatus?.trim() !== '1 token example') {
        throw new Error(`token filtering returned ${visibleTokens} rows and status ${tokenStatus}`);
    }

    await page.locator('cem-action button').click();
    await page.locator('cem-action button').click();
    const actionCount = await page.locator('[data-action-count]').textContent();
    if (actionCount?.trim() !== 'Action count: 2') {
        throw new Error(`production action did not retain native click interaction: ${actionCount}`);
    }

    const contract = await page.evaluate(() => {
        const action = document.querySelector('cem-action');
        const field = document.querySelector('cem-field');
        const fixture = document.querySelector('[data-cem-fixture-instance]');
        const nativeOutput = document.querySelector('[data-native-output]')?.textContent ?? '';
        const importMap = JSON.parse(document.querySelector('script[type="importmap"]')?.textContent ?? '{}');
        return {
            actionLightDom: Boolean(action?.querySelector(':scope > template[data-cem-island]')),
            actionShadow: Boolean(action?.shadowRoot),
            fieldLightDom: Boolean(field?.querySelector('input')),
            fieldShadow: Boolean(field?.shadowRoot),
            fixtureLightDom: Boolean(fixture?.querySelector('.cem-site-greeting')),
            fixtureShadow: Boolean(fixture?.shadowRoot),
            greeting: fixture?.textContent?.trim(),
            nativeOutput,
            importMap,
            styleSheets: [...document.styleSheets].map(({ href }) => href).filter(Boolean),
        };
    });

    if (
        !contract.actionLightDom ||
        contract.actionShadow ||
        !contract.fieldLightDom ||
        contract.fieldShadow ||
        !contract.fixtureLightDom ||
        contract.fixtureShadow ||
        contract.greeting !== 'Hello, Ada!' ||
        !contract.nativeOutput.includes('class="cem-site-greeting"') ||
        !contract.nativeOutput.includes('Hello, Ada!')
    ) {
        throw new Error(`interactive light-DOM contract drifted: ${JSON.stringify(contract)}`);
    }

    const expectedImports = ['@epa-wg/cem-components/primitives', '@epa-wg/cem-site/runtime', '@epa-wg/custom-element'];
    if (JSON.stringify(Object.keys(contract.importMap.imports ?? {}).sort()) !== JSON.stringify(expectedImports)) {
        throw new Error('the browser import map does not expose the exact interactive entrypoints');
    }
    if (
        contract.styleSheets.length !== 2 ||
        !contract.styleSheets.every((href) => href.includes('/examples/interactive/assets/'))
    ) {
        throw new Error(`interactive stylesheets drifted: ${contract.styleSheets.join(', ')}`);
    }

    const report = {
        version: 1,
        route: fixturePath,
        tokenFilterVisibleCount: visibleTokens,
        actionCount: 2,
        lightDom: {
            action: contract.actionLightDom,
            field: contract.fieldLightDom,
            cemFixture: contract.fixtureLightDom,
        },
        importSpecifiers: expectedImports,
        stylesheetCount: contract.styleSheets.length,
        runtimeErrors: [],
    };
    await mkdir(reportRoot, { recursive: true });
    const reportPath = join(reportRoot, 'interactive.json');
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log(`CEM Site interactive verification passed: ${relative(workspaceRoot, reportPath)}`);
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
        case '.css':
            return 'text/css; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.json':
            return 'application/json; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
