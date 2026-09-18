import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const source = readFileSync(new URL('../../demo/xpath-sort.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-sort.cemt', import.meta.url), 'utf8');

describe('XPath sorting demos', () => {
    it('separates typed word keys from retained node selection', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. Text and numeric keys', '2. Multiple keys and source selection',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(2);
        expect(source.match(/xpath-functions="\.\/xpath-sort.cemt"/gu)).toHaveLength(2);
        for (const text of ['function($word)', 'function($row)', 'castable as xs:decimal', 'not($valid)', 'reverse(sort(reverse($words)))', 'preceding-sibling::row[1]']) {
            expect(library).toContain(text);
        }
        expect(source).toContain('@projection=xpath');
        expect(source).toContain('document.error != ""');
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('seq:sorted');
        expect(source).not.toContain('{function');
    });
    it('explains ties, authored invalid-last keys, ownership and supported collations', () => {
        for (const text of ['Equal keys', 'empty keys first', 'validity', 'previous source sibling', 'codepoint collation', './xpath-nodes.html', './xpath-sequences.html']) {
            expect(source).toContain(text);
        }
    });
});
