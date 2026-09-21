import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, installCemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = { title: 'CEM Elements/Declaration Remount', tags: ['test'] };
export default meta;
type Story = StoryObj;
const css = '<style>span { color: rgb(1, 2, 3); }</style><span>Ready</span>';
const styles = (root: ParentNode) => root.querySelectorAll('style[data-cem-declaration-style]');
const codes = (runtime: CemElementRuntime, declaration: HTMLElement) =>
    runtime.diagnosticsFor(declaration).map(diagnostic => diagnostic.code);

function declaration(tag: string, source = css, declarationTag = 'div'): HTMLElement {
    const owner = document.createElement(declarationTag);
    owner.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.innerHTML = source;
    owner.append(template);
    return owner;
}

export const IdenticalRemountAndOriginalReconnect: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationTag: 'remount-declaration', declarationScope: scope });
        runtime.install(window);
        const tag = 'remount-styled-card';
        const first = declaration(tag, css, runtime.declarationTag);
        canvasElement.append(first);
        await runtime.whenDeclarationSettled(first);
        const instance = document.createElement(tag);
        canvasElement.append(instance);
        await runtime.whenRenderSettled(instance);
        const color = () => getComputedStyle(instance.querySelector('span') as Element).color;
        expect(color()).toBe('rgb(1, 2, 3)');
        const style = styles(first)[0];
        const constructor = customElements.get(tag);

        const duplicate = declaration(tag, css, runtime.declarationTag);
        canvasElement.append(duplicate);
        await runtime.whenDeclarationSettled(duplicate);
        expect(codes(runtime, duplicate)).toEqual(['cem-element.registry_same_scope_duplicate']);
        expect(styles(duplicate)).toHaveLength(0);
        duplicate.remove();
        first.remove();
        expect(color()).not.toBe('rgb(1, 2, 3)');

        const incompatible = declaration(tag, '<span>Changed</span>', runtime.declarationTag);
        canvasElement.append(incompatible);
        await runtime.whenDeclarationSettled(incompatible);
        expect(codes(runtime, incompatible)).toEqual(['cem-element.registry_same_scope_duplicate']);
        incompatible.remove();

        const differentStyleScope = declaration(tag, css, runtime.declarationTag);
        differentStyleScope.setAttribute('scope', 'different-style-scope');
        canvasElement.append(differentStyleScope);
        await runtime.whenDeclarationSettled(differentStyleScope);
        expect(codes(runtime, differentStyleScope)).toEqual(['cem-element.registry_same_scope_duplicate']);
        expect(styles(differentStyleScope)).toHaveLength(0);
        differentStyleScope.remove();

        const replacement = declaration(tag, css, runtime.declarationTag);
        canvasElement.append(replacement);
        await runtime.whenDeclarationSettled(replacement);
        expect(codes(runtime, replacement)).toEqual([]);
        expect(customElements.get(tag)).toBe(constructor);
        expect(styles(replacement)[0]).toBe(style);
        expect(color()).toBe('rgb(1, 2, 3)');

        canvasElement.append(first);
        expect(styles(canvasElement)).toHaveLength(1);
        expect(styles(replacement)[0]).toBe(style);
        replacement.remove();
        await waitFor(() => expect(styles(first)[0]).toBe(style));
        expect(color()).toBe('rgb(1, 2, 3)');
        first.remove();
        await waitFor(() => expect(style.isConnected).toBe(false));
        canvasElement.append(replacement);
        await waitFor(() => expect(styles(replacement)[0]).toBe(style));
        expect(color()).toBe('rgb(1, 2, 3)');
        scope.dispose();
        expect(style.isConnected).toBe(false);
        replacement.remove();
        canvasElement.append(replacement);
        expect(codes(runtime, replacement)).toContain('cem-element.scope_disposed');
        expect(style.isConnected).toBe(false);
    },
};

export const ManualOwnersAliasesAndDisposal: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const processingScope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationScope: processingScope });
        const tag = 'remount-manual-card';
        const first = declaration(tag);
        runtime.registerDeclaration(first);
        await runtime.whenDeclarationSettled(first);
        const reservedDuplicate = declaration(tag);
        runtime.registerDeclaration(reservedDuplicate);
        await runtime.whenDeclarationSettled(reservedDuplicate);
        expect(codes(runtime, reservedDuplicate)).toEqual(['cem-element.registry_same_scope_duplicate']);
        canvasElement.append(first, document.createElement(tag));
        await waitFor(() => expect(styles(first)).toHaveLength(1));
        const style = styles(first)[0];

        const ancestor = createCemDeclarationScope({ document });
        const aliasScope = createCemDeclarationScope({ document, parent: ancestor });
        const aliasRuntime = new CemElementRuntime({ declarationScope: aliasScope });
        const alias = declaration(tag);
        canvasElement.append(alias);
        aliasRuntime.registerDeclaration(alias);
        await aliasRuntime.whenDeclarationSettled(alias);
        expect(codes(aliasRuntime, alias)).toEqual([]);
        const differentScopeRuntime = new CemElementRuntime({ declarationScope: createCemDeclarationScope({ document }) });
        const differentStyleScope = declaration(tag);
        differentStyleScope.setAttribute('scope', 'different-alias-scope');
        canvasElement.append(differentStyleScope);
        differentScopeRuntime.registerDeclaration(differentStyleScope);
        await differentScopeRuntime.whenDeclarationSettled(differentStyleScope);
        expect(codes(differentScopeRuntime, differentStyleScope)).toEqual(['cem-element.browser_tag_collision']);
        differentStyleScope.remove();
        first.remove();
        await waitFor(() => expect(styles(alias)[0]).toBe(style));

        const independentScope = createCemDeclarationScope({ document });
        const independentRuntime = new CemElementRuntime({ declarationScope: independentScope });
        const independent = declaration(tag);
        canvasElement.append(independent);
        independentRuntime.registerDeclaration(independent);
        await independentRuntime.whenDeclarationSettled(independent);
        ancestor.dispose();
        expect(styles(alias)).toHaveLength(0);
        expect(styles(independent)[0]).toBe(style);
        expect(aliasRuntime.registerDeclaration(alias)).toBe(false);
        expect(codes(aliasRuntime, alias)).toContain('cem-element.scope_ancestor_disposed');

        independent.remove();
        await waitFor(() => expect(style.isConnected).toBe(false));
        canvasElement.append(first);
        await waitFor(() => expect(styles(first)[0]).toBe(style));
        processingScope.dispose();
        expect(style.isConnected).toBe(false);
        const freshRuntime = new CemElementRuntime({ declarationScope: createCemDeclarationScope({ document }) });
        const afterDisposal = declaration(tag);
        canvasElement.append(afterDisposal);
        freshRuntime.registerDeclaration(afterDisposal);
        await freshRuntime.whenDeclarationSettled(afterDisposal);
        expect(codes(freshRuntime, afterDisposal)).toContain('cem-element.scope_disposed');
        expect(styles(canvasElement)).toHaveLength(0);
        independentScope.dispose();
    },
};

export const RemovalBeforeExternalLoadSettles: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        let release = (_source: string): void => { throw new Error('source promise not initialized'); };
        const source = new Promise<string>(resolve => { release = resolve; });
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({
            declarationTag: 'remount-delayed-declaration', declarationScope: scope,
            loadSrcDocument: () => source,
        });
        runtime.install(window);
        const mount = () => {
            const owner = document.createElement(runtime.declarationTag);
            owner.setAttribute('tag', 'remount-delayed-card');
            owner.setAttribute('src', './delayed-remount.html#card');
            canvasElement.append(owner);
            return owner;
        };
        const first = mount();
        first.remove();
        release(`<template id="card">${css}</template>`);
        await runtime.whenDeclarationSettled(first);
        expect(codes(runtime, first)).toEqual([]);
        expect(styles(canvasElement)).toHaveLength(0);
        const replacement = mount();
        await runtime.whenDeclarationSettled(replacement);
        expect(codes(runtime, replacement)).toEqual([]);
        expect(styles(replacement)).toHaveLength(1);
        const instance = document.createElement('remount-delayed-card');
        canvasElement.append(instance);
        await runtime.whenRenderSettled(instance);
        expect(getComputedStyle(instance.querySelector('span') as Element).color).toBe('rgb(1, 2, 3)');
        scope.dispose();
    },
};

export const ManualMountAndRemovalInOneTask: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationScope: scope });
        const first = declaration('remount-batched-card');
        runtime.registerDeclaration(first);
        await runtime.whenDeclarationSettled(first);
        canvasElement.append(first);
        first.remove();
        // No observer checkpoint has occurred: registration must consume the
        // queued connection evidence before deciding on reuse.
        const replacement = declaration('remount-batched-card');
        canvasElement.append(replacement);
        runtime.registerDeclaration(replacement);
        await runtime.whenDeclarationSettled(replacement);
        expect(codes(runtime, replacement)).toEqual([]);
        expect(styles(replacement)).toHaveLength(1);
        scope.dispose();
    },
};

export const AdoptionCannotMoveRegistrationOwnership: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationTag: 'remount-adoption-declaration', declarationScope: scope });
        runtime.install(window);
        const first = declaration('remount-adoption-card', css, runtime.declarationTag);
        canvasElement.append(first);
        await runtime.whenDeclarationSettled(first);
        const style = styles(first)[0];
        const frame = document.createElement('iframe');
        canvasElement.append(frame);
        const destination = frame.contentDocument;
        if (!destination) throw new Error('expected same-origin frame document');
        destination.body.append(destination.adoptNode(first));
        await waitFor(() => expect(style.isConnected).toBe(false));
        expect(codes(runtime, first)).toContain('cem-element.scope_document_mismatch');
        expect(styles(destination)).toHaveLength(0);
        const replacement = declaration('remount-adoption-card', css, runtime.declarationTag);
        canvasElement.append(replacement);
        await runtime.whenDeclarationSettled(replacement);
        expect(codes(runtime, replacement)).toEqual([]);
        expect(styles(replacement)[0]).toBe(style);
        expect(style.ownerDocument).toBe(document);
        scope.dispose();
    },
};

export const RepeatedSourceLoadedGallery: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const runtime = installCemElementRuntime(window);
        const src = new URL('../../demo/hex-grid.html', import.meta.url).href;
        let retainedStyles: Element[] = [];
        let constructor: CustomElementConstructor | undefined;
        for (let mount = 0; mount < 3; mount++) {
            const owner = document.createElement('cem-element');
            owner.setAttribute('tag', 'remount-hex-gallery');
            owner.setAttribute('src', src);
            const page = document.createElement('remount-hex-gallery');
            canvasElement.replaceChildren(owner, page);
            await runtime.whenDeclarationSettled(owner);
            expect(codes(runtime, owner)).toEqual([]);
            await runtime.whenRenderSettled(page);
            await waitFor(() => {
                expect(page.querySelectorAll('.hex-link').length).toBeGreaterThanOrEqual(14);
                expect(styles(page)).toHaveLength(4);
                expect(getComputedStyle(page.querySelector('.hex-link') as Element).clipPath).toContain('polygon');
                expect(getComputedStyle(page.querySelector('.hex-grid') as Element).display).toBe('flex');
            }, { timeout: 15000 });
            for (const nested of page.querySelectorAll<HTMLElement>('cem-element')) {
                await runtime.whenDeclarationSettled(nested);
                expect(codes(runtime, nested)).toEqual([]);
            }
            if (mount === 0) {
                retainedStyles = Array.from(styles(page));
                constructor = customElements.get('cem-hex-grid');
            } else {
                expect(Array.from(styles(page)).every((style, index) => style === retainedStyles[index])).toBe(true);
                expect(customElements.get('cem-hex-grid')).toBe(constructor);
            }
            canvasElement.replaceChildren();
            expect(retainedStyles.every(style => !style.isConnected)).toBe(true);
        }
    },
};
