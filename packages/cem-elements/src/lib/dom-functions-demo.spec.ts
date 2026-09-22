import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('../../demo/functions/dom.html', import.meta.url), 'utf8');
const legends = [
    "Wrap values",
    "Parent",
    "Element children",
    "All child nodes",
    "Ancestors",
    "Closest ancestor",
    "Local names",
    "Node text",
    "Unqualified attribute",
    "Qualified attribute",
    "First match",
    "Last match",
    "Filter",
    "First value",
    "Last value",
    "Take a prefix",
    "Skip a prefix",
    "Map values",
    "Flatten mapped values",
    "Any match",
    "All match",
    "Empty chain",
    "Count values",
    "Sort values",
    "Sort native nodes",
    "Reverse order"
];

describe('CEM-QL DOM functions authored samples', () => {
    it('has one independently authored card per documented use case', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual(legends);
    });
    it.each(legends)('%s is self-contained and readable', legend => {
        const sample = source.split(`<cem-demo-element legend="${legend}"`)[1]?.split('</cem-demo-element>')[0] ?? '';
        expect(sample).toContain('description=');
        expect(sample).toMatch(/<template>\n<cem-element>/u);
        expect(sample).toContain('{output |');
        expect(sample).not.toContain('tag=');
        expect(sample).not.toContain('<script');
    });
    it('covers all chain methods and keeps native import explicit', () => {
        for (const name of ['parent', 'children', 'child_nodes', 'ancestors', 'closest', 'name', 'text',
            'attribute', 'find', 'find_last', 'filter', 'first', 'last', 'take', 'skip', 'map', 'flat_map',
            'any', 'all', 'is_empty', 'count', 'sorted', 'sorted_by_key', 'reversed']) {
            expect(source).toContain(`.${name}(`);
        }
        expect(source).toContain('dom::chain(');
        expect(source).toContain('data::read(');
        expect(source).toContain('href="../cell-overrides.html"');
        expect(source).toContain("from '../../dist/index.js'");
        expect(source).toContain('src="../../../cem-demo-element/dist/index.js"');
    });
});
