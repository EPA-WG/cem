import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const source = readFileSync(new URL('../../demo/xpath-maps-arrays.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-maps-arrays.cemt', import.meta.url), 'utf8');

describe('XPath maps and arrays demos', () => {
    it('keeps optional entries and XML/JSON imported members in separate declarative cases', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. An IP-filter map with an optional note', '2. Select a retained fruit by array position',
            '3. Query an imported JSON tree',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(3);
        expect(source.match(/xpath-functions="\.\/xpath-maps-arrays.cemt"/gu)).toHaveLength(3);
        for (const expression of ['map:contains', 'map:get', 'map:keys', 'array:size', 'array:get', '$filter?action', 'array { $document/basket/child::* }', '[ $document/basket/@note ]']) {
            expect(library).toContain(expression);
        }
        expect(source).toContain('@projection=json-to-xml');
        expect(source).toContain('@type=xml}');
        expect(source).toContain('document.error != ""');
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('tag=');
        expect(source).not.toContain('{function');
    });
    it('explains member sequences, missing entries and input limits', () => {
        for (const text of ['positions start at 1', 'empty members', 'not portable', 'JSON null mapping', '32 KiB', 'no address validation', './xpath-sequences.html', './xpath-aggregates.html', './data-table.html']) {
            expect(source).toContain(text);
        }
    });
});
