import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import authoredPage from '../../demo/cell-overrides.html?raw';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = { title: 'CEM Elements/Cell Template Overrides', tags: ['test'] };
export default meta;
type Story = StoryObj;
const SOURCE_URL = new URL('../../demo/cell-overrides.html', import.meta.url);
function renderDocument(): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', 'story-cell-overrides-document');
    declaration.setAttribute('src', SOURCE_URL.href);
    root.append(declaration, document.createElement('story-cell-overrides-document'));
    return root;
}
async function ready(root: HTMLElement, tag: string): Promise<HTMLElement> {
    await waitFor(() => expect(root.querySelector(`${tag} table`)).not.toBeNull(), { timeout: 30000 });
    return root.querySelector(tag) as HTMLElement;
}
function change(input: HTMLTextAreaElement, source: string): void {
    input.value = source;
    input.dispatchEvent(new Event('change', { bubbles: true }));
}
function select(element: HTMLElement, value: string): void {
    (element as HTMLSelectElement).value = value;
    element.dispatchEvent(new Event('change', { bubbles: true }));
}

export const AuthoredSourcePreviews: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const authored = document.createElement('template');
        authored.innerHTML = authoredPage;
        const expected = Array.from(authored.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: (card.querySelector('template') as HTMLTemplateElement).innerHTML,
        }));
        expect(expected).toHaveLength(3);
        await waitFor(() => {
            expect(canvasElement.querySelectorAll('cem-demo-element[data-state=ready]')).toHaveLength(3);
        }, { timeout: 30000 });
        expect(Array.from(canvasElement.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: card.querySelector('[slot=text] code')?.textContent,
        }))).toEqual(expected);
        const template = canvasElement.querySelector('cem-demo-element > template') as HTMLTemplateElement;
        expect(template.content.querySelector('cem-element')?.hasAttribute('data-cem-render-node-id')).toBe(true);
        await waitFor(() => {
            expect(canvasElement.querySelectorAll('cem-pokemon-cells img')).toHaveLength(2);
            expect(canvasElement.querySelector('cem-stock-cells strong')).toHaveTextContent('Out of stock (0)');
            expect(canvasElement.querySelector('cem-native-value-card article')).toHaveTextContent('Next count: 3');
        }, { timeout: 10000 });
    },
};

export const PokemonCellPictures: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const viewer = await ready(canvasElement, 'cem-pokemon-cells');
        const controls = within(viewer);
        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        const images = () => Array.from(viewer.querySelectorAll('img'));
        await waitFor(() => {
            expect(images().map(image => image.alt)).toEqual(['venusaur', 'ivysaur']);
            expect(images().every(image => image.complete && image.naturalWidth > 0)).toBe(true);
        }, { timeout: 10000 });
        for (const image of images()) expect(image.src).toContain(new URL('./pokemon/', SOURCE_URL).href);
        expect(viewer.querySelector('table')).toHaveTextContent('venusaur');
        expect(viewer.querySelector('table')).toHaveTextContent('ivysaur');
        expect(Array.from(viewer.querySelectorAll('thead th'), th => th.textContent?.trim())).toEqual(['✓', '#text', 'id', 'name', 'type']);
        await userEvent.click(viewer.querySelector('tbody button') as HTMLButtonElement);
        await waitFor(() => expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('venusaur'), { timeout: 10000 });
        select(controls.getByRole('combobox', { name: 'Sort column' }), 'name');
        await waitFor(() => expect(images().map(image => image.alt)).toEqual(['ivysaur', 'venusaur']), { timeout: 10000 });
        expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('venusaur');
        expect(input.value).toBe(original);
        input.focus();
        change(input, original.replace('<name>venusaur</name>', '<title>venusaur</title>'));
        await waitFor(() => expect(images()).toHaveLength(1), { timeout: 10000 });
        expect(viewer.querySelector('table')).toHaveTextContent('venusaur');
        expect(viewer.querySelector('table')).toHaveTextContent('grass');
        expect(document.activeElement).toBe(input);
        change(input, '<broken>');
        await waitFor(() => expect(controls.getByRole('alert')).toBeVisible(), { timeout: 10000 });
        expect(viewer.querySelector('table')).toBeNull();
        expect(images()).toHaveLength(0);
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        await waitFor(() => expect(images()).toHaveLength(2), { timeout: 10000 });
        expect(input.value).toBe(original);
        expect(viewer.querySelectorAll('td .pokemon-name')).toHaveLength(2);
        for (const cell of viewer.querySelectorAll('td .pokemon-name')) {
            expect(cell).toHaveTextContent((cell.querySelector('img') as HTMLImageElement).alt);
        }
    },
};

export const ImplicitAndExplicitModules: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const scope = createCemDeclarationScope({ document });
        const loaded: string[] = [];
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-module-forms-declaration', declarationScope: scope,
            loadSrcDocument: async (path, baseDocument) => {
                const url = new URL(path, baseDocument.baseURI);
                expect(url.pathname).toMatch(/\/implicit-library\.cemt$/u);
                loaded.push(url.href);
                const source = `{template @name=present @visibility=public | {param @name=subject}
                    {apply-templates @select=subject @mode=cell}}
                    {template @mode=cell @match=true | {span | {$dom:text()}}}`;
                return { resolvedUrl: url.href, contentType: 'text/cem-ml',
                    body: new ReadableStream<Uint8Array>({ start(controller) {
                        controller.enqueue(new TextEncoder().encode(source)); controller.close();
                    } }) };
            },
        });
        runtime.install(window);
        try {
            for (const moduleWrapper of [false, true]) {
                for (const bodyWrapper of [false, true]) {
                    const tag = `story-module-form-${Number(moduleWrapper)}-${Number(bodyWrapper)}`;
                    const declaration = document.createElement(runtime.declarationTag);
                    declaration.setAttribute('tag', tag);
                    const template = document.createElement('template');
                    template.setAttribute('type', 'text/cem-ml');
                    const declarations = `{import @as=base @src="./implicit-library.cemt"}
                        {template @mode=cell @match='node == "ivy"' | {b | {$dom:text()}}}`;
                    const content = `{i | before}{call @from=base @template=present
                        @with:subject='{datadom.attributes.label}'}{i | after}`;
                    const source = declarations + (bodyWrapper ? `{body | ${content}}` : content);
                    template.textContent = moduleWrapper ? `{module | ${source}}` : source;
                    declaration.append(template);
                    const instance = document.createElement(tag);
                    instance.setAttribute('label', 'ivy');
                    canvasElement.append(declaration, instance);
                    await waitFor(() => expect(instance.querySelector('b')).toHaveTextContent('ivy'), { timeout: 30000 });
                    expect(Array.from(instance.querySelectorAll('i, b, span'), node => node.textContent?.trim()))
                        .toEqual(['before', 'ivy', 'after']);
                    expect(runtime.diagnosticsFor(declaration)).toEqual([]);
                    expect(runtime.diagnosticsFor(instance)).toEqual([]);
                    instance.setAttribute('label', 'saur');
                    await waitFor(() => {
                        expect(instance.querySelector('span')).toHaveTextContent('saur');
                        expect(instance.querySelector('b')).toBeNull();
                    }, { timeout: 10000 });
                }
            }
            expect(loaded.length).toBeGreaterThan(0);
        } finally {
            canvasElement.replaceChildren();
            scope.dispose();
        }
    },
};

export const InvalidModuleBodies: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-invalid-module-declaration', declarationScope: scope,
        });
        runtime.install(window);
        try {
            for (const [index, source] of [
                '{body | {b | first}}{i | second}',
                '{body | {b | first}}{body | {i | second}}',
                '{module | {body | {b | first}}{i | second}}',
                '{module | {body | {b | first}}{body | {i | second}}}',
            ].entries()) {
                const tag = `story-invalid-module-${index}`;
                const declaration = document.createElement(runtime.declarationTag);
                declaration.setAttribute('tag', tag);
                const template = document.createElement('template');
                template.setAttribute('type', 'text/cem-ml');
                template.textContent = source;
                declaration.append(template);
                const instance = document.createElement(tag);
                canvasElement.append(declaration, instance);
                await waitFor(() => expect([
                    ...runtime.diagnosticsFor(declaration), ...runtime.diagnosticsFor(instance),
                ].some(diagnostic => diagnostic.code === 'cem.transform_template.declaration_invalid')).toBe(true), { timeout: 30000 });
                expect(instance.querySelector('b, i')).toBeNull();
            }
        } finally {
            canvasElement.replaceChildren();
            scope.dispose();
        }
    },
};

export const ConditionalStockFallback: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const viewer = await ready(canvasElement, 'cem-stock-cells');
        const controls = within(viewer);
        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        expect(viewer.querySelectorAll('strong')).toHaveLength(1);
        expect(viewer.querySelector('strong')).toHaveTextContent('Out of stock (0)');
        expect(viewer.querySelector('table')).toHaveTextContent('5');
        const reference = Array.from(viewer.querySelectorAll('details')).find(details =>
            details.querySelector(':scope > summary')?.textContent?.includes('document/reference'));
        expect(reference).toHaveTextContent('0');
        expect(reference?.querySelector('strong')).toBeNull();
        input.focus();
        change(input, original.replace('<stock>0</stock>', '<stock>7</stock>'));
        await waitFor(() => expect(viewer.querySelector('table')).toHaveTextContent('7'), { timeout: 10000 });
        expect(viewer.querySelector('strong')).toBeNull();
        expect(document.activeElement).toBe(input);
        change(input, original.replace('<stock>5</stock>', '<stock>0</stock>'));
        await waitFor(() => expect(viewer.querySelectorAll('strong')).toHaveLength(2), { timeout: 10000 });
        expect(input.value).toContain('<name>Lemon</name><stock>0</stock>');
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        await waitFor(() => expect(viewer.querySelectorAll('strong')).toHaveLength(1), { timeout: 10000 });
        expect(input.value).toBe(original);
    },
};

export const DelayedStockDeclaration: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        let release!: () => void;
        const gate = new Promise<void>(resolve => { release = resolve; });
        let requested = false;
        const scope = createCemDeclarationScope({ document });
        // Give this fixture its own registrations so an earlier story cannot
        // satisfy readiness while this declaration is deliberately withheld.
        const pageSource = authoredPage
            .replaceAll('cem-element', 'cem-delayed-cell-declaration')
            .replaceAll('cem-pokemon-cells', 'story-delayed-pokemon-cells')
            .replaceAll('cem-stock-cells', 'story-delayed-stock-cells')
            .replaceAll('cem-native-value-parent', 'story-delayed-native-value-parent')
            .replaceAll('cem-native-value-card', 'story-delayed-native-value-card');
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-delayed-cell-declaration', declarationScope: scope,
            loadSrcDocument: async (path, baseDocument) => {
                const url = new URL(path, baseDocument.baseURI);
                if (url.href === SOURCE_URL.href) return pageSource;
                if (url.pathname.endsWith('/stock-cell.cemt')) {
                    requested = true;
                    await gate;
                }
                const response = await fetch(url);
                if (!response.ok || !response.body) throw new Error(`Cannot load ${url}: ${response.status}`);
                return { body: response.body, resolvedUrl: response.url,
                    resolverIdentity: 'delayed-cell-fixture', contentType: response.headers.get('content-type') ?? undefined };
            },
        });
        runtime.install(window);
        const declaration = document.createElement(runtime.declarationTag);
        declaration.setAttribute('tag', 'story-delayed-cell-page');
        declaration.setAttribute('src', SOURCE_URL.href);
        const page = document.createElement('story-delayed-cell-page');
        canvasElement.append(declaration, page);
        try {
            await waitFor(() => expect(requested).toBe(true), { timeout: 10000 });
            const pokemon = await ready(page, 'story-delayed-pokemon-cells');
            const source = pokemon.querySelector('textarea') as HTMLTextAreaElement;
            change(source, '<catalog><pokemon><id>3</id><title>venusaur</title></pokemon><pokemon><id>2</id><name>ivysaur</name></pokemon></catalog>');
            await waitFor(() => expect(pokemon.querySelectorAll('img')).toHaveLength(1), { timeout: 10000 });
            const owner = page.querySelector(`${runtime.declarationTag}[tag="story-delayed-stock-cells"]`) as HTMLElement;
            const stock = page.querySelector('story-delayed-stock-cells') as HTMLElement;
            expect(owner).not.toBeNull();
            expect(customElements.get(stock.localName)).toBeUndefined();
            expect(stock.querySelector('strong')).toBeNull();
            expect(owner.querySelector('style[data-cem-declaration-style]')).toBeNull();

            release();
            await runtime.whenDeclarationSettled(owner);
            await runtime.whenRenderSettled(stock);
            await waitFor(() => expect(stock.querySelector('strong')).toHaveTextContent('Out of stock (0)'), { timeout: 10000 });
            expect(stock.querySelectorAll('strong')).toHaveLength(1);
            expect(owner.querySelector('style[data-cem-declaration-style]')).not.toBeNull();
            expect(runtime.diagnosticsFor(owner)).toEqual([]);
            expect(runtime.diagnosticsFor(stock)).toEqual([]);
            const stockSource = stock.querySelector('textarea') as HTMLTextAreaElement;
            expect(stockSource.value).toContain('<name>Cherry</name><stock>0</stock>');
            change(stockSource, stockSource.value.replace('<stock>0</stock>', '<stock>7</stock>'));
            await waitFor(() => {
                expect(stock.querySelector('table')).toHaveTextContent('7');
                expect(stock.querySelector('strong')).toBeNull();
            }, { timeout: 10000 });
        } finally {
            release();
            page.remove();
            declaration.remove();
            scope.dispose();
        }
    },
};

export const FailedStockDeclarationCanRemount: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        let failing = true;
        let requests = 0;
        const scope = createCemDeclarationScope({ document });
        const pageSource = authoredPage
            .replaceAll('cem-element', 'cem-retry-cell-declaration')
            .replaceAll('cem-pokemon-cells', 'story-retry-pokemon-cells')
            .replaceAll('cem-stock-cells', 'story-retry-stock-cells')
            .replaceAll('cem-native-value-parent', 'story-retry-native-value-parent')
            .replaceAll('cem-native-value-card', 'story-retry-native-value-card');
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-retry-cell-declaration', declarationScope: scope,
            loadSrcDocument: async (path, baseDocument) => {
                const url = new URL(path, baseDocument.baseURI);
                if (url.href === SOURCE_URL.href) return pageSource;
                if (url.pathname.endsWith('/stock-cell.cemt')) {
                    requests++;
                    if (failing) throw new Error('HTTP 503: controlled stock source failure');
                }
                const response = await fetch(url);
                if (!response.ok || !response.body) throw new Error(`Cannot load ${url}: ${response.status}`);
                return { body: response.body, resolvedUrl: response.url,
                    resolverIdentity: 'retry-cell-fixture', contentType: response.headers.get('content-type') ?? undefined };
            },
        });
        runtime.install(window);
        function mountPage(tag: string): HTMLElement {
            const declaration = document.createElement(runtime.declarationTag);
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('src', SOURCE_URL.href);
            const page = document.createElement(tag);
            canvasElement.append(declaration, page);
            return page;
        }
        try {
            const first = mountPage('story-retry-cell-first-page');
            await ready(first, 'story-retry-pokemon-cells');
            await waitFor(() => expect(first.querySelectorAll('cem-demo-element[data-state=ready]')).toHaveLength(3), { timeout: 10000 });
            const failedOwner = first.querySelector(`${runtime.declarationTag}[tag="story-retry-stock-cells"]`) as HTMLElement;
            await runtime.whenDeclarationSettled(failedOwner);
            expect(runtime.diagnosticsFor(failedOwner).map(diagnostic => diagnostic.code)).toEqual(['cem-element.src_load_failed']);
            expect(customElements.get('story-retry-stock-cells')).toBeUndefined();
            expect(first.querySelector('story-retry-stock-cells strong')).toBeNull();
            expect(failedOwner.querySelector('style[data-cem-declaration-style]')).toBeNull();
            const failedRequests = requests;
            expect(failedRequests).toBeGreaterThan(0);

            canvasElement.replaceChildren();
            failing = false;
            const replacement = mountPage('story-retry-cell-replacement-page');
            await ready(replacement, 'story-retry-pokemon-cells');
            await waitFor(() => expect(requests).toBe(failedRequests + 1), { timeout: 10000 });
            const stock = await ready(replacement, 'story-retry-stock-cells');
            const owner = replacement.querySelector(`${runtime.declarationTag}[tag="story-retry-stock-cells"]`) as HTMLElement;
            await runtime.whenDeclarationSettled(owner);
            await waitFor(() => {
                expect(stock.querySelector('strong')).toHaveTextContent('Out of stock (0)');
                expect(owner.querySelector('style[data-cem-declaration-style]')).not.toBeNull();
            }, { timeout: 10000 });
            expect(requests).toBe(failedRequests + 1);
            expect(runtime.diagnosticsFor(owner)).toEqual([]);
            expect(runtime.diagnosticsFor(stock)).toEqual([]);
            expect(runtime.diagnosticsFor(failedOwner).map(diagnostic => diagnostic.code)).toEqual(['cem-element.src_load_failed']);
            const input = stock.querySelector('textarea') as HTMLTextAreaElement;
            change(input, input.value.replace('<stock>0</stock>', '<stock>7</stock>'));
            await waitFor(() => {
                expect(stock.querySelector('table')).toHaveTextContent('7');
                expect(stock.querySelector('strong')).toBeNull();
            }, { timeout: 10000 });
        } finally {
            canvasElement.replaceChildren();
            scope.dispose();
        }
    },
};

export const NativeAttributeValues: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelector('cem-native-value-card article')).not.toBeNull(), { timeout: 10000 });
        const card = canvasElement.querySelector('cem-native-value-card article');
        expect(card).toHaveTextContent('Next count: 3');
        expect(card).toHaveTextContent('Date: 2024-02-29');
        expect(card?.querySelector('.value-label > name > em')).toHaveTextContent('saur');
        expect(card?.querySelector(':scope > section > p')).toHaveTextContent('Text-only label: ivysaur');
        expect(card?.querySelector(':scope > section name')).toBeNull();
    },
};
