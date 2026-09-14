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
            "localStorage.setItem('cemDemoLiveText','text value')",
            "localStorage.setItem('cemDemoLiveText','another value')",
            "localStorage.setItem('cemDemoLiveText','')",
            "localStorage.removeItem('cemDemoLiveText')",
            '@key=cemDemoLiveText',
            '@slice=liveText',
            '@live=true',
            'liveText ?? "null"',
        ],
        named: false,
    },
    {
        legend: '1. Always override a stored value',
        includes: [
            '@key=cemDemoOverride',
            '@slice=overrideValue',
            '@value=ABC',
            '@live=true',
            "localStorage.setItem('cemDemoOverride','text value')",
            "localStorage.removeItem('cemDemoOverride')",
        ],
        named: false,
    },
    {
        legend: '2. Stored value with a default',
        includes: [
            '@key=cemDemoPersistedDefault',
            '@slice=persistedDefault',
            "localStorage.setItem('cemDemoPersistedDefault','remember me')",
            "localStorage.removeItem('cemDemoPersistedDefault')",
        ],
        named: false,
    },
    {
        legend: '3a. Date validation',
        includes: [
            '@type=date',
            '@live=true',
            "localStorage.setItem('cemDemoDate','ABC')",
            "localStorage.setItem('cemDemoDate',this.elements.raw.value)",
        ],
        named: false,
    },
    {
        legend: '3b. Time validation',
        includes: [
            '@type=time',
            '@live=true',
            "localStorage.setItem('cemDemoTime','25:00')",
        ],
        named: false,
    },
    {
        legend: '3c. Local date and time validation',
        includes: [
            '@type=datetime-local',
            '@live=true',
            "localStorage.setItem('cemDemoLocalDateTime','ABC')",
        ],
        named: false,
    },
    {
        legend: '3d. Number validation',
        includes: [
            '@type=number',
            '@type=text',
            '@live=true',
            "localStorage.setItem('cemDemoNumber','0001')",
            "localStorage.setItem('cemDemoNumber','0')",
            "localStorage.setItem('cemDemoNumber','ABC')",
            'number ?? "null"',
        ],
        named: false,
    },
    {
        legend: '3e. JSON validation',
        includes: [
            '@type=json',
            '@type=text',
            '@live=true',
            "localStorage.setItem('cemDemoJson',JSON.stringify('ABC'))",
            "localStorage.setItem('cemDemoJson','ABC')",
            "localStorage.setItem('cemDemoJson','false')",
            'record:entries(datadom.slices.json)',
            'item:kind(datadom.slices.json)',
            '@select=datadom.slices.json @as=item',
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
            "localStorage.setItem('cemDemoCherries','24')",
        ],
        named: true,
    },
    {
        legend: '5. Live JSON basket',
        includes: [
            '@key=cemDemoBasket',
            '@slice=basket',
            '@type=json',
            'datadom.slices.basket.cherries',
            'datadom.slices.basket.lemons',
            "localStorage.setItem('cemDemoBasket', JSON.stringify(basket))",
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
            'aria-label="Add lemon"',
            'aria-label="Add cherry"',
            'aria-label="Add apple"',
            'aria-label="Add banana"',
            "Number(localStorage.getItem('cemDemoFruitLemons')) + 1",
            '{h2 | Watched fruit counts}',
            '@live=true',
        ],
        named: false,
    },
    {
        legend: '7. Write a slice back to storage',
        includes: [
            '@key=cemDemoSliceEditor',
            '@slice-value="$target.value"',
            '<cem-storage-editor label="Editor A">',
            '<cem-storage-editor label="Editor B">',
        ],
        named: true,
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

    it('keeps each typed case independent and distinguishes the two write directions', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(samples).toHaveLength(12);
        expect(DEMO_SOURCE).not.toContain('data-role=');
        expect(DEMO_SOURCE).not.toContain('data-testid=');
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns flush-left declarations', ({ legend, includes, named, declarations = 1 }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();

        const source = sample?.source ?? '';
        const normalizedSource = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<\S/u);
        expect(source.match(/<cem-element(?:\s|>)/gu)).toHaveLength(declarations);
        expect(source.match(/^<cem-element(?:\s|>)/gmu)).toHaveLength(declarations);
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

    it('drives external writes through plain controls outside the observing DCE', () => {
        for (const sample of samples.filter(({ legend }) => !legend.startsWith('7.'))) {
            const writer = sample.source.split('<cem-element')[0];
            expect(writer).toContain('onclick=');
            expect(writer).toContain('localStorage.setItem(');
            expect(sample.source).not.toContain('@slice-event=');
        }
        expect(samples.find(({ legend }) => legend.startsWith('4.'))?.source).not.toContain('@live');
        expect(samples.find(({ legend }) => legend.startsWith('2.'))?.source).not.toContain('@value=');
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

    it('pairs symbolic fruit controls with readable names and tooltips', () => {
        const buttons = Array.from(DEMO_SOURCE.matchAll(/<button\b[^>]*>[\s\S]*?<\/button>/gu), (match) => match[0]);
        for (const [name, symbol] of [
            ['Store 24 cherries', '24🍒'], ['Store 12 cherries', '12🍒'],
            ['Add cherry', '+🍒'], ['Add lemon', '+🍋'],
            ['Reset basket', '↺🛒'], ['Add apple', '+🍏'], ['Add banana', '+🍌'],
        ]) {
            const controls = buttons.filter((button) => button.includes(`aria-label="${name}"`));
            expect(controls.length, name).toBeGreaterThan(0);
            for (const control of controls) {
                expect(control).toContain(`title="${name}"`);
                expect(control.replace(/^[\s\S]*?>|<\/button>$/gu, '').trim()).toBe(symbol);
            }
        }
        for (const [name, symbol] of [
            ['Cherries', '🍒'], ['Lemons', '🍋'], ['Apples', '🍏'],
            ['Bananas', '🍌'], ['Total', '🛒'],
        ]) {
            expect(DEMO_SOURCE).toContain(`{dt @aria-label=${name} @title=${name} | ${symbol}}`);
        }
    });
});
