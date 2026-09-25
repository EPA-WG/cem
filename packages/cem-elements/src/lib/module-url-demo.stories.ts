import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
import { cemDiagnosticCodes, traceCemReadiness, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-module-url-document';
const MODULE_URL_DEMO_URL = new URL('../../demo/module-url.html', import.meta.url);
const EXPECTED_LEGENDS = [
    'this page import maps',
    '1. module path by symbolic name',
    '2. src forms: relative URL',
    '3. src forms: absolute URL',
    '4. Relative declaration source',
    '4a. Mapped declaration source',
    '4b. Missing import-map entry',
    '4c. Mapped fragment with a relative dependency',
    '4d. Mapped image and same-library fragment',
    '5. component-local map: naked',
    '6. component-local map: wrapper override',
    '7. component-local map: node referrer',
    'image-link',
] as const;

const meta: Meta = { title: 'CEM Elements/Module URL Demo', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded module URL demo coverage');
        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', MODULE_URL_DEMO_URL.href);
        declaration.setAttribute('link-base', 'source');
        root.append(declaration, document.createElement(SOURCE_TAG));
        traceCemReadiness(root, 'module-url/EveryAuthoredSample');
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const originalUrl = location.href;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await whenCemSourceRendered(host);
        readinessCheckpoint('module-url-initial-settled');
        await waitForCondition(() => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all module-url samples render from the HTML source');
        const samples = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]'));
        expect(samples.map(sample => normalize(sample.getAttribute('legend') ?? ''))).toEqual(EXPECTED_LEGENDS);
        const sourceResponse = await fetch(MODULE_URL_DEMO_URL);
        expect(sourceResponse.ok).toBe(true);
        const sourceDocument = new DOMParser().parseFromString(await sourceResponse.text(), 'text/html');
        const url = (relative: string) => new URL(relative, MODULE_URL_DEMO_URL).href;
        const library = url('./lib-dir/embed-lib.html');
        const imageUrls: Record<number, string> = {
            1: url('./wc-square.svg'), 2: url('./lib-dir/Smiley.svg?src=relative'),
            9: url('./lib-dir/Smiley.svg?owner=component'), 10: url('./confused.svg?owner=wrapper'),
            11: url('./wc-square.svg?owner=component'), 12: url('./confused.svg'),
        };
        for (const [index, sample] of samples.entries()) {
            await step(EXPECTED_LEGENDS[index], async () => {
                readinessCheckpoint('module-url-sample-start', { index, legend: EXPECTED_LEGENDS[index],
                    images: Array.from(sample.querySelectorAll('img'), image => ({
                        complete: image.complete, decoded: image.naturalWidth > 0,
                    })) });
                if (index === 0) {
                    const authoredMap = JSON.parse(sourceDocument.querySelector('script[type="importmap"]')?.textContent ?? '');
                    // The displayed map documents example resources, omitting the
                    // formatter's browser bootstrap entry.
                    delete authoredMap.imports['@epa-wg/cem-ml/wasm'];
                    expect(JSON.parse(requiredElement(sample, '[slot="demo"] pre').textContent ?? ''))
                        .toEqual(authoredMap);
                } else if (index in imageUrls || index === 3) {
                    const helper = requiredElement(sample, 'image-link');
                    await whenCemRendered(helper);
                    const expected = index === 3 ? helper.getAttribute('src') ?? '' : imageUrls[index];
                    if (index === 3) expect(expected.startsWith('data:image/svg+xml,')).toBe(true);
                    await waitForCondition(() => {
                        const image = helper.querySelector<HTMLImageElement>('img');
                        return image?.getAttribute('src') === expected && image.complete && image.naturalWidth > 0
                            && helper.querySelector('a')?.getAttribute('href') === expected;
                    }, `decoded image and complete link for ${EXPECTED_LEGENDS[index]}`, 400);
                    const anchor = requiredElement(helper, 'a');
                    expect(normalize(anchor.textContent ?? '')).toBe(shortenMiddle(expected, 32));
                    const disclosure = requiredElement(helper, 'details') as HTMLDetailsElement;
                    const summary = requiredElement(disclosure, 'summary');
                    expect(disclosure.open).toBe(false);
                    await userEvent.click(summary);
                    expect(disclosure.open).toBe(true);
                    expect(disclosure.textContent).toContain(expected);
                    await userEvent.click(summary);
                    expect(disclosure.open).toBe(false);
                    expect(requiredElement(helper, 'details')).toBe(disclosure);
                    if (index === 11) {
                        expect(Array.from(sample.querySelectorAll('thead th'), th => normalize(th.textContent ?? '')))
                            .toEqual(['relative URL src', 'module path src', 'absolute URL src']);
                        expect(Array.from(sample.querySelectorAll('table td a'), a => a.getAttribute('href'))).toEqual([
                            url('./lib-dir/Smiley.svg?referrer=node'), imageUrls[11], 'https://assets.example.test/logo.svg',
                        ]);
                        expect(sample.querySelector('cem-local-map-referrer')?.textContent)
                            .toContain('Child owns the inner-only module mapping');
                    }
                    if (index === 10) {
                        const comparison = requiredElement(sample, 'cem-local-map-override-wrapper > img') as HTMLImageElement;
                        await waitForCondition(() => comparison.src === url('./lib-dir/Smiley.svg')
                            && comparison.complete && comparison.naturalWidth > 0, 'decoded source-relative comparison image');
                    }
                } else if (index === 6) {
                    expect(normalize(requiredElement(sample, 'output').textContent ?? '')).toBe('not published');
                } else if (index === 8) {
                    await waitForCondition(() => sample.querySelectorAll('article img').length === 2
                        && sample.querySelector('article')?.textContent?.includes('👋 from embed-lib-component') === true
                        && Array.from(sample.querySelectorAll<HTMLImageElement>('article img'))
                            .every(image => image.src === url('./lib-dir/Smiley.svg') && image.complete && image.naturalWidth > 0),
                    'both mapped/library images and the nested declaration render', 900);
                    expect(requiredElement(sample, 'article > a').getAttribute('href')).toBe(`${library}#embed-relative-hash`);
                    expect(sample.querySelector('img[alt="Library Smiley"]')?.closest('a')?.getAttribute('href'))
                        .toBe(`${library}#embed-lib-component`);
                } else {
                    const expected = index === 4 ? url('./embed-1.html')
                        : index === 5 ? library : `${library}#embed-relative-file`;
                    const tag = index === 4 ? 'cem-module-relative-declaration'
                        : index === 5 ? 'cem-module-mapped-declaration' : 'cem-module-mapped-fragment';
                    const instance = requiredElement(sample, tag);
                    await whenCemRendered(instance);
                    await waitForCondition(() => normalize(sample.querySelector('output')?.textContent ?? '') === expected
                        && instance.innerText.includes(index === 5 ? '👋 from embed-lib-component' : '🖖'),
                    `resolved URL and loaded declaration for ${EXPECTED_LEGENDS[index]}`, 300);
                    expect((index === 4 ? instance.closest('a') : sample.querySelector('output')?.closest('a'))?.getAttribute('href'))
                        .toBe(expected);
                    expect(normalize(instance.innerText)).not.toContain(index === 4 ? 'loading ./embed-1.html'
                        : index === 5 ? 'failed to load embed-lib' : 'failed to load mapped fragment');
                    if (index === 4) expect(normalize(instance.innerText)).toBe('embed-1.html 🖖');
                    if (index === 7) {
                        expect(instance.textContent).toContain('👍 from embed-relative-file');
                        expect(requiredElement(instance, 'a').getAttribute('href')).toBe(url('./embed-1.html'));
                    }
                }
                const declarations = Array.from(sample.querySelectorAll<HTMLElement>('cem-element[tag]'));
                const diagnostics = new Set<string>();
                for (const declaration of declarations) {
                    expect(cemDiagnosticCodes(declaration)).toEqual([]);
                    for (const instance of sample.querySelectorAll<HTMLElement>(declaration.getAttribute('tag') ?? '')) {
                        await whenCemRendered(instance);
                        for (const code of cemDiagnosticCodes(instance)) diagnostics.add(code);
                    }
                }
                expect([...diagnostics]).toEqual(index === 6 ? ['cem-element.module_url_resolve_failed'] : []);
                const source = requiredElement(sample, '[slot="text"] pre').textContent ?? '';
                expect(source.replace(/^\r?\n/u, '').startsWith(index === 0 ? '<style>' : '<cem-element')).toBe(true);
                expect(sample.querySelector('[slot="status"]')?.textContent?.trim() ?? '').toBe('');
                readinessCheckpoint('module-url-sample-verified', { index });
            }).catch(error => {
                readinessCheckpoint('module-url-sample-failed', { index });
                throw error;
            });
        }
        const navigation = ['../index.html', './set-url.html', './external-template.html',
            './module-url-referrer.html', './functions/str.html'];
        for (const path of navigation) {
            const anchor = Array.from(host.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'))
                .find(anchor => anchor.href === url(path));
            expect(anchor, `source navigation to ${path}`).toBeDefined();
        }
        const images = Array.from(host.querySelectorAll<HTMLImageElement>('cem-demo-element [slot="demo"] img'));
        expect(images).toHaveLength(10);
        expect(images.every(image => image.alt.trim().length > 0)).toBe(true);
        expect(host.querySelectorAll('cem-module-url')).toHaveLength(0);
        expect(cemDiagnosticCodes(host)).toEqual([]);
        expect(location.href).toBe(originalUrl);
    },
};

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

function shortenMiddle(input: string, maxLength: number): string {
    const codepoints = Array.from(input);
    if (codepoints.length <= maxLength) return input;
    const prefixLength = Math.floor((maxLength - 1) / 2);
    return `${codepoints.slice(0, prefixLength).join('')}…${codepoints.slice(-(maxLength - 1 - prefixLength)).join('')}`;
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 120): Promise<void> {
    const mark = readinessWait(message, attempts);
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) {
            mark('ready', attempt);
            return;
        }
        await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    }
    mark('timeout', attempts);
    throw new Error(message);
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
