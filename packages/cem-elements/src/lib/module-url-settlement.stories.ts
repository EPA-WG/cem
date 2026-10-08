import { startReadinessTiming, readinessCheckpoint } from '../../.storybook/readiness-timing.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Module URL Settlement', tags: ['test'] };
export default meta;
type Story = StoryObj;
let sequence = 0;

async function mount(root: HTMLElement) {
    const pending = new Map<string, (value: string) => void>();
    const failures = new Map<string, (error: Error) => void>();
    let resolversReady!: () => void;
    const discovered = new Promise<void>(resolve => { resolversReady = resolve; });
    const runtime = new CemElementRuntime({
        declarationTag: `cem-module-batch-${++sequence}`,
        resolveModuleUrl: specifier => new Promise<string>((resolve, reject) => {
            pending.set(specifier, resolve);
            failures.set(specifier, reject);
            if (pending.size === 4) resolversReady();
        }),
    });
    runtime.install(window);
    const declaration = document.createElement(runtime.declarationTag);
    const tag = `story-module-batch-${sequence}`;
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.type = 'text/cem-ml';
    template.textContent = '{slice @name=active | true}{cem:if @test="datadom.slices.active" |' + ['alphaUrl', 'betaUrl', 'gammaUrl', 'deltaUrl'].map(name => `
{cem-module-url @slice=${name} @src="./${name}.css"}
{cem:if @test="datadom.slices.${name}" |
{a @data-resource=${name} @href="{$datadom.slices.${name}}" | ${name}}}
`).join('') + '}';
    declaration.append(template);
    root.append(declaration);
    runtime.registerDeclaration(declaration);
    await runtime.whenDeclarationSettled(declaration);
    const instance = document.createElement(tag);
    root.append(instance);
    startReadinessTiming(root, 'module-url/SiblingDiscovery', runtime);
    // Render settlement depends on the held results; wait only for admission here.
    await discovered;
    expect(pending.size, JSON.stringify(runtime.diagnosticsFor(instance))).toBe(4);
    readinessCheckpoint('resolvers-discovered', { resolvers: pending.size });
    return { runtime, instance, pending, failures };
}

export const SiblingsCommitTogether: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance, pending } = await mount(canvasElement);
        try {
            pending.get('./alphaUrl.css')?.('https://example.test/alphaUrl.css');
            // Let the alphaUrl resolver finish while its siblings remain controlled.
            await new Promise(resolve => setTimeout(resolve, 50));
            expect(runtime.snapshotInstance(instance).slices.alphaUrl).toBeUndefined();
            for (const [specifier, resolve] of pending) resolve(new URL(specifier, 'https://example.test/').href);
            await runtime.whenRenderSettled(instance);
            for (const name of ['alphaUrl', 'betaUrl', 'gammaUrl', 'deltaUrl']) {
                const value = `https://example.test/${name}.css`;
                expect(runtime.snapshotInstance(instance).slices[name]).toBe(value);
                expect(runtime.snapshotInstance(instance).eventPayloads[name]).toMatchObject({ type: 'module-url', src: `./${name}.css`, value });
                expect(instance.querySelector(`[data-resource="${name}"]`)?.getAttribute('href')).toBe(value);
            }
        } finally {
            instance.remove();
            for (const resolve of pending.values()) resolve('https://example.test/cleanup.css');
        }
    },
};

export const DisconnectedBatchIsDiscarded: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance, pending } = await mount(canvasElement);
        instance.remove();
        for (const resolve of pending.values()) resolve('https://example.test/disconnected.css');
        await runtime.whenRenderSettled(instance);
        for (const name of ['alphaUrl', 'betaUrl', 'gammaUrl', 'deltaUrl']) {
            expect(runtime.snapshotInstance(instance).slices[name]).toBeUndefined();
        }
    },
};

export const SupersededBatchIsDiscarded: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance, pending } = await mount(canvasElement);
        runtime.setInstanceSlices(instance, { active: false });
        for (const resolve of pending.values()) resolve('https://example.test/stale.css');
        await runtime.whenRenderSettled(instance);
        for (const name of ['alphaUrl', 'betaUrl', 'gammaUrl', 'deltaUrl']) {
            expect(runtime.snapshotInstance(instance).slices[name]).toBeUndefined();
        }
        expect(instance.querySelector('[data-resource]')).toBeNull();
    },
};

export const FailedSiblingKeepsSuccessfulResults: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance, pending, failures } = await mount(canvasElement);
        failures.get('./betaUrl.css')?.(new Error('betaUrl unavailable'));
        failures.get('./gammaUrl.css')?.(new Error('gammaUrl unavailable'));
        pending.get('./alphaUrl.css')?.('https://example.test/alphaUrl.css');
        pending.get('./deltaUrl.css')?.('https://example.test/deltaUrl.css');
        await runtime.whenRenderSettled(instance);
        expect(runtime.snapshotInstance(instance).slices.alphaUrl).toBe('https://example.test/alphaUrl.css');
        expect(runtime.snapshotInstance(instance).slices.deltaUrl).toBe('https://example.test/deltaUrl.css');
        expect(runtime.snapshotInstance(instance).slices.betaUrl).toBeUndefined();
        expect(runtime.snapshotInstance(instance).eventPayloads.betaUrl).toBeUndefined();
        const messages = runtime.diagnosticsFor(instance)
            .filter(item => item.code === 'cem-element.module_url_resolve_failed')
            .map(item => item.message).join(' ');
        expect(messages).toContain('betaUrl unavailable');
        expect(messages).toContain('gammaUrl unavailable');
    },
};
