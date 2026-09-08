import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/npm-versions-demo.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS = [
    {
        legend: '1. Default to the latest version',
        includes: ['tag="cem-npm-version-default"', 'src="#npm-version"', 'package="@epa-wg/cem-elements"'],
    },
    {
        legend: '2. Preselect a version and show dates',
        includes: ['initialversion="0.0.22"', 'showdate="true"'],
    },
    {
        legend: '3. Propagate the selected value',
        includes: ['@slice=selectedVersion', '@slice-event=change', '@slice-value="$target.value"'],
    },
    {
        legend: '4. Override the label slot',
        includes: ['{i @slot=label | Select a release:}', '@slice=selectedVersion'],
    },
    {
        legend: '5. Synchronize the selected version with the URL',
        includes: [
            '@value=0.0.22',
            '{slice @name=targetVersion}',
            '@slice-value="$selectedVersion"',
            '{location-element @slice=current @live=true}',
            '@method=location.hash',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('npm versions demo source contracts', () => {
    it('documents the reusable picker contract and deterministic registry fixture', () => {
        const normalized = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(DEMO_SOURCE).toContain('Choose a package version from registry data');
        expect(normalized).toContain('current version, optional publication dates, a projected label, value propagation');
        expect(normalized).toContain('small same-origin registry fixture');
        expect(DEMO_SOURCE).toContain('href="./http-request.html"');
        expect(DEMO_SOURCE).toContain('href="./location-element.html"');
        expect(DEMO_SOURCE).toContain('href="./set-url.html"');
    });

    it('defines one shared source fragment with HTTP, selection, reflection, and slot behavior', () => {
        expect(DEMO_SOURCE).toContain('<template id="npm-version" type="text/cem-ml">');
        expect(DEMO_SOURCE).toContain('@url="./npm-versions.json"');
        expect(DEMO_SOURCE).toContain('@name=value');
        expect(DEMO_SOURCE).toContain('datadom.eventPayloads.selectedVersion');
        expect(DEMO_SOURCE).toContain('{slot @name=label |');
        expect(DEMO_SOURCE).toContain("@test='version.version == initialversion'");
        expect(DEMO_SOURCE).toContain("@test='showdate == \"true\"'");
        expect(DEMO_SOURCE).toContain('@src="#version={$datadom.slices.targetVersion}"');
    });

    it('ports all five legacy use cases and avoids test-only demo markup', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
        expect(DEMO_SOURCE).not.toContain('data-role=');
        expect(DEMO_SOURCE).not.toContain('data-testid=');
        expect(DEMO_SOURCE).not.toContain('onclick=');
    });

    it.each(SAMPLE_CONTRACTS)('$legend is flush-left and self-declares its picker', ({ legend, includes }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();
        const source = sample?.source ?? '';
        const normalized = source.replace(/\s+/gu, ' ');
        expect(source).toMatch(/<template>\n<cem-element/u);
        expect(source).toContain('src="#npm-version"');
        for (const required of includes) {
            expect(normalized, `${legend} must include ${required}`).toContain(required.replace(/\s+/gu, ' '));
        }
    });
});
