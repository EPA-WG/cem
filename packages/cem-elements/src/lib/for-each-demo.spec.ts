import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleContract {
    legend: string;
    tag: string;
    includes: readonly string[];
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/for-each.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleContract[] = [
    {
        legend: '1. Simple for-each',
        tag: 'cem-loop-simple',
        includes: ['@as=fruit', '{$fruit}'],
    },
    {
        legend: '2. for-each with position()',
        tag: 'cem-loop-position',
        includes: ['@as=color', '{$position}. {$color.label}'],
    },
    {
        legend: '3. Conditional for-each',
        tag: 'cem-loop-conditional',
        includes: ['@slice=show-items', '@as=item', '{$position}:{$item}'],
    },
    {
        legend: '4. Nested for-each table',
        tag: 'cem-loop-nested-table',
        includes: ['@as=row', '@select="$row.cells" @as=cell', '{td | {$cell}}'],
    },
    {
        legend: '5. for-each with attributes',
        tag: 'cem-loop-attributes',
        includes: ['@as=user', '{$user.id}', '{$user.name}', '{$user.role}'],
    },
    {
        legend: '6. Dynamic table with toggle',
        tag: 'cem-loop-dynamic-table',
        includes: ['@slice=show-products', '@as=product', '{$product.name}'],
    },
    {
        legend: '7. for-each over payload data',
        tag: 'cem-loop-payload',
        includes: ['datadom.elementsByAttribute.feed', 'label="payload-alpha"', 'label="payload-beta"'],
    },
    {
        legend: '8. for-each over location data',
        tag: 'cem-loop-location',
        includes: ['{location-element @slice=loop-location', 'datadom.slices.loop-location.paramEntries'],
    },
    {
        legend: '9. for-each over HTTP JSON/XML data',
        tag: 'cem-loop-http',
        includes: ['{http-request @slice=loop-json', '{http-request @slice=loop-xml', 'json-url="./http-data.json"'],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<html-demo-element\s+legend="([^"]+)"[\s\S]*?<\/html-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('for-each demo source contracts', () => {
    it('has one independent html-demo-element for every use case', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(samples).toHaveLength(9);
    });

    it.each(SAMPLE_CONTRACTS)('$legend owns only its declaration and instance', ({ legend, tag, includes }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();
        const normalizedSource = (sample?.source ?? '').replace(/\s+/gu, ' ');
        expect(normalizedSource).toContain(`<cem-element tag="${tag}">`);
        expect(normalizedSource).toContain(`<${tag}`);
        expect(normalizedSource).toContain(`</${tag}>`);
        expect(normalizedSource.match(/<cem-element(?:\s|>)/gu)).toHaveLength(1);
        for (const required of includes) {
            expect(normalizedSource, `${legend} must include ${required}`).toContain(
                required.replace(/\s+/gu, ' ')
            );
        }
    });
});
