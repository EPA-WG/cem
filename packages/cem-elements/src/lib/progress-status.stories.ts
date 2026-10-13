import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemStoryRuntime } from '../../.storybook/preview.js';

const meta: Meta = { title: 'CEM Elements/Progress Status', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const NormalizationAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `progress-status-${crypto.randomUUID()}`;
            const declaration = await declare(`<cem-element tag="${tag}" capability="progress-status"><template type="text/cem-ml">
                {span @part=root @data-mode="{$datadom.slices.mode}" @data-max="{$datadom.slices.max}" @data-value={datadom.slices.value} @data-dash="{$datadom.slices.dashArray}" | Progress}
            </template></cem-element>`, tag);
            const host = document.createElement(tag);
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const owner = host.querySelector<HTMLElement>('[part=root]');
                if (!owner) throw new Error('Missing progress owner');
                expect(owner.dataset.mode).toBe('indeterminate');
                expect(owner.dataset.value).toBeUndefined();
                expect(owner.dataset.dash).toBe('25 75');
                const events: string[] = [];
                for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
                for (const [value, max, expectedValue, expectedMax, dash] of [
                    ['3', '12', '3', '12', '25 75'],
                    ['120', '80', '80', '80', '100 0'],
                    ['-20', '100', '0', '100', '0 100'],
                    ['bad', '0', '0', '100', '0 100'],
                    ['', '', '0', '100', '0 100'],
                    ['Infinity', '-1', '0', '100', '0 100'],
                    ['1e308', 'Infinity', '100', '100', '100 0'],
                    ['1e308', '1e308', '1e+308', '1e+308', '100 0'],
                    ['0.125', '0.5', '0.125', '0.5', '25 75'],
                ]) {
                    host.setAttribute('value', value); host.setAttribute('max', max);
                    await runtime.whenRenderSettled(host);
                    expect(host.querySelector('[part=root]')).toBe(owner);
                    expect(owner.dataset.mode).toBe('determinate');
                    expect(Number(owner.dataset.value)).toBe(Number(expectedValue));
                    expect(Number(owner.dataset.max)).toBe(Number(expectedMax));
                    expect(owner.dataset.dash).toBe(dash);
                    expect(host.getAttribute('value')).toBe(value);
                    expect(host.getAttribute('max')).toBe(max);
                }
                const snapshot = runtime.snapshotInstance(host).slices;
                owner.click(); owner.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
                expect(runtime.snapshotInstance(host).slices).toEqual(snapshot);
                expect(events).toEqual([]);
                host.removeAttribute('value'); await runtime.whenRenderSettled(host);
                expect(owner.dataset.mode).toBe('indeterminate'); expect(owner.dataset.value).toBeUndefined();
                host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
                expect(host.querySelector('[part=root]')).toBe(owner);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
        }
    },
};
