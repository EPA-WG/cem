import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/set-url.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS = [
    {
        legend: '1. Set the page hash',
        includes: ['@method=location.hash', '@value="#hash-one"', '@value="#hash-two"'],
    },
    {
        legend: '2. Select the URL write method',
        includes: [
            '@value=location.href',
            '@value=location.hash',
            '@value=location.assign',
            '@value=location.replace',
            '@value=history.pushState',
            '@value=history.replaceState',
            '@method="{$datadom.slices.method}"',
        ],
    },
    {
        legend: '3. Conditionally inject a URL writer',
        includes: ['@value="#conditional-writer"', '{cem:if @test=datadom.slices.requestedUrl |', '@method=location.href'],
    },
    {
        legend: '4. Set URL from form controls',
        includes: [
            '{slice @name=method | history.pushState}',
            '{slice @name=url | #form-driven}',
            '@slice-event=input',
            '@slice-value="$target.value"',
            '@method="{$datadom.slices.method}"',
            '@src="{$datadom.slices.url}"',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('set-url demo source contracts', () => {
    it('explains declarative URL writes, supported methods, and the render guard', () => {
        const normalized = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(DEMO_SOURCE).toContain('Write browser location declaratively');
        expect(normalized).toContain('Supported methods mirror <code>location.href</code>');
        expect(normalized).toContain('Render a writer only in response to deliberate state or an event');
        expect(DEMO_SOURCE).toContain('href="./location-element.html"');
        expect(DEMO_SOURCE).toContain('href="./data-slices.html"');
    });

    it('ports each legacy writer use case into an independent sample', () => {
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
