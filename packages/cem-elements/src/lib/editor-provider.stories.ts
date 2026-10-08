import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { getCemEditorProvider } from './form-control-capability.js';

const meta: Meta = { title: 'CEM Elements/Editor Provider', tags: ['test'] };
export default meta;
type Story = StoryObj;
function required<T>(value: T | null | undefined): T {
    if (value == null) throw new Error("Missing editor-provider fixture value");
    return value;
}
interface Field extends HTMLElement { value: string; setCustomValidity(message: string): void; validity: ValidityState; formStateRestoreCallback(value: string, mode: 'restore'): void }

export const Transactions: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const runtime = new CemElementRuntime({ declarationTag: 'editor-provider-declaration' });
        runtime.install(window);
        const declaration = document.createElement('editor-provider-declaration');
        declaration.setAttribute('tag', 'story-editor-provider');
        declaration.setAttribute('capability', 'form-control');
        const template = document.createElement('template');
        template.type = 'text/cem-ml';
        template.textContent = '{input @part=control @form="" @type=text @value={datadom.slices.value} @required={if seq:count(datadom.attributes.required) > 0 { true } else { null }} @slice=value @slice-event=input @slice-value="$target.value"}';
        declaration.append(template); canvasElement.append(declaration);
        runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
        const form = document.createElement('form');
        form.innerHTML = '<story-editor-provider name="choice" value="default"></story-editor-provider><button type="submit">Submit</button>';
        canvasElement.append(form);
        const field = form.querySelector('story-editor-provider') as Field;
        await runtime.whenRenderSettled(field);
        const provider = required(getCemEditorProvider(field));
        const isolatedUrl = new URL('./form-control-capability.ts?editor-provider-interop', import.meta.url).href;
        const isolated = await import(/* @vite-ignore */ isolatedUrl) as typeof import('./form-control-capability.js');
        expect(isolated.getCemEditorProvider(field)).toBe(provider);
        const input = required(field.querySelector('input'));
        expect(provider.control).toBe(input);
        const changes: string[] = [];
        const release = provider.subscribe(update => changes.push(update.cause));
        const lease = provider.lease({});
        expect(isolated.isCemEditorLeaseFor(lease, provider)).toBe(true);
        expect(isolated.isCemEditorLeaseFor({ ...lease }, provider)).toBe(false);
        const seen: unknown[] = [];
        input.addEventListener('input', () => seen.push([field.value, input.value, new FormData(form).get('choice'), runtime.snapshotInstance(field).slices.value]));
        const events: string[] = [];
        form.addEventListener('input', () => events.push('input'));
        form.addEventListener('change', () => events.push('change'));
        expect(lease.commit('apple', { revision: provider.revision })).toBe(true);
        expect(seen).toEqual([['apple', 'apple', 'apple', 'apple']]);
        expect(events).toEqual(['input', 'change']);
        expect(changes).toEqual(['claims', 'commit']);
        await runtime.whenRenderSettled(field);
        expect(input.value).toBe('apple');
        expect(field.getAttribute('value')).toBe('default');

        const veto = (event: Event) => event.preventDefault();
        input.addEventListener('beforeinput', veto);
        expect(lease.commit('pear', { revision: provider.revision })).toBe(false);
        expect(field.value).toBe('apple');
        input.removeEventListener('beforeinput', veto);
        const revision = provider.revision;
        field.value = 'apple';
        expect(provider.revision).toBeGreaterThan(revision);
        expect(changes.at(-1)).toBe('programmatic');
        expect(lease.commit('pear', { revision })).toBe(false);
        await runtime.whenRenderSettled(field);

        const competing = provider.lease({});
        expect(lease.valid).toBe(false); expect(competing.valid).toBe(false);
        expect(lease.current).toBe(true); expect(competing.current).toBe(true);
        expect(lease.commit('pear', { revision: provider.revision })).toBe(false);
        competing.release(); expect(lease.valid).toBe(true);

        const validityLease = provider.validity({});
        validityLease.set('Choose a suggestion.');
        expect(field.validity.customError).toBe(true);
        field.setAttribute('required', ''); field.value = '';
        await runtime.whenRenderSettled(field);
        expect(field.validity.valueMissing).toBe(true);
        expect(input.validationMessage).not.toBe('Choose a suggestion.');
        field.setCustomValidity('Author error');
        expect(input.validationMessage).toBe('Author error');
        validityLease.release();
        expect(input.validationMessage).toBe('Author error');
        field.removeAttribute('required'); field.value = 'apple'; await runtime.whenRenderSettled(field);
        field.setCustomValidity(''); expect(field.validity.valid).toBe(true);

        const mutate = () => { field.value = 'listener'; };
        input.addEventListener('beforeinput', mutate);
        expect(lease.commit('pear', { revision: provider.revision })).toBe(false);
        expect(field.value).toBe('listener');
        input.removeEventListener('beforeinput', mutate);
        await runtime.whenRenderSettled(field);

        form.reset(); expect(changes.at(-1)).toBe('reset');
        await runtime.whenRenderSettled(field); expect(field.value).toBe('default');
        field.formStateRestoreCallback('restored', 'restore'); expect(changes.at(-1)).toBe('restore');
        await runtime.whenRenderSettled(field); expect(field.value).toBe('restored');
        if (import.meta.env.MODE === 'test') {
            const { userEvent: nativeUser } = await import('vitest/browser');
            const button = required(form.querySelector('button')); button.type = 'button';
            const raw: boolean[] = []; const capture = (event: Event) => raw.push(event.isTrusted);
            form.addEventListener('change', capture, true);
            field.value = ''; await runtime.whenRenderSettled(field);
            input.focus(); await nativeUser.keyboard('typed'); await runtime.whenRenderSettled(field);
            const beforeChanges = events.filter(type => type === 'change').length;
            expect(lease.commit('apple', { revision: provider.revision })).toBe(true);
            button.focus(); await new Promise(resolve => setTimeout(resolve));
            expect(events.filter(type => type === 'change')).toHaveLength(beforeChanges + 1);
            // Raw ancestor capture can see a trusted platform event before reconciliation.
            expect(raw.filter(trusted => !trusted)).toHaveLength(1);
            input.focus(); await nativeUser.keyboard('x'); button.focus();
            await new Promise(resolve => setTimeout(resolve));
            expect(events.filter(type => type === 'change')).toHaveLength(beforeChanges + 2);
            expect(new FormData(form).get('choice')).toBe(input.value);
            form.removeEventListener('change', capture, true);

            button.type = 'submit'; let submissions = 0;
            form.addEventListener('submit', event => { event.preventDefault(); submissions++; });
            input.focus();
            input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
            const key = new KeyboardEvent('keydown', { key: 'Process', code: 'KeyA', bubbles: true });
            input.dispatchEvent(key);
            expect(isolated.isCemEditorCompositionKey(key)).toBe(true);
            input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Process', code: 'KeyA', bubbles: true }));
            input.addEventListener('keydown', () => input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })), { once: true });
            await nativeUser.keyboard('{Enter}'); await new Promise(resolve => setTimeout(resolve));
            expect(submissions).toBe(0);
            await nativeUser.keyboard('{Enter}'); await new Promise(resolve => setTimeout(resolve));
            expect(submissions).toBe(1);
            // This verifies captured composition ownership plus a native key,
            // not actual device/IME composition ordering.
        }
        lease.release(); release();
        expect(runtime.diagnosticsFor(field)).toEqual([]);
    },
};

export const ReentrancyAndAdmission: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const runtime = new CemElementRuntime({ declarationTag: 'editor-admission-declaration' }); runtime.install(window);
        const define = async (tag: string, source: string) => {
            const declaration = document.createElement('editor-admission-declaration'); declaration.setAttribute('tag', tag);
            declaration.setAttribute('capability', 'form-control');
            const template = document.createElement('template'); template.type = 'text/cem-ml'; template.textContent = source;
            declaration.append(template); canvasElement.append(declaration); runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
        };
        await define('story-editor-ambiguous', '{input @part=control}{input @part=control}');
        const ambiguous = document.createElement('story-editor-ambiguous'); canvasElement.append(ambiguous);
        await runtime.whenRenderSettled(ambiguous); expect(required(getCemEditorProvider(ambiguous)).control).toBeNull();
        await define('story-editor-reentrant', '{input @part=control @form="" @value={datadom.slices.value}}');
        const field = document.createElement('story-editor-reentrant') as Field; canvasElement.append(field);
        await runtime.whenRenderSettled(field);
        const provider = required(getCemEditorProvider(field)), lease = provider.lease({});
        const input = required(provider.control);
        let changes = 0; field.addEventListener('change', () => changes++);
        input.addEventListener('input', () => { field.value = 'superseded'; }, { once: true });
        expect(lease.commit('accepted', { revision: provider.revision })).toBe(true);
        expect(field.value).toBe('superseded'); expect(changes).toBe(0);
        await runtime.whenRenderSettled(field); expect(input.value).toBe('superseded');
        input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        expect(provider.composing).toBe(true);
        expect(lease.commit('blocked', { revision: provider.revision })).toBe(false);
        input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
        await new Promise(resolve => setTimeout(resolve));
        expect(provider.composing).toBe(false);
        input.focus();
        const press = new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true });
        input.addEventListener('keydown', event => lease.handlePress(event as KeyboardEvent), { once: true });
        input.dispatchEvent(press); expect(press.defaultPrevented).toBe(true);
        const repeat = new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', repeat: true, bubbles: true, cancelable: true });
        input.dispatchEvent(repeat); expect(repeat.defaultPrevented).toBe(true);
        input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
        const next = new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true });
        input.dispatchEvent(next); expect(next.defaultPrevented).toBe(false);
        const contribution = provider.validity({}); contribution.set('Transient error');
        field.remove(); expect(lease.valid).toBe(false); expect(lease.current).toBe(false);
        canvasElement.append(field); await runtime.whenRenderSettled(field);
        expect(lease.valid).toBe(false);
        expect(contribution.set('Stale error')).toBe(false);
        expect(field.validity.customError).toBe(false);
        const fresh = provider.lease({}); expect(fresh.valid).toBe(true); expect(fresh.current).toBe(true); fresh.release(); expect(fresh.current).toBe(false);
        lease.release(); contribution.release();
        await define('story-editor-nested-owner', '{story-editor-reentrant}');
        const outer = document.createElement('story-editor-nested-owner'); canvasElement.append(outer);
        await runtime.whenRenderSettled(outer);
        await runtime.whenRenderSettled(required(outer.querySelector<HTMLElement>('story-editor-reentrant')));
        expect(required(getCemEditorProvider(outer)).control).toBeNull();
    },
};
