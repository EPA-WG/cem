import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/location-element.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS = [
    {
        legend: '1. Window location live update',
        includes: [
            '{location-element @slice=current @live=true}',
            '@value=history.pushState',
            '@value=history.replaceState',
            '?mode={$datadom.slices.method}&amp;tag=one&amp;tag=two#checked',
            'datadom.slices.current.paramEntries',
        ],
    },
    {
        legend: '2. Window location initial read',
        includes: [
            '{location-element @slice=initial}',
            'datadom.slices.initial.source',
            'datadom.slices.initial.origin',
            'datadom.slices.initial.search',
        ],
    },
    {
        legend: '3. External URL from href',
        includes: [
            '@href="https://my.example/docs?a=1&amp;b=2&amp;b=3#details"',
            'datadom.slices.external.hostname',
            'datadom.slices.external.paramEntries',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('location-element demo source contracts', () => {
    it('explains current, live, explicit URL, and structured URL data use cases', () => {
        const normalized = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(DEMO_SOURCE).toContain('Read URLs as structured slice data');
        expect(normalized).toContain('Omit <code>href</code> from <code>location-element</code> to read the current page URL');
        expect(normalized).toContain('Add <code>live</code> when history, hash, or navigation changes');
        expect(normalized).toContain('Supply <code>href</code> to parse another URL');
        expect(DEMO_SOURCE).toContain('href="./set-url.html"');
        expect(DEMO_SOURCE).toContain('href="./module-url.html"');
    });

    it('keeps the three legacy use cases independent and free of test hooks', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(DEMO_SOURCE).not.toContain('data-role=');
        expect(DEMO_SOURCE).not.toContain('data-testid=');
        expect(DEMO_SOURCE).not.toContain('onclick=');
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns one anonymous, flush-left declaration', ({ legend, includes }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();
        const source = sample?.source ?? '';
        const normalized = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<cem-element>/u);
        expect(source.match(/<cem-element(?:\s|>)/gu)).toHaveLength(1);
        expect(source).not.toMatch(/<cem-element\s+[^>]*\btag=/u);
        for (const required of includes) {
            expect(normalized, `${legend} must include ${required}`).toContain(required.replace(/\s+/gu, ' '));
        }
    });
});
