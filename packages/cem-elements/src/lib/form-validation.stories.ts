import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Form Validation Refresh', tags: ['test'] };
export default meta;
type Story = StoryObj;
type Mode = 'dom' | 'worker' | 'wasm';

async function mount(root: HTMLElement, name: string, source: string, mode: Mode = 'worker', sharedRuntime?: CemElementRuntime) {
    const declarationTag = sharedRuntime?.declarationTag ?? `cem-form-refresh-${name}`;
    const tag = `story-form-refresh-${name}`;
    const runtime = sharedRuntime ?? new CemElementRuntime({ declarationTag });
    runtime.install(window);
    const declaration = document.createElement(declarationTag);
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    if (mode === 'dom') template.innerHTML = source;
    else {
        template.type = 'text/cem-ml';
        template.textContent = (mode === 'wasm'
            ? '{location-element @slice=route @href="https://example.test/forms"}' : '') + source;
    }
    declaration.append(template);
    root.append(declaration);
    runtime.registerDeclaration(declaration);
    await runtime.whenDeclarationSettled(declaration);
    const instance = document.createElement(tag);
    root.append(instance);
    await runtime.whenRenderSettled(instance);
    return { runtime, instance };
}

function output(instance: HTMLElement, name: string): string {
    return instance.querySelector(`output[name="${name}"]`)?.textContent?.trim() ?? '';
}

function persistedField(instance: HTMLElement, section: string, ...names: string[]): Element | undefined {
    // Read committed state without snapshotInstance(), which itself captures forms.
    const island = instance.querySelector<HTMLTemplateElement>('template[data-cem-island="instance"]');
    let field = Array.from(island?.content.querySelectorAll('*') ?? []).find(node => node.localName === section);
    for (const name of names) field = Array.from(field?.children ?? []).find(node => node.getAttribute('name') === name);
    return field;
}

const initialDom = '<form slice="signup"><input name="email" required value="" />'
    + '<output name="valid">${$datadom.validationState.signup.valid}</output>'
    + '<output name="message">${$datadom.validationState.signup.controls.email.validationMessage}</output></form>';

export const InitialDomForm: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance } = await mount(canvasElement, 'dom', initialDom, 'dom');
        const input = instance.querySelector<HTMLInputElement>('input');
        if (!input) throw new Error('Required input is missing');
        expect(output(instance, 'valid')).toBe('false');
        expect(output(instance, 'message')).toBe(input.validationMessage);
        const revision = instance.querySelector('form')?.getAttribute('data-cem-data-revision');
        await runtime.whenRenderSettled(instance);
        expect(instance.querySelector('input')).toBe(input);
        expect(instance.querySelector('form')?.getAttribute('data-cem-data-revision')).toBe(revision);
        expect(runtime.diagnosticsFor(instance)).toEqual([]);
    },
};

function conditionalForm(mode: 'worker' | 'wasm'): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const { runtime, instance } = await mount(canvasElement, mode, `
{slice @name=show | false}
{form @slice=signup |
    {cem:if @test=show | {input @name=password @required=true}}
    {output @name=valid | {$datadom.validationState.signup.valid}}
    {output @name=message | {$datadom.validationState.signup.validationMessage}}
    {output @name=mirror | {$datadom.slices.signup.valid}}
}`, mode);
            const form = instance.querySelector('form');
            expect(output(instance, 'valid')).toBe('true');
            for (const shown of [true, false, true, false]) {
                runtime.setInstanceSlices(instance, { show: shown });
                await runtime.whenRenderSettled(instance);
                expect(instance.querySelector('form')).toBe(form);
                expect(output(instance, 'valid')).toBe(String(!shown));
                expect(output(instance, 'mirror')).toBe(String(!shown));
                const input = instance.querySelector<HTMLInputElement>('input');
                expect(Boolean(input)).toBe(shown);
                expect(output(instance, 'message')).toBe(input?.validationMessage ?? '');
                const data = persistedField(instance, 'cem-form:form-state', 'signup');
                expect(Array.from(data?.children ?? [], node => node.getAttribute('name'))).toEqual(shown ? ['password'] : []);
                expect(persistedField(instance, 'cem-validation:validation-state', 'signup', 'valid')?.textContent).toBe(String(!shown));
            }
            runtime.setInstanceSlices(instance, { show: true });
            runtime.setInstanceSlices(instance, { show: false });
            await runtime.whenRenderSettled(instance);
            expect(instance.querySelector('input')).toBeNull();
            expect(output(instance, 'valid')).toBe('true');
            expect(runtime.diagnosticsFor(instance)).toEqual([]);
        },
    };
}

export const ConditionalWorkerForm = conditionalForm('worker');
export const ConditionalWasmForm = conditionalForm('wasm');

export const NestedFormOwnership: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance: seed } = await mount(canvasElement, 'nested-child',
            '<form slice="child"><input name="email" required />'
            + '<output name="valid">${$datadom.validationState.child.valid}</output>'
            + '<output name="email">${$datadom.formData.child.email}</output></form>', 'dom');
        seed.remove();
        const { instance: parent } = await mount(canvasElement, 'nested-parent',
            '<output name="tick">${$tick}</output><form slice="parent"><input name="email" value="parent" required />'
            + '<output name="valid">${$datadom.validationState.parent.valid}</output></form>'
            + '<story-form-refresh-nested-child></story-form-refresh-nested-child>', 'dom', runtime);
        const child = parent.querySelector<HTMLElement>('story-form-refresh-nested-child');
        if (!child) throw new Error('Nested instance is missing');
        await runtime.whenRenderSettled(child);
        const input = child.querySelector('input');
        if (!input) throw new Error('Nested input is missing');
        runtime.setInstanceSlices(parent, { tick: 'changed' });
        await runtime.whenRenderSettled(parent);
        expect(output(parent, 'valid')).toBe('true');
        expect(output(child, 'valid')).toBe('false');
        expect(Array.from(persistedField(parent, 'cem-form:form-state')?.children ?? [], field => field.getAttribute('name')))
            .toEqual(['parent']);
        expect(child.querySelector('input')).toBe(input);
        input.value = 'child';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await runtime.whenRenderSettled(child);
        expect(output(child, 'email')).toBe('child');
        expect(output(child, 'valid')).toBe('true');
        expect(runtime.diagnosticsFor(parent)).toEqual([]);
        expect(runtime.diagnosticsFor(child)).toEqual([]);
    },
};

export const CircularFormRules: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, instance } = await mount(canvasElement, 'cycle', `
{slice @name=stop | false}
{form @slice=cycle |
    {cem:if @test='!stop && !(datadom.validationState.cycle.controls.required.required ?? false)' |
        {input @name=required @required=true}
    }
    {output @name=valid | {$datadom.validationState.cycle.valid}}
}`);
        expect(runtime.diagnosticsFor(instance).map(({ code }) => code)).toEqual(['cem-element.form_state_unstable']);
        runtime.setInstanceSlices(instance, { stop: true });
        await runtime.whenRenderSettled(instance);
        expect(instance.querySelector('input')).toBeNull();
        expect(output(instance, 'valid')).toBe('true');
        expect(runtime.diagnosticsFor(instance)).toHaveLength(1);
    },
};
