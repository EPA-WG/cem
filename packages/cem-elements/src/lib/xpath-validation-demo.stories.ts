import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-xpath-validation-document';
const DEMO_URL = new URL('../../demo/xpath-validation.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Validation', tags: ['test'] };
export default meta;
type Story = StoryObj;

const userCases: [string, string][] = [
    ...['Ada', 'A123456789012345', 'a_9']
        .map(value => [value, 'Valid user name'] as [string, string]),
    ...['', '  ', 'Ab', 'A1234567890123456', '7Ada', '_Ada', 'Ada-', 'Äda', 'Ada ', 'Ada\u00a0']
        .map(value => [value, 'Use 3–16 ASCII letters, digits or underscores; start with a letter'] as [string, string]),
    ['Grace_2', 'Valid user name'],
];
const ageCases: [string, string][] = [
    ...['18', '120', '018']
        .map(value => [value, 'Age in range'] as [string, string]),
    ...['0', '17', '121', '999']
        .map(value => [value, 'Use an age from 18 to 120'] as [string, string]),
    ...['', '  ', '1000', '18.0', '1e2', '+18', '-18', ' 18 ', '１８']
        .map(value => [value, 'Enter an integer age'] as [string, string]),
    ['120', 'Age in range'],
];
const tagsCases: [string, string][] = [
    ['A', 'A'],
    ['ABCDEFGHIJKL', 'ABCDEFGHIJKL'],
    ['  Red\t, GOLD ; blue  ', 'Red / GOLD / blue'],
    ...[
        '', '  ', 'ABCDEFGHIJKLM', 'red,,blue', ',red', 'red;', 'red, ;blue', 'red blue', 'blué', 'red1',
        'red\u00a0,blue',
    ]
        .map(value => [value, 'Use 1–12 ASCII letters per tag, separated by commas or semicolons'] as [string, string]),
    ['Red; GOLD', 'Red / GOLD'],
];
const addressCases: [string, string][] = [
    ...['255.255.255.255', '0.0.0.0', ' 192.0.2.10/24 ', '192.0.2.10/32']
        .map(value => [value, 'Allowed by the local prefix rule'] as [string, string]),
    ['192.0.2.10/16', 'Blocked by the local prefix rule'],
    ...['256.0.2.10/24', '192.0.2.256', '192.0.2.10/33', '192.0.2.10/99']
        .map(value => [value, 'Octets must be 0–255 and prefix length 0–32'] as [string, string]),
    ...[
        '', ' ', '192.00.2.10/24', '192.0.2.10/024', '192.0.2.10/00', '::1', '192.0.2', '192.0.2.10.1',
        '192.0.2.10/', '192.0.2.10/-1', '192.0.2.10/100', '192.0. 2.10', '192.0.2.10\u00a0',
    ]
        .map(value => [value, 'Enter IPv4 with an optional /prefix; no leading zeros'] as [string, string]),
    ['192.0.2.10/24', 'Allowed by the local prefix rule'],
];
const prefixesCases: [string, string][] = [
    ...['24', ' 24\t32 24 ', '+24 032', '024']
        .map(value => [value, 'Allowed by the local prefix rule'] as [string, string]),
    ...['0', '32']
        .map(value => [value, 'Blocked by the local prefix rule'] as [string, string]),
    ...['', ' ', '24 bad', '24 33', '24 -1', '24 1.0', '24 1e1', '24,32', '24\u00a032']
        .map(value => [value, 'Enter allowed prefix lengths from 0 to 32'] as [string, string]),
    ['24 32', 'Allowed by the local prefix rule'],
];

export const FormAndPrefixRules: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('validation demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        const outputs = (article: Element) => Array.from(article.querySelectorAll('output'), node => node.textContent);
        const initialForm = ['Valid user name', 'Age in range', 'Blue / green / RED'];
        await waitFor(() => expect(outputs(sample('1. Form validation preview'))).toEqual(initialForm), { timeout: 20000 });
        await whenCemSourceRendered(root);
        const form = sample('1. Form validation preview');
        const ip = sample('2. IPv4 prefix-rule preview');
        const formInputs = Array.from(form.querySelectorAll('input'));
        const ipInputs = Array.from(ip.querySelectorAll('input'));
        const edit = async (article: Element, index: number, input: HTMLInputElement, value: string, expected: string[]) => {
            input.focus();
            input.value = value;
            const caret = Math.min(2, value.length);
            input.setSelectionRange(caret, caret);
            input.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(outputs(article)).toEqual(expected));
            await whenCemSourceRendered(root);
            expect(outputs(article)).toEqual(expected);
            expect(article.querySelectorAll('input')[index]).toBe(input);
            expect(input.value).toBe(value);
            expect(document.activeElement).toBe(input);
            expect([input.selectionStart, input.selectionEnd]).toEqual([caret, caret]);
        };
        await step('1. Form validation preview', async () => {
            expect(formInputs.map(input => input.value)).toEqual(['Ada_7', '21', 'Blue, green; RED']);
            expect(formInputs.map(input => input.labels?.[0]?.textContent?.trim())).toEqual(['User name', 'Age', 'Tags']);
            const expected = [...initialForm];
            const recover = ['Grace_2', '120', 'Red; GOLD'];
            const recovered = ['Valid user name', 'Age in range', 'Red / GOLD'];
            for (const [index, cases] of [userCases, ageCases, tagsCases].entries()) {
                for (const [value, result] of cases) {
                    expected[index] = result;
                    await edit(form, index, formInputs[index], value, expected);
                    expected[index] = recovered[index];
                    await edit(form, index, formInputs[index], recover[index], expected);
                }
            }
            expect(formInputs.map(input => input.value)).toEqual(recover);
            expect(outputs(ip)).toEqual(['Allowed by the local prefix rule']);
            expect(ipInputs.map(input => input.value)).toEqual(['192.0.2.10/24', '24 32']);
        });
        await step('2. IPv4 prefix-rule preview', async () => {
            const [address, prefixes] = ipInputs;
            expect(ipInputs.map(input => input.labels?.[0]?.textContent?.trim())).toEqual([
                'IPv4 address, optionally /prefix', 'Allowed prefix lengths, separated by spaces',
            ]);
            for (const [value, expected] of addressCases) {
                await edit(ip, 0, address, value, [expected]);
                expect(prefixes.value).toBe('24 32');
                await edit(ip, 0, address, '192.0.2.10/24', ['Allowed by the local prefix rule']);
            }
            for (const [value, expected] of prefixesCases) {
                await edit(ip, 1, prefixes, value, [expected]);
                expect(address.value).toBe('192.0.2.10/24');
                await edit(ip, 1, prefixes, '24 32', ['Allowed by the local prefix rule']);
            }
            await edit(ip, 1, prefixes, '0 32', ['Blocked by the local prefix rule']);
            await edit(ip, 0, address, '0.0.0.0/0', ['Allowed by the local prefix rule']);
            await edit(ip, 1, prefixes, '32', ['Blocked by the local prefix rule']);
            await edit(ip, 0, address, '0.0.0.0', ['Allowed by the local prefix rule']);
            await edit(ip, 1, prefixes, 'bad', ['Enter allowed prefix lengths from 0 to 32']);
            await edit(ip, 0, address, '999.0.0.0', ['Octets must be 0–255 and prefix length 0–32']);
            await edit(ip, 0, address, '::1', ['Enter IPv4 with an optional /prefix; no leading zeros']);
            await edit(ip, 0, address, '192.0.2.10/24', ['Enter allowed prefix lengths from 0 to 32']);
            await edit(ip, 1, prefixes, '24 32', ['Allowed by the local prefix rule']);
            expect(outputs(form)).toEqual(['Valid user name', 'Age in range', 'Red / GOLD']);
            expect(formInputs.map(input => input.value)).toEqual(['Grace_2', '120', 'Red; GOLD']);
        });
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../index.html', './xpath-validation.cemt', '../../cem_ml/schema-packages/xpath/v1/README.md',
                './dom-merge.html', './xpath-maps-arrays.html', './xpath-sort.html'].map(relative => new URL(relative, DEMO_URL).href));
        await whenCemSourceRendered(root);
        expect(cemDiagnosticCodes(root)).toEqual([]);
        for (const declaration of canvasElement.querySelectorAll<HTMLElement>('cem-element[tag]')) {
            expect(cemDiagnosticCodes(declaration)).toEqual([]);
            const tag = declaration.getAttribute('tag');
            if (tag) for (const instance of canvasElement.querySelectorAll<HTMLElement>(tag)) {
                expect(cemDiagnosticCodes(instance)).toEqual([]);
            }
        }
    },
};
