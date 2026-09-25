import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';
import { cemDiagnosticCodes, traceCemReadiness, whenCemRendered } from '../../.storybook/preview.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';

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
    play: async ({ canvasElement, step }) => {
        const originalUrl = location.href;
        const originalState = history.state;
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
            const initialValues = definitionEntries(initial);
            readinessCheckpoint('initial-values-captured', { readers: readers() });
            const initialUrl = new URL(originalUrl);
            expect(initialValues).toEqual([
                ['source', 'window'], ['origin', initialUrl.origin], ['pathname', initialUrl.pathname],
                ['search', initialUrl.search], ['hash', initialUrl.hash],
            ]);
            const liveArticle = requiredElement(live, 'article');
            const initialArticle = requiredElement(initial, 'article');
            const input = requiredElement(live, 'input') as HTMLInputElement;
            const form = requiredElement(live, 'form') as HTMLFormElement;
            const liveState = async (url: URL) => {
                const expected = [['href', url.href], ['pathname', url.pathname], ['hash', url.hash]];
                await waitForCondition(() => location.href === url.href
                    && JSON.stringify(definitionEntries(live)) === JSON.stringify(expected)
                    && JSON.stringify(textList(live, 'li')) === JSON.stringify(parameterRows(url)),
                'live URL fields and parameters match the browser');
                await whenCemRendered(liveArticle.parentElement as HTMLElement);
                expect(requiredElement(live, 'article')).toBe(liveArticle);
                expect(requiredElement(live, 'input')).toBe(input);
                expect(requiredElement(initial, 'article')).toBe(initialArticle);
                expect(definitionEntries(initial)).toEqual(initialValues);
            };
            await step(EXPECTED_LEGENDS[0], async () => {
                await liveState(initialUrl);
                expect(input.value).toBe('hello world');
                expect(input.labels?.[0]?.textContent?.trim()).toBe('Query');
                expect(form.method).toBe('get');
                expect(form.action).toBe(location.href);
                const historyLength = history.length;
                const pushed = new URL('?mode=history.pushState&tag=one&tag=two#checked', originalUrl);
                const replaced = new URL('?mode=history.replaceState&tag=one&tag=two#checked', originalUrl);
                await userEvent.click(buttonByName(live, 'history.pushState'));
                await liveState(pushed);
                expect(history.length).toBe(historyLength + 1);
                await userEvent.click(buttonByName(live, 'history.replaceState'));
                await liveState(replaced);
                expect(history.length).toBe(historyLength + 1);

                const link = requiredElement(live, 'a') as HTMLAnchorElement;
                const linked = new URL('#native-link', replaced);
                expect(link.href).toBe(linked.href);
                // Vitest installs <base target="_parent">. Actual link navigation,
                // Back/Forward and GET run in both independent gallery modes.

                // Typing is only a draft until native GET submission (tested in the gallery).
                await userEvent.clear(input);
                await userEvent.type(input, 'pear & cherry');
                await liveState(replaced);
                expect(document.activeElement).toBe(input);
                expect(input.selectionStart).toBe(input.value.length);
                expect(Array.from(new FormData(form))).toEqual([['query', 'pear & cherry']]);
                const submit = buttonByName(live, 'Navigate with GET (reloads)');
                expect(submit.type).toBe('submit');
                expect(submit.title).toBe('Navigate with GET (reloads)');
                expect(normalize(submit.textContent ?? '')).toBe('→ GET ↻');
            });

            await step(EXPECTED_LEGENDS[1], async () => {
                const changed = new URL('#after-initial-read', location.href);
                await userEvent.click(buttonByName(initial, 'Change hash after initial read'));
                await liveState(changed);
                await whenCemRendered(initialArticle.parentElement as HTMLElement);
                expect(definitionEntries(initial)).toEqual(initialValues);
            });

            await step(EXPECTED_LEGENDS[2], async () => {
                const external = sampleByLegend(host, EXPECTED_LEGENDS[2]);
                const url = new URL('https://my.example/docs?a=1&b=2&b=3#details');
                const expected = [['href', url.href], ['hostname', url.hostname], ['pathname', url.pathname], ['hash', url.hash]];
                const current = location.href;
                await waitForCondition(() => JSON.stringify(definitionEntries(external)) === JSON.stringify(expected)
                    && JSON.stringify(textList(external, 'li')) === JSON.stringify(['a = 1', 'b = 2,3']),
                'the external href retains every URL field and repeated parameter');
                expect(location.href).toBe(current);
                expect(external.querySelectorAll('li')).toHaveLength(2);
            });
            for (const legend of EXPECTED_LEGENDS) {
                const owner = requiredElement(sampleByLegend(host, legend), 'article').parentElement as HTMLElement;
                await whenCemRendered(owner);
                expect(cemDiagnosticCodes(owner), legend).toEqual([]);
            }
            expect(cemDiagnosticCodes(host)).toEqual([]);
            readinessCheckpoint('location-journey-verified', { readers: readers() });
        } catch (error) {
            readinessCheckpoint('location-assertion-failed', { readers: readers() });
            throw error;
        } finally {
            history.replaceState(originalState, '', originalUrl);
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

function definitionEntries(root: ParentNode): string[][] {
    return Array.from(root.querySelectorAll('dt'), term => [normalize(term.textContent ?? ''),
        normalize(term.nextElementSibling?.textContent ?? '')]);
}

function textList(root: ParentNode, selector: string): string[] {
    return Array.from(root.querySelectorAll(selector), element => normalize(element.textContent ?? ''));
}

function parameterRows(url: URL): string[] {
    return [...new Set(url.searchParams.keys())].map(name => normalize(`${name} = ${url.searchParams.getAll(name).join(',')}`));
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
