import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
const SOURCE_TAG = 'story-xpath-maps-arrays-document';
const DEMO_URL = new URL('../../demo/xpath-maps-arrays.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Maps and Arrays', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const OptionalEntriesAndRetainedMembers: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('map/array demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        const values = (article: Element) => Array.from(article.querySelectorAll('output'), item => item.textContent);
        const edit = (control: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement, value: string) => {
            control.value = value;
            control.dispatchEvent(new Event(control instanceof HTMLSelectElement ? 'change' : 'input', { bubbles: true }));
        };
        await waitFor(() => expect(values(sample('1. An IP-filter map with an optional note'))).toEqual(['allow: 192.0.2.0/24', 'Absent entry', '2']), { timeout: 20000 });
        const filter = sample('1. An IP-filter map with an optional note');
        const address = filter.querySelector('input') as HTMLInputElement;
        const [action, note] = Array.from(filter.querySelectorAll('select'));
        address.focus();
        address.value = '198.51.100.0/24';
        address.setSelectionRange(4, 4);
        address.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(values(filter)[0]).toBe('allow: 198.51.100.0/24'));
        expect(filter.querySelector('input')).toBe(address);
        expect(document.activeElement).toBe(address);
        expect(address.selectionStart).toBe(4);
        edit(action, 'deny');
        await waitFor(() => expect(values(filter)[0]).toBe('deny: 198.51.100.0/24'));
        for (const [state, expected, count] of [['empty', 'Present, empty sequence', '3'], ['value', 'Local preview', '3'], ['absent', 'Absent entry', '2']]) {
            edit(note, state);
            await waitFor(() => expect(values(filter).slice(1)).toEqual([expected, count]));
        }
        const basket = sample('2. Select a retained fruit by array position');
        await waitFor(() => expect(values(basket)).toEqual(['2', 'apple: 2', 'Empty member (array size 1)']));
        const position = basket.querySelector('input') as HTMLInputElement;
        const xml = basket.querySelector('textarea') as HTMLTextAreaElement;
        edit(position, '2');
        await waitFor(() => expect(values(basket)[1]).toBe('pear: 3'));
        for (const invalid of ['0', '3', '-1', '1.5', 'bad', '']) {
            edit(position, invalid);
            await waitFor(() => expect(values(basket)[1]).toBe('No member at this position'));
        }
        edit(position, '3');
        xml.focus();
        xml.value = '<basket note="Fresh"><apple>2</apple><pear>3</pear><plum>4</plum></basket>';
        xml.setSelectionRange(8, 8);
        xml.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(values(basket)).toEqual(['3', 'plum: 4', 'Note: Fresh']));
        expect(basket.querySelector('textarea')).toBe(xml);
        expect(document.activeElement).toBe(xml);
        expect(xml.selectionStart).toBe(8);
        edit(xml, '<basket note=""/>');
        await waitFor(() => expect(values(basket)).toEqual(['0', 'No member at this position', 'Note: ']));
        for (const invalid of ['<basket>', '<other/>']) {
            edit(xml, invalid);
            await waitFor(() => expect(basket.querySelector('[role="alert"]')?.textContent).toBeTruthy());
            expect(values(basket)).toEqual([]);
        }
        edit(xml, '<basket><cherry>5</cherry></basket>');
        edit(position, '1');
        await waitFor(() => expect(values(basket)).toEqual(['1', 'cherry: 5', 'Empty member (array size 1)']));
        expect(values(filter)).toEqual(['deny: 198.51.100.0/24', 'Absent entry', '2']);
        const imported = sample('3. Query an imported JSON tree');
        const json = imported.querySelector('textarea') as HTMLTextAreaElement;
        await waitFor(() => expect(values(imported)).toEqual(['3 members; numeric total 5', 'Null value']));
        json.focus();
        json.value = '{"cherry":4,"note":""}';
        json.setSelectionRange(5, 5);
        json.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(values(imported)).toEqual(['2 members; numeric total 4', 'Empty string']));
        expect(imported.querySelector('textarea')).toBe(json);
        expect(document.activeElement).toBe(json);
        expect(json.selectionStart).toBe(5);
        edit(json, '{"plum":1e2}');
        await waitFor(() => expect(values(imported)).toEqual(['1 members; numeric total 100', 'Absent member']));
        for (const invalid of ['{', '[]']) {
            edit(json, invalid);
            await waitFor(() => expect(imported.querySelector('[role="alert"]')?.textContent).toBeTruthy());
            expect(values(imported)).toEqual([]);
        }
        edit(json, '{"note":null}');
        await waitFor(() => expect(values(imported)).toEqual(['1 members; numeric total 0', 'Null value']));
        expect(values(basket)).toEqual(['1', 'cherry: 5', 'Empty member (array size 1)']);
    },
};
