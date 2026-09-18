import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const source = readFileSync(new URL('../../demo/xpath-functions.html', import.meta.url), 'utf8');
const library = readFileSync(new URL('../../demo/xpath-functions.cemt', import.meta.url), 'utf8');

describe('external XPath library demo source contract', () => {
    it('gives named invocation and boolean matching separate flush-left examples', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), (match) => match[1])).toEqual([
            '1. Named XPath function', '2. Shared XPath predicate', '3. XML nodes and matching',
        ]);
        expect(source.match(/<template>\n<cem-element>/gu)).toHaveLength(3);
        expect(source.match(/xpath-functions="\.\/xpath-functions.cemt"/gu)).toHaveLength(3);
        expect(source).toContain('native:call("demo.label", datadom.slices.text)');
        expect(source).toContain('@match=\'native:call("demo.accept", node)\'');
        expect(source).not.toContain('{function');
        expect(library).toContain('@returns=boolean');
        expect(library).toContain('@sequence-type="xs:string"');
        expect(source).toContain('@projection=xpath');
        expect(source).toContain('native:call("demo.items", document.root)');
        expect(source).toContain('document.error != ""');
        expect(library).toContain('@sequence-type="node()*"');
        expect(source).not.toContain('onclick=');
        expect(source).not.toContain('tag=');
    });
    it('explains the dependency, bounds and related features', () => {
        for (const text of ['32 KiB', '64 functions', '16 documents', '4096 events', 'See also', './dom-merge.html', './functions/str.html']) {
            expect(source).toContain(text);
        }
    });
});
