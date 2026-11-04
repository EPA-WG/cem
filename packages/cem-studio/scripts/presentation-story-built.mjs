import assert from 'node:assert/strict';
import { readFile, stat, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { chromium } from 'playwright';
import { workspaceRoot } from './presenter-project.mjs';
import { loadStoryMap, verifyStoryMap } from './presentation-story.mjs';
import { loadPresenterResources } from './presentation-resources.mjs';

const data = await loadStoryMap(), result = await verifyStoryMap(data);
const root = resolve(workspaceRoot, 'dist/hosting/cem-site');
const file = resolve(root, '.' + data.siteRoute, 'index.html');
await stat(file);
const mime = { '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript',
    '.mjs': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.wasm': 'application/wasm' };
async function publicationFile(pathname) {
    let target = resolve(root, '.' + pathname);
    assert.ok(target === root || target.startsWith(root + sep), 'Outside prepared publication');
    let info = await stat(target).catch(error => {
        if (error.code !== 'ENOENT') throw error;
    });
    if (!info && !extname(target)) { target += '.html'; info = await stat(target); }
    if (info?.isDirectory()) target = resolve(target, 'index.html');
    await stat(target);
    return target;
}
const server = createServer(async (request, response) => {
    if (request.url === '/favicon.ico') { response.writeHead(204); response.end(); return; }
    try {
        const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
        const target = await publicationFile(pathname);
        const bytes = await readFile(target);
        response.setHeader('Content-Type', mime[extname(target)] ?? 'application/octet-stream');
        response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
try {
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage(); const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(origin + data.siteRoute);
    await page.waitForFunction(() => [...document.querySelectorAll('cem-demo-element[demo="false"][source]')]
        .length === 2 && [...document.querySelectorAll('cem-demo-element[demo="false"][source]')]
        .every(node => node.getAttribute('data-state') === 'ready'));
    const observed = await page.evaluate(() => ({
        title: document.title,
        headings: [...document.querySelectorAll('main h2')].map(element => element.textContent.replace(/\s+/g, ' ').trim()),
        headingIds: [...document.querySelectorAll('main h1, main h2')].map(element => element.id),
        tables: document.querySelectorAll('main table').length,
        tableRows: [...document.querySelectorAll('main table tr')].map(row => row.textContent),
        code: [...document.querySelectorAll('main cem-demo-element[demo="false"][source]')].map(element => ({
            source: element.getAttribute('source'), text: element.querySelector('[slot=text] code')?.textContent,
            colored: Boolean(element.querySelector('[slot=text] code strong, [slot=text] code i, [slot=text] code var, [slot=text] code b, [slot=text] code dfn')),
            renderedChildren: element.querySelector('[slot=demo]')?.childNodes.length,
        })),
        hrefs: [...document.querySelectorAll('a[href]')].map(element => element.getAttribute('href')),
        scripts: [...document.querySelectorAll('script')].map(element => ({ type: element.type,
            src: element.getAttribute('src'), text: element.textContent })),
        mains: document.querySelectorAll('main').length,
        text: document.querySelector('main')?.textContent.replace(/\s+/g, ' ').trim(),
    }));
    assert.equal(observed.title, 'One document, reusable capabilities');
    assert.deepEqual(observed.headings, data.blocks.map(block => block.heading));
    const manifest = JSON.parse(await readFile(resolve(workspaceRoot, 'apps/cem-site/site.routes.json'), 'utf8'));
    assert.deepEqual(observed.headingIds, manifest.searchDocuments.find(row => row.route === data.siteRoute).headings.map(heading => heading.id), 'Search fragments differ from native headings');
    assert.equal(observed.mains, 1);
    assert.deepEqual(observed.scripts.map(({ type, src }) => ({ type, src })), [
        { type: 'importmap', src: null }, { type: 'module', src: '/packages/cem-demo-element/dist/index.js' },
    ]);
    assert.deepEqual(JSON.parse(observed.scripts[0].text), { imports: {
        '@epa-wg/cem-ml/wasm': '/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js',
    } });
    for (const snippet of observed.code) {
        assert.equal(snippet.source, snippet.source.trim());
        assert.equal(snippet.text, snippet.source);
        assert.equal(snippet.colored, true);
        assert.equal(snippet.renderedChildren, 0);
    }
    assert.equal(observed.tables, 1, 'Platform roles must be a real HTML table');
    assert.equal(observed.tableRows.length, 8);
    assert.ok(observed.code.some(({ text }) => text.includes('<h1 id="login-title">Sign in</h1>')));
    assert.ok(observed.code.some(({ text }) => text.includes('{h1 @id=login-title | Sign in}')));
    const resources = await loadPresenterResources();
    for (const resource of resources.publicResources) assert.ok(observed.hrefs.includes(resource.url), 'Missing pinned participant source');
    assert.ok(observed.hrefs.includes(resources.resourceHub));
    for (const href of observed.hrefs.filter(href => href.startsWith('/'))) {
        await publicationFile(href.split(/[?#]/)[0]);
    }
    assert.match(observed.text, /Entered passwords\/OTP block recovery until explicitly cleared/);
    assert.match(observed.text, /automatic whole-Studio recovery remains disabled/);
    assert.deepEqual(errors, []);
    const report = { principles: result.principles, pitchSections: result.pitchSections, blocks: 6,
        plannedMinutes: 60, route: data.siteRoute, platformTableRows: observed.tableRows.length,
        codeExcerpts: observed.code.length, pinnedResourceLinks: 3, scripts: observed.scripts.length, sourceOnlyViewers: observed.code.length, pageErrors: 0,
        publication: data.publication, measuredRehearsal: false };
    const output = resolve(workspaceRoot, 'dist/reports/cem-studio/presentation-story.json');
    await mkdir(resolve(output, '..'), { recursive: true }); await writeFile(output, JSON.stringify(report, null, 2) + '\n');
    console.log('Verified built Amsterdam guide: six sections, native HTML table, source pair, participant links and privacy boundaries.');
} finally { if (browser) await browser.close(); await new Promise(resolve => server.close(resolve)); }
