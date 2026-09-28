import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Form Control Event Values', tags: ['test'] };
export default meta;
type Story = StoryObj;

function fixture(mode: 'dom' | 'cem-ml'): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const tag = `story-form-value-${mode}`;
            const controlTag = `story-value-control-${mode}`;
            if (!customElements.get(controlTag)) {
                customElements.define(controlTag, class extends HTMLElement {
                    static formAssociated = true;
                    value: unknown = 'live';
                });
            }
            const runtime = new CemElementRuntime({ declarationTag: `story-form-declaration-${mode}` });
            runtime.install(window);
            const declaration = document.createElement('div');
            declaration.setAttribute('tag', tag);
            const template = document.createElement('template');
            const expressions = [null, '$target.value', '@value', '//@value'];
            if (mode === 'dom') {
                template.innerHTML = expressions.map((value, index) => `<${controlTag} value="stale" slice="value${index}" slice-event="change"${value === null ? '' : ` slice-value="${value}"`}></${controlTag}>`).join('')
                    + '<input value="native" slice="native" slice-event="input" slice-value="$target.value" />'
                    + '<div value="attribute" slice="nonform" slice-event="change" slice-value="$target.value"></div>';
            } else {
                template.type = 'text/cem-ml';
                template.textContent = expressions.map((value, index) => `{${controlTag} @value=stale @slice=value${index} @slice-event=change${value === null ? '' : ` @slice-value="${value}"`}}`).join('')
                    + '{input @value=native @slice=native @slice-event=input @slice-value="$target.value"}'
                    + '{div @value=attribute @slice=nonform @slice-event=change @slice-value="$target.value"}';
            }
            declaration.append(template);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const host = document.createElement(tag);
            canvasElement.append(host);
            await runtime.whenRenderSettled(host);
            const controls = [...host.querySelectorAll<HTMLElement & { value: unknown }>(controlTag)];
            await expect(controls).toHaveLength(4);
            for (const [index, control] of controls.entries()) {
                control.value = `live-${index}`;
                control.dispatchEvent(new Event('change', { bubbles: true }));
                await runtime.whenRenderSettled(host);
                const snapshot = runtime.snapshotInstance(host);
                await expect(snapshot.slices[`value${index}`]).toBe(`live-${index}`);
                await expect(snapshot.eventPayloads[`value${index}`]?.target?.value).toBe(`live-${index}`);
            }
            const input = host.querySelector('input') as HTMLInputElement;
            input.value = 'edited';
            input.dispatchEvent(new Event('input', { bubbles: true }));
            await runtime.whenRenderSettled(host);
            await expect(runtime.snapshotInstance(host).slices.native).toBe('edited');
            const ordinary = host.querySelector('div') as HTMLElement & { value: string };
            ordinary.value = 'not a form control';
            ordinary.dispatchEvent(new Event('change', { bubbles: true }));
            await runtime.whenRenderSettled(host);
            await expect(runtime.snapshotInstance(host).eventPayloads.nonform?.target?.value).toBeNull();
            controls[1].value = { secret: 'not a string' };
            controls[1].dispatchEvent(new Event('change', { bubbles: true }));
            await runtime.whenRenderSettled(host);
            await expect(runtime.snapshotInstance(host).eventPayloads.value1?.target?.value).toBeNull();
            host.remove();
        },
    };
}
export const Dom = fixture('dom');
export const CemMl = fixture('cem-ml');
