import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';
import { traceCemReadiness } from '../../.storybook/preview.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-location-element-demo-document';
const DEMO_URL = new URL('../../demo/location-element.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Window location live update',
    '2. Window location initial read',
    '3. External URL from href',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Location Element Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(SOURCE_TAG, DEMO_URL),
    play: async ({ canvasElement }) => {
        const originalUrl = location.href;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        // Only counts and readiness booleans enter the trace, never URL values.
        const readers = () => Array.from(host.querySelectorAll('cem-demo-element[legend]'), sample => {
            const field = (term: string) => Array.from(sample.querySelectorAll('dt'))
                .find(dt => normalize(dt.textContent ?? '') === term)?.nextElementSibling?.textContent?.trim();
            return { legend: sample.getAttribute('legend'), definitions: sample.querySelectorAll('dd').length,
                windowSource: field('source') === 'window', originMatches: field('origin') === location.origin,
                pathnamePublished: !!field('pathname') };
        });
        try {
            await waitForCondition(
                () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
                'all three location-element samples render from the HTML source'
            );
            assertDeepEqual(sampleLegends(host), [...EXPECTED_LEGENDS], 'location sample inventory');

            const live = sampleByLegend(host, EXPECTED_LEGENDS[0]);
            const initial = sampleByLegend(host, EXPECTED_LEGENDS[1]);
            try {
                await waitForCondition(
                    () => live.querySelector('dd') !== null && initial.querySelectorAll('dd').length === 5,
                    'both location readers finish rendering'
                );
            } catch (error) {
                readinessCheckpoint('initial-readers-failed', { readers: readers() });
                throw error;
            }
            readinessCheckpoint('initial-readers-ready', { readers: readers() });
            const initialValues = normalize(initial.querySelector('dl')?.textContent ?? '');
            readinessCheckpoint('initial-values-captured', { readers: readers() });
            buttonByName(live, 'history.pushState').click();
            await waitForCondition(
                () => definitionValue(live, 'hash') === '#checked'
                    && normalize(live.querySelector('ul')?.textContent ?? '').includes('mode = history.pushState')
                    && normalize(live.querySelector('ul')?.textContent ?? '').includes('tag = one,two'),
                'the live reader observes the history write and repeated parameters'
            );

            await waitForCondition(
                () => definitionValue(initial, 'source') === 'window'
                    && definitionValue(initial, 'origin') === location.origin,
                'the initial reader publishes current URL fields'
            );
            buttonByName(live, 'history.replaceState').click();
            await waitForCondition(
                () => normalize(live.querySelector('ul')?.textContent ?? '').includes('mode = history.replaceState'),
                'replaceState is observed independently'
            );
            buttonByName(initial, 'Change hash after initial read').click();
            await waitForCondition(() => definitionValue(live, 'hash') === '#after-initial-read', 'native hash change reaches the live reader');
            if (normalize(initial.querySelector('dl')?.textContent ?? '') !== initialValues) {
                throw new Error('the initial-only reader changed after navigation');
            }

            const external = sampleByLegend(host, EXPECTED_LEGENDS[2]);
            await waitForCondition(
                () => definitionValue(external, 'hostname') === 'my.example'
                    && definitionValue(external, 'pathname') === '/docs'
                    && definitionValue(external, 'hash') === '#details'
                    && normalize(external.querySelector('ul')?.textContent ?? '').includes('b = 2,3'),
                'the href reader parses an external URL and repeated parameters'
            );
            readinessCheckpoint('location-journey-verified', { readers: readers() });
        } catch (error) {
            readinessCheckpoint('location-assertion-failed', { readers: readers() });
            throw error;
        } finally {
            history.replaceState({}, '', originalUrl);
        }
    },
};

function sourceLoadedDemo(tag: string, url: URL): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded location-element demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', url.href);
    root.append(declaration, document.createElement(tag));
    traceCemReadiness(root, 'location/EveryAuthoredSample');
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

function buttonByName(root: ParentNode, expected: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(
        (candidate) => normalize(candidate.getAttribute('aria-label') ?? candidate.textContent ?? '') === expected
    );
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
}

function definitionValue(root: ParentNode, term: string): string {
    const dt = Array.from(root.querySelectorAll('dt')).find(
        (candidate) => normalize(candidate.textContent ?? '') === term
    );
    const dd = dt?.nextElementSibling;
    if (!(dd instanceof HTMLElement) || dd.localName !== 'dd') throw new Error(`expected ${term} value`);
    return normalize(dd.textContent ?? '');
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 180): Promise<void> {
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
