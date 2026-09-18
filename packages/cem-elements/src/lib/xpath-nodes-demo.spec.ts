import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const source = readFileSync(new URL('../../demo/xpath-nodes.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-nodes.cemt', import.meta.url), 'utf8');

describe('native XPath node demos', () => {
    it('keeps table and tree cases self-contained and declarative', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. XML table with native navigation', '2. XML tree with attributes and mixed text',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(2);
        expect(source.match(/xpath-functions="\.\/xpath-nodes.cemt"/gu)).toHaveLength(2);
        expect(source.match(/\{cem-data[^}]*@projection=xpath/gu)).toHaveLength(2);
        expect(source.match(/document.error != ""/gu)).toHaveLength(2);
        expect(source).toContain('seq:sorted(rows, fn(row)');
        expect(source).toContain('apply-templates @mode=tree');
        expect(source).not.toContain('tag=');
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('{function');
        for (const expression of ['local-name()', 'namespace-uri()', 'data(@qty)', 'string(.)', 'preceding-sibling::*[1]', 'parent::node()']) {
            expect(library).toContain(expression);
        }
    });
    it('explains source navigation, scope, limits and related viewers', () => {
        for (const text of ['previous sibling', 'CDATA', '32 KiB', '64 levels', '4096 events', 'HTTP XML', 'fn:sort', './xpath-functions.html', './data-table.html']) {
            expect(source).toContain(text);
        }
    });
});
