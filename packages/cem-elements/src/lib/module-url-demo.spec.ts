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
        legend: '4. module path by symbolic name',
        includes: [
            '<cem-element tag="cem-module-link">',
            '{cem-module-url @slice=packageUrl @src="@epa-wg/material"}',
            '{cem-module-url @slice=logoUrl @src="@epa-wg/cem-elements/demo/lib-dir/Smiley.svg"}',
            '{image-link @class=resolved-logo',
            '@src="{$datadom.slices.logoUrl}"',
            '@href="{$datadom.slices.packageUrl}README.md"',
            '<cem-module-link></cem-module-link>',
        ],
        excludes: ['{module-url', '{img ', '{a '],
    },
    {
        legend: '5. src forms: relative URL, module path, and absolute URL',
        includes: [
            '<cem-element tag="cem-module-src-forms">',
            '@slice=relativeUrl @src="./lib-dir/Smiley.svg?src=relative"',
            '@slice=moduleUrl @src="demo-src-image"',
            '@slice=absoluteUrl @src="data:image/svg+xml,',
            '{image-link @class=resolved-logo @src="{$datadom.slices.relativeUrl}" @href="{$datadom.slices.relativeUrl}"',
            '{image-link @class=resolved-logo @src="{$datadom.slices.moduleUrl}" @href="{$datadom.slices.moduleUrl}"',
            '{image-link @class=resolved-logo @src="{$datadom.slices.absoluteUrl}" @href="{$datadom.slices.absoluteUrl}"',
            '<cem-module-src-forms></cem-module-src-forms>',
        ],
        excludes: ['{module-url', '{img ', '{a '],
    },
    {
        legend: '6. src by scalar referrer matrix',
        includes: [
            '<cem-element tag="cem-module-referrer-matrix">',
            '@src="../lib-dir/Smiley.svg?case=relative-relative" @referrer="./relative-referrer/component.js"',
            '@src="demo-referrer-image" @referrer="demo-module-referrer"',
            '@src="https://assets.example.test/logo.svg" @referrer="https://referrer.example.test/absolute/component.js"',
            '{$datadom.slices.relativeByRelative}',
            '{$datadom.slices.moduleByModule}',
            '{$datadom.slices.absoluteByAbsolute}',
            '<cem-module-referrer-matrix></cem-module-referrer-matrix>',
        ],
        excludes: ['{module-url'],
    },
    {
        legend: '7. component-local map: naked, wrapper override, and node referrer',
        includes: [
            '<cem-element tag="cem-local-map-image">',
            '{module-map |',
            '@specifier="demo-component-image" @target="./lib-dir/Smiley.svg?owner=component"',
            '@specifier="demo-inner-only-image" @target="./wc-square.svg?owner=component"',
            '<cem-element tag="cem-local-map-wrapper">',
            '@specifier="demo-component-image" @target="./confused.svg?owner=wrapper"',
            '@src="./lib-dir/Smiley.svg?referrer=node" @referrer-selector="cem-local-map-image"',
            '@src="demo-inner-only-image" @referrer-selector="cem-local-map-image"',
            '@src="https://assets.example.test/logo.svg" @referrer-selector="cem-local-map-image"',
            '{$datadom.slices.relativeFromChildUrl}',
            '{$datadom.slices.innerOnlyImageUrl}',
            '{$datadom.slices.absoluteFromChildUrl}',
            '{image-link @class="resolved-logo node-referrer-image"',
            '@src="{$datadom.slices.innerOnlyImageUrl}"',
            '@href="{$datadom.slices.innerOnlyImageUrl}"',
            '<cem-local-map-image></cem-local-map-image>',
            '<cem-local-map-wrapper></cem-local-map-wrapper>',
        ],
        excludes: ['{module-url', '{img ', '{a ', '@ target='],
    },
    {
        legend: '8. str:shorten query/result matrix',
        includes: [
            '<cem-element tag="cem-str-shorten-matrix">',
            '{code | str:shorten("short", 8)}',
            '{$str:shorten("short", 8)}',
            '{$str:shorten("abcdefghij", 7)}',
            '{$str:shorten("abcdefghij", 8)}',
            '{$str:shorten("abcdefghij", 8, "...")}',
            '{$str:shorten("abcdefghij", 6, "")}',
            '{$str:shorten("αβ😀δεζη", 5, "💠")}',
            '{$str:shorten( "https://example.test/lib/semantic-card.cem" , 32)}',
            '<cem-str-shorten-matrix></cem-str-shorten-matrix>',
        ],
    },
    {
        legend: 'image-link',
        includes: [
            '<cem-element tag="expando-link">',
            '{a @href="{$href}" |{$str:shorten(href, 32)} }',
            '<cem-element tag="image-link">',
            '{cem-module-url @slice=imageUrl @src="{$src}"}',
            '{cem-module-url @slice=linkUrl @src="{$href}"}',
            '{expando-link @href="{$datadom.slices.linkUrl}"}',
            '<image-link src="./confused.svg"',
            'href="./confused.svg"',
        ],
        excludes: ['{str:shorten($src, 32)}', '{str:shorten(src, 32)}'],
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
