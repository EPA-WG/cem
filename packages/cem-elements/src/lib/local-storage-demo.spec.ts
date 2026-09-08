import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleContract {
    legend: string;
    includes: readonly string[];
    named: boolean;
    declarations?: number;
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/local-storage.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleContract[] = [
    {
        legend: '0. Read a live text value',
        includes: [
            'tag="cem-storage-live-text"',
            '@key=cemDemoLiveText',
            '@slice=liveText',
            '@live=true',
            '<cem-storage-live-text label="Editor A">',
            '<cem-storage-live-text label="Editor B">',
        ],
        named: true,
    },
    {
        legend: '1. Always override a stored value',
        includes: [
            '@key=cemDemoOverride',
            '@slice=overrideValue',
            '@value=ABC',
            '@slice-value="\'text value\'"',
        ],
        named: false,
    },
    {
        legend: '2. Stored value with a default',
        includes: [
            '@key=cemDemoPersistedDefault',
            '@slice=persistedDefault',
            '@slice-value="$target.value"',
        ],
        named: false,
    },
    {
        legend: '3. Typed localStorage values',
        includes: [
            '@type=date',
            '@type=time',
            '@type=datetime-local',
            '@type=number',
            '@type=json',
            'invalidNumber ?? "null"',
        ],
        named: false,
    },
    {
        legend: '4. Simplest initial read',
        includes: [
            'tag="cem-storage-cherries"',
            '@key=cemDemoCherries',
            '@slice=cherries',
            '<cem-storage-cherries>🍒</cem-storage-cherries>',
        ],
        named: true,
    },
    {
        legend: '5. Live JSON basket',
        includes: [
            '@key=cemDemoBasket',
            '@slice=basketText',
            '@slice=basket',
            '@type=json',
            'datadom.slices.basket.cherries',
            'datadom.slices.basket.lemons',
        ],
        named: false,
    },
    {
        legend: '6. Fruit buttons and a storage watcher',
        includes: [
            '@key=cemDemoFruitLemons',
            '@key=cemDemoFruitCherries',
            '@key=cemDemoFruitApples',
            '@key=cemDemoFruitBananas',
            '@aria-label="Add lemon"',
            '@aria-label="Add cherry"',
            '@aria-label="Add apple"',
            '@aria-label="Add banana"',
            '@slice-value="//lemons + 1"',
            '{h2 | Watched fruit counts}',
            '@live=true',
        ],
        named: false,
        declarations: 2,
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('local-storage demo source contracts', () => {
    it('explains persistence, coercion, authoritative values, safety, and related demos', () => {
        const normalizedSource = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(DEMO_SOURCE).toContain('Persistent browser state as slices');
        expect(normalizedSource).toContain('independent component instances remain synchronized');
        expect(normalizedSource).toContain('an invalid value becomes null');
        expect(normalizedSource).toContain('An authored <code>value</code> is authoritative');
        expect(normalizedSource).toContain('do not use it for secrets');
        expect(DEMO_SOURCE).toContain('href="./data-slices.html"');
        expect(DEMO_SOURCE).toContain('href="./dom-merge.html"');
        expect(DEMO_SOURCE).toContain('href="./form.html"');
    });

    it('keeps every legacy use case independent and adds the fruit writer/watcher case', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(samples).toHaveLength(7);
        expect(DEMO_SOURCE).not.toContain('onclick=');
        expect(DEMO_SOURCE).not.toContain('data-role=');
        expect(DEMO_SOURCE).not.toContain('data-testid=');
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns flush-left declarations', ({ legend, includes, named, declarations = 1 }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();

        const source = sample?.source ?? '';
        const normalizedSource = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<cem-element/u);
        expect(source.match(/<cem-element(?:\s|>)/gu)).toHaveLength(declarations);
        expect(source.match(/(?:<template>|<\/cem-element>)\n\n?<cem-element(?:\s|>)/gu)).toHaveLength(declarations);
        if (named) {
            expect(source).toMatch(/<cem-element\s+[^>]*\btag=/u);
        } else {
            expect(source).not.toMatch(/<cem-element\s+[^>]*\btag=/u);
        }

        for (const required of includes) {
            expect(normalizedSource, `${legend} must include ${required}`).toContain(
                required.replace(/\s+/gu, ' ')
            );
        }
    });

    it('seeds defaults only when storage has no existing value', () => {
        expect(DEMO_SOURCE).toContain("cemDemoPersistedDefault: 'DEF'");
        expect(DEMO_SOURCE).toContain("cemDemoCherries: '12'");
        expect(DEMO_SOURCE).toContain('JSON.stringify({ cherries: 12, lemons: 1 })');
        expect(DEMO_SOURCE).toContain("cemDemoFruitLemons: '1'");
        expect(DEMO_SOURCE).toContain("cemDemoFruitCherries: '12'");
        expect(DEMO_SOURCE).toContain("cemDemoFruitApples: '0'");
        expect(DEMO_SOURCE).toContain("cemDemoFruitBananas: '0'");
        expect(DEMO_SOURCE).toContain(
            'if (localStorage.getItem(key) === null) localStorage.setItem(key, value);'
        );
        expect(DEMO_SOURCE).not.toContain('localStorage.clear()');
    });
});
