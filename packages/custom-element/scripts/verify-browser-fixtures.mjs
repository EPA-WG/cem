import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = dirname(dirname(projectRoot));
const fixturePaths = [
    '/packages/custom-element/test-fixtures/browser-smoke.html',
    '/packages/custom-element/test-fixtures/browser-smoke-dist.html',
];

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(
            requestUrl.pathname === '/' ? '/packages/custom-element/test-fixtures/browser-smoke.html' : requestUrl.pathname
        );
        const filePath = normalize(join(workspaceRoot, pathname));
        if (!filePath.startsWith(workspaceRoot + sep)) {
            response.writeHead(403);
            response.end('Forbidden');
            return;
        }
        const fileStat = await stat(filePath);
        if (!fileStat.isFile()) {
            response.writeHead(404);
            response.end('Not found');
            return;
        }
        response.writeHead(200, { 'content-type': contentType(filePath) });
        createReadStream(filePath).pipe(response);
    } catch {
        response.writeHead(404);
        response.end('Not found');
    }
});

await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));

const address = server.address();
const port = typeof address === 'object' && address ? address.port : 0;
const browser = await chromium.launch({ headless: true });

try {
    for (const fixturePath of fixturePaths) {
        const pageErrors = [];
        const page = await browser.newPage();
        page.on('pageerror', (error) => pageErrors.push(error.message));
        page.on('console', (message) => {
            if (message.type() === 'error') {
                pageErrors.push(message.text());
            }
        });

        await page.goto(`http://127.0.0.1:${port}${fixturePath}`);
        await page.waitForFunction(() => globalThis.__customElementFixture?.done === true);
        const result = await page.evaluate(() => globalThis.__customElementFixture);
        const errors = [...pageErrors, ...(result.errors ?? [])];
        await page.close();
        if (errors.length > 0) {
            throw new Error(`${fixturePath} failed:\n${errors.map((error) => `- ${error}`).join('\n')}`);
        }
    }
} finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.html':
            return 'text/html; charset=utf-8';
        case '.js':
            return 'text/javascript; charset=utf-8';
        case '.json':
            return 'application/json; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.css':
            return 'text/css; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
