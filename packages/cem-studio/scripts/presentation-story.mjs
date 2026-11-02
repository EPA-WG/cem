import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { workspaceRoot } from './presenter-project.mjs';

const docs = resolve(workspaceRoot, 'docs/declarative-amsterdam-2026');
export const loadStoryMap = async () => JSON.parse(await readFile(resolve(docs, 'story-map.json'), 'utf8'));
const headings = source => [...source.matchAll(/^## (.+)$/gm)].map(match => match[1]);
export async function verifyStoryMap(data) {
    data ??= await loadStoryMap();
    assert.equal(data.schemaVersion, 1);
    assert.equal(data.siteRoute, '/guides/declarative-amsterdam-2026/');
    assert.equal(data.siteSource, 'apps/cem-site/content/guides/declarative-amsterdam-2026.md');
    assert.equal(data.measuredRehearsal, false);
    assert.equal(data.publication, 'repository-build-only; public host not verified');
    const native = await readFile(resolve(docs, 'native-slide-order.md'), 'utf8');
    const ids = new Set([...native.matchAll(/^\| \d{2} \| `([^`]+)`/gm)].map(match => match[1]));
    const article = await readFile(resolve(workspaceRoot, data.siteSource), 'utf8');
    assert.deepEqual(data.blocks.map(block => block.minutes), [[0, 7], [7, 17], [17, 35], [35, 47], [47, 54], [54, 60]], 'Published intervals changed');
    assert.deepEqual(data.blocks.map(block => block.heading), headings(article), 'Website story order changed');
    for (const block of data.blocks) assert.ok(article.includes(`**${block.minutes[0]}–${block.minutes[1]} minutes.**`), 'Website interval missing');
    for (const source of ['cem-manifesto.md', 'cem-platform-pitch.md']) {
        const expected = headings(await readFile(resolve(docs, source), 'utf8'));
        assert.deepEqual(data.mappings.filter(row => row.source === source).map(row => row.heading), expected, 'Manifesto/pitch coverage incomplete');
    }
    for (const row of data.mappings) {
        assert.ok(row.slides.length && row.slides.every(id => ids.has(id)), 'Unknown native slide owner');
        assert.ok(headings(article).includes(row.websiteHeading), 'Missing website story section');
        assert.ok(['demonstrated', 'bounded', 'direction'].includes(row.status));
        assert.ok(row.message.length > 10 && row.boundary.length > 30, 'Missing audience message or evidence boundary');
        assert.ok(!row.evidence.includes('..') && !row.evidence.startsWith('/'));
        await readFile(resolve(docs, row.evidence), 'utf8');
        if (['Insulate every included scope', 'Make generated software reviewable', 'Compose vendors with security insulation'].includes(row.heading)) {
            assert.equal(row.status, 'direction', 'Do not promote an architectural target to a demonstrated security claim');
        }
    }
    const routes = JSON.parse(await readFile(resolve(workspaceRoot, 'apps/cem-site/site.routes.json'), 'utf8'));
    const route = routes.entries.find(row => row.route === data.siteRoute);
    assert.ok(route && route.owner === 'cem-site' && route.source === data.siteSource && route.upstreamTarget === null, 'Missing authored publication route');
    assert.ok(routes.searchDocuments.some(row => row.route === data.siteRoute), 'Missing search route');
    const graph = await readFile(resolve(workspaceRoot, 'apps/cem-site/site.cem'), 'utf8');
    assert.ok(graph.includes('@id=amsterdam-source') && graph.includes('@id=amsterdam ') && graph.includes('guides/declarative-amsterdam-2026/index.html'), 'Missing native publication branch');
    return { principles: 14, pitchSections: 7, blocks: 6, plannedMinutes: 60, route: data.siteRoute, article };
}
