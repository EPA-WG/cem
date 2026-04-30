/**
 * Headless Playwright inspector for CEM generators and token specs.
 *
 * Usage:
 *   node tools/scripts/debug-cem.mjs [path-to-file]
 *
 * Examples:
 *   node tools/scripts/debug-cem.mjs packages/cem-theme/src/lib/css-generators/cem-colors.html
 *   node tools/scripts/debug-cem.mjs packages/cem-theme/dist/lib/tokens/cem-colors.xhtml
 *
 * Edit the page.evaluate() block below to inspect whatever you need.
 * Browser console and page errors are forwarded to stderr automatically.
 */
import fs from "node:fs/promises";
import path from "node:path";
import http from "node:http";

const MIME_TYPES = {
    '.html':  'text/html',
    '.xhtml': 'application/xhtml+xml',
    '.js':    'application/javascript',
    '.mjs':   'application/javascript',
    '.css':   'text/css',
    '.json':  'application/json',
    '.xml':   'application/xml',
    '.svg':   'image/svg+xml',
    '.png':   'image/png',
    '.jpg':   'image/jpeg',
    '.gif':   'image/gif',
};

const docRoot = path.parse(process.cwd()).root;
const server = http.createServer(async (req, res) => {
    const filePath = path.join(docRoot, decodeURIComponent(req.url.split('?')[0]));
    try {
        const data = await fs.readFile(filePath);
        const ext = path.extname(filePath).toLowerCase();
        res.writeHead(200, { 'Content-Type': MIME_TYPES[ext] || 'application/octet-stream' });
        res.end(data);
    } catch {
        res.writeHead(404);
        res.end('Not found');
    }
});

server.listen(0, '127.0.0.1', async () => {
    const { chromium } = await import("playwright");
    const port = server.address().port;
    const file = process.argv[2] || "packages/cem-theme/src/lib/css-generators/cem-colors.html";
    const url = `http://127.0.0.1:${port}/${path.relative(docRoot, path.resolve(file))}`;

    const browser = await chromium.launch({ headless: true });
    const page = await (await browser.newContext({ javaScriptEnabled: true, bypassCSP: true })).newPage();
    page.on('console', msg => console.error(`[browser ${msg.type()}]`, msg.text()));
    page.on('pageerror', err => console.error(`[page error]`, err.message));

    await page.goto(url, { waitUntil: "networkidle" });
    await page.waitForTimeout(2000);

    // ── EDIT THIS BLOCK ──────────────────────────────────────────────────────
    const result = await page.evaluate(() => {
        const root = document.documentElement;
        return {
            // CSS variable on :root
            comfort: getComputedStyle(root).getPropertyValue('--cem-palette-comfort') || '(empty)',
            // injected stylesheets
            styles: Array.from(document.head.querySelectorAll('style')).map(s => ({
                id: s.id, bytes: s.textContent.length,
            })),
        };
    });
    // ─────────────────────────────────────────────────────────────────────────

    console.log(JSON.stringify(result, null, 2));
    await browser.close();
    server.close();
});
