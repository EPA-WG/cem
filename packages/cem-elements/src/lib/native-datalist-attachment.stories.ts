import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemStoryRuntime, cemComponentDeclarationSource } from '../../.storybook/preview.js';

export default { title: 'CEM Elements/Native Datalist Attachment', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
export const WorkerAndFallbackKeepNativeTypesAndRecoverSources: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            runtime.setLocalSuggestionsEnabled(true);
            const suffix = crypto.randomUUID(), fieldTag = `native-field-${suffix}`, attachmentTag = `native-attachment-${suffix}`;
            await declare(await cemComponentDeclarationSource('cem-field'), fieldTag);
            await declare(await cemComponentDeclarationSource('cem-suggestions'), attachmentTag);
            const form = document.createElement('form'); canvasElement.append(form);
            try {
                for (const type of ['text', 'search', 'tel', 'url', 'email', 'number']) {
                    const host = document.createElement(attachmentTag); host.setAttribute('profile', 'native-datalist');
                    host.innerHTML = `<template><${fieldTag} slot="editor" type="${type}" name="choice" label="Choice"></${fieldTag}><template slot="options"><option value="1" label="One"></option><option value="2" label="Two"></option></template></template>`;
                    form.append(host); await runtime.whenRenderSettled(host);
                    const field = host.querySelector<HTMLElement>('[slot=editor]') as HTMLElement; await runtime.whenRenderSettled(field);
                    const input = field.querySelector('input') as HTMLInputElement;
                    await waitFor(() => expect(input.list?.options.length, JSON.stringify(runtime.diagnosticsFor(host))).toBe(2));
                    expect(input.type).toBe(type); expect(input.hasAttribute('role')).toBe(false); expect(input.hasAttribute('aria-controls')).toBe(false);
                    const original = input;
                    if (type === 'email') {
                        input.multiple = true; await waitFor(() => expect(input.hasAttribute('list')).toBe(false));
                        input.multiple = false; await waitFor(() => expect(input.list?.options.length).toBe(2));
                    }
                    if (type === 'text') {
                        const source = runtime.localSuggestionsEnvironmentFor(host)?.optionsSources[0] as HTMLTemplateElement;
                        source.innerHTML = '<optgroup label="Unsupported"><option value="x" label="X"></option></optgroup>';
                        await waitFor(() => expect(input.hasAttribute('list')).toBe(false));
                        source.innerHTML = '<option value="3" label="Three"></option>';
                        await waitFor(() => expect(input.list?.options[0].value).toBe('3'));
                        host.setAttribute('options-state', 'failed'); await runtime.whenRenderSettled(host);
                        await waitFor(() => expect(input.hasAttribute('list')).toBe(false));
                        await waitFor(() => expect(host.querySelector('datalist')?.options.length).toBe(0));
                        host.removeAttribute('options-state'); await runtime.whenRenderSettled(host);
                        await waitFor(() => expect(input.list?.options.length).toBe(1));
                        runtime.setLocalSuggestionsEnabled(false); expect(input.hasAttribute('list')).toBe(false);
                        runtime.setLocalSuggestionsEnabled(true); await waitFor(() => expect(input.list?.options.length).toBe(1));
                        input.setAttribute('list', 'author'); await waitFor(() => expect(input.getAttribute('list')).toBe('author'));
                        host.remove(); expect(input.getAttribute('list')).toBe('author');
                        input.removeAttribute('list'); form.append(host); await runtime.whenRenderSettled(host);
                        await waitFor(() => expect(input.list?.options.length).toBe(1));
                    }
                    expect(field.querySelector('input')).toBe(original); host.remove(); expect(input.hasAttribute('list')).toBe(false);
                }
            } finally { form.remove(); scope.dispose(); }
        }
    },
};
