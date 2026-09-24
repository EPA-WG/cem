import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import authoredPage from '../../demo/cell-overrides.html?raw';
import pokemonSource from '../../demo/pokemon-cells.json?raw';
import stockSource from '../../demo/stock-cells.xml?raw';
import stockTemplate from '../../demo/stock-cell.cemt?raw';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = { title: 'CEM Elements/Cell Template Overrides', tags: ['test'] };
export default meta;
type Story = StoryObj;
const POKEMON_NAMES = ['bulbasaur', 'ivysaur', 'venusaur', 'charmander', 'charmeleon', 'charizard', 'squirtle', 'wartortle', 'blastoise', 'caterpie'];
const STOCK = { Cherry: 'Out of stock (0)', Lemon: '5', Apple: '12', Pear: '3', Plum: '8' };
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
function select(element: HTMLElement, value: string): void {
    (element as HTMLSelectElement).value = value;
    element.dispatchEvent(new Event('change', { bubbles: true }));
}

function text(element: Element | null): string {
    return (element?.textContent ?? '').replace(/\s+/gu, ' ').trim();
}

async function expectPokemonRows(viewer: HTMLElement, names: readonly string[]): Promise<void> {
    await waitFor(() => {
        const rows = Array.from(viewer.querySelectorAll('table > tbody > tr'));
        expect(rows.map(row => text(row.querySelector('.pokemon-name')))).toEqual(names);
        expect(viewer.querySelectorAll('img')).toHaveLength(names.length);
        for (const [index, row] of rows.entries()) {
            const name = names[index];
            const id = POKEMON_NAMES.indexOf(name) + 1;
            const cells = row.querySelectorAll('td');
            expect(cells).toHaveLength(2);
            expect(cells[0].querySelectorAll('img')).toHaveLength(1);
            expect(cells[0].querySelector('img')).toHaveAttribute('alt', name);
            expect(cells[0].querySelector('img')).toHaveAttribute('src',
                `https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/${id}.svg`);
            expect(text(cells[1])).toBe(`https://pokeapi.co/api/v2/pokemon/${id}/`);
            expect(cells[1].querySelector('img')).toBeNull();
        }
    }, { timeout: 10000 });
}

async function expectStockRows(viewer: HTMLElement, names: readonly (keyof typeof STOCK)[]): Promise<void> {
    await waitFor(() => {
        const headings = Array.from(viewer.querySelectorAll('thead th'), text);
        const nameColumn = headings.indexOf('name');
        const stockColumn = headings.indexOf('stock');
        expect(nameColumn).toBeGreaterThanOrEqual(0);
        expect(stockColumn).toBeGreaterThanOrEqual(0);
        const rows = Array.from(viewer.querySelectorAll('table > tbody > tr'));
        expect(rows.map(row => text(row.children.item(nameColumn)?.querySelector('.value') ?? null))).toEqual(names);
        for (const [index, row] of rows.entries()) {
            expect(text(row.children.item(stockColumn)?.querySelector('strong, .value') ?? null)).toBe(STOCK[names[index]]);
        }
        expect(viewer.querySelectorAll('strong')).toHaveLength(1);
        const reference = Array.from(viewer.querySelectorAll('details')).find(details =>
            details.querySelector(':scope > summary')?.textContent?.includes('document/reference'));
        expect(reference).toBeDefined();
        expect(text(reference?.querySelector('.value') ?? null)).toBe('0');
        expect(reference?.querySelector('strong')).toBeNull();
    }, { timeout: 10000 });
}

async function expectNativeValues(root: HTMLElement): Promise<void> {
    await waitFor(() => {
        expect(root.querySelectorAll('cem-native-value-card article')).toHaveLength(1);
        const card = root.querySelector('cem-native-value-card article');
        expect(Array.from(card?.querySelectorAll('p') ?? [], text)).toEqual([
            'ivysaur', 'Next count: 3', 'Date: 2024-02-29', 'Text-only label: ivysaur',
        ]);
        const name = card?.querySelector('.value-label > name');
        expect(name?.firstChild?.textContent).toBe('ivy');
        expect(name?.querySelector('em')).toHaveTextContent('saur');
        expect(card?.querySelector(':scope > section name')).toBeNull();
    }, { timeout: 10000 });
}

export const AuthoredSourcePreviews: Story = {
    render: renderDocument,
    play: async ({ canvasElement, step }) => {
        const authored = document.createElement('template');
        authored.innerHTML = authoredPage;
        const expected = Array.from(authored.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: ({ './pokemon-cells.json': pokemonSource, './stock-cells.xml': stockSource,
                './stock-cell.cemt': stockTemplate } as Record<string, string>)[card.getAttribute('src') ?? '']
                ?? card.querySelector('template')?.innerHTML ?? '',
        }));
        expect(expected).toHaveLength(6);
        await waitFor(() => {
            expect(canvasElement.querySelectorAll('cem-demo-element[data-state=ready]')).toHaveLength(6);
        }, { timeout: 30000 });
        const cards = Array.from(canvasElement.querySelectorAll<HTMLElement>('cem-demo-element'));
        expect(cards.map(card => card.getAttribute('legend'))).toEqual(expected.map(card => card.legend));
        for (const [index, card] of cards.entries()) {
            await step(expected[index].legend ?? '', async () => {
                await waitFor(() => expect(card.querySelector('[slot=text] code')?.textContent).toBe(expected[index].source));
                if (card.hasAttribute('src')) {
                    expect(card).toHaveAttribute('demo', 'false');
                    expect(card.querySelector('[slot=demo]')?.textContent).toBe('');
                    expect(card.querySelector('[slot=demo]')?.children).toHaveLength(0);
                    const code = card.querySelector('[slot=text] code');
                    const token = code?.querySelector(card.getAttribute('type') === 'json' ? 'i' : 'b');
                    expect(token).not.toBeNull();
                    expect(getComputedStyle(token as Element).color).not.toBe(getComputedStyle(code as Element).color);
                } else if (card.querySelector('cem-pokemon-cells')) {
                    await expectPokemonRows(await ready(card, 'cem-pokemon-cells'), POKEMON_NAMES);
                } else if (card.querySelector('cem-stock-cells')) {
                    await expectStockRows(await ready(card, 'cem-stock-cells'), ['Cherry', 'Lemon', 'Apple', 'Pear', 'Plum']);
                } else {
                    await expectNativeValues(card);
                }
            });
        }
        const template = canvasElement.querySelector('cem-demo-element > template') as HTMLTemplateElement;
        expect(template.content.querySelector('cem-element')?.hasAttribute('data-cem-render-node-id')).toBe(true);
    },
};

export const PokemonCellPictures: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const viewer = await ready(canvasElement, 'cem-pokemon-cells');
        const controls = within(viewer);
        expect(controls.queryByRole('textbox', { name: 'Source' })).toBeNull();
        expect(controls.queryByRole('button', { name: 'Reset source' })).toBeNull();
        const preview = canvasElement.querySelector('cem-demo-element[legend="pokemon-cells.json"]');
        await waitFor(() => expect(preview?.querySelector('[slot=text] code')?.textContent).toBe(pokemonSource));
        expect(preview?.querySelector('[slot=demo]')?.textContent).toBe('');
        await expectPokemonRows(viewer, POKEMON_NAMES);
        expect(Array.from(viewer.querySelectorAll('thead th'), th => th.textContent?.trim())).toEqual(['✓', 'name', 'url']);
        await userEvent.click(viewer.querySelector('tbody button') as HTMLButtonElement);
        await waitFor(() => expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('bulbasaur'), { timeout: 10000 });
        select(controls.getByRole('combobox', { name: 'Sort column' }), 'name');
        await expectPokemonRows(viewer, [...POKEMON_NAMES].sort());
        expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('bulbasaur');
        select(controls.getByRole('combobox', { name: 'Direction' }), 'descending');
        await expectPokemonRows(viewer, [...POKEMON_NAMES].sort().reverse());
        expect(viewer.querySelectorAll('tr[aria-selected=true]')).toHaveLength(1);
        expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('bulbasaur');
        expect(viewer.querySelector('tr[aria-selected=true] button')).toHaveAttribute('aria-pressed', 'true');
        expect(preview?.querySelector('[slot=text] code')?.textContent).toBe(pokemonSource);
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
        expect(controls.queryByRole('textbox', { name: 'Source' })).toBeNull();
        expect(controls.queryByRole('button', { name: 'Reset source' })).toBeNull();
        await expectStockRows(viewer, ['Cherry', 'Lemon', 'Apple', 'Pear', 'Plum']);
        const preview = canvasElement.querySelector('cem-demo-element[legend="stock-cells.xml"]');
        await waitFor(() => expect(preview?.querySelector('[slot=text] code')?.textContent).toBe(stockSource));
        expect(preview?.querySelector('[slot=demo]')?.textContent).toBe('');
        await userEvent.click(viewer.querySelector('tbody button') as HTMLButtonElement);
        await waitFor(() => expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('Cherry'));
        select(controls.getByRole('combobox', { name: 'Sort column' }), 'name');
        await expectStockRows(viewer, ['Apple', 'Cherry', 'Lemon', 'Pear', 'Plum']);
        expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('Out of stock (0)');
        select(controls.getByRole('combobox', { name: 'Direction' }), 'descending');
        await expectStockRows(viewer, ['Plum', 'Pear', 'Lemon', 'Cherry', 'Apple']);
        expect(viewer.querySelectorAll('tr[aria-selected=true]')).toHaveLength(1);
        expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('Cherry');
        expect(viewer.querySelector('tr[aria-selected=true] button')).toHaveAttribute('aria-pressed', 'true');
        expect(preview?.querySelector('[slot=text] code')?.textContent).toBe(stockSource);
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
            select(within(pokemon).getByRole('combobox', { name: 'Sort column' }), 'name');
            await waitFor(() => expect(pokemon.querySelector('tbody tr')).toHaveTextContent('blastoise'), { timeout: 10000 });
            expect(pokemon.querySelectorAll('img')).toHaveLength(10);
            expect(pokemon.querySelector('textarea')).toBeNull();
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
            expect(stock.querySelector('textarea')).toBeNull();
            expect(stock.querySelectorAll('tbody tr')).toHaveLength(5);
            select(within(stock).getByRole('combobox', { name: 'Sort column' }), 'name');
            await waitFor(() => expect(stock.querySelector('tbody tr')).toHaveTextContent('Apple'), { timeout: 10000 });
            expect(stock.querySelectorAll('strong')).toHaveLength(1);
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
            await waitFor(() => expect(first.querySelectorAll('cem-demo-element[data-state=ready]')).toHaveLength(6), { timeout: 10000 });
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
            expect(stock.querySelector('textarea')).toBeNull();
            expect(stock.querySelectorAll('tbody tr')).toHaveLength(5);
            select(within(stock).getByRole('combobox', { name: 'Sort column' }), 'name');
            await waitFor(() => expect(stock.querySelector('tbody tr')).toHaveTextContent('Apple'), { timeout: 10000 });
            expect(stock.querySelectorAll('strong')).toHaveLength(1);
        } finally {
            canvasElement.replaceChildren();
            scope.dispose();
        }
    },
};

export const NativeAttributeValues: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await expectNativeValues(canvasElement);
    },
};
