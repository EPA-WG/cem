import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const source = readFileSync(new URL('../../demo/xpath-sequences.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-sequences.cemt', import.meta.url), 'utf8');

describe('XPath sequence demos', () => {
    it('keeps windowing and column discovery in separate declarative examples', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. A window into a word sequence', '2. First-seen XML columns',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(2);
        expect(source.match(/xpath-functions="\.\/xpath-sequences.cemt"/gu)).toHaveLength(2);
        for (const expression of ['subsequence(', 'reverse(', 'distinct-values(', 'head(', 'tail(', 'some $earlier', 'namespace-uri($cell)', 'self::attribute()']) {
            expect(library).toContain(expression);
        }
        expect(source).toContain('native:call("table.columns", document.root)');
        expect(source).toContain('@projection=xpath');
        expect(source).toContain('document.error != ""');
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('tag=');
        expect(source).not.toContain('{function');
    });
    it('explains positions, ordering, limits and related examples', () => {
        for (const text of ['positions start at 1', 'implementation-dependent order', 'case-sensitive', '32 KiB', '4096 events', 'other collations', './xpath-nodes.html', './data-table.html']) {
            expect(source).toContain(text);
        }
    });
});
