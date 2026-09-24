import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import { cemDiagnosticCodes, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-for-each-document';
const DEMO_URL = new URL('../../demo/for-each.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Simple for-each',
    '2. for-each with position()',
    '3. Conditional for-each',
    '4. Nested for-each table',
    '5. for-each with attributes',
    '6. Dynamic table with toggle',
    '7. for-each over payload data',
    '8. for-each over location data',
    '9. for-each over HTTP JSON/XML data',
] as const;
const PREVIEW_FILES = ['http-data.json', 'http-data.xml'] as const;

const meta: Meta = {
    title: 'CEM Elements/For Each Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded for-each demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', DEMO_URL.href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await whenCemSourceRendered(host);
        expect(Array.from(host.querySelectorAll('cem-demo-element[legend]'), sample => sample.getAttribute('legend')))
            .toEqual([...EXPECTED_LEGENDS, ...PREVIEW_FILES]);
        const audit = async (index: number, check: (sample: HTMLElement) => void | Promise<void>) => {
            const sample = requiredElement(host, `cem-demo-element[legend="${EXPECTED_LEGENDS[index]}"]`);
            await step(EXPECTED_LEGENDS[index], async () => {
                await check(sample);
                expect(cemDiagnosticCodes(requiredElement(sample, 'article').parentElement as HTMLElement)).toEqual([]);
            });
        };

        await audit(0, async sample => {
            await waitFor(() => expect(textList(sample, 'ul > li')).toEqual(['🍏', '🍌', '🍒']));
            expect(sample.querySelectorAll('ul')).toHaveLength(1);
        });
        await audit(1, async sample => {
            await waitFor(() => expect(textList(sample, 'article > div > div')).toEqual(['1. Red', '2. Green', '3. Blue']));
            const rows = sample.querySelectorAll('article > div > div');
            expect(Array.from(rows, row => getComputedStyle(row).backgroundColor))
                .toEqual(['rgb(193, 18, 31)', 'rgb(21, 128, 61)', 'rgb(29, 78, 216)']);
            expect(Array.from(rows, row => getComputedStyle(row).color))
                .toEqual(['rgb(255, 255, 255)', 'rgb(255, 255, 255)', 'rgb(255, 255, 255)']);
        });
        await audit(2, async sample => {
            const checkbox = within(sample).getByRole('checkbox', { name: 'Show items' }) as HTMLInputElement;
            await toggleCycles(sample, checkbox, shown => {
                expect(textList(sample, 'article > div span')).toEqual(shown ? ['1:First', '2:Second', '3:Third'] : []);
                const content = requiredElement(sample, 'article > div').textContent ?? '';
                expect(content.trimStart().startsWith('BEFORE')).toBe(true);
                expect(content.trimEnd().endsWith('AFTER')).toBe(true);
                expect(sample.querySelectorAll('input')).toHaveLength(1);
            });
        });
        await audit(3, async sample => {
            await waitFor(() => expect(tableRows(sample)).toEqual([
                ['A1', 'A2', 'A3'], ['B1', 'B2', 'B3'], ['C1', 'C2', 'C3'],
            ]));
            expect(textList(sample, 'thead th')).toEqual(['Col 1', 'Col 2', 'Col 3']);
            expect(sample.querySelectorAll('table')).toHaveLength(1);
            expect(sample.querySelector('article > tr, article > td')).toBeNull();
        });
        await audit(4, async sample => {
            await waitFor(() => expect(textList(sample, 'article > div')).toEqual([
                '#1 Alice (admin)', '#2 Bob (editor)', '#3 Charlie (viewer)',
            ]));
            expect(textList(sample, 'article > div > strong')).toEqual(['#1', '#2', '#3']);
            expect(textList(sample, 'article > div > em')).toEqual(['(admin)', '(editor)', '(viewer)']);
        });
        await audit(5, async sample => {
            const checkbox = within(sample).getByRole('checkbox', { name: 'Show products' }) as HTMLInputElement;
            const table = requiredElement(sample, 'table');
            const header = requiredElement(sample, 'thead');
            await toggleCycles(sample, checkbox, shown => {
                expect(tableRows(sample)).toEqual(shown ? [
                    ['1', 'Widget', '$10'], ['2', 'Gadget', '$25'], ['3', 'Gizmo', '$15'],
                ] : []);
                expect(textList(sample, 'thead th')).toEqual(['#', 'Product', 'Price']);
                expect(requiredElement(sample, 'table')).toBe(table);
                expect(requiredElement(sample, 'thead')).toBe(header);
                expect(table.isConnected && header.isConnected).toBe(true);
                expect(sample.querySelectorAll('table')).toHaveLength(1);
            });
        });
        await audit(6, async sample => {
            await waitFor(() => expect(textList(sample, '.payload-feed li')).toEqual([
                '1. payload-alpha: Payload Alpha', '2. payload-beta: Payload Beta',
            ]));
            expect(sample.querySelectorAll('cem-loop-payload .payload-feed')).toHaveLength(1);
        });
        await audit(7, async sample => {
            await waitFor(() => expect(textList(sample, '.location-feed li'))
                .toEqual(['topic = feeds', 'item = payload,resource']));
        });
        await audit(8, async sample => {
            await waitFor(() => {
                expect(textList(sample, 'output')).toEqual(['loaded', 'loaded']);
                expect(textList(sample, '.http-json-feed li')).toEqual(['alpha: ready', 'beta: loaded']);
                expect(textList(sample, '.http-xml-feed li')).toEqual(['gamma: xml-ready', 'delta: xml-loaded']);
            }, { timeout: 15000 });
        });
        for (const file of PREVIEW_FILES) {
            await step(file, () => verifyExternalFilePreviews(host, DEMO_URL, [file]));
        }
    },
};

function textList(root: ParentNode, selector: string): string[] {
    return Array.from(root.querySelectorAll(selector), element => (element.textContent ?? '').replace(/\s+/gu, ' ').trim());
}

function tableRows(sample: HTMLElement): string[][] {
    return Array.from(sample.querySelectorAll('table > tbody > tr'), row => textList(row, ':scope > td'));
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function toggleCycles(sample: HTMLElement, checkbox: HTMLInputElement, check: (shown: boolean) => void): Promise<void> {
    const instance = requiredElement(sample, 'article').parentElement as HTMLElement;
    const assertState = async (shown: boolean) => {
        await whenCemRendered(instance);
        await waitFor(() => {
            expect(requiredElement(sample, 'input[type=checkbox]')).toBe(checkbox);
            expect(checkbox.isConnected).toBe(true);
            expect(checkbox.checked).toBe(shown);
            check(shown);
        });
    };
    await assertState(false);
    for (const shown of [true, false, true, false]) {
        // Exercise pointer and keyboard activation against the current live control.
        if (shown) await userEvent.click(checkbox);
        else await userEvent.keyboard(' ');
        await assertState(shown);
        expect(document.activeElement).toBe(checkbox);
    }
}
