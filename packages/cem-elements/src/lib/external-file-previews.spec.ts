import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';

const pages: Record<string, readonly string[]> = {
    '../../demo/cell-overrides.html': ['pokemon-cells.json', 'stock-cells.xml', 'stock-cell.cemt'],
    '../../demo/http-request.html': ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json'],
    '../../demo/for-each.html': ['http-data.json', 'http-data.xml'],
    '../../demo/data-tree.html': ['tree-source.xml', 'tree-source.json'],
    '../../demo/npm-versions-demo.html': ['npm-versions.json'],
    '../../demo/external-template.html': ['embed-1.html', 'embed-lib.html'],
    '../../../custom-element/demo/http-request.html': ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json'],
    '../../../custom-element/demo/npm-versions-demo.html': ['npm-versions.json'],
    '../../../cem-demo-element/demo/index.html': ['dwarfs.json'],
};

it.each(Object.entries(pages))('%s previews external files directly', (path, files) => {
    const pageUrl = new URL(path, import.meta.url);
    const source = readFileSync(pageUrl, 'utf8');
    const previews = Array.from(source.matchAll(/<cem-demo-element\b[^>]*\bsrc="([^"]+)"[^>]*>/gu));
    expect(previews.map(match => match[1])).toEqual(files.map(file => `./${file}`));
    for (const [markup, src] of previews) {
        const type = src.endsWith('.cemt') ? 'cem-ml' : src.split('.').pop();
        expect(markup).toMatch(new RegExp(`\\btype=(?:"${type}"|${type})(?:\\s|>)`, 'u'));
        expect(markup).toMatch(/\bdemo=(?:"false"|false)(?:\s|>)/u);
        expect(markup).toMatch(/\blegend="[^"]+"/u);
        expect(markup).toMatch(/\bdescription="[^"]+"/u);
        expect(readFileSync(new URL(src, pageUrl), 'utf8').length).toBeGreaterThan(0);
    }
});
