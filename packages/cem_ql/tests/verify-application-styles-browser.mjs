import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const server = await createServer({
    configFile: false,
    publicDir: false,
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null },
});
let browser;
try {
    await server.listen();
    const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
    browser = await chromium.launch({ executablePath: process.env.CEM_CHROME });
    for (const mode of ['fallback', 'worker']) {
        const page = await browser.newPage();
        const external = [];
        await page.route('**/*', route => {
            const url = route.request().url();
            if (url === `${origin}/fixture`) return route.fulfill({ contentType: 'text/html', body: '<!doctype html><body></body>' });
            if (new URL(url).origin === origin) return route.continue();
            external.push(url);
            return route.abort();
        });
        try {
            await page.goto(`${origin}/fixture`);
            await page.evaluate(async mode => {
                const fixture = await import('/packages/cem_ql/tests/application-styles-browser.mjs');
                await fixture.verifyApplicationStylesheets(mode);
            }, mode);
            assert.deepEqual(external, []);
            console.log(`Application styles (${mode}): real native CSS, sharing, rejected and accepted replacement, and cleanup passed.`);
        } finally { await page.close(); }
    }
} finally {
    await browser?.close();
    await server.close();
}
