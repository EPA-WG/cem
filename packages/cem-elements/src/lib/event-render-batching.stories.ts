import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

export default { title: 'CEM Elements/Event Render Batching', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
let sequence = 0;

async function mount(root: HTMLElement) {
    const id = ++sequence;
    let renders = 0;
    const runtime = new CemElementRuntime({ declarationTag: `cem-event-batch-${id}` });
    runtime.install(window);
    const declaration = document.createElement('div');
    const tag = `story-event-batch-${id}`;
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.type = 'text/cem-ml';
    template.textContent = `
{div @slice=parent @slice-event="click follow-up" @slice-value="$event.type" |
  {button @type=button @slice=child @slice-event=click @slice-value="$event.type" | Run}
  {output | {$datadom.slices.parent}}
}`;
    declaration.append(template);
    root.append(declaration);
    runtime.registerDeclaration(declaration, { behavior: { beforeRender() { renders++; } }, behaviorIdentity: 'event-batch-observer' });
    await runtime.whenDeclarationSettled(declaration);
    const instance = document.createElement(tag);
    root.append(instance);
    await runtime.whenRenderSettled(instance);
    return { runtime, instance, button: instance.querySelector('button') as HTMLButtonElement, renders: () => renders };
}

export const BubblingAndFollowupsRenderOnce: Story = {
    render: () => '<section>Trusted event checks run in the browser test runner.</section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const { runtime, instance, button, renders } = await mount(canvasElement);
        const baseline = renders();
        const during: number[] = [];
        const duringOutput: (string | undefined)[] = [];
        button.addEventListener('click', () => {
            during.push(renders());
            runtime.setInstanceSlices(instance, { additional: 'latest' });
            instance.setAttribute('data-edit', 'updated');
            button.dispatchEvent(new Event('follow-up', { bubbles: true }));
        });
        instance.addEventListener('click', () => {
            during.push(renders());
            duringOutput.push(instance.querySelector('output')?.textContent?.trim());
            queueMicrotask(() => button.dispatchEvent(new Event('follow-up', { bubbles: true })));
        });
        await userEvent.click(button);
        await runtime.whenRenderSettled(instance);
        expect(during).toEqual([baseline, baseline]);
        expect(duringOutput).toEqual(['']);
        expect(renders() - baseline).toBe(1);
        const snapshot = runtime.snapshotInstance(instance);
        expect(snapshot.slices).toMatchObject({ child: 'click', parent: 'follow-up', additional: 'latest' });
        expect(snapshot.eventPayloads.parent).toMatchObject({ type: 'follow-up', target: { tag: 'button' }, currentTarget: { tag: 'div' } });
        expect(instance.querySelector('output')?.textContent?.trim()).toBe('follow-up');
        expect(instance.querySelector('button')).toBe(button);
    },
};

export const SettlementAndDisconnect: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const { runtime, instance, button, renders } = await mount(canvasElement);
        const baseline = renders();
        button.click();
        button.click();
        expect(renders()).toBe(baseline);
        await runtime.whenRenderSettled(instance);
        expect(renders()).toBe(baseline + 1);
        expect(runtime.snapshotInstance(instance).eventPayloads.child).toMatchObject({ revision: 2 });
        button.click();
        instance.remove();
        await runtime.whenRenderSettled(instance);
        expect(renders()).toBe(baseline + 1);
        canvasElement.append(instance);
        await runtime.whenRenderSettled(instance);
        expect(renders()).toBe(baseline + 2);
        expect(runtime.snapshotInstance(instance).eventPayloads.child).toMatchObject({ revision: 3 });
    },
};
