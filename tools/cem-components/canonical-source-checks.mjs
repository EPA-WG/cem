import assert from 'node:assert/strict';

export const componentRoot = '@epa-wg/cem-components/components/';

export function canonicalSourcesInPage(source, imports, page) {
    const sources = new Map();
    for (const match of source.matchAll(/\{cem-module-url\s+[^{}]*?@src\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s{}]+))/gu)) {
        const specifier = match[1] ?? match[2] ?? match[3];
        if (!specifier.startsWith(componentRoot)) continue;
        const relative = specifier.slice(componentRoot.length);
        const declaration = /^(cem-[a-z0-9-]+)\/\1\.xhtml$/u.exec(relative);
        assert.ok(declaration, `${page}: canonical source must name its XHTML document: ${specifier}`);
        assert.ok(imports?.[componentRoot]?.endsWith('/'), `${page}: missing component folder mapping`);
        sources.set(specifier, { tag: declaration[1], path: imports[componentRoot] + relative });
    }
    return [...sources.values()];
}

export async function verifyCanonicalSources(page, sources, expectedSource) {
    for (const { tag, path } of sources) {
        const url = new URL(path, page.url()).href;
        const viewer = page.locator(`cem-demo-element[src="${url}"]`);
        await viewer.first().waitFor({ state: 'attached' });
        await page.waitForFunction(url => {
            const viewers = [...document.querySelectorAll('cem-demo-element[src]')].filter(node => node.getAttribute('src') === url);
            return viewers.length > 0 && viewers.every(node => node.dataset.state === 'ready');
        }, url);
        const response = await page.request.get(url);
        assert.equal(response.status(), 200, `${page.url()}: canonical source response`);
        const source = await response.text();
        if (expectedSource !== undefined) assert.equal(source, expectedSource, `${url}: workspace declaration bytes`);
        const displays = await viewer.evaluateAll((viewers, { source, tag }) => {
            const xhtml = 'http://www.w3.org/1999/xhtml';
            const document = new DOMParser().parseFromString(source, 'application/xhtml+xml');
            const declaration = [...document.getElementsByTagNameNS(xhtml, 'cem-element')]
                .find(node => node.getAttribute('tag') === tag);
            const template = Array.from(declaration?.children ?? []).find(node =>
                node.namespaceURI === xhtml && node.localName === 'template' && node.getAttribute('id') === tag);
            return !document.querySelector('parsererror') && template?.getAttribute('type') === 'text/cem-ml'
                && viewers.every(node => node.source === source && node.getAttribute('demo') === 'false');
        }, { source, tag });
        assert.ok(displays, `${page.url()}: source-only viewer must display the canonical ${tag} declaration`);
        assert.ok((await page.locator('a[href]').evaluateAll(links => links.map(node => node.href))).includes(url),
            `${page.url()}: canonical source link`);
    }
}
