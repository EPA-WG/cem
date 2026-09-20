import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { scopeCssText } from './projection.js';

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), 'utf8');
const source = read('../../demo/data-tree.html');
const view = read('../../demo/data-tree-view.cemt');
const request = read('../../demo/data-tree-request.cemt');

describe('retained document tree demo', () => {
    it('separates editing, format neutrality, repair and local request lessons', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), (match) => match[1])).toEqual([
            '1. XML branches: independent selection',
            '2. JSON through the same CEM tree',
            '3. Malformed source and repair',
            '4. Load and release a local document',
        ]);
        expect(source.match(/<template>\n<cem-/gu)).toHaveLength(4);
        expect(source).toContain('src="./data-tree-view.cemt"');
        expect(source).toContain('src="./data-tree-request.cemt"');
        expect(source).toContain('See also');
        expect(source).toContain('32 KiB');
    });

    it('uses only retained common CEM nodes after import and the typed inspection writer', () => {
        expect(view).toContain('{cem-data @name=document @select=source');
        expect(view).toContain('cemml:inspect(document)');
        expect(view).toContain('@select=node.children');
        expect(view).toContain('fn(attr) => attr.namespace != "http://www.w3.org/2000/xmlns/"');
        expect(view).not.toContain('cem:generic-data');
        expect(request).toContain('{http-request @slice=resource');
        expect(request).toContain('@with:document="{$datadom.slices.resource.data}"');
        expect(request).not.toContain('cem-data');
        for (const forbidden of ['JSON.parse', 'DOMParser', 'XSLTProcessor', 'onclick=', 'data-testid', 'response.json']) {
            expect(source + view + request).not.toContain(forbidden);
        }
    });

    it('ties branch selection to source replacement and keeps disclosure separate', () => {
        expect(view).toContain('datadom.eventPayloads.source.revision');
        expect(view).toContain('entry.value == revision');
        expect(view).toContain('{cem:if @test=selected | {attribute @name=checked}}');
        expect(view).toContain('{details @open=open');
        expect(view).toContain('@aria-label="Selected branches"');
        expect(view).toContain('{strong | Selected}');
        expect(view).toContain('Reload original');
        expect(request).toContain('datadom.slices.resource.resourceRevision');
        expect(source).toContain('Collapsing preserves selection.');
    });

    it('keeps static presentation scoped and long inspection text contained', () => {
        const css = view.match(/\{style \|```([\s\S]*?)```\}/u)?.[1] ?? '';
        expect(css).toContain('overflow-wrap: anywhere');
        expect(css).toContain('overflow: auto');
        expect(scopeCssText(css, 'tree').diagnostics).toEqual([]);
        for (const page of ['../../index.html', '../../demo/data-table.html', '../../demo/external-template.html', '../../demo/http-request.html']) {
            expect(read(page)).toContain('data-tree.html');
        }
    });
});
