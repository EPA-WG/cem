import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { cemDiagnosticCodes, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-module-url-referrer-document';
const DEMO_URL = new URL('../../demo/module-url-referrer.html', import.meta.url);
const MATRIX_LEGEND = 'src by scalar referrer matrix';

const meta: Meta = {
    title: 'CEM Elements/Module URL Referrer Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded module URL referrer demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', DEMO_URL.href);
        declaration.setAttribute('link-base', 'source');

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const originalUrl = location.href;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await whenCemSourceRendered(host);
        await waitForCondition(
            () => host.querySelectorAll('cem-demo-element[legend]').length === 1,
            'the module-URL referrer sample renders from the HTML source'
        );

        const sample = requiredElement(host, `cem-demo-element[legend="${MATRIX_LEGEND}"]`);
        const declaration = requiredElement(sample, '[slot=demo] cem-element[tag]');
        const tag = declaration.getAttribute('tag');
        assert(tag, 'the matrix declaration has its generated tag');
        const instance = requiredElement(sample, tag);
        const cells = () => Array.from(
            sample.querySelectorAll('tbody td'),
            (cell) => normalize(cell.textContent ?? '')
        );
        const smileyUrl = new URL('./lib-dir/Smiley.svg', DEMO_URL);
        const confusedUrl = new URL('./confused.svg', DEMO_URL);
        const squareUrl = new URL('./wc-square.svg', DEMO_URL);
        const expected = [
            withSearch(smileyUrl, 'case', 'relative-relative'),
            withSearch(smileyUrl, 'referrer', 'relative'),
            'https://assets.example.test/logo.svg',
            withSearch(smileyUrl, 'case', 'relative-module'),
            withSearch(confusedUrl, 'referrer', 'module'),
            'https://assets.example.test/logo.svg',
            'https://referrer.example.test/lib-dir/Smiley.svg?case=relative-absolute',
            withSearch(squareUrl, 'referrer', 'absolute'),
            'https://assets.example.test/logo.svg',
        ];
        await waitForCondition(
            () => cells().length === expected.length
                && cells().every((value, index) => value === expected[index]),
            'all scalar src-by-referrer combinations publish their expected URLs'
        );
        await whenCemRendered(instance);
        const labels = ['relative URL', 'module path', 'absolute URL'];
        expect(texts(sample, 'thead th')).toEqual(['referrer / src', ...labels]);
        expect(texts(sample, 'tbody th')).toEqual(labels);
        expect(sample.querySelectorAll('thead th[scope="col"]')).toHaveLength(4);
        expect(sample.querySelectorAll('tbody th[scope="row"]')).toHaveLength(3);
        expect(sample.querySelectorAll('tbody tr')).toHaveLength(3);
        const rows = Array.from(sample.querySelectorAll('tbody tr'));
        for (const [rowIndex, label] of labels.entries()) {
            await step(`${label} referrer resolves all three src forms`, async () => {
                expect(texts(rows[rowIndex], 'td')).toEqual(expected.slice(rowIndex * 3, rowIndex * 3 + 3));
            });
        }
        await step('The source presents all nine inputs and bindings', async () => {
            const source = requiredElement(sample, '[slot="text"] pre').textContent ?? '';
            expect(source.replace(/^\r?\n/u, '').startsWith('<cem-element>')).toBe(true);
            const controls = [...source.matchAll(/\{cem-module-url\s+([^}]+)\}/gu)];
            expect(controls).toHaveLength(9);
            const suffixes = ['Relative', 'Module', 'Absolute'];
            const referrers = ['./relative-referrer/component.js', 'demo-module-referrer',
                'https://referrer.example.test/absolute/component.js'];
            for (const [row, suffix] of suffixes.entries()) {
                const sources = [`../lib-dir/Smiley.svg?case=relative-${suffix.toLowerCase()}`,
                    'demo-referrer-image', 'https://assets.example.test/logo.svg'];
                for (const [column, prefix] of ['relative', 'module', 'absolute'].entries()) {
                    const control = normalize(controls[row * 3 + column][1]);
                    expect(control).toBe(`@slice=${prefix}By${suffix} @src="${sources[column]}" @referrer="${referrers[row]}"`);
                    expect(source).toMatch(new RegExp(`\\{\\$datadom\\.slices\\.${prefix}By${suffix}\\s*\\}`));
                }
            }
            expect(sample.querySelector('[slot="status"]')?.textContent?.trim() ?? '').toBe('');
        });
        expect(location.href).toBe(originalUrl);
        const related = requiredElement(host, 'main section a[href$="module-url.html"]') as HTMLAnchorElement;
        expect(related.href).toBe(new URL('./module-url.html', DEMO_URL).href);
        const index = requiredElement(host, 'nav a') as HTMLAnchorElement;
        expect(index.href).toBe(new URL('../index.html', DEMO_URL).href);
        expect(host.querySelectorAll('cem-module-url')).toHaveLength(0);
        expect(cemDiagnosticCodes(declaration)).toEqual([]);
        expect(cemDiagnosticCodes(instance)).toEqual([]);
        expect(cemDiagnosticCodes(host)).toEqual([]);
    },
};


function withSearch(input: URL, name: string, value: string): string {
    const url = new URL(input);
    url.searchParams.set(name, value);
    return url.href;
}

async function waitForCondition(
    condition: () => boolean,
    message: string,
    attempts = 120
): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    assert(element, `expected ${selector}`);
    return element;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}

function texts(root: ParentNode, selector: string): string[] {
    return Array.from(root.querySelectorAll(selector), node => normalize(node.textContent ?? ''));
}
