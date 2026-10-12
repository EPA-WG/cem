// Generate a reviewable code image without Studio or hand-authored token colors.
// Run from the repository: node docs/declarative-amsterdam-2026/generate-code-preview.mjs
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = path.join(root, 'dist/declarative-amsterdam-2026/code-preview');
const source = 'examples/cem-ml/login.cem';
const cli = 'dist/target/cem_ml_cli/debug/cem-ml';
await mkdir(output, { recursive: true });
const sourceBytes = await readFile(path.join(root, source));
const args = [
    'convert', source,
    '--content-type', 'application/cem', '--schema', 'https://cem.dev/ns/cem-ml/1',
    '--to-content-type', 'application/cem', '--to-schema', 'https://cem.dev/ns/cem-ml/1',
    '--cemt-formatter-profile', 'tabular', '--output-color-type', 'none',
    '--out', path.join(output, 'login.tabular.cem'),
    '--report-json', path.join(output, 'cli-report.json'),
];
const formatted = spawnSync(path.join(root, cli), args, { cwd: root, encoding: 'utf8' });
await writeFile(path.join(output, 'cli-diagnostics.txt'), formatted.stderr || '');
if (formatted.status !== 0) throw new Error(formatted.stderr || 'Native formatting failed');
const plain = await readFile(path.join(output, 'login.tabular.cem'), 'utf8');
const theme = await readFile(path.join(root, 'packages/cem-theme/dist/lib/css/cem-combined.css'), 'utf8');

// Reuse the source-only mode, semantic HTML colorer and palette of syntax-coloring.html.
const layout = `
* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; }
body { padding: 58px 64px; font-family: system-ui, sans-serif; }
header { margin-bottom: 36px; }
.eyebrow { margin: 0 0 12px; font-size: 16px; letter-spacing: .14em; text-transform: uppercase; }
h1 { margin: 0; font-size: 42px; font-weight: 650; letter-spacing: -.03em; }
.caption { margin: 12px 0 0; font-size: 20px; }
main cem-demo-element { border-style: solid; border-radius: 16px; }
main cem-demo-element > [slot="text"] pre { padding: 32px; overflow: visible; }
main cem-demo-element > [slot="text"] code { font-size: 23px; line-height: 1.7; }
main cem-demo-element > [slot="demo"], main cem-demo-element > [slot="status"] { display: none; }
footer { display: flex; justify-content: space-between; gap: 28px; margin-top: 26px; font-size: 16px; }
footer a { color: inherit; }
`;
const pageSource = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>CEM-ML login — formatted source preview</title>
<link rel="icon" href="data:,">
<style>${theme}</style><style data-preview-layout>${layout}</style>
<script type="importmap">{"imports":{"@epa-wg/cem-ml/wasm":"/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js"}}</script>
<script type="module" src="/packages/cem-demo-element/dist/index.js"></script>
</head><body data-theme="cem-theme-light">
<header><p class="eyebrow">Declarative Amsterdam 2026 · source preview</p>
<h1>One login document in CEM-ML</h1>
<p class="caption">Native tabular formatting · CEM syntax colors</p></header>
<main><cem-demo-element type="cem-ml" demo="false" src="./login.tabular.cem"></cem-demo-element></main>
<footer><span>${source}</span><a href="https://github.com/EPA-WG/cem/discussions/24">Resources and updates ↗</a></footer>
</body></html>`;
await writeFile(path.join(output, 'login-preview-live.html'), pageSource);

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.cem': 'text/plain' };
const server = http.createServer(async (req, res) => {
    try {
        const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
        const file = path.resolve(root, '.' + pathname);
        if (!file.startsWith(root)) { res.writeHead(403); res.end(); return; }
        const body = await readFile(file);
        res.writeHead(200, { 'Content-Type': types[path.extname(file)] || 'application/octet-stream' });
        res.end(body);
    } catch { res.writeHead(404); res.end('Not found'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
try {
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1, colorScheme: 'light' });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`${origin}/dist/declarative-amsterdam-2026/code-preview/login-preview-live.html`);
    await page.waitForSelector('cem-demo-element code.cem-source-code > b');
    await page.evaluate(() => document.fonts.ready);
    const evidence = await page.evaluate(async expected => {
        // Check the actual CEM WASM response, not the component's fallback highlighter.
        const { highlightCemSource } = await import('/packages/cem-demo-element/dist/cem-ml-runtime.js');
        const highlighted = await highlightCemSource(expected, 'application/cem');
        const normalized = document.createElement('template');
        normalized.innerHTML = highlighted.html;
        const code = document.querySelector('code.cem-source-code');
        const pre = code.parentElement;
        const token = tag => {
            const element = code.querySelector(tag);
            return element ? { sample: element.textContent, color: getComputedStyle(element).color } : null;
        };
        return {
            highlighterStatus: highlighted.status,
            highlighterDiagnostics: highlighted.diagnostics,
            sourceMatches: code.textContent === expected,
            nativeHtmlMatches: code.innerHTML === normalized.innerHTML,
            tokenCount: code.children.length,
            colors: { name: token('b'), attribute: token('var'), keyword: token('strong'), string: token('i'), text: token('samp') },
            horizontalOverflow: pre.scrollWidth > pre.clientWidth,
            pageOverflow: document.documentElement.scrollHeight > innerHeight,
            liveFormCount: document.querySelectorAll('form,input,button').length,
        };
    }, plain);
    if (!evidence.sourceMatches || !evidence.nativeHtmlMatches || evidence.highlighterStatus !== 'highlighted') {
        throw new Error(`Source/highlighter mismatch: ${JSON.stringify(evidence)}`);
    }
    if (errors.length || evidence.horizontalOverflow || evidence.pageOverflow || evidence.liveFormCount) {
        throw new Error(`Preview rendering issue: ${JSON.stringify({ evidence, errors })}`);
    }
    const colors = Object.values(evidence.colors).filter(Boolean).map(token => token.color);
    if (new Set(colors).size < 3) throw new Error('Syntax colors did not resolve');

    // Freeze the already colored DOM and its original theme styles for an offline artifact.
    const snapshot = await page.evaluate(() => {
        const html = document.documentElement.cloneNode(true);
        html.querySelectorAll('script').forEach(node => node.remove());
        html.querySelector('cem-demo-element').removeAttribute('src');
        html.querySelector('cem-demo-element').removeAttribute('type');
        return '<!doctype html>\n' + html.outerHTML;
    });
    await writeFile(path.join(output, 'login-preview.html'), snapshot);
    await page.goto(`${origin}/dist/declarative-amsterdam-2026/code-preview/login-preview.html`);
    await page.evaluate(() => document.fonts.ready);
    const savedText = await page.locator('code.cem-source-code').textContent();
    if (savedText !== plain) throw new Error('Offline snapshot changed source');
    await page.screenshot({ path: path.join(output, 'login-preview.png') });
    await page.locator('cem-demo-element').screenshot({ path: path.join(output, 'login-code.png') });
    await writeFile(path.join(output, 'preview-evidence.json'), JSON.stringify({
        source, sourceSha256: createHash('sha256').update(sourceBytes).digest('hex'),
        cli, args, browserVersion: browser.version(), ...evidence, errors,
        note: 'Native CLI formats the source; existing CEM WASM HTML colorer supplies semantic markup. Direct CLI HTML output currently emits plain CEM.',
    }, null, 2) + '\n');
    console.log(JSON.stringify({ html: path.join(output, 'login-preview.html'), image: path.join(output, 'login-preview.png'), ...evidence }, null, 2));
} finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
}
