import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Bundle Source Context', tags: ['test'] };
export default meta;
type Story = StoryObj;
let sequence = 0;

// Characterize the existing loading contract before choosing a bundle base policy.
// The fixture copies template text unchanged; it does not implement a bundler.
export const RelativeDependencyUsesLoadedDocument: Story = {
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
            ['combined', bundle, 'https://components.example.test/dist/icon.svg'],
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
        }
    },
};
