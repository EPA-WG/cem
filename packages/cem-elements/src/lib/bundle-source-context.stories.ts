import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime, CEM_RESOURCE_BASE_URL } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Bundle Source Context', tags: ['test'] };
export default meta;
type Story = StoryObj;
let sequence = 0;

// Verify source-base metadata through the external fragment loader.
// The fixture copies template text unchanged; it does not implement a bundler.
export const RelativeDependencyRetainsSourceBase: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const original = 'https://components.example.test/src/components/card/card.xhtml';
        const bundle = 'https://components.example.test/dist/components.xhtml';
        const template = `<template id="card" type="text/cem-ml">
{cem-module-url @slice=assetUrl @src="./icon.svg"}
{cem:if @test="datadom.slices.assetUrl" | {a @href="{$datadom.slices.assetUrl}" | Asset}}
</template>`;
        const source = `<cem-element xmlns="http://www.w3.org/1999/xhtml" tag="example-card">${template}</cem-element>`;
        const combined = `<html xmlns="http://www.w3.org/1999/xhtml"><body>
<cem-element tag="example-card" xml:base="../src/components/card/card.xhtml">${template}</cem-element>
</body></html>`;
        const runtime = new CemElementRuntime({
            declarationTag: `cem-bundle-context-${++sequence}`,
            loadSrcDocument: async path => {
                if (path === original) return source;
                if (path === bundle) return combined;
                throw new Error(`Unexpected source ${path}`);
            },
        });
        runtime.install(window);
        for (const [name, url, expected] of [
            ['individual', original, 'https://components.example.test/src/components/card/icon.svg'],
            ['combined', bundle, 'https://components.example.test/src/components/card/icon.svg'],
        ]) {
            const tag = `story-bundle-${name}-${sequence}`;
            const declaration = document.createElement(runtime.declarationTag);
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('src', `${url}#card`);
            canvasElement.append(declaration);
            await runtime.whenDeclarationSettled(declaration);
            expect(runtime.diagnosticsFor(declaration)).toEqual([]);
            const instance = document.createElement(tag);
            canvasElement.append(instance);
            await runtime.whenRenderSettled(instance);
            expect(runtime.snapshotInstance(instance).slices.assetUrl).toBe(expected);
            expect(instance.querySelector('a')?.getAttribute('href')).toBe(expected);
            expect((instance as HTMLElement & { [CEM_RESOURCE_BASE_URL]: string })[CEM_RESOURCE_BASE_URL]).toBe(original);
        }
    },
};

export const InvalidSourceBaseRejectsRegistration: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const base of ['http://[', 'card.xhtml#fragment']) {
            const id = ++sequence;
            const runtime = new CemElementRuntime({
                declarationTag: `cem-bundle-invalid-${id}`,
                loadSrcDocument: async () => `<div xml:base="${base}"><template id="card" type="text/cem-ml">{p | Invalid}</template></div>`,
            });
            runtime.install(window);
            const declaration = document.createElement(runtime.declarationTag);
            const tag = `story-bundle-invalid-${id}`;
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('src', 'https://example.test/bundle.xhtml#card');
            canvasElement.append(declaration);
            await runtime.whenDeclarationSettled(declaration);
            expect(runtime.diagnosticsFor(declaration).map(item => item.code)).toContain('cem-element.src_base_invalid');
            expect(customElements.get(tag)).toBeUndefined();
        }
    },
};

export const RedirectedBundleKeepsSeparateFragmentBases: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const id = ++sequence;
        const reads: string[] = [];
        const finalUrl = 'https://cdn.example.test/release/components.xhtml';
        const source = `<html xml:base="../src/"><body>
${['one', 'two'].map(name => `<div xml:base="${name}/">
<template id="${name}" xml:base="card.xhtml" type="text/cem-ml">
{style | @import "./theme.css";}
{cem-module-url @slice=assetUrl @src="./icon.svg"}
{p | Source context}
</template></div>`).join('')}
</body></html>`;
        let loads = 0;
        const runtime = new CemElementRuntime({
            declarationTag: `cem-bundle-redirect-${id}`,
            loadSrcDocument: async () => {
                loads++;
                return {
                    body: (async function* () { yield new TextEncoder().encode(source); })(),
                    resolvedUrl: finalUrl,
                    resolverIdentity: 'bundle-fixture',
                    contentType: 'application/xhtml+xml',
                };
            },
            retainedStylesheets: {
                read: async request => {
                    reads.push(request.url);
                    return {
                        bytes: new TextEncoder().encode(':scope { --bundle-source: retained; }').buffer,
                        finalUrl: request.url,
                        contentType: 'text/css',
                    };
                },
            },
        });
        runtime.install(window);
        for (const name of ['one', 'two']) {
            const tag = `story-bundle-${name}-${id}`;
            const declaration = document.createElement(runtime.declarationTag);
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('src', `https://example.test/redirect.xhtml#${name}`);
            canvasElement.append(declaration);
            await runtime.whenDeclarationSettled(declaration);
            expect(runtime.diagnosticsFor(declaration)).toEqual([]);
            const instance = document.createElement(tag);
            canvasElement.append(instance);
            await runtime.whenRenderSettled(instance);
            expect(runtime.snapshotInstance(instance).slices.assetUrl).toBe(`https://cdn.example.test/src/${name}/icon.svg`);
            expect(reads).toContain(`https://cdn.example.test/src/${name}/theme.css`);
            expect(getComputedStyle(instance).getPropertyValue('--bundle-source').trim()).toBe('retained');
        }
        expect(loads).toBe(1);
    },
};
