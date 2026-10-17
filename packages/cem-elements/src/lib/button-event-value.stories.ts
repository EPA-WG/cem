import { expect, userEvent } from 'storybook/test';
import preview, { createCemStoryRuntime } from '../../.storybook/preview.js';

const meta = preview.meta({ title: 'CEM Elements/Button Event Values' });

export const NestedActivationValues = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `button-values-${crypto.randomUUID()}`;
            const bindings = ['', ' @slice-value="$target.value"', ' @slice-value="@value"', ' @slice-value="//@value"'];
            const source = `<cem-element tag="${tag}"><template type="text/cem-ml">`
                + bindings.map((binding, index) => `{div @slice=choice${index} @slice-event=click${binding} | {button @name=choice${index} @value=theme${index} | {span | Choice ${index}}}}`).join('')
                + '{span @part=ordinary @slice=ordinary @slice-event=click @slice-value="$target.value" | Ordinary content}'
                + '</template></cem-element>';
            const declaration = await declare(source, tag), host = document.createElement(tag); canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                for (let index = 0; index < bindings.length; index++) {
                    const label = host.querySelector<HTMLElement>(`button[name="choice${index}"] span`);
                    if (!label) throw new Error('Missing nested button label');
                    await userEvent.click(label); await runtime.whenRenderSettled(host);
                    const snapshot = runtime.snapshotInstance(host);
                    expect(snapshot.slices[`choice${index}`]).toBe(`theme${index}`);
                    expect(snapshot.eventPayloads[`choice${index}`]).toMatchObject({
                        target: { tag: 'span', value: `theme${index}` }, currentTarget: { tag: 'div' }, sliceValue: `theme${index}`,
                    });
                }
                const ordinary = host.querySelector<HTMLElement>('[part=ordinary]');
                if (!ordinary) throw new Error('Missing ordinary event target');
                await userEvent.click(ordinary); await runtime.whenRenderSettled(host);
                expect(runtime.snapshotInstance(host).slices.ordinary).toBeNull();
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
        }
    },
});
