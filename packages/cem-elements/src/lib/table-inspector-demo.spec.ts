import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), 'utf8');
const page = read('../../demo/table-inspector.html');
const viewer = read('../../demo/data-table-view.cemt');

describe('table inspector teaching cases', () => {
    it('isolates heterogeneous columns, text, nesting and multiple selection', () => {
        expect(Array.from(page.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. Columns from every row',
            '2. Text-only rows stay visible',
            '3. Nested tables keep their own state',
            '4. Multiple selections survive sorting',
        ]);
        expect(page.match(/<template>\n<cem-table-inspector/gu)).toHaveLength(4);
        expect(page.match(/format="xml" inspector="true"/gu)).toHaveLength(4);
        expect(page).toContain('src="./data-table-view.cemt"');
    });

    it('documents source identity, independent controls and bounded native import', () => {
        for (const lesson of ['native source keys', 'equal', 'source order', '32 KiB', 'identical text', 'single-selection']) {
            expect(page).toContain(lesson);
        }
        expect(viewer).toContain('data:node_key(record.source)');
        expect(viewer).toContain('datadom.eventPayloads.source.revision');
        expect(viewer).toContain('{attribute @name=aria-sort');
        expect(viewer).toContain('{caption |');
        for (const forbidden of ['JSON.parse', 'DOMParser', 'XSLTProcessor', 'onclick=', 'data-testid']) {
            expect(page + viewer).not.toContain(forbidden);
        }
    });

    it('links both directions from the index and related viewers', () => {
        expect(page).toContain('See also');
        for (const related of ['data-table.html', 'data-tree.html']) {
            expect(page).toContain(`href="./${related}"`);
            expect(read(`../../demo/${related}`)).toContain('table-inspector.html');
        }
        expect(read('../../index.html')).toContain('table-inspector.html');
    });
});
