import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
import { cemDiagnosticCodes, traceCemReadiness, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-set-url-demo-document';
const DEMO_URL = new URL('../../demo/set-url.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Set the page hash',
    '2. Select the URL write method',
    '3. Conditionally inject a URL writer',
    '4. Set URL from form controls',
] as const;
const METHODS = ['location.href', 'location.hash', 'location.assign', 'location.replace',
    'history.pushState', 'history.replaceState'] as const;

const meta: Meta = { title: 'CEM Elements/Set URL Demo', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(),
    play: async ({ canvasElement, step }) => {
        const originalUrl = location.href;
        const originalState = history.state;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        const readHash = async (hash: string): Promise<void> => {
            await waitForCondition(() => location.hash === hash
                && Array.from(host.querySelectorAll('cem-demo-element[legend]')).every(sample =>
                    Array.from(sample.querySelectorAll('output')).at(-1)?.textContent === hash),
            `all four live readers show ${hash}`);
            await whenCemSourceRendered(host);
            expect(location.hash).toBe(hash);
        };
        const command = async (button: HTMLButtonElement, method: string, hash: string): Promise<void> => {
            const before = history.length;
            const sameTarget = location.hash === hash;
            await userEvent.click(button);
            await readHash(hash);
            expect(history.length).toBe(before + (sameTarget || method === 'history.replaceState' || method === 'location.replace' ? 0 : 1));
            const after = history.length;
            await userEvent.click(button);
            await whenCemSourceRendered(host);
            expect(location.hash).toBe(hash);
            expect(history.length).toBe(after);
        };
        try {
            await waitForCondition(() => host.querySelectorAll('cem-demo-element[legend]').length === 4,
                'all four set-url samples render from the HTML source');
            await whenCemSourceRendered(host);
            const samples = EXPECTED_LEGENDS.map(legend => sampleByLegend(host, legend));
            expect(samples.map(sample => sample.getAttribute('legend'))).toEqual([...EXPECTED_LEGENDS]);
            readinessCheckpoint('set-url-initial-settled', {
                samples: samples.map(sample => ({ legend: sample.getAttribute('legend'),
                    buttons: sample.querySelectorAll('button').length,
                    outputs: sample.querySelectorAll('output').length,
                    inputs: sample.querySelectorAll('input').length })),
            });
            const initialHash = location.hash;
            expect(location.href).toBe(originalUrl);
            expect(outputs(samples[0])).toEqual(['', initialHash]);
            expect(outputs(samples[1])).toEqual(['', initialHash]);
            expect(outputs(samples[2])).toEqual([initialHash]);
            expect(outputs(samples[3])).toEqual(['history.pushState', '#form-driven', initialHash]);

            await step(EXPECTED_LEGENDS[0], async () => {
                const sample = samples[0];
                expect(Array.from(sample.querySelectorAll('button'), button => button.value)).toEqual(['#hash-one', '#hash-two']);
                for (const hash of ['#hash-one', '#hash-two']) {
                    await command(buttonByName(sample, hash), 'location.hash', hash);
                    expect(outputs(sample)).toEqual([hash, hash]);
                }
            });
            await step(EXPECTED_LEGENDS[1], async () => {
                const sample = samples[1];
                expect(Array.from(sample.querySelectorAll('button'), button => button.value)).toEqual([...METHODS]);
                for (const method of METHODS) {
                    await command(buttonByName(sample, method), method, `#${method}`);
                    expect(outputs(sample)).toEqual([method, `#${method}`]);
                }
            });
            await step(EXPECTED_LEGENDS[2], async () => {
                const button = buttonByName(samples[2], 'Set');
                expect(button.type).toBe('button');
                expect(button.title).toBe('Set');
                button.focus();
                await userEvent.keyboard('{Enter}');
                await readHash('#conditional-writer');
                expect(requiredElement(samples[2], 'button')).toBe(button);
            });
            await step(EXPECTED_LEGENDS[3], async () => {
                const sample = samples[3];
                const input = requiredElement(sample, 'input[type="text"]') as HTMLInputElement;
                expect(input.value).toBe('#form-driven');
                expect(sample.querySelectorAll('input[type="radio"]')).toHaveLength(6);
                expect((requiredElement(sample, 'input:checked') as HTMLInputElement).value).toBe('history.pushState');
                const button = buttonByName(sample, 'Set');
                for (const method of METHODS) {
                    const previousHash = location.hash;
                    const previousLength = history.length;
                    const radio = requiredElement(sample, `input[value="${method}"]`) as HTMLInputElement;
                    await userEvent.click(radio);
                    await userEvent.clear(input);
                    await userEvent.type(input, `#form-${method}`);
                    await waitForCondition(() => outputs(sample)[0] === method && outputs(sample)[1] === input.value,
                        `the ${method} draft is visible`);
                    await whenCemSourceRendered(host);
                    expect(outputs(sample)).toEqual([method, `#form-${method}`, previousHash]);
                    expect(location.hash).toBe(previousHash);
                    expect(history.length).toBe(previousLength);
                    expect(radio.checked).toBe(true);
                    expect(sample.querySelectorAll('input:checked')).toHaveLength(1);
                    await command(button, method, `#form-${method}`);
                    expect(requiredElement(sample, 'input[type="text"]')).toBe(input);
                    expect(input.value).toBe(`#form-${method}`);
                }
                // A new click can reapply the same draft after someone else navigates.
                history.replaceState({}, '', '#external-change');
                await readHash('#external-change');
                button.focus();
                await userEvent.keyboard(' ');
                await readHash('#form-history.replaceState');
                expect(document.activeElement).toBe(button);
                expect(requiredElement(sample, 'button')).toBe(button);
            });
            await step('History traversal does not replay old commands', async () => {
                await userEvent.click(buttonByName(samples[0], '#hash-one'));
                await readHash('#hash-one');
                await userEvent.click(buttonByName(samples[0], '#hash-two'));
                await readHash('#hash-two');
                await userEvent.click(buttonByName(samples[1], 'history.replaceState'));
                await readHash('#history.replaceState');
                history.back();
                await readHash('#hash-one');
                history.forward();
                await readHash('#history.replaceState');
                expect(outputs(samples[0])).toEqual(['#hash-two', '#history.replaceState']);
                expect(outputs(samples[3])).toEqual(['history.replaceState', '#form-history.replaceState', '#history.replaceState']);
            });
            expect(Array.from(host.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
                .toEqual(['../index.html', './location-element.html', './data-slices.html'].map(relative => new URL(relative, DEMO_URL).href));
            expect(cemDiagnosticCodes(host)).toEqual([]);
            for (const declaration of canvasElement.querySelectorAll<HTMLElement>('cem-element[tag]')) {
                expect(cemDiagnosticCodes(declaration)).toEqual([]);
                const tag = declaration.getAttribute('tag');
                if (tag) for (const instance of canvasElement.querySelectorAll<HTMLElement>(tag)) {
                    expect(cemDiagnosticCodes(instance)).toEqual([]);
                }
            }
            readinessCheckpoint('set-url-journey-verified');
        } catch (error) {
            readinessCheckpoint('set-url-assertion-failed');
            throw error;
        } finally {
            history.replaceState(originalState, '', originalUrl);
        }
    },
};

function sourceLoadedDemo(): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded set-url demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', SOURCE_TAG);
    declaration.setAttribute('src', DEMO_URL.href);
    declaration.setAttribute('link-base', 'source');
    root.append(declaration, document.createElement(SOURCE_TAG));
    traceCemReadiness(root, 'set-url/EveryAuthoredSample');
    return root;
}

function outputs(root: ParentNode): string[] {
    return Array.from(root.querySelectorAll('output'), output => output.textContent ?? '');
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    return requiredElement(host, `cem-demo-element[legend="${legend}"]`);
}

function buttonByName(root: ParentNode, name: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(candidate =>
        (candidate.getAttribute('aria-label') ?? candidate.textContent ?? '').trim() === name);
    if (!button) throw new Error(`expected ${name} button`);
    return button;
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
