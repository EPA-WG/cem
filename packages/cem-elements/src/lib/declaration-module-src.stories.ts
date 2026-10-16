import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Declaration Module Sources', tags: ['test'] };
export default meta;
type Story = StoryObj;
let sequence = 0;
const PREFIX = '@epa-wg/cem-components/components/';
const DOCUMENT_URL = 'https://components.example.test/cem-toast/cem-toast.xhtml?revision=1';
const SOURCE = '<template id="cem-toast" type="text/cem-ml">{p | Toast}</template>'
    + '<template id="other" type="text/cem-ml">{p | Other}</template>';

function runtime(options: CemElementRuntimeOptions = {}) {
    const value = new CemElementRuntime({
        ...options, declarationTag: `cem-module-src-${++sequence}`,
    });
    value.install(window);
    return value;
}

function mount(root: HTMLElement, value: CemElementRuntime, src: string) {
    const declaration = document.createElement(value.declarationTag);
    const tag = `story-module-src-${++sequence}`;
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', src);
    const instance = document.createElement(tag);
    root.append(declaration, instance);
    value.registerDeclaration(declaration);
    return { declaration, instance };
}

async function ready(value: CemElementRuntime, owner: ReturnType<typeof mount>, text: string) {
    await value.whenDeclarationSettled(owner.declaration);
    expect(value.diagnosticsFor(owner.declaration)).toEqual([]);
    expect(customElements.get(owner.instance.localName)).toBeDefined();
    await value.whenRenderSettled(owner.instance);
    expect(owner.instance.querySelector('p')?.textContent).toBe(text);
}

function required(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`missing source fixture element: ${selector}`);
    return element;
}

function mappedSourceStory(exact: boolean): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const specifier = exact ? 'toast-document' : `${PREFIX}cem-toast/cem-toast.xhtml?revision=1`;
            const loads: string[] = [];
            const value = runtime({
                moduleUrlRoot: { importMap: { imports: exact
                    ? { 'toast-document': DOCUMENT_URL }
                    : { [PREFIX]: 'https://components.example.test/' } } },
                loadSrcDocument: async path => {
                    loads.push(path);
                    if (path !== DOCUMENT_URL) throw new Error(`unexpected document URL: ${path}`);
                    return SOURCE;
                },
            });
            const first = mount(canvasElement, value, `${specifier}#cem-toast`);
            const second = mount(canvasElement, value, `${specifier}#other`);
            await ready(value, first, 'Toast');
            await ready(value, second, 'Other');
            expect(loads).toEqual([DOCUMENT_URL]);
        },
    };
}

export const PrefixMappedXhtmlAndFragments = mappedSourceStory(false);
export const ExactMappedXhtmlAndFragments = mappedSourceStory(true);

export const HostResolversReceiveFragmentFreeSources: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const scoped of [true, false]) {
            const calls: string[] = [];
            const value = runtime({
                ...(scoped ? {
                    resolveScopedModuleUrl: request => {
                        expect(request.purpose).toBe('template-import');
                        expect(request.currentContext.baseUrl).toBe(document.baseURI);
                        calls.push(request.authoredSpecifier);
                        return DOCUMENT_URL;
                    },
                } : {
                    resolveModuleUrl: (specifier, baseDocument, resourceBaseUrl) => {
                        expect(baseDocument).toBe(document);
                        expect(resourceBaseUrl).toBe(document.baseURI);
                        calls.push(specifier);
                        return DOCUMENT_URL;
                    },
                }),
                loadSrcDocument: async path => {
                    expect(path).toBe(DOCUMENT_URL);
                    return SOURCE;
                },
            });
            await ready(value, mount(canvasElement, value, '@scope/toast.xhtml#cem-toast'), 'Toast');
            expect(calls).toEqual(['@scope/toast.xhtml']);
        }
    },
};

export const RelativeAndLocalSourcesKeepTheirLoadingPath: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const loads: string[] = [];
        const value = runtime({
            resolveScopedModuleUrl: () => { throw new Error('relative sources must bypass module resolution'); },
            loadSrcDocument: async path => { loads.push(path); return SOURCE; },
        });
        for (const path of ['./toast.xhtml', '../toast.xhtml', '/toast.xhtml', DOCUMENT_URL]) {
            await ready(value, mount(canvasElement, value, `${path}#cem-toast`), 'Toast');
        }
        expect(loads).toEqual(['./toast.xhtml', '../toast.xhtml', '/toast.xhtml', DOCUMENT_URL]);
        const local = document.createElement('template');
        local.id = `module-source-local-${++sequence}`;
        local.type = 'text/cem-ml';
        local.textContent = '{p | Local}';
        canvasElement.append(local);
        await ready(value, mount(canvasElement, value, `#${local.id}`), 'Local');
        expect(loads).toHaveLength(4);
    },
};

export const NestedSourceUsesItsParentModuleMap: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const loads: string[] = [];
        const parentUrl = 'https://components.example.test/parent/parent.xhtml';
        const childUrl = 'https://components.example.test/parent/child.xhtml';
        const childTag = `story-module-src-child-${++sequence}`;
        let declarationTag = '';
        const value = runtime({
            loadSrcDocument: async path => {
                loads.push(path);
                if (path === './parent.xhtml') return {
                    resolvedUrl: parentUrl,
                    resolverIdentity: 'module-src-parent',
                    body: new ReadableStream({ start(controller) {
                        controller.enqueue(new TextEncoder().encode(`<template id="parent" type="text/cem-ml">
                            {module-map | {resource @specifier=child-source @target="./child.xhtml"}}
                            {${declarationTag} @tag=${childTag} @src="child-source#cem-toast"}
                            {${childTag}}
                        </template>`));
                        controller.close();
                    } }),
                };
                if (path !== childUrl) throw new Error(`unexpected child URL: ${path}`);
                return SOURCE;
            },
        });
        declarationTag = value.declarationTag;
        const parent = mount(canvasElement, value, './parent.xhtml#parent');
        await value.whenDeclarationSettled(parent.declaration);
        await value.whenRenderSettled(parent.instance);
        await waitFor(() => expect(parent.instance.querySelector(`${declarationTag}[tag="${childTag}"]`)).not.toBeNull());
        const childDeclaration = required(parent.instance, `${declarationTag}[tag="${childTag}"]`);
        await value.whenDeclarationSettled(childDeclaration);
        expect(value.diagnosticsFor(childDeclaration)).toEqual([]);
        const child = required(parent.instance, childTag);
        await value.whenRenderSettled(child);
        expect(child.querySelector('p')?.textContent).toBe('Toast');
        expect(loads).toEqual(['./parent.xhtml', childUrl]);
    },
};

export const UnresolvedAndBlockedSourcesDoNotFetch: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const blocked of [false, true]) {
            let loads = 0;
            const value = runtime({
                moduleUrlRoot: { importMap: { imports: blocked ? { [PREFIX]: null } : {} } },
                ...(blocked ? { loadSrcDocument: async () => { loads++; return SOURCE; } } : {}),
            });
            const owner = mount(canvasElement, value, `${PREFIX}cem-toast/cem-toast.xhtml#cem-toast`);
            await value.whenDeclarationSettled(owner.declaration);
            expect(customElements.get(owner.instance.localName)).toBeUndefined();
            expect(value.diagnosticsFor(owner.declaration)).toEqual([
                expect.objectContaining({ code: 'cem-element.src_load_failed',
                    message: expect.stringContaining(blocked ? 'blocked' : 'not mapped') }),
            ]);
            expect(loads).toBe(0);
        }
    },
};

export const FailedSourceResolutionCanRetry: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        let attempts = 0;
        let loads = 0;
        const value = runtime({
            resolveScopedModuleUrl: async () => {
                if (++attempts === 1) throw new Error('controlled resolver failure');
                return DOCUMENT_URL;
            },
            loadSrcDocument: async path => {
                expect(path).toBe(DOCUMENT_URL);
                loads++;
                return SOURCE;
            },
        });
        const owner = mount(canvasElement, value, '@scope/toast.xhtml#cem-toast');
        await value.whenDeclarationSettled(owner.declaration);
        expect(loads).toBe(0);
        expect(value.diagnosticsFor(owner.declaration)).toEqual([
            expect.objectContaining({ code: 'cem-element.src_load_failed', message: expect.stringContaining('controlled resolver failure') }),
        ]);
        value.registerDeclaration(owner.declaration);
        await value.whenDeclarationSettled(owner.declaration);
        expect(customElements.get(owner.instance.localName)).toBeDefined();
        await value.whenRenderSettled(owner.instance);
        expect(owner.instance.querySelector('p')?.textContent).toBe('Toast');
        expect(attempts).toBe(2);
        expect(loads).toBe(1);
    },
};

export const LegacyHostLoaderSourcesRemainSupported: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const loads: string[] = [];
        const value = runtime({ loadSrcDocument: async path => { loads.push(path); return SOURCE; } });
        for (const specifier of ['@legacy/cards.xhtml', 'cards.xhtml']) {
            await ready(value, mount(canvasElement, value, `${specifier}#cem-toast`), 'Toast');
        }
        expect(loads).toEqual(['@legacy/cards.xhtml', 'cards.xhtml']);
    },
};

export const SiblingModuleMapsDoNotShareSourceDocuments: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const loads: string[] = [];
        const value = runtime({ loadSrcDocument: async path => {
            loads.push(path);
            return `<template id="card" type="text/cem-ml">{p | ${path}}</template>`;
        } });
        for (const name of ['first', 'second']) {
            const url = `https://components.example.test/${name}.xhtml`;
            const childTag = `story-module-src-child-${++sequence}`;
            const declaration = document.createElement(value.declarationTag);
            const parentTag = `story-module-src-parent-${++sequence}`;
            declaration.setAttribute('tag', parentTag);
            const template = document.createElement('template');
            template.type = 'text/cem-ml';
            template.textContent = `{module-map | {resource @specifier=child-document @target="${url}"}}
                {${value.declarationTag} @tag=${childTag} @src="child-document#card"}
                {${childTag}}`;
            declaration.append(template);
            const parent = document.createElement(parentTag);
            canvasElement.append(declaration, parent);
            value.registerDeclaration(declaration);
            await value.whenDeclarationSettled(declaration);
            await value.whenRenderSettled(parent);
            await waitFor(() => expect(parent.querySelector(`${value.declarationTag}[tag="${childTag}"]`)).not.toBeNull());
            const childDeclaration = required(parent, `${value.declarationTag}[tag="${childTag}"]`);
            await value.whenDeclarationSettled(childDeclaration);
            expect(value.diagnosticsFor(childDeclaration)).toEqual([]);
            const child = required(parent, childTag);
            await value.whenRenderSettled(child);
            expect(child.querySelector('p')?.textContent).toBe(url);
        }
        expect(loads).toEqual(['https://components.example.test/first.xhtml', 'https://components.example.test/second.xhtml']);
    },
};
