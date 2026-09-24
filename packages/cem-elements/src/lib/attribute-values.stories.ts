import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';

export default { title: 'CEM Elements/Attribute Values', tags: ['test'] } satisfies Meta;

const cemMl = `{attribute @name=value | fallback}
    {slice @name=flag | false}
    {p @class=alias | {$plain}}
    {p @class=selected | {$value}}
    {p @class=path | {$datadom.attributes.value}}
    {p @class=flag | {$flag}}`;
const dom = '<attribute name="value">fallback</attribute><slice name="flag">false</slice>' +
    '<p class="alias">${$plain}</p><p class="selected">${$value}</p>' +
    '<p class="path">${$datadom.attributes.value}</p><p class="flag">${$flag}</p>';

function register(tag: string, type: 'cem-ml' | 'dom' = 'cem-ml'): CemElementRuntime {
    const runtime = new CemElementRuntime({ declarationTag: `declaration-${tag}` });
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('version', '1.0.0');
    const template = document.createElement('template');
    if (type === 'cem-ml') {
        template.setAttribute('type', 'text/cem-ml');
        template.textContent = cemMl;
    } else template.innerHTML = dom;
    declaration.append(template);
    expect(runtime.registerDeclaration(declaration)).toBe(true);
    return runtime;
}

function emptyHost(tag: string): HTMLElement {
    const host = document.createElement(tag);
    host.setAttribute('value', '');
    host.setAttribute('plain', '');
    host.setAttribute('data-empty', '');
    return host;
}

async function rendered(runtime: CemElementRuntime, host: HTMLElement): Promise<void> {
    await waitFor(() => expect(host.querySelector('p.selected')).not.toBeNull(), { timeout: 15000 });
    await runtime.whenRenderSettled(host);
}

async function expectState(host: HTMLElement, value: string, flag: boolean): Promise<void> {
    await waitFor(() => {
        expect(host).toHaveAttribute('value', value);
        expect(host.querySelector('p.alias')?.textContent).toBe('');
        expect(host.querySelector('p.selected')?.textContent).toBe(value);
        expect(host.querySelector('p.path')?.textContent).toBe(value);
        expect(host.querySelector('p.flag')?.textContent).toBe(String(flag));
    });
}

function expectSnapshot(runtime: CemElementRuntime, host: HTMLElement, value: string, flag = false): void {
    const snapshot = runtime.snapshotInstance(host);
    expect(snapshot.hostAttributes).toMatchObject({ value, plain: '', 'data-empty': '' });
    expect(snapshot.dataset.empty).toBe('');
    expect(snapshot.slices.flag).toBe(flag);
    expect(runtime.diagnosticsFor(host)).toEqual([]);
}

async function exerciseHost(canvas: HTMLElement, tag: string, type: 'cem-ml' | 'dom'): Promise<void> {
    const runtime = register(tag, type);
    const host = emptyHost(tag);
    canvas.append(host);
    await rendered(runtime, host);
    runtime.setInstanceSlices(host, { flag: false });
    await expectState(host, '', false);
    expectSnapshot(runtime, host, '');
    for (const value of ['changed', '', 'false', '']) {
        host.setAttribute('value', value);
        await expectState(host, value, false);
        await runtime.whenRenderSettled(host);
        expectSnapshot(runtime, host, value);
    }
    host.removeAttribute('value');
    if (type === 'cem-ml') {
        await expectState(host, 'fallback', false);
    } else {
        // DOM templates apply declaration defaults without reflecting them to the host.
        await waitFor(() => expect(host.querySelector('p.selected')?.textContent).toBe('fallback'));
        expect(host).not.toHaveAttribute('value');
    }
    host.setAttribute('value', '');
    await expectState(host, '', false);
    runtime.setInstanceSlices(host, { flag: true });
    await expectState(host, '', true);
    runtime.setInstanceSlices(host, { flag: false });
    await expectState(host, '', false);
    expectSnapshot(runtime, host, '');
}

export const CemMlEmptyHostAttributes: StoryObj = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => exerciseHost(canvasElement, 'story-empty-cem-ml', 'cem-ml'),
};

export const DomEmptyHostAttributes: StoryObj = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => exerciseHost(canvasElement, 'story-empty-dom', 'dom'),
};

export const SerializedEmptyHostAttributes: StoryObj = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const tag = 'story-empty-hydration';
        const runtime = register(tag);
        const snapshot = runtime.snapshotInstance(emptyHost(tag));
        snapshot.hostAttributes = { value: '', plain: '', 'data-empty': '' };
        snapshot.dataset = { empty: '' };
        snapshot.slices = { flag: true };
        const island = document.createElement('template');
        island.setAttribute('data-cem-island', 'instance');
        writeDataIslandHydrationData(island, snapshot);

        // Parse actual serialized state, with stale live host values to reconcile.
        const host = emptyHost(tag);
        host.setAttribute('value', 'stale live value');
        host.innerHTML = island.outerHTML;
        canvasElement.append(host);
        await rendered(runtime, host);
        await expectState(host, '', true);
        // Capture render and island at the same revision before taking another snapshot.
        const serialized = host.outerHTML;
        expectSnapshot(runtime, host, '', true);
        host.remove();
        const parsed = document.createElement('template');
        parsed.innerHTML = serialized;
        const resumed = parsed.content.firstElementChild as HTMLElement;
        canvasElement.append(resumed);
        await rendered(runtime, resumed);
        await expectState(resumed, '', true);
        expectSnapshot(runtime, resumed, '', true);
        resumed.setAttribute('value', 'resumed change');
        await expectState(resumed, 'resumed change', true);
        resumed.setAttribute('value', '');
        await expectState(resumed, '', true);
        runtime.setInstanceSlices(resumed, { flag: false });
        await expectState(resumed, '', false);
        expectSnapshot(runtime, resumed, '');
    },
};
