import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('../../demo/xpath-validation.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-validation.cemt', import.meta.url), 'utf8');

describe('XPath validation demos', () => {
    it('keeps the two declarative cases in an independently loaded XPath library', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. Form validation preview', '2. IPv4 prefix-rule preview',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(2);
        expect(source.match(/xpath-functions="\.\/xpath-validation.cemt"/gu)).toHaveLength(2);
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('JSON.parse');
        expect(source).not.toContain('{function');
        for (const text of ['matches(', 'replace(', 'tokenize(', 'every $', 'some $', 'castable as xs:integer']) {
            expect(library).toContain(text);
        }
    });
    it('explains numeric rules, explicit subset exclusions and preview scope', () => {
        for (const text of ['18 to 120', 'No slash means /32', 'Leading zeros', 'Backreferences',
            'explicitly unsupported', 'subnet membership', 'Regex alone', './dom-merge.html', './xpath-maps-arrays.html']) {
            expect(source).toContain(text);
        }
    });
});
