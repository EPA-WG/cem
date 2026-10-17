import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime, type CemDeclarationRegistrationOptions } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Declaration Source Capabilities', tags: ['test'] };
export default meta;
type Story = StoryObj;
let sequence = 0;
const CONTROL = '{input @part=control @form="" @aria-label=Account @value={datadom.slices.value} @required={if seq:count(datadom.attributes.required) > 0 { true } else { null }} @disabled={if datadom.slices.formDisabled { true } else { null }} @slice=value @slice-event=input @slice-value="$target.value"}';

interface FormHost extends HTMLElement {
    value: string;
    form: HTMLFormElement | null;
    checkValidity(): boolean;
}

function runtime(source: string | ((declarationTag: string) => string), loads?: string[]) {
    const declarationTag = `source-capability-${++sequence}`;
    const value = new CemElementRuntime({
        declarationTag,
        moduleUrlRoot: { importMap: { imports: { 'controls/': 'https://components.example.test/' } } },
        loadSrcDocument: async path => { loads?.push(path); return typeof source === 'string' ? source : source(declarationTag); },
    });
    value.install(window);
    return value;
}

function mount(root: HTMLElement, value: CemElementRuntime, src: string, capability?: string, options?: CemDeclarationRegistrationOptions) {
    const declaration = document.createElement(value.declarationTag);
    const tag = `story-source-capability-${++sequence}`;
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', src);
    if (capability !== undefined) declaration.setAttribute('capability', capability);
    value.registerDeclaration(declaration, options);
    root.append(declaration);
    return { declaration, tag };
}

async function ready(value: CemElementRuntime, owner: ReturnType<typeof mount>, formAssociated: boolean) {
    await value.whenDeclarationSettled(owner.declaration);
    expect(value.diagnosticsFor(owner.declaration)).toEqual([]);
    const constructor = customElements.get(owner.tag) as (CustomElementConstructor & { formAssociated: boolean });
    expect(constructor).toBeDefined();
    expect(constructor.formAssociated).toBe(formAssociated);
}

export const ComponentSourcesOwnFormCapabilities: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const tag of ['cem-field', 'cem-text-field']) {
            const response = await fetch(`/cem-manager-assets/components/${tag}/${tag}.xhtml`);
            expect(response.ok).toBe(true);
            const value = runtime(await response.text());
            const owner = mount(canvasElement, value, `controls/${tag}/${tag}.xhtml#${tag}`);
            await ready(value, owner, true);
            expect(owner.declaration.hasAttribute('capability')).toBe(false);
            const form = document.createElement('form');
            const fieldset = document.createElement('fieldset');
            const host = document.createElement(owner.tag) as FormHost;
            host.setAttribute('name', 'account');
            host.setAttribute('value', 'initial');
            host.setAttribute('required', '');
            fieldset.append(host); form.append(fieldset); canvasElement.append(form);
            const settle = () => value.whenRenderSettled(host);
            await settle();
            expect(host.form).toBe(form);
            expect(new FormData(form).getAll('account')).toEqual(['initial']);
            host.value = '';
            await settle();
            expect(host.checkValidity()).toBe(false);
            const input = host.querySelector('input') as HTMLInputElement;
            input.value = 'edited';
            input.dispatchEvent(new Event('input', { bubbles: true }));
            await settle();
            expect(host.value).toBe('edited');
            expect(host.checkValidity()).toBe(true);
            expect(new FormData(form).getAll('account')).toEqual(['edited']);
            fieldset.disabled = true;
            await settle();
            expect(input.disabled).toBe(true);
            expect(new FormData(form).getAll('account')).toEqual([]);
            fieldset.disabled = false;
            form.reset();
            await settle();
            expect(host.value).toBe('initial');
            expect(new FormData(form).getAll('account')).toEqual(['initial']);
        }
    },
};

export const BundleFragmentsKeepTheirOwnCapabilities: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const loads: string[] = [];
        const value = runtime(`<main>
            <cem-element id="owner" tag="source-field" capability="form-control"><template id="field" type="text/cem-ml">${CONTROL}</template></cem-element>
            <cem-element tag="source-plain"><template id="plain" type="text/cem-ml">{p | Plain}</template></cem-element>
        </main>`, loads);
        const field = mount(canvasElement, value, 'controls/bundle.xhtml#field');
        const plain = mount(canvasElement, value, 'controls/bundle.xhtml#plain');
        const owner = mount(canvasElement, value, 'controls/bundle.xhtml#owner');
        await ready(value, field, true);
        await ready(value, plain, false);
        await ready(value, owner, true);
        expect(loads).toEqual(['https://components.example.test/bundle.xhtml']);
    },
};

export const TemplateCapabilitiesWorkForInlineLocalAndRelativeSources: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const value = runtime(`<template id="control" type="text/cem-ml" capability="form-control">${CONTROL}</template>`);
        await ready(value, mount(canvasElement, value, './control.xhtml#control'), true);
        const template = document.createElement('template');
        template.id = `local-capability-${++sequence}`;
        template.type = 'text/cem-ml';
        template.setAttribute('capability', 'form-control');
        template.textContent = CONTROL;
        canvasElement.append(template);
        await ready(value, mount(canvasElement, value, `#${template.id}`), true);
        const declaration = document.createElement(value.declarationTag);
        const tag = `story-source-capability-${++sequence}`;
        declaration.setAttribute('tag', tag);
        declaration.append(template.cloneNode(true));
        canvasElement.append(declaration);
        await ready(value, { declaration, tag }, true);
    },
};

export const ExplicitCapabilitiesOverrideSourceDefaults: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const value = runtime(`<cem-element tag="source-field" capability="unknown-capability"><template id="control" type="text/cem-ml" capability="form-control">${CONTROL}</template></cem-element>`);
        await ready(value, mount(canvasElement, value, './control.xhtml#control'), true);
        await ready(value, mount(canvasElement, value, './control.xhtml#control', ''), false);
        const invalid = runtime(`<template id="control" type="text/cem-ml" capability="unknown-capability">${CONTROL}</template>`);
        await ready(invalid, mount(canvasElement, invalid, './control.xhtml#control', 'form-control'), true);
        const unrelated = runtime(`<div capability="form-control"><template id="control" type="text/cem-ml">${CONTROL}</template></div>`);
        await ready(unrelated, mount(canvasElement, unrelated, './control.xhtml#control'), false);
    },
};

export const LocalOwnersAndConfiguredDeclarationNames: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const value = runtime(declarationTag => `<${declarationTag} tag="source-field" capability="form-control"><template id="control" type="text/cem-ml">${CONTROL}</template></${declarationTag}>`);
        await ready(value, mount(canvasElement, value, './control.xhtml#control'), true);
        const declaration = document.createElement(value.declarationTag);
        declaration.setAttribute('tag', `story-local-owner-${++sequence}`);
        declaration.setAttribute('capability', 'form-control');
        const template = document.createElement('template');
        template.id = `local-owner-capability-${++sequence}`;
        template.type = 'text/cem-ml';
        template.textContent = CONTROL;
        declaration.append(template);
        canvasElement.append(declaration);
        await value.whenDeclarationSettled(declaration);
        await ready(value, mount(canvasElement, value, `#${template.id}`), true);
        expect(template.hasAttribute('capability')).toBe(false);
    },
};

export const SourceCapabilitiesUseExistingDiagnostics: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const capability of ['unknown-capability', 'form-control']) {
            const value = runtime(`<cem-element tag="source-field" capability="${capability}"><template id="control" type="text/cem-ml">${CONTROL}</template></cem-element>`);
            const owner = mount(canvasElement, value, './control.xhtml#control', undefined,
                capability === 'form-control' ? { behavior: {}, behaviorIdentity: 'test-host-behavior-v1' } : undefined);
            await value.whenDeclarationSettled(owner.declaration);
            expect(value.diagnosticsFor(owner.declaration).map(({ code }) => code)).toEqual([
                capability === 'form-control' ? 'cem-element.capability_behavior_conflict' : 'cem-element.capability_unknown',
            ]);
            expect(customElements.get(owner.tag)).toBeUndefined();
        }
    },
};
