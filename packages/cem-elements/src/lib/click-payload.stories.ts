import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Repeated Native Click Payload', tags: ['test'] };
export default meta;
type Story = StoryObj;
let sequence = 0;

export const SameSliceRetainsLatestNativeTarget: Story = {
    render: () => '<section>Native pointer and keyboard checks require the browser test runner.</section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const id = ++sequence;
        const runtime = new CemElementRuntime({ declarationTag: `cem-click-payload-${id}` });
        runtime.install(window);
        const declaration = document.createElement(runtime.declarationTag);
        const tag = `story-click-payload-${id}`;
        declaration.setAttribute('tag', tag);
        const template = document.createElement('template');
        template.type = 'text/cem-ml';
        template.textContent = `{button @type=button @aria-label="Command" @slice=pressed @slice-event=click @slice-value="$event.type" | {span @aria-hidden=true | Icon}}`;
        declaration.append(template);
        canvasElement.append(declaration);
        await runtime.whenDeclarationSettled(declaration);
        const instance = document.createElement(tag);
        canvasElement.append(instance);
        await runtime.whenRenderSettled(instance);
        const button = instance.querySelector('button') as HTMLButtonElement;
        const icon = instance.querySelector('span') as HTMLElement;
        const clicks: boolean[] = [];
        button.addEventListener('click', event => clicks.push(event.isTrusted));
        try {
            await userEvent.click(icon);
            await runtime.whenRenderSettled(instance);
            const pointer = runtime.snapshotInstance(instance);
            expect(pointer.slices.pressed).toBe('click');
            expect(pointer.eventPayloads.pressed).toMatchObject({
                type: 'click', sliceValue: 'click', target: { tag: 'span' }, currentTarget: { tag: 'button' },
            });
            expect(document.activeElement).toBe(button);

            await userEvent.keyboard('[Space>]');
            await runtime.whenRenderSettled(instance);
            expect(runtime.snapshotInstance(instance).eventPayloads).toEqual(pointer.eventPayloads);
            expect(clicks).toEqual([true]);

            await userEvent.keyboard('[/Space]');
            await runtime.whenRenderSettled(instance);
            const keyboard = runtime.snapshotInstance(instance);
            expect(keyboard.slices).toEqual(pointer.slices);
            expect(keyboard.eventPayloads.pressed).toMatchObject({
                type: 'click', sliceValue: 'click', target: { tag: 'button' }, currentTarget: { tag: 'button' },
            });
            expect(keyboard.eventPayloads.pressed).not.toEqual(pointer.eventPayloads.pressed);
            expect(instance.querySelector('button')).toBe(button);
            expect(clicks).toEqual([true, true]);
        } finally {
            await userEvent.keyboard('[/Space]');
            instance.remove();
            declaration.remove();
        }
    },
};
