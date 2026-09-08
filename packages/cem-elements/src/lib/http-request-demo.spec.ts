import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleContract {
    legend: string;
    includes: readonly string[];
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/http-request.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleContract[] = [
    {
        legend: '0. URL from text to http-request',
        includes: [
            '@slice=selectedUrl',
            '@slice=requestUrl',
            '@slice-value="$selectedUrl"',
            './http-data-compact.json',
        ],
    },
    {
        legend: '1. Simplest http-request',
        includes: [
            '@slice=catalog',
            '@url="./http-pokemon.json"',
            'Pokemon buttons from API',
            'datadom.slices.catalog.data.results',
            '@aria-label="{$item.name}"',
            '@src="{$spriteUrl}/{$item.id}.svg"',
        ],
    },
    {
        legend: '2. http-request response and headers',
        includes: [
            '@slice=inspection',
            '@header-x-demo="ported-from-legacy"',
            'inspection.request.headers.x-demo',
            'inspection.response.contentType',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('http-request demo source contracts', () => {
    it('explains the resource envelope, host policy, and related demos', () => {
        const normalizedSource = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(DEMO_SOURCE).toContain('HTTP responses as declarative data');
        expect(DEMO_SOURCE).toContain('datadom.slices.&lt;slice&gt;');
        expect(normalizedSource).toContain('cross-origin requests require explicit host authorization');
        expect(DEMO_SOURCE).toContain('href="./for-each.html"');
        expect(DEMO_SOURCE).toContain('href="./module-url.html"');
    });

    it('adapts every legacy use case into an independent local sample', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(samples).toHaveLength(3);
        expect(DEMO_SOURCE).not.toContain('pokeapi.co');
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns one anonymous, flush-left declaration', ({ legend, includes }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();

        const source = sample?.source ?? '';
        const normalizedSource = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<cem-element>/u);
        expect(source.match(/<cem-element(?:\s|>)/gu)).toHaveLength(1);
        expect(source).not.toMatch(/<cem-element\s+[^>]*\btag=/u);
        expect(source).not.toContain('data-role=');
        expect(source).not.toContain('data-testid=');

        for (const required of includes) {
            expect(normalizedSource, `${legend} must include ${required}`).toContain(
                required.replace(/\s+/gu, ' ')
            );
        }
    });

    it('keeps the Pokémon and compact responses in explicit local fixtures', () => {
        const pokemon = JSON.parse(readFileSync(
            fileURLToPath(new URL('../../demo/http-pokemon.json', import.meta.url)),
            'utf8'
        )) as { results: Array<{ id?: number; name?: string }> };
        const compact = JSON.parse(readFileSync(
            fileURLToPath(new URL('../../demo/http-data-compact.json', import.meta.url)),
            'utf8'
        )) as { results: Array<{ name?: string; status?: string }> };

        expect(pokemon.results.map(({ name }) => name)).toEqual([
            'bulbasaur',
            'ivysaur',
            'venusaur',
            'charmander',
            'charmeleon',
            'charizard',
        ]);
        expect(pokemon.results.map(({ id }) => id)).toEqual([1, 2, 3, 4, 5, 6]);
        expect(compact.results).toEqual([
            expect.objectContaining({ name: 'solo', status: 'compact' }),
        ]);
    });
});
