import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleSourceContract {
    legend: string;
    includes: readonly string[];
    excludes?: readonly string[];
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/data-slices.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleSourceContract[] = [
    {
        legend: 'A1. inline slice initialization, change on event',
        includes: [
            '<cem-element tag="cem-slice-counter-input-default">',
            '@slice=clickcount @slice-event=click @slice-value="//clickcount + 1"',
            '@slice=clickcount @slice-event=click @slice-value="//clickcount - 1"',
            '{input @slice=clickcount @type=number @value="{datadom.slices.clickcount ?? 0}"}',
            '{$datadom.slices.clickcount ?? 0}',
        ],
        excludes: ['{slice @name=clickcount', '@slice-event="click tap"'],
    },
    {
        legend: 'A2. slice initialization, change on event',
        includes: [
            '<cem-element tag="cem-slice-counter">',
            '{slice @name=clickcount | 0}',
            '@slice-event="click tap"',
            '{input @type=number @slice=clickcount @value="{$clickcount}"}',
            '{output | {$clickcount}}',
        ],
    },
    {
        legend: 'B. slice event data.',
        includes: [
            '<cem-element tag="cem-slice-event-data">',
            '@slice-value=\'concat("x:", //@pageX)\'',
            '@slice-event="mousemove click"',
            '{datadom.eventPayloads.s.offsetX ?? 0',
            'datadom.eventPayloads.s.offsetY',
            'datadom.eventPayloads.s.type',
        ],
        excludes: [
            '{$datadom.eventPayloads.s.offsetX ?? 0',
            '{$datadom.eventPayloads.s.offsetY ?? 0',
        ],
    },
    {
        legend: '1. slice change on event. 1:1 slice⮂value',
        includes: ['<cem-element tag="cem-slice-basic">', '{slice @name=typed}', '{input @slice=typed}'],
        excludes: ['@slice-event', '@slice-value'],
    },
    {
        legend: '2. initial slice value, slice change on event. slice⮂value',
        includes: [
            '<cem-element tag="cem-slice-initial-change">',
            '{slice @name=s | B}',
            '{input @slice=s @value="{$s}"}',
        ],
        excludes: ['@slice-event', '@slice-value'],
    },
    {
        legend: '3. on input event. slice⮂value',
        includes: [
            '<cem-element tag="cem-slice-initial-input">',
            '{slice @name=s | B}',
            '{input @slice=s @slice-event=input @value="{$s}"}',
        ],
    },
    {
        legend: '4. initial slice value from attribute',
        includes: [
            '<cem-element tag="cem-slice-attribute-initial">',
            '{attribute @name=a | 😁}',
            '@slice-event=keyup',
            '<cem-slice-attribute-initial></cem-slice-attribute-initial>',
            '<cem-slice-attribute-initial a="🤗"></cem-slice-attribute-initial>',
        ],
    },
    {
        legend: '5. slice value computed from event',
        includes: [
            '<cem-element tag="cem-slice-transform">',
            '{slice @name=s | xB}',
            '@value="{str:substring(datadom.slices.s, 2)}"',
            '@slice-value=\'"x" + @value\'',
        ],
    },
    {
        legend: '6. button ignored till change on click.',
        includes: [
            '<cem-element tag="cem-slice-broccoli">',
            '{slice @name=nickname | anonymous}',
            '@slice-value=\'"broccoli"\' @slice-event=click',
        ],
    },
    {
        legend: '7. initial slice value from SLICE element',
        includes: [
            '<cem-element tag="cem-slice-declared-counter">',
            '{slice @name=clickcount}',
            '{slice @slice=clickcount @value=0}',
            '@slice-event="click tap"',
        ],
    },
    {
        legend: '8. multiple slices by SLICE element',
        includes: [
            '<cem-element tag="cem-slice-multi-directive">',
            '{slice @slice=clicked @value=0}',
            '{slice @slice=focused @value=0}',
            '@slice-event="click tap"',
            '@slice-event=focus',
            '@slice-event=blur',
        ],
    },
    {
        legend: '9. slice in attribute',
        includes: [
            '<cem-element tag="cem-slice-emotion-attribute">',
            '@slice="/datadom/attributes/emotion"',
            '<cem-slice-emotion-attribute emotion=":)"',
            '<cem-slice-emotion-attribute></cem-slice-emotion-attribute>',
        ],
    },
    {
        legend: '10. multiple slices by same field',
        includes: [
            '<cem-element tag="cem-slice-fanout">',
            '@slice="s1|s2" @slice-event=input',
            '{$datadom.slices.s1}',
            '{$datadom.slices.s2}',
        ],
    },
    {
        legend: '11. slices and attribute',
        includes: [
            '<cem-element tag="cem-slice-attribute-fanout">',
            '{attribute @name=emotion | 😃}',
            '@slice="/datadom/attributes/emotion | s1"',
            '{$emotion}',
            '{$datadom.slices.s1}',
        ],
    },
    {
        legend: '12. checkbox use',
        includes: [
            '<cem-element tag="cem-slice-checkboxes">',
            '{cem:variable @name=v1 @select=\'"V1"\'}',
            '@type=checkbox @slice=is-checked @value=V0 @checked',
            '@type=checkbox @slice=s2 @slice-value="{$v1}"',
            '@type=checkbox @slice=s3 @value="{$v1}"',
        ],
    },
    {
        legend: '13. Radio group',
        includes: [
            '<cem-element tag="cem-slice-radios">',
            '{slice @name=radio-group | V1}',
            '@type=radio @slice=radio-group @value=V0 @name=g1',
            '@type=radio @slice=radio-group @value=V1 @name=g1 @checked',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<html-demo-element\s+legend="([^"]+)"[\s\S]*?<\/html-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('data-slices demo source contracts', () => {
    it('has one unit contract for every authored sample legend', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
    });

    it('keeps test-only selectors out of the authored examples', () => {
        expect(DEMO_SOURCE).not.toMatch(/@data-(?:role|testid)=|\sid="/u);
    });

    it.each(SAMPLE_CONTRACTS)('$legend', ({ legend, includes, excludes = [] }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();
        const source = sample?.source ?? '';
        const producedTag = source.match(/<cem-element\s+tag="([^"]+)"/)?.[1];

        expect(producedTag, `${legend} declares a produced tag`).toBeTruthy();
        expect(source).toContain(`<${producedTag}`);
        expect(source).toContain('<template type="text/cem-ml">');
        const normalizedSource = source.replace(/\s+/gu, ' ');
        for (const required of includes) {
            expect(normalizedSource, `${legend} must include ${required}`).toContain(
                required.replace(/\s+/gu, ' ')
            );
        }
        for (const forbidden of excludes) {
            expect(normalizedSource, `${legend} must exclude ${forbidden}`).not.toContain(
                forbidden.replace(/\s+/gu, ' ')
            );
        }
    });
});
