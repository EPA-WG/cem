import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

interface SampleSourceContract {
    legend: string;
    includes: readonly string[];
    excludes?: readonly string[];
}

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/module-url.html', import.meta.url)),
    'utf8'
);

const SAMPLE_CONTRACTS: readonly SampleSourceContract[] = [
    {
        legend: 'this page import maps',
        includes: [
            '"demo-src-image": "./lib-dir/Smiley.svg?src=module"',
            '"demo-module-referrer": "./module-referrer/component.js"',
            '"./relative-referrer/"',
            '"./module-referrer/"',
            '"https://referrer.example.test/absolute/"',
        ],
    },
    {
        legend: '1. module path by symbolic name',
        includes: [
            '<cem-element>',
            '@slice=logoUrl',
            '@src="@epa-wg/cem-elements/demo/wc-square.svg"',
            '{image-link',
            '@src="{$datadom.slices.logoUrl}"',
        ],
        excludes: ['{module-url', '{img ', '{a '],
    },
    {
        legend: '2. src forms: relative URL',
        includes: [
            '<cem-element>',
            '@slice=relativeUrl',
            '@src="./lib-dir/Smiley.svg?src=relative"',
            '{image-link',
            '@src="{$datadom.slices.relativeUrl}"',
        ],
        excludes: ['{module-url', '{img ', '{a '],
    },
    {
        legend: '3. src forms: absolute URL',
        includes: [
            '<cem-element>',
            '@slice=absoluteUrl',
            '@src="data:image/svg+xml,',
            '{image-link',
            '@src="{$datadom.slices.absoluteUrl}"',
        ],
        excludes: ['{module-url', '{img ', '{a '],
    },
    {
        legend: '5. component-local map: naked',
        includes: [
            '<cem-element tag="cem-local-map-naked-image">',
            '{module-map |',
            '@specifier="demo-component-image" @target="./lib-dir/Smiley.svg?owner=component"',
            '{cem-module-url @slice=imageUrl @src="demo-component-image"}',
            '{image-link @class="component-owned-image"',
            '@src="{$datadom.slices.imageUrl}"',
            '<cem-local-map-naked-image></cem-local-map-naked-image>',
        ],
        excludes: ['{module-url', '{img ', '{a ', '@ target='],
    },
    {
        legend: '6. component-local map: wrapper override',
        includes: [
            '<cem-element tag="cem-local-map-override-image">',
            '@specifier="demo-component-image" @target="./lib-dir/Smiley.svg?owner=component"',
            '<cem-element tag="cem-local-map-override-wrapper">',
            '@specifier="demo-component-image" @target="./confused.svg?owner=wrapper"',
            '{cem-local-map-override-image}',
            '<cem-local-map-override-wrapper></cem-local-map-override-wrapper>',
        ],
        excludes: ['{module-url', '{a ', '@ target='],
    },
    {
        legend: '7. component-local map: node referrer',
        includes: [
            '<cem-element tag="cem-local-map-referrer">',
            '@specifier="demo-inner-only-image" @target="./wc-square.svg?owner=component"',
            '<cem-element tag="cem-local-map-referrer-demo">',
            '@src="./lib-dir/Smiley.svg?referrer=node" @referrer-selector="cem-local-map-referrer"',
            '@src="demo-inner-only-image" @referrer-selector="cem-local-map-referrer"',
            '@src="https://assets.example.test/logo.svg" @referrer-selector="cem-local-map-referrer"',
            '{image-link @class="node-referrer-image"',
            '@src="{$datadom.slices.innerOnlyImageUrl}"',
            '{expando-link @href="{$datadom.slices.relativeFromChildUrl}"}',
            '{expando-link @href="{$datadom.slices.innerOnlyImageUrl}"}',
            '{expando-link @href="{$datadom.slices.absoluteFromChildUrl}"}',
            '<cem-local-map-referrer-demo></cem-local-map-referrer-demo>',
        ],
        excludes: ['{module-url', '{img ', '{a ', '@ target='],
    },
    {
        legend: 'image-link',
        includes: [
            '<cem-element tag="expando-link">',
            '{a @href="{$href}" |{$str:shorten(href, 32)} }',
            '<cem-element tag="image-link">',
            '{cem-module-url @slice=imageUrl @src="{$src}"}',
            '{expando-link @href="{$datadom.slices.imageUrl}"}',
            '<image-link src="./confused.svg"',
            'href="./confused.svg"',
        ],
        excludes: [
            '{cem-module-url @slice=linkUrl',
            '{str:shorten($src, 32)}',
            '{str:shorten(src, 32)}',
        ],
    },
] as const;

const samples = Array.from(
    DEMO_SOURCE.matchAll(/<html-demo-element\s+legend="([^"]+)"[\s\S]*?<\/html-demo-element>/gu),
    (match) => ({ legend: match[1].replace(/\s+/gu, ' ').trim(), source: match[0] })
);

describe('module-url demo source contracts', () => {
    it('has one unit contract for every authored sample legend', () => {
        expect(samples.map(({ legend }) => legend)).toEqual(SAMPLE_CONTRACTS.map(({ legend }) => legend));
    });

    it('declares the executable page import map used by the examples', () => {
        const source = DEMO_SOURCE.match(/<script type="importmap">([\s\S]*?)<\/script>/u)?.[1];
        expect(source, 'page import map').toBeDefined();
        expect(() => JSON.parse(source ?? '')).not.toThrow();
        expect(JSON.parse(source ?? '{}')).toMatchObject({
            imports: {
                '@epa-wg/cem-elements/': '../',
                'demo-src-image': './lib-dir/Smiley.svg?src=module',
                'demo-referrer-image': './lib-dir/Smiley.svg?referrer=default',
                'demo-module-referrer': './module-referrer/component.js',
            },
            scopes: {
                './relative-referrer/': {
                    'demo-referrer-image': './lib-dir/Smiley.svg?referrer=relative',
                },
                './module-referrer/': {
                    'demo-referrer-image': './confused.svg?referrer=module',
                },
                'https://referrer.example.test/absolute/': {
                    'demo-referrer-image': './wc-square.svg?referrer=absolute',
                },
            },
        });
    });

    it('uses the shared resolver rather than a demo-only compatibility callback', () => {
        expect(DEMO_SOURCE).toContain('installCemElementRuntime(window);');
        expect(DEMO_SOURCE).not.toContain('resolveModuleUrl(specifier');
    });

    it('links to the extracted referrer and string-function demos', () => {
        expect(DEMO_SOURCE).toContain('href="./module-url-referrer.html"');
        expect(DEMO_SOURCE).toContain('href="./functions/str.html"');
    });

    it.each(SAMPLE_CONTRACTS)('$legend', ({ legend, includes, excludes = [] }) => {
        const sample = samples.find((candidate) => candidate.legend === legend);
        expect(sample, `missing authored sample ${legend}`).toBeDefined();
        const normalizedSource = (sample?.source ?? '').replace(/\s+/gu, ' ');
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
