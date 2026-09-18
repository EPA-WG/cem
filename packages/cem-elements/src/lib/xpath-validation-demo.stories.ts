import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';

const SOURCE_TAG = 'story-xpath-validation-document';
const DEMO_URL = new URL('../../demo/xpath-validation.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Validation', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const FormAndPrefixRules: Story = {
    render: () => '<cem-element tag="' + SOURCE_TAG + '" src="' + DEMO_URL.href +
        '" hidden></cem-element><' + SOURCE_TAG + '></' + SOURCE_TAG + '>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('validation demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector('cem-demo-element[legend="' + legend + '"] article');
            if (!article) throw new Error('Missing ' + legend);
            return article;
        };
        const outputs = (article: Element) => Array.from(article.querySelectorAll('output'), node => node.textContent);
        const edit = (input: HTMLInputElement, value: string) => {
            input.focus();
            input.value = value;
            input.setSelectionRange(1, 1);
            input.dispatchEvent(new Event('input', { bubbles: true }));
        };
        await waitFor(() => expect(outputs(sample('1. Form validation preview'))).toEqual([
            'Valid user name', 'Age in range', 'Blue / green / RED',
        ]), { timeout: 20000 });
        const form = sample('1. Form validation preview');
        const [user, age, tags] = Array.from(form.querySelectorAll('input'));
        edit(user, '7Ada');
        await waitFor(() => expect(outputs(form)[0]).toContain('start with a letter'));
        expect(document.activeElement).toBe(user);
        expect(user.selectionStart).toBe(1);
        edit(age, '17');
        await waitFor(() => expect(outputs(form)[1]).toBe('Use an age from 18 to 120'));
        edit(age, '1e2');
        await waitFor(() => expect(outputs(form)[1]).toBe('Enter an integer age'));
        edit(tags, 'red,,blue');
        await waitFor(() => expect(outputs(form)[2]).toContain('per tag'));
        edit(user, 'Grace_2');
        edit(age, '120');
        edit(tags, 'Red; GOLD');
        await waitFor(() => expect(outputs(form)).toEqual(['Valid user name', 'Age in range', 'Red / GOLD']));

        const ip = sample('2. IPv4 prefix-rule preview');
        const [address, prefixes] = Array.from(ip.querySelectorAll('input'));
        await waitFor(() => expect(outputs(ip)[0]).toBe('Allowed by the local prefix rule'));
        for (const [value, expected] of [
            ['192.0.2.10/16', 'Blocked by the local prefix rule'],
            ['256.0.2.10/24', 'Octets must be 0–255 and prefix length 0–32'],
            ['192.0.2.10/33', 'Octets must be 0–255 and prefix length 0–32'],
            ['192.00.2.10/24', 'Enter IPv4 with an optional /prefix; no leading zeros'],
            ['::1', 'Enter IPv4 with an optional /prefix; no leading zeros'],
            ['', 'Enter IPv4 with an optional /prefix; no leading zeros'],
            ['255.255.255.255', 'Allowed by the local prefix rule'],
        ]) {
            edit(address, value);
            await waitFor(() => expect(outputs(ip)[0]).toBe(expected));
        }
        expect(document.activeElement).toBe(address);
        expect(address.selectionStart).toBe(1);
        edit(prefixes, 'bad');
        await waitFor(() => expect(outputs(ip)[0]).toBe('Enter allowed prefix lengths from 0 to 32'));
        edit(prefixes, '24');
        await waitFor(() => expect(outputs(ip)[0]).toBe('Blocked by the local prefix rule'));
        edit(address, '192.0.2.10/24');
        await waitFor(() => expect(outputs(ip)[0]).toBe('Allowed by the local prefix rule'));
        expect(outputs(form)).toEqual(['Valid user name', 'Age in range', 'Red / GOLD']);
    },
};
