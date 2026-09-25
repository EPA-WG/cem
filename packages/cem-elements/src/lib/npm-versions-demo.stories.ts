import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';
import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';
import { cemDiagnosticCodes, traceCemReadiness, whenCemSourceRendered } from '../../.storybook/preview.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';

const SOURCE_TAG = 'story-npm-versions-demo-document';
const DEMO_URL = new URL('../../demo/npm-versions-demo.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Default to the latest version',
    '2. Preselect a version and show dates',
    '3. Propagate the selected value',
    '4. Override the label slot',
    '5. Synchronize the selected version with the URL',
] as const;
const PREVIEW_FILES = ['npm-versions.json'] as const;
const RELEASES = [
    ['0.1.0', '2026-08-01'], ['0.0.25', '2024-05-18'],
    ['0.0.22', '2024-04-20'], ['0.0.21', '2024-03-09'],
] as const;

const meta: Meta = {
    title: 'CEM Elements/NPM Versions Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(),
    play: async ({ canvasElement, step }) => {
        const originalUrl = location.href;
        const originalState = history.state;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        try {
            await waitForCondition(
                () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length + PREVIEW_FILES.length,
                'all five npm-version samples render from the HTML source',
                300
            );
            assertDeepEqual(sampleLegends(host), [...EXPECTED_LEGENDS, ...PREVIEW_FILES], 'npm-version sample inventory');
            await whenCemSourceRendered(host);
            readinessCheckpoint('initial-render-settled');
            // Initial rendering can precede HTTP completion. Keep the resource
            // and interaction predicates below at their existing frame limits.
            const samples = EXPECTED_LEGENDS.map(legend => sampleByLegend(host, legend));
            for (const [index, sample] of samples.entries()) {
                await waitForCondition(() => sample.querySelectorAll('select option').length === RELEASES.length,
                    `${EXPECTED_LEGENDS[index]} loads all releases`);
                const select = requiredSelect(sample);
                expect(Array.from(select.options, option => option.value)).toEqual(RELEASES.map(([version]) => version));
                expect(Array.from(select.options, option => normalize(option.textContent ?? ''))).toEqual(
                    RELEASES.map(([version, date]) => index === 1 || index === 4 ? `${version} - ${date}` : version)
                );
                expect(select.labels).toHaveLength(1);
                expect(select.labels?.[0]).toBe(sample.querySelector('label'));
                expect(select.name).toBe('version');
                expect(normalize(sample.querySelector('label span')?.textContent ?? '')).toBe(
                    index === 3 ? 'Select a release:' : '@epa-wg/cem-elements version:'
                );
                expect(picker(sample).getAttribute('value')).toBe('');
            }
            const [defaults, preselected, propagated, label, url] = samples;
            await step(EXPECTED_LEGENDS[0], async () => {
                expect(requiredSelect(defaults).value).toBe('0.1.0');
                await verifySelection(defaults, '0.0.25');
                await verifySelection(defaults, '0.1.0');
            });
            await step(EXPECTED_LEGENDS[1], async () => {
                expect(requiredSelect(preselected).value).toBe('0.0.22');
                await verifySelection(preselected, '0.0.21');
                await verifySelection(preselected, '0.0.22');
                expect(picker(preselected).getAttribute('initialversion')).toBe('0.0.22');
            });
            await step(EXPECTED_LEGENDS[2], async () => {
                expect(normalize(propagated.querySelector('output')?.textContent ?? '')).toBe('');
                for (const value of ['0.0.25', '0.0.21', '0.0.25']) await verifySelection(propagated, value, true);
            });
            await step(EXPECTED_LEGENDS[3], async () => {
                expect(label.querySelector('label code')).toBeNull();
                expect(normalize(label.querySelector('i[slot="label"]')?.textContent ?? '')).toBe('Select a release:');
                for (const value of ['0.0.21', '0.1.0', '0.0.21']) await verifySelection(label, value, true);
            });
            await step(EXPECTED_LEGENDS[4], async () => {
                buttonByName(url, 'Set URL to 0.0.22').click();
                await verifyUrl(url, '#version=0.0.22', '0.0.22', '');
                choose(url, '0.1.0');
                await verifyUrl(url, '#version=0.1.0', '0.1.0', '0.1.0');
                buttonByName(url, 'Set URL to 0.0.25').click();
                await verifyUrl(url, '#version=0.0.25', '0.0.25', '0.1.0');
                history.back();
                await verifyUrl(url, '#version=0.1.0', '0.1.0', '0.1.0');
                history.forward();
                await verifyUrl(url, '#version=0.0.25', '0.0.25', '0.1.0');
                choose(url, '0.1.0');
                await waitForCondition(() => location.hash === '#version=0.1.0', 'user selection reaches the URL');
                buttonByName(url, 'Set URL to 0.0.25').click();
                await verifyUrl(url, '#version=0.0.25', '0.0.25', '0.1.0');
                buttonByName(url, 'Clear URL version').click();
                await verifyUrl(url, '', '0.1.0', '0.1.0');
            });
            await step(PREVIEW_FILES[0], async () => {
                await verifyExternalFilePreviews(host, DEMO_URL, PREVIEW_FILES);
            });
            // Later selections and URL navigation must not alter sibling pickers.
            expect(samples.slice(0, 4).map(sample => requiredSelect(sample).value)).toEqual(
                ['0.1.0', '0.0.22', '0.0.25', '0.0.21']
            );
            expect(samples.slice(0, 4).map(sample => picker(sample).getAttribute('value'))).toEqual(
                ['0.1.0', '0.0.22', '0.0.25', '0.0.21']
            );
            for (const relative of ['../index.html', './http-request.html', './location-element.html', './set-url.html']) {
                const target = new URL(relative, DEMO_URL).href;
                expect(Array.from(host.querySelectorAll('nav a, main > section a'), link => (link as HTMLAnchorElement).href))
                    .toContain(target);
            }
            await whenCemSourceRendered(host);
            expect(cemDiagnosticCodes(host)).toEqual([]);
            for (const declaration of canvasElement.querySelectorAll<HTMLElement>('cem-element[tag]')) {
                expect(cemDiagnosticCodes(declaration)).toEqual([]);
                const tag = declaration.getAttribute('tag');
                if (!tag) throw new Error('expected a produced tag');
                for (const instance of canvasElement.querySelectorAll<HTMLElement>(tag)) {
                    expect(cemDiagnosticCodes(instance)).toEqual([]);
                }
            }
        } finally {
            history.replaceState(originalState, '', originalUrl);
        }
    },
};

function picker(sample: ParentNode): HTMLElement {
    return requiredElement(sample, '[package]');
}

async function verifySelection(sample: ParentNode, value: string, propagated = false): Promise<void> {
    const select = requiredSelect(sample);
    choose(sample, value);
    await waitForCondition(() => requiredSelect(sample).value === value && picker(sample).getAttribute('value') === value
        && (!propagated || normalize(sample.querySelector('output')?.textContent ?? '') === value),
    `selection ${value} reaches the live picker, reflected value and wrapper`);
    expect(requiredSelect(sample)).toBe(select);
    expect(Array.from(select.selectedOptions, option => option.value)).toEqual([value]);
}

async function verifyUrl(sample: ParentNode, hash: string, version: string, lastSelection: string): Promise<void> {
    await waitForCondition(() => location.hash === hash && requiredSelect(sample).value === version
        && (picker(sample).getAttribute('currentversion') ?? '') === (hash ? version : '')
        && (requiredSelect(sample).getAttribute('value') ?? '') === (hash ? version : '')
        && picker(sample).getAttribute('value') === lastSelection
        && JSON.stringify(Array.from(sample.querySelectorAll('output'), output => normalize(output.textContent ?? '')))
            === JSON.stringify([hash, lastSelection]),
    `URL ${hash || '(empty)'} selects ${version} and retains user selection ${lastSelection || '(none)'}`).catch(error => {
        throw new Error(`${error.message}; currentversion=${JSON.stringify(picker(sample).getAttribute('currentversion'))}; defaults=${JSON.stringify(Array.from(sample.querySelectorAll<HTMLOptionElement>('option[selected]'), option => option.value))}`, { cause: error });
    });
}

function sourceLoadedDemo(): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded npm versions demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', SOURCE_TAG);
    declaration.setAttribute('src', DEMO_URL.href);
    declaration.setAttribute('link-base', 'source');
    root.append(declaration, document.createElement(SOURCE_TAG));
    traceCemReadiness(root, 'npm/EveryAuthoredSample');
    return root;
}

function sampleLegends(host: ParentNode): string[] {
    return Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
        normalize(sample.getAttribute('legend') ?? '')
    );
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function requiredSelect(root: ParentNode): HTMLSelectElement {
    const select = root.querySelector('select');
    if (!(select instanceof HTMLSelectElement)) throw new Error('expected version select');
    return select;
}

function choose(root: ParentNode, value: string): void {
    const select = requiredSelect(root);
    select.value = value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
}

function buttonByName(root: ParentNode, expected: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(
        (candidate) => normalize(candidate.getAttribute('aria-label') ?? candidate.textContent ?? '') === expected
    );
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 200): Promise<void> {
    const mark = readinessWait(message, attempts);
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) {
            mark('ready', attempt);
            return;
        }
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    mark('timeout', attempts);
    throw new Error(message);
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
