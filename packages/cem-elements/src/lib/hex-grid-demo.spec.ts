import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_URL = new URL('../../demo/hex-grid.html', import.meta.url);
const DEMO_SOURCE = readFileSync(fileURLToPath(DEMO_URL), 'utf8');
const LEGEND = '1. Responsive framework link honeycomb';
const COMPACT_LEGEND = '2. Compact percentage links';
const FIXED_LEGEND = '3. Fixed-length links';
const ALTERNATE_LEGEND = '4. Alternating backgrounds';
const WRAPPING_LEGEND = '5. Wrapping long label';
const FALLBACK_LEGEND = '6. Missing-image fallback';
const WRAPPER_LEGEND = '7. Wrapper DCE theme';
const IMAGE_BUTTON_LEGEND = '8. Image-button presentation';
const LOCAL_LOGOS = [
    'wc-square',
    'angular',
    'semantic-ui',
    'open-wc',
    'flutter',
    'refine',
    'bootstrap',
    'vue',
    'lit',
    'redux',
    'svelte',
    'solid',
    'nextjs',
] as const;

describe('hex-grid demo source contract', () => {
    it('maps the local CEM-ML formatter before loading module scripts', () => {
        const importMapIndex = DEMO_SOURCE.indexOf('"@epa-wg/cem-ml/wasm"');
        const moduleScriptIndex = DEMO_SOURCE.indexOf('<script type="module">');
        expect(importMapIndex).toBeGreaterThan(-1);
        expect(DEMO_SOURCE).toContain(
            '"@epa-wg/cem-ml/wasm": "../../cem-ml-npm/dist/wasm/browser/cem_ml.js"',
        );
        expect(importMapIndex).toBeLessThan(moduleScriptIndex);
    });

    it('documents the responsive honeycomb purpose and its related features', () => {
        const normalized = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(normalized).toContain('Render responsive navigation as a honeycomb');
        expect(normalized).toContain('semantic list');
        expect(normalized).toContain('occupies 95% of its responsive');
        expect(DEMO_SOURCE).toContain('<code>.hex</code> cell');
        expect(normalized).toContain('Change every cell and link together');
        expect(DEMO_SOURCE).toContain('defaults to\n            <code>72rem</code>');
        expect(DEMO_SOURCE).toContain('CSS lengths such as <code>35rem</code> are also accepted');
        expect(normalized).toContain('Background alternation is disabled by default');
        expect(DEMO_SOURCE).toContain('<code>alternate="true"</code>');
        expect(normalized).toContain('white image plate is only a loading/error fallback');
        expect(normalized).toContain('A wrapper DCE can theme a nested grid');
        expect(DEMO_SOURCE).toContain('<code>--cem-hex-background-*</code>');
        expect(DEMO_SOURCE).toContain('<code>--cem-hex-hover-background-*</code>');
        expect(DEMO_SOURCE).toContain('<code>--cem-hex-label-*</code>');
        expect(DEMO_SOURCE).toContain('<code>--cem-hex-link-hover-filter</code>');
        expect(DEMO_SOURCE).toContain('https://web-tiki.github.io/responsive-grid-of-hexagons/');
        expect(DEMO_SOURCE).toContain('href="./for-each.html"');
        expect(DEMO_SOURCE).toContain('href="./module-url.html"');
        expect(DEMO_SOURCE).toContain('href="./scoped-css.html"');
    });

    it('keeps one focused use case per demo element', () => {
        const legends = Array.from(
            DEMO_SOURCE.matchAll(/<cem-demo-element[\s\S]*?legend="([^"]+)"/gu),
            (match) => match[1],
        );
        expect(legends).toEqual([
            LEGEND,
            COMPACT_LEGEND,
            FIXED_LEGEND,
            ALTERNATE_LEGEND,
            WRAPPING_LEGEND,
            FALLBACK_LEGEND,
            WRAPPER_LEGEND,
            IMAGE_BUTTON_LEGEND,
        ]);
        expect(DEMO_SOURCE).toContain('<template>\n<cem-element tag="cem-hex-image-link">');
        expect(DEMO_SOURCE).toContain('<cem-element tag="cem-hex-grid">');
        expect(DEMO_SOURCE).not.toContain('data-testid=');
    });

    it('accepts semantic link-with-image payload and resolves both URL attributes', () => {
        expect(DEMO_SOURCE).toContain('{cem-module-url @slice=linkUrl @src="{$href}"}');
        expect(DEMO_SOURCE).toContain('{cem-module-url @slice=imageUrl @src="{$src}"}');
        expect(DEMO_SOURCE).toContain('@select=datadom.payload.nodes');
        expect(DEMO_SOURCE).toContain('@select="$link.children"');
        expect(DEMO_SOURCE).toContain('@href="{$link.attributes.href}"');
        expect(DEMO_SOURCE).toContain('@src="{$image.attributes.src}"');
        expect(DEMO_SOURCE).toContain('@class=hex-grid');
        expect(DEMO_SOURCE).toContain('{li @class="hex hex-{$alternate}"');
        expect(DEMO_SOURCE).toContain('{a  @class=hex-link');
        expect(DEMO_SOURCE.match(/<a href="[^"]+">\s*<img[\s\S]*?alt="[^"]+">\s*<\/a>/gu)).toHaveLength(35);
        expect(DEMO_SOURCE).toContain(
            '<a href="./module-url.html"><img src="./framework-logos/wc-square.svg" alt="DCE"></a>',
        );
        expect(DEMO_SOURCE).toContain(
            '<a href="https://react.dev/"><img src="https://upload.wikimedia.org/wikipedia/commons/a/a7/React-icon.svg" alt="React"></a>',
        );
        expect(DEMO_SOURCE).not.toContain('assetbase=');
        expect(DEMO_SOURCE).not.toContain(' framework logo=');
    });

    it('supports default, percentage, and fixed-length honeycomb widths', () => {
        expect(DEMO_SOURCE).toContain('width: min(100%, var(--cem-hex-grid-width, 72rem))');
        expect(DEMO_SOURCE).toContain('{attribute @name=size | 72rem}');
        expect(DEMO_SOURCE).toContain('@style="--cem-hex-grid-width: {$size}"');
        expect(DEMO_SOURCE).toContain('<cem-hex-grid size="65%">');
        expect(DEMO_SOURCE).toContain('<cem-hex-grid size="35rem">');
        expect(DEMO_SOURCE.match(/<cem-hex-grid[^>]+size=/gu)).toHaveLength(6);
    });

    it('keeps backgrounds uniform by default and makes alternation opt-in', () => {
        expect(DEMO_SOURCE).toContain('{attribute @name=alternate | false}');
        expect(DEMO_SOURCE).toContain('@class="hex hex-{$alternate}"');
        expect(DEMO_SOURCE).toContain('.hex-true:nth-child(3n + 2)');
        expect(DEMO_SOURCE).toContain('.hex-true:nth-child(3n)');
        expect(DEMO_SOURCE).toContain('<cem-hex-grid alternate="true">');
        expect(DEMO_SOURCE).not.toContain('.hex:nth-child(3n + 2) { --hex-start:');
    });

    it('shows the white plate only while an image is loading or unavailable', () => {
        expect(DEMO_SOURCE).not.toContain('radial-gradient');
        expect(DEMO_SOURCE).toContain('{slice @name=imageState | loading}');
        expect(DEMO_SOURCE).toContain('@class="image-fallback image-fallback-{$datadom.slices.imageState}"');
        expect(DEMO_SOURCE).toContain('@class="hex-logo hex-logo-{$datadom.slices.imageState}"');
        expect(DEMO_SOURCE).toContain('@slice-event="load error"');
        expect(DEMO_SOURCE).toContain('@slice-value="$event.type"');
        expect(DEMO_SOURCE).toContain('.image-fallback-load');
        expect(DEMO_SOURCE).toContain('.hex-logo-load { opacity: 1; }');
        expect(DEMO_SOURCE).toContain('alt="Declarative Custom Element Framework With A Long Name"');
        expect(DEMO_SOURCE).toContain('src="./framework-logos/README.md" alt="Unavailable logo"');
    });

    it('lets a wrapper DCE theme a projected grid through public properties', () => {
        expect(DEMO_SOURCE).toContain('<cem-element tag="cem-themed-framework-grid">');
        expect(DEMO_SOURCE).toContain('<template type="text/cem-ml">\n{style |```');
        expect(DEMO_SOURCE).toContain('{section @class=theme-frame');
        expect(DEMO_SOURCE).toContain('{cem:project-payload @select=datadom.payload.nodes | }');
        expect(DEMO_SOURCE).toContain('--hex-start: var(--cem-hex-background-start, #dbeafe)');
        expect(DEMO_SOURCE).toContain('--hex-end: var(--cem-hex-background-end, #ccfbf1)');
        expect(DEMO_SOURCE).toContain('color: var(--cem-hex-label-color, white)');
        expect(DEMO_SOURCE).toContain('background: var(--cem-hex-label-background, rgb(15 118 110 / 0.88))');
        expect(DEMO_SOURCE).toContain('--cem-hex-background-start: #312e81');
        expect(DEMO_SOURCE).toContain('--cem-hex-label-background: rgb(254 240 138 / 0.92)');
        expect(DEMO_SOURCE).toContain('--cem-hex-hover-background-start: green');
        expect(DEMO_SOURCE).toContain('--cem-hex-hover-background-end: yellow');
        expect(DEMO_SOURCE).toContain('var(--cem-hex-hover-background-start, var(--hex-start))');
        expect(DEMO_SOURCE).toContain('var(--cem-hex-hover-background-end, var(--hex-end))');
        expect(DEMO_SOURCE).not.toContain(':host:hover {');
        expect(DEMO_SOURCE).not.toContain('&:hover a');
    });

    it('offers a wrapper-styled image-button presentation', () => {
        expect(DEMO_SOURCE).toContain('<cem-element tag="cem-image-button-grid">');
        expect(DEMO_SOURCE).toContain('--label-shift: var(--cem-hex-label-shift, 220%)');
        expect(DEMO_SOURCE).toContain('filter: var(--cem-hex-link-filter, none)');
        expect(DEMO_SOURCE).toContain('filter: var(--cem-hex-link-hover-filter,');
        expect(DEMO_SOURCE).toContain('--cem-hex-label-shift: 0%');
        expect(DEMO_SOURCE).toContain('--cem-hex-link-hover-filter:');
        expect(DEMO_SOURCE).toContain('drop-shadow(0 0.75rem 0.55rem rgb(15 23 42 / 0.48))');
    });

    it('adapts Web Tiki row geometry with current CSS primitives', () => {
        for (const required of [
            'display: flex',
            'flex-wrap: wrap',
            'aspect-ratio: 0.8660254',
            'clip-path: polygon',
            '.hex:nth-child(9n + 6)',
            '.hex:nth-child(7n + 5)',
            '.hex:nth-child(5n + 4)',
            '.hex:nth-child(3n + 3)',
            '@container (max-width: 28rem)',
            '.hex-link:focus-visible',
            '.hex-link:focus-visible::after',
            '--focus-stripe: var(--cem-zebra-strip-size, 2px)',
            '--focus-slope: calc(1.5 * var(--focus-stripe))',
            'var(--cem-zebra-angle, 45deg)',
            'var(--cem-zebra-color-1, CanvasText)',
            'clip-path: polygon(evenodd,',
            '@media (forced-colors: active)',
            'container-type: inline-size',
            'box-sizing: border-box',
            'inset: auto 0 16%',
            'min-block-size: clamp(2rem, 18cqi, 2.75rem)',
            'text-wrap: balance',
            'overflow-wrap: anywhere',
        ]) {
            expect(DEMO_SOURCE, `missing CSS principle ${required}`).toContain(required);
        }
    });

    it('vendors every relative framework logo used by the payload', () => {
        for (const logo of LOCAL_LOGOS) {
            expect(DEMO_SOURCE).toContain(`src="./framework-logos/${logo}.svg"`);
            expect(
                existsSync(fileURLToPath(new URL(`../../demo/framework-logos/${logo}.svg`, import.meta.url))),
                `missing local ${logo} logo`,
            ).toBe(true);
        }
    });
});
