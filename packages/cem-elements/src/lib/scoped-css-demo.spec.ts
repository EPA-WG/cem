import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/scoped-css.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS = [
    ['1. Private declaration CSS and ordinary outer cascade', ['cem-css-private', 'Browser default border']],
    ['2. Component in a named scope shares default declaration styles with peers in the same scope', ['scope="css-samples"', 'sample-shared-bare']],
    ['3. Style can be scoped explicitly', ['{style @scope="css-samples"', 'cem-css-explicit-peer']],
    ['4. Mixed private and shared styles', ['sample-mixed', 'sample-mixed-shared']],
    ['5. Invalid and mismatched scopes fail closed', ['scope="css samples"', '@scope="other-lib"']],
    ['6. Payload style belongs to one instance', ['--instance-border: red', '<cem-css-instance>blue</cem-css-instance>']],
    ['7. Declaration styles must be static', ['@scope="{$scope}"', 'dynamic styles rejected']],
    ['8. Fragment template CSS uses the effective produced tag', ['id="css-fragment-template"', 'src="#css-fragment-template"']],
    ['9. Anonymous declaration CSS uses its generated tag', ['uid-seed="demo/css/anonymous"', 'sample-anonymous']],
    ['10. uid-seed stabilizes keyframe names', ['@keyframes seeded-pulse', 'uid-seed="demo/css/keyframes"']],
    ['11. Descendant selectors stay inside the component', ['input:checked + b', '{input @type=checkbox @checked=true}']],
    ['12. CSS from an external template fragment', ['./external-template-templates.html#scoped-css-external-template', 'cem-css-external-fragment']],
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"[\s\S]*?<\/cem-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('scoped CSS demo source contracts', () => {
    it('states the CSS ownership purpose and cross-references external examples', () => {
        const normalized = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(DEMO_SOURCE).toContain('Keep component CSS local while preserving intentional sharing');
        expect(normalized).toContain('Declaration styles are compiled into native <code>@scope</code> boundaries');
        expect(DEMO_SOURCE).toContain('href="./external-template.html"');
        expect(DEMO_SOURCE).toContain('href="./hex-grid.html"');
    });

    it('retains the modern contract cases and covers the remaining legacy descendant and external cases', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(([legend]) => legend));
        expect(DEMO_SOURCE).not.toContain('data-testid=');
    });

    it.each(SAMPLE_CONTRACTS)('%s', (legend, includes) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();
        const normalized = (sample?.source ?? '').replace(/\s+/gu, ' ');
        for (const required of includes) {
            expect(normalized, `${legend} must include ${required}`).toContain(required.replace(/\s+/gu, ' '));
        }
    });
});
