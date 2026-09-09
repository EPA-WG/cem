import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';

import advancedDemoSource from '../demo/advanced.html?raw';
import demoCss from '../demo/demo.css?raw';
import dwarfsSource from '../demo/dwarfs.json?raw';
import basicDemoSource from '../demo/index.html?raw';
import syntaxColoringDemoSource from '../demo/syntax-coloring.html?raw';
import cemMlCompleteSource from '../demo/syntax/cem-ml-complete.cem?raw';
import cemMlErrorSource from '../demo/syntax/cem-ml-error.cem?raw';
import htmlCompleteSource from '../demo/syntax/html-complete.html?raw';
import htmlRecoverySource from '../demo/syntax/html-recovery.html?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- The offline syntax fixture embeds the generated CEM theme used by the authored demo page.
import cemThemeCss from '../../cem-theme/dist/lib/css/cem-combined.css?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- The offline document fixture initializes the exact generated browser loader as a local blob.
import wasmLoaderSource from '../../cem-ml-npm/dist/wasm/browser/cem_ml.js?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- The offline document fixture supplies the exact local WASM bytes to that generated loader.
import wasmBinaryUrl from '../../cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm?url';
import builtElementSource from '../dist/cem-demo-element.js?raw';
import builtRuntimeSource from '../dist/cem-ml-runtime.js?raw';
import builtHighlightSource from '../dist/source-highlight.js?raw';
import { CemDemoElement } from './cem-demo-element.js';

const wasmLoaderUrl = moduleBlobUrl(wasmLoaderSource);
const highlightModuleUrl = moduleBlobUrl(builtHighlightSource);
const runtimeModuleUrl = moduleBlobUrl(builtRuntimeSource);
const elementModuleUrl = moduleBlobUrl(
    builtElementSource
        .replaceAll("'./cem-ml-runtime.js'", JSON.stringify(runtimeModuleUrl))
        .replaceAll("'./source-highlight.js'", JSON.stringify(highlightModuleUrl))
);
const fetchedHtmlUrl = new URL('../demo/fetched.fixture', import.meta.url).href;

const meta: Meta = {
    title: 'CEM Demo Element/Behavior',
    component: 'cem-demo-element',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const InlineBody: Story = {
    render: () => `
        <cem-demo-element legend="Inline body">
            <button type="button">Keep this node</button>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;

        await expect(demo).toHaveAttribute('data-state', 'ready');
        await expect(canvas.getByRole('heading', { name: 'Inline body' })).toBeVisible();
        await expect(canvas.getByRole('button', { name: 'Keep this node' })).toBeVisible();
        await expect(requiredRegion(demo, 'text').querySelector('code')).toHaveTextContent(
            '<button type="button">Keep this node</button>'
        );
    },
};

export const TemplateLegendAndDescription: Story = {
    render: () => `
        <cem-demo-element
            legend="Template source"
            description="The description can contain <b>trusted markup</b>."
        >
            <template><p class="message">Candle 🕯️</p></template>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;

        await expect(
            within(requiredRegion(demo, 'demo')).getByText('Candle 🕯️')
        ).toHaveClass('message');
        await expect(requiredRegion(demo, 'description').querySelector('b')).toHaveTextContent(
            'trusted markup'
        );
        const code = requiredRegion(demo, 'text').querySelector('code');
        await expect(code).toHaveTextContent('<p class="message">Candle 🕯️</p>');
        await expect(code).toHaveClass('cem-source-code', 'language-html');
        await expect(code?.querySelector(':scope > [class], :scope > [data-role]')).toBeNull();
        const name = code?.querySelector(':scope > b');
        const attribute = code?.querySelector(':scope > var');
        const value = code?.querySelector(':scope > i');
        await expect(name).toHaveTextContent('p');
        await expect(attribute).toHaveTextContent('class');
        await expect(value).toHaveTextContent('message');
        await expect(
            getComputedStyle(demo).getPropertyValue('--cem-color-syntax-name').trim()
        ).toBe('#002f65');
        await expect(getComputedStyle(name as Element).color).toBe('rgb(0, 47, 101)');
        await expect(getComputedStyle(attribute as Element).color).toBe('rgb(80, 36, 0)');
        await expect(getComputedStyle(value as Element).color).toBe('rgb(106, 27, 154)');
        await expect(getComputedStyle(name as Element).fontWeight).toBe('700');
        await expect(getComputedStyle(attribute as Element).fontWeight).toBe('700');
    },
};

export const ExplicitRegions: Story = {
    render: () => `
        <cem-demo-element legend="Legend moved to the bottom">
            <template>Ignored default template</template>
            <template slot="source"><i>Croissant 🥐</i></template>
            <p><code>slot="source"</code> selects the code to present.</p>
            <p><code>slot="demo"</code> receives the live HTML.</p>
            <div slot="demo">Replaced demo placeholder</div>
            <p><code>slot="text"</code> receives the highlighted source.</p>
            <div slot="text">Replaced source placeholder</div>
            <div slot="description"><b>Description moved above the legend</b></div>
            <div slot="legend"></div>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;

        const templates = demo.querySelectorAll('template');
        const defaultTemplate = templates[0];
        const sourceTemplate = requiredRegion(demo, 'source');
        const liveDemo = requiredRegion(demo, 'demo');
        const sourceText = requiredRegion(demo, 'text');
        const description = requiredRegion(demo, 'description');
        const legend = requiredRegion(demo, 'legend');
        const bodyText = demo.querySelector(':scope > p');
        if (!bodyText) throw new Error('Expected authored body text');

        await expect(templates).toHaveLength(2);
        await expect(defaultTemplate).not.toBe(sourceTemplate);
        await expect((defaultTemplate as HTMLTemplateElement).content.textContent).toContain(
            'Ignored default template'
        );
        await expect(liveDemo.querySelector('i')).toHaveTextContent('Croissant 🥐');
        await expect(liveDemo).not.toHaveTextContent('Replaced demo placeholder');
        await expect(sourceText.querySelector('code')).toHaveTextContent('<i>Croissant 🥐</i>');
        await expect(sourceText).not.toHaveTextContent('Replaced source placeholder');
        await expect(demo.querySelectorAll(':scope > p')).toHaveLength(3);
        await expect(Number.parseFloat(getComputedStyle(bodyText).paddingInlineStart)).toBeGreaterThan(0);
        await expect(description.querySelector('b')).toHaveTextContent(
            'Description moved above the legend'
        );
        await expect(legend).toHaveTextContent('Legend moved to the bottom');
        await expect(legend.previousElementSibling).toBe(description);
        await expect(demo.lastElementChild).toBe(legend);
    },
};

export const ProgrammaticSource: Story = {
    render: () => `
        <button type="button">Use node source</button>
        <cem-demo-element legend="Programmatic source">
            <template><p>Original source</p></template>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;
        const sourceNode = document.createElement('section');
        sourceNode.innerHTML = '<strong>Node source</strong>';

        await userEvent.click(canvas.getByRole('button', { name: 'Use node source' }));
        demo.source = sourceNode;
        await demo.updateComplete;
        await expect(requiredRegion(demo, 'text').querySelector('code')).toHaveTextContent(
            '<strong>Node source</strong>'
        );

        demo.setAttribute('source', 'const answer = 42;');
        demo.type = 'js';
        demo.legend = 'Updated programmatically';
        demo.description = 'Property-backed description';
        await demo.updateComplete;
        const code = requiredRegion(demo, 'text').querySelector('code');
        await expect(code).toHaveTextContent('const answer = 42;');
        await expect(code?.querySelector(':scope > strong')).toHaveTextContent('const');
        await expect(code?.querySelector(':scope > u')).toHaveTextContent('42');
        await expect(requiredRegion(demo, 'legend')).toHaveTextContent('Updated programmatically');
        await expect(requiredRegion(demo, 'description')).toHaveTextContent(
            'Property-backed description'
        );
    },
};

export const ExternalSourceWithAutoType: Story = {
    render: () => '<cem-demo-element legend="Fetched source" type="auto"></cem-demo-element>',
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        const source = 'body { color: rebeccapurple; }';
        demo.src = `data:text/css;charset=utf-8,${encodeURIComponent(source)}`;
        await waitFor(() => expect(demo).toHaveAttribute('data-state', 'ready'));

        const code = requiredRegion(demo, 'text').querySelector('code');
        await expect(code).toHaveAttribute('data-language', 'css');
        await expect(code).toHaveTextContent(source);
        await expect(demo.source).toBe(source);
    },
};

export const ExternalHtmlSource: Story = {
    render: () => `<cem-demo-element
        legend="Fetched HTML source"
        type="html"
        src="${fetchedHtmlUrl}"
    ></cem-demo-element>`,
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        await waitFor(() => expect(demo).toHaveAttribute('data-state', 'ready'));

        const code = requiredRegion(demo, 'text').querySelector('code');
        const live = within(requiredRegion(demo, 'demo'));
        await expect(code).toHaveAttribute('data-language', 'html');
        await expect(code?.querySelector('b')).toHaveTextContent('article');
        await expect(live.getByRole('heading', { name: 'Fetched HTML is live' })).toBeVisible();
        await expect(live.getByRole('link', { name: 'Open the neighboring data file' }))
            .toHaveAttribute('href', new URL('./dwarfs.json', fetchedHtmlUrl).href);
    },
};

export const CemMlWasmRender: Story = {
    render: () => `
        <cem-demo-element legend="CEM-ML to HTML" type="cem-ml">
            <template>{article @id="welcome" | {strong | Hello from CEM-ML}}</template>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;

        const article = requiredRegion(demo, 'demo').querySelector('article#welcome');
        await expect(article?.querySelector('strong')).toHaveTextContent('Hello from CEM-ML');
        await expect(demo).toHaveAttribute('data-state', 'ready');
        await expect(demo.lastResult?.schemaVersion).toBe(1);
        await expect(demo.lastResult?.status).toBe('rendered');
        await expect(demo.lastResult?.outputSpans.length).toBeGreaterThan(0);
        const code = requiredRegion(demo, 'text').querySelector('code');
        await expect(code?.querySelector(':scope > b')).toHaveTextContent('article');
        await expect(code?.querySelector(':scope > var')).toHaveTextContent('id');
        await expect(code?.querySelector(':scope > i')).toHaveTextContent('welcome');
    },
};

export const CemMlDiagnostic: Story = {
    render: () => `
        <cem-demo-element legend="Invalid CEM-ML" type="cem-ml">
            <template>{article @id=unfinished |</template>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;

        await expect(demo).toHaveAttribute('data-state', 'error');
        await expect(requiredRegion(demo, 'demo')).toBeEmptyDOMElement();
        const diagnostics = requiredRegion(demo, 'status');
        await expect(diagnostics).toHaveAttribute('role', 'status');
        await expect(diagnostics).toHaveAttribute('aria-live', 'polite');
        await expect(diagnostics.querySelector('li[data-severity]')).not.toBeNull();
        await expect(diagnostics.textContent).not.toBe('');
    },
};

export const OfflineResourceBoundary: Story = {
    render: () => `
        <cem-demo-element legend="No CDN dependencies">
            <template><p>All component resources are local.</p></template>
        </cem-demo-element>
    `,
    play: async ({ canvasElement }) => {
        const demo = requiredDemo(canvasElement);
        await demo.updateComplete;
        const remoteResources = performance
            .getEntriesByType('resource')
            .map((entry) => entry.name)
            .filter((name) => /^https?:/.test(name))
            .filter((name) => new URL(name).origin !== location.origin);

        await expect(remoteResources).toEqual([]);
        await expect(document.querySelector('style[data-cem-demo-element-styles]')).not.toBeNull();
        await expect(
            document.querySelector('link[href*="unpkg"],script[src*="unpkg"],link[href*="jsdelivr"],script[src*="jsdelivr"]')
        ).toBeNull();
    },
};

export const OfflineDemoDocuments: Story = {
    render: () => {
        const root = document.createElement('section');
        const basic = document.createElement('iframe');
        basic.title = 'Basic demo document';
        basic.srcdoc = offlineDemoDocument(basicDemoSource);
        const advanced = document.createElement('iframe');
        advanced.title = 'Advanced demo document';
        advanced.srcdoc = offlineDemoDocument(advancedDemoSource);
        const syntaxColoring = document.createElement('iframe');
        syntaxColoring.title = 'Syntax coloring decision document';
        syntaxColoring.srcdoc = offlineDemoDocument(syntaxColoringDemoSource);
        root.append(basic, advanced, syntaxColoring);
        return root;
    },
    play: async ({ canvasElement }) => {
        const frames = Array.from(canvasElement.querySelectorAll('iframe'));
        await expect(frames).toHaveLength(3);

        for (const frame of frames) {
            await waitFor(() => {
                expect(frame.contentDocument?.querySelector('cem-demo-element')).not.toBeNull();
            });
            const childWindow = frame.contentWindow;
            const childDocument = frame.contentDocument;
            if (!childWindow || !childDocument) throw new Error('Expected a same-origin demo iframe');
            await childWindow.customElements.whenDefined('cem-demo-element');
            await waitFor(() => {
                const demos = Array.from(childDocument.querySelectorAll('cem-demo-element'));
                expect(demos.length).toBeGreaterThan(0);
                expect(demos.every((demo) => demo.getAttribute('data-state') !== 'loading')).toBe(true);
            });

            const remoteResources = childWindow.performance
                .getEntriesByType('resource')
                .map((entry) => entry.name)
                .filter((name) => /^https?:/.test(name))
                .filter((name) => new URL(name).origin !== location.origin);
            await expect(remoteResources).toEqual([]);
            await expect(
                childDocument.querySelector('script[src^="http"],link[href^="http"]')
            ).toBeNull();
        }

        const basicDocument = frames[0]?.contentDocument;
        const cemMlDemo = basicDocument?.querySelector('cem-demo-element[type="cem-ml"]');
        const renderedStrong = cemMlDemo?.querySelector('[slot="demo"] article.message strong');
        if (!renderedStrong) {
            throw new Error(`Expected offline CEM-ML output; observed ${cemMlDemo?.outerHTML ?? 'no CEM-ML demo'}`);
        }
        await expect(renderedStrong).toHaveTextContent('Hello from CEM-ML');

        const syntaxWindow = frames[2]?.contentWindow;
        const syntaxDocument = frames[2]?.contentDocument;
        if (!syntaxWindow || !syntaxDocument) {
            throw new Error('Expected the same-origin syntax-coloring iframe');
        }
        const roleRows = syntaxDocument.querySelectorAll('[data-syntax-role]');
        await expect(roleRows).toHaveLength(10);
        const sourceModelKinds = Array.from(
            syntaxDocument.querySelectorAll('.syntax-model-key dd code'),
            (element) => element.textContent
        );
        await expect(sourceModelKinds).toHaveLength(28);
        await expect(sourceModelKinds).toEqual(expect.arrayContaining([
            'Doctype',
            'StartElement',
            'EndElement',
            'Text',
            'RawText',
            'Rcdata',
            'Comment',
            'Delimiter',
            'ElementName',
            'Whitespace',
            'AttributeName',
            'Equals',
            'Quote',
            'AttributeValue',
            'Keyword',
            'Raw',
            'NodeStart',
            'NodeEnd',
            'Attribute',
            'Trivia',
            'ProcessingInstruction',
            'ExpressionNode',
            'AnonymousScopeStart',
            'Directive',
            'RichContent',
            'Error',
        ]));

        const htmlTags = sourceTags(syntaxDocument, '#html-syntax-complete');
        await expect(htmlTags).toEqual(expect.arrayContaining([
            'strong', 'b', 'var', 'i', 'small', 'samp',
        ]));
        const cemMlTags = sourceTags(syntaxDocument, '#cem-ml-syntax-complete');
        await expect(cemMlTags).toEqual(expect.arrayContaining([
            'strong', 'b', 'var', 'i', 'small',
        ]));
        await expect(
            syntaxDocument.querySelector('#cem-ml-syntax-error')
        ).toHaveAttribute('data-state', 'error');
        await expect(
            syntaxDocument.querySelector('#cem-ml-syntax-error [slot="text"] mark')
        ).toHaveTextContent('{42}');

        const select = syntaxDocument.querySelector<HTMLSelectElement>('#theme-mode');
        const description = syntaxDocument.querySelector<HTMLOutputElement>('#theme-description');
        if (!select || !description) throw new Error('Expected syntax theme controls');
        const themes = [
            'cem-theme-native',
            'cem-theme-light',
            'cem-theme-dark',
            'cem-theme-contrast-light',
            'cem-theme-contrast-dark',
        ];
        for (const theme of themes) {
            select.value = theme;
            select.dispatchEvent(new syntaxWindow.Event('change', { bubbles: true }));
            await expect(syntaxDocument.body.dataset.theme).toBe(theme);
            await expect(description.value).not.toBe('');

            const htmlColors = sourceColors(
                syntaxWindow,
                syntaxDocument,
                '#html-syntax-complete'
            );
            const cemMlColors = sourceColors(
                syntaxWindow,
                syntaxDocument,
                '#cem-ml-syntax-complete'
            );
            await expect(cemMlColors).toEqual(htmlColors);
            await expect(htmlColors.every((color) => color !== 'rgba(0, 0, 0, 0)')).toBe(true);
            await expect(htmlColors).toEqual(
                syntaxRoleKeyColors(syntaxWindow, syntaxDocument)
            );
            await expect(
                sourceRoleColor(
                    syntaxWindow,
                    syntaxDocument,
                    '#cem-ml-syntax-error',
                    'diagnostic.error'
                )
            ).toBe(syntaxRoleKeyColor(syntaxWindow, syntaxDocument, 'error'));
            await expect(
                syntaxWindow.getComputedStyle(
                    requiredSourceToken(
                        syntaxDocument,
                        '#html-syntax-complete',
                        'syntax.name'
                    )
                ).fontWeight
            ).toBe('700');
            await expect(
                syntaxWindow.getComputedStyle(
                    requiredSourceToken(
                        syntaxDocument,
                        '#html-syntax-complete',
                        'syntax.attribute'
                    )
                ).fontWeight
            ).toBe('700');
        }
    },
};

function requiredDemo(root: ParentNode): CemDemoElement {
    const demo = root.querySelector('cem-demo-element');
    if (!(demo instanceof CemDemoElement)) throw new Error('Expected one cem-demo-element');
    return demo;
}

function offlineDemoDocument(source: string): string {
    const importMap = `<script type="importmap">${JSON.stringify({
        imports: { '@epa-wg/cem-ml/wasm': wasmLoaderUrl },
    })}</script>`;
    const componentImport = `<script type="module">
        const wasm = await import('@epa-wg/cem-ml/wasm');
        const wasmResponse = await fetch(${JSON.stringify(wasmBinaryUrl)});
        await wasm.default({ module_or_path: wasmResponse });
        const { defineCemDemoElement } = await import(${JSON.stringify(elementModuleUrl)});
        defineCemDemoElement(customElements);
    </script>`;
    const dwarfsUrl = `data:application/json;charset=utf-8,${encodeURIComponent(dwarfsSource)}`;
    const syntaxFixtures = new Map([
        ['./syntax/html-complete.html', sourceDataUrl('text/html', htmlCompleteSource)],
        ['./syntax/html-recovery.html', sourceDataUrl('text/html', htmlRecoverySource)],
        ['./syntax/cem-ml-complete.cem', sourceDataUrl('application/cem', cemMlCompleteSource)],
        ['./syntax/cem-ml-error.cem', sourceDataUrl('application/cem', cemMlErrorSource)],
    ]);

    let documentSource = source
        .replace(/<link rel="stylesheet" href="\.\/demo\.css">/u, `<style>${demoCss}</style>`)
        .replace(
            '<link rel="stylesheet" href="../../cem-theme/dist/lib/css/cem-combined.css">',
            `<style>${cemThemeCss}</style>`
        )
        .replace(/<script type="importmap">[\s\S]*?<\/script>/u, importMap)
        .replace('<script type="module" src="../dist/index.js"></script>', componentImport)
        .replaceAll('./dwarfs.json', dwarfsUrl);
    for (const [path, dataUrl] of syntaxFixtures) {
        documentSource = documentSource.replaceAll(path, dataUrl);
    }
    return documentSource;
}

function sourceDataUrl(contentType: string, source: string): string {
    return `data:${contentType};charset=utf-8,${encodeURIComponent(source)}`;
}

function sourceTags(document: Document, demoSelector: string): string[] {
    return Array.from(
        document.querySelectorAll(`${demoSelector} [slot="text"] code > *`),
        (element) => element.localName
    );
}

function sourceColors(
    window: Window,
    document: Document,
    demoSelector: string
): [string, string, string, string] {
    return ['syntax.name', 'syntax.attribute', 'syntax.keyword', 'syntax.string'].map(
        (role) => sourceRoleColor(window, document, demoSelector, role)
    ) as [string, string, string, string];
}

function sourceRoleColor(
    window: Window,
    document: Document,
    demoSelector: string,
    role: string
): string {
    return window.getComputedStyle(requiredSourceToken(document, demoSelector, role)).color;
}

function requiredSourceToken(
    document: Document,
    demoSelector: string,
    role: string
): Element {
    const tag = sourceRoleTag(role);
    const token = document.querySelector(`${demoSelector} [slot="text"] code > ${tag}`);
    if (!token) throw new Error(`Expected ${role} in ${demoSelector}`);
    return token;
}

function sourceRoleTag(role: string): string {
    const tags: Record<string, string> = {
        'syntax.name': 'b',
        'syntax.attribute': 'var',
        'syntax.keyword': 'strong',
        'syntax.string': 'i',
        'syntax.number': 'u',
        'syntax.comment': 'small',
        'syntax.text': 'samp',
        'diagnostic.error': 'mark',
    };
    const tag = tags[role];
    if (!tag) throw new Error(`No semantic source tag for ${role}`);
    return tag;
}

function syntaxRoleKeyColors(
    window: Window,
    document: Document
): [string, string, string, string] {
    return ['name', 'attribute', 'keyword', 'string'].map((role) =>
        syntaxRoleKeyColor(window, document, role)
    ) as [string, string, string, string];
}

function syntaxRoleKeyColor(window: Window, document: Document, role: string): string {
    const token = document.querySelector(
        `[data-syntax-role="${role}"] td:nth-child(2) code > *`
    );
    if (!token) throw new Error(`Expected ${role} syntax-role specimen`);
    return window.getComputedStyle(token).color;
}

function moduleBlobUrl(source: string): string {
    return URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
}

function requiredRegion(demo: CemDemoElement, name: string): HTMLElement {
    const region = Array.from(demo.children).find(
        (child): child is HTMLElement => child instanceof HTMLElement && child.slot === name
    );
    if (!region) throw new Error(`Expected ${name} region`);
    return region;
}
