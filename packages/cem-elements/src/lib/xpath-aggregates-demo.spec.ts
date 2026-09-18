import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const source = readFileSync(new URL('../../demo/xpath-aggregates.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-aggregates.cemt', import.meta.url), 'utf8');

describe('XPath aggregate demos', () => {
    it('keeps decimal statistics and dynamic fruit totals in separate declarative cases', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. Decimal sequence statistics', '2. A basket that accepts new fruits',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(2);
        expect(source.match(/xpath-functions="\.\/xpath-aggregates.cemt"/gu)).toHaveLength(2);
        for (const expression of ['sum($values)', 'min($values)', 'max($values)', 'avg($values)', 'castable as xs:decimal', '/basket/child::* ! xs:decimal(.)']) {
            expect(library).toContain(expression);
        }
        expect(library).not.toContain('/basket/apple');
        expect(source).toContain('@projection=xpath');
        expect(source).toContain('document.error != ""');
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('tag=');
        expect(source).not.toContain('{function');
    });
    it('explains empty input, invalid values, numeric precision and related examples', () => {
        for (const text of ['18 significant digits', 'half-even', 'sum 0', 'shown as ∅', 'instead of', '32 KiB', 'duration aggregates remain unsupported', './xpath-sequences.html', './xpath-nodes.html', './data-table.html']) {
            expect(source).toContain(text);
        }
    });
});
