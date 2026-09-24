import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Choice Validation Focus', tags: ['test'] };
export default meta;
type Story = StoryObj;
type Choice = HTMLElement & { value: string; reportValidity(): boolean; checkValidity(): boolean };

async function mount(root: HTMLElement, name: string, source: string) {
    const runtime = new CemElementRuntime({ declarationTag: `cem-choice-validation-${name}` });
    runtime.install(window);
    const declaration = document.createElement(runtime.declarationTag);
    declaration.setAttribute('tag', `story-choice-validation-${name}`);
    declaration.setAttribute('capability', 'choice-select');
    const template = document.createElement('template');
    template.type = 'text/cem-ml';
    template.textContent = source;
    declaration.append(template);
    root.append(declaration);
    runtime.registerDeclaration(declaration);
    await runtime.whenDeclarationSettled(declaration);
    const form = document.createElement('form');
    const choice = document.createElement(`story-choice-validation-${name}`) as Choice;
    choice.setAttribute('name', 'fruit');
    choice.setAttribute('required', '');
    choice.innerHTML = '<cem-option value="">Choose fruit</cem-option><cem-option value="apple">Apple</cem-option>';
    const submit = document.createElement('button');
    submit.type = 'submit';
    submit.textContent = 'Submit';
    form.append(choice, submit);
    root.append(form);
    submit.focus();
    await runtime.whenRenderSettled(choice);
    expect(document.activeElement).toBe(submit);
    return { runtime, form, choice, submit };
}

export const WrapperValidationFocus: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, form, choice, submit } = await mount(canvasElement, 'wrapper', `
{slice @name=blockFirst | false}
{fieldset @class=cem-select__control |
    {legend | Fruit}
    {button @type=button @disabled=true | Disabled}
    {button @type=button @hidden=true | Hidden}
    {span @inert=true | {button @type=button | Inert}}
    {button @type=button @style="display:none" | No display}
    {button @type=button @style="visibility:hidden" | No visibility}
    {element @name=svg @namespace="http://www.w3.org/2000/svg" |
        {attribute @name=tabindex @value=0}
        {attribute @name=width @value=10}
        {attribute @name=height @value=10}}
    {button @type=button @data-option-index=0 @disabled={if blockFirst { true } else { null }} | Choose fruit}
    {button @type=button @data-option-index=1 | Apple}
}`);
        const anchor = choice.querySelector<HTMLButtonElement>('[data-option-index="0"]');
        expect(anchor).not.toBeNull();
        expect(choice.querySelector('svg')?.namespaceURI).toBe('http://www.w3.org/2000/svg');
        expect(choice.checkValidity()).toBe(false);
        expect(document.activeElement).toBe(submit);
        expect(choice.reportValidity()).toBe(false);
        expect(document.activeElement).toBe(anchor);
        expect(choice.value).toBe('');
        expect(Array.from(new FormData(form).entries())).toEqual([['fruit', '']]);

        // A later render must replace an anchor that becomes unusable.
        runtime.setInstanceSlices(choice, { blockFirst: true });
        await runtime.whenRenderSettled(choice);
        expect(choice.querySelector('[data-option-index="0"]')).toBe(anchor);
        expect(anchor?.disabled).toBe(true);
        const next = choice.querySelector('[data-option-index="1"]');
        expect(choice.reportValidity()).toBe(false);
        expect(document.activeElement).toBe(next);
        expect(choice.querySelector('fieldset')?.hasAttribute('tabindex')).toBe(false);
        expect(choice.hasAttribute('tabindex')).toBe(false);

        let submissions = 0;
        form.addEventListener('submit', event => { submissions += 1; event.preventDefault(); });
        submit.focus();
        submit.click();
        expect(submissions).toBe(0);
        expect(document.activeElement).toBe(next);
        choice.value = 'apple';
        await runtime.whenRenderSettled(choice);
        expect(choice.reportValidity()).toBe(true);
        submit.click();
        expect(submissions).toBe(1);
        expect(Array.from(new FormData(form).entries())).toEqual([['fruit', 'apple']]);
        expect(runtime.diagnosticsFor(choice)).toEqual([]);
    },
};

export const ReplacedValidationAnchor: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, choice, submit } = await mount(canvasElement, 'replacement', `
{cem:choose |
    {cem:when @test='datadom.slices.mode == "dropdown"' |
        {button @type=button @class=cem-select__control | Choose fruit}}
    {cem:otherwise |
        {div @class=cem-select__control @tabindex=-1 | Choose fruit}}
}`);
        const first = choice.querySelector('button');
        expect(first).not.toBeNull();
        expect(choice.reportValidity()).toBe(false);
        expect(document.activeElement).toBe(first);
        submit.focus();
        choice.setAttribute('size', '3');
        await runtime.whenRenderSettled(choice);
        const replacement = choice.querySelector('div');
        expect(replacement).not.toBeNull();
        expect(first?.isConnected).toBe(false);
        expect(document.activeElement).toBe(submit);
        expect(choice.reportValidity()).toBe(false);
        expect(document.activeElement).toBe(replacement);
        expect(replacement?.getAttribute('tabindex')).toBe('-1');
        expect(runtime.diagnosticsFor(choice)).toEqual([]);
    },
};
