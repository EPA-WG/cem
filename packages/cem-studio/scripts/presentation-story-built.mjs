import assert from 'node:assert/strict';
import { readFile, stat, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { workspaceRoot } from './presenter-project.mjs';
import { loadStoryMap, verifyStoryMap } from './presentation-story.mjs';
import { loadPresenterResources } from './presentation-resources.mjs';

const data = await loadStoryMap(), result = await verifyStoryMap(data);
const root = resolve(workspaceRoot, 'dist/apps/cem-site');
const file = resolve(root, '.' + data.siteRoute, 'index.html');
const html = await readFile(file);
const server = createServer((request, response) => {
    if (request.url === data.siteRoute) { response.setHeader('Content-Type', 'text/html; charset=utf-8'); response.end(html); }
    else if (request.url === '/favicon.ico') { response.writeHead(204); response.end(); }
    else { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
try {
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage(); const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(origin + data.siteRoute);
    const observed = await page.evaluate(() => ({
        title: document.title,
        headings: [...document.querySelectorAll('main h2')].map(element => element.textContent.replace(/\s+/g, ' ').trim()),
        tables: document.querySelectorAll('main table').length,
        tableRows: [...document.querySelectorAll('main table tr')].map(row => row.textContent),
        code: [...document.querySelectorAll('main pre code')].map(element => element.textContent),
        hrefs: [...document.querySelectorAll('a[href]')].map(element => element.getAttribute('href')),
        scripts: document.querySelectorAll('script').length,
        mains: document.querySelectorAll('main').length,
        text: document.querySelector('main')?.textContent.replace(/\s+/g, ' ').trim(),
    }));
    assert.equal(observed.title, 'One document, reusable capabilities');
    assert.deepEqual(observed.headings, data.blocks.map(block => block.heading));
    assert.equal(observed.mains, 1); assert.equal(observed.scripts, 0);
    assert.equal(observed.tables, 1, 'Platform roles must be a real HTML table');
    assert.equal(observed.tableRows.length, 8);
    assert.ok(observed.code.some(text => text.includes('<h1 id="login-title">Sign in</h1>')));
    assert.ok(observed.code.some(text => text.includes('{h1 @id=login-title | Sign in}')));
    const resources = await loadPresenterResources();
    for (const resource of resources.publicResources) assert.ok(observed.hrefs.includes(resource.url), 'Missing pinned participant source');
    assert.ok(observed.hrefs.includes(resources.resourceHub));
    for (const href of observed.hrefs.filter(href => href.startsWith('/'))) {
        const destination = resolve(root, '.' + href.split('#')[0]);
        const info = await stat(destination); await stat(info.isDirectory() ? resolve(destination, 'index.html') : destination);
    }
    assert.match(observed.text, /Entered passwords\/OTP block recovery until explicitly cleared/);
    assert.match(observed.text, /automatic whole-Studio recovery remains disabled/);
    assert.deepEqual(errors, []);
    const report = { principles: result.principles, pitchSections: result.pitchSections, blocks: 6,
        plannedMinutes: 60, route: data.siteRoute, platformTableRows: observed.tableRows.length,
        codeExcerpts: observed.code.length, pinnedResourceLinks: 3, scripts: 0, pageErrors: 0,
        publication: data.publication, measuredRehearsal: false };
    const output = resolve(workspaceRoot, 'dist/reports/cem-studio/presentation-story.json');
    await mkdir(resolve(output, '..'), { recursive: true }); await writeFile(output, JSON.stringify(report, null, 2) + '\n');
    console.log('Verified built Amsterdam guide: six sections, native HTML table, source pair, participant links and privacy boundaries.');
} finally { if (browser) await browser.close(); await new Promise(resolve => server.close(resolve)); }
