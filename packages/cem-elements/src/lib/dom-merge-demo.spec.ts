import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleContract {
    legend: string;
    includes: readonly string[];
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/dom-merge.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleContract[] = [
    {
        legend: '1. Textarea word count',
        includes: ['{textarea', '@slice-event=input', 'Word count:'],
    },
    {
        legend: '2. Input word and character count',
        includes: ['{input', 'Character count:', 'Word count:', 'Current slice:'],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<html-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/html-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('DOM merge demo source contracts', () => {
    it('describes the feature and links its related demos', () => {
        expect(DEMO_SOURCE).toContain('Live editing without replacing controls');
        expect(DEMO_SOURCE).toContain('href="./data-slices.html"');
        expect(DEMO_SOURCE).toContain('href="./functions/str.html"');
    });

    it('has one independent html-demo-element for each counter use case', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(samples).toHaveLength(2);
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns one anonymous, flush-left declaration', ({ legend, includes }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();

        const source = sample?.source ?? '';
        const normalizedSource = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<cem-element>/u);
        expect(source.match(/<cem-element(?:\s|>)/gu)).toHaveLength(1);
        expect(source).not.toMatch(/<cem-element\s+[^>]*\btag=/u);

        for (const required of includes) {
            expect(normalizedSource, `${legend} must include ${required}`).toContain(
                required.replace(/\s+/gu, ' ')
            );
        }
    });
});
