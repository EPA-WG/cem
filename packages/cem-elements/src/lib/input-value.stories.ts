import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import {
    applyPatchFramesToRange, applyRenderPlanToRange, diffRenderPlansToPatchFrames,
    renderPlanIdentity, type RenderPlan,
} from './projection.js';

const meta: Meta = { title: 'CEM Elements/Input Value Refresh', tags: ['test'] };
export default meta;
type Story = StoryObj;
type Mode = 'dom' | 'worker' | 'wasm';

function boundInput(mode: Mode): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const declarationTag = `cem-input-value-${mode}`;
            const tag = `story-input-value-${mode}`;
            const runtime = new CemElementRuntime({ declarationTag });
            runtime.install(window);
            const declaration = document.createElement(declarationTag);
            declaration.setAttribute('tag', tag);
            const template = document.createElement('template');
            if (mode === 'dom') {
                template.innerHTML = '<slice name="text">initial</slice><slice name="live">initial</slice>'
                    + '<input aria-label="On change" value="{$datadom.slices.text}" slice="text" slice-event="change" slice-value="$target.value" />'
                    + '<input aria-label="On input" value="{$datadom.slices.live}" slice="live" slice-event="input" slice-value="$target.value" />'
                    + '<output>${$datadom.slices.text}</output><output>${$datadom.slices.tick}</output>';
            } else {
                template.type = 'text/cem-ml';
                template.textContent = (mode === 'wasm'
                    ? '{location-element @slice=route @href="https://example.test/inputs"}' : '') + `
{slice @name=text | initial}{slice @name=live | initial}
{input @aria-label="On change" @value="{$datadom.slices.text}" @slice=text @slice-event=change @slice-value="$target.value"}
{input @aria-label="On input" @value="{$datadom.slices.live}" @slice=live @slice-event=input @slice-value="$target.value"}
{output | {$datadom.slices.text}}{output | {$datadom.slices.tick}}`;
            }
            declaration.append(template);
            canvasElement.append(declaration);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const instance = document.createElement(tag);
            canvasElement.append(instance);
            await runtime.whenRenderSettled(instance);
            const input = instance.querySelector<HTMLInputElement>('input[aria-label="On change"]');
            const live = instance.querySelector<HTMLInputElement>('input[aria-label="On input"]');
            if (!input || !live) throw new Error('Bound inputs are missing');
            const outputs = () => Array.from(instance.querySelectorAll('output'), node => node.textContent?.trim());
            const set = async (values: Record<string, string>) => {
                runtime.setInstanceSlices(instance, values);
                await runtime.whenRenderSettled(instance);
                expect(instance.querySelectorAll('input')[0]).toBe(input);
                expect(instance.querySelectorAll('input')[1]).toBe(live);
            };
            expect(input.value).toBe('initial');
            await userEvent.clear(input);
            await userEvent.type(input, 'unfinished');
            input.setSelectionRange(2, 5, 'backward');
            await set({ tick: 'unrelated render' });
            expect(outputs()).toEqual(['initial', 'unrelated render']);
            expect(input.value).toBe('unfinished');
            expect(input.defaultValue).toBe('initial');
            expect(document.activeElement).toBe(input);
            expect([input.selectionStart, input.selectionEnd, input.selectionDirection]).toEqual([2, 5, 'backward']);

            // Native blur commits a change-bound edit, then a preset returns to the initial value.
            await userEvent.tab();
            await runtime.whenRenderSettled(instance);
            expect(outputs()[0]).toBe('unfinished');
            expect(input.defaultValue).toBe('unfinished');
            await set({ text: 'initial' });
            expect(input.value).toBe('initial');
            expect(input.defaultValue).toBe('initial');
            expect(document.activeElement).toBe(live);

            input.focus();
            input.setSelectionRange(2, 5, 'backward');
            await set({ text: 'replacement' });
            expect(input.value).toBe('replacement');
            expect(document.activeElement).toBe(input);
            expect([input.selectionStart, input.selectionEnd, input.selectionDirection]).toEqual([2, 5, 'backward']);
            await set({ text: 'x' });
            expect(input.value).toBe('x');
            expect([input.selectionStart, input.selectionEnd]).toEqual([1, 1]);
            await set({ text: '' });
            expect(input.value).toBe('');
            expect(input.defaultValue).toBe('');

            // Input-bound typing must keep the caret, including a middle insertion.
            await userEvent.clear(live);
            await userEvent.type(live, 'abcd');
            await runtime.whenRenderSettled(instance);
            expect(live.value).toBe('abcd');
            expect(live.defaultValue).toBe('abcd');
            live.setSelectionRange(2, 2);
            await userEvent.keyboard('X');
            await runtime.whenRenderSettled(instance);
            expect(live.value).toBe('abXcd');
            expect(live.defaultValue).toBe('abXcd');
            expect(document.activeElement).toBe(live);
            expect([live.selectionStart, live.selectionEnd]).toEqual([3, 3]);
            expect(runtime.diagnosticsFor(declaration)).toEqual([]);
            expect(runtime.diagnosticsFor(instance)).toEqual([]);
        },
    };
}

export const DomBoundInput = boundInput('dom');
export const WorkerBoundInput = boundInput('worker');
export const WasmBoundInput = boundInput('wasm');

function nativeInputValues(patches: boolean): Story {
    return {
        render: () => document.createElement('section'),
        play: ({ canvasElement }) => {
            const bounds = { start: document.createComment('start'), end: document.createComment('end') };
            canvasElement.append(bounds.start, bounds.end);
            const types = ['text', 'number', 'range', 'color', 'date', 'checkbox', 'radio', 'hidden', 'button', 'file'];
            const initial = ['initial', '12', '12', '#112233', '2026-01-01', 'initial', 'initial', 'initial', 'initial', 'initial'];
            const dirty = ['draft', '34', '34', '#445566', '2026-02-02', 'initial', 'initial', 'initial', 'initial', ''];
            const next = ['replacement', '56', '56', '#778899', '2026-03-03', 'replacement', 'replacement', 'replacement', 'replacement', 'replacement'];
            let revision = 0;
            let previous: RenderPlan | null = null;
            const render = (values: Array<string | null>) => {
                const plan: RenderPlan = {
                    producedTag: 'input-value-test', instanceId: 'input-value-test', templateArtifactId: 'input-value-test',
                    dataRevision: String(++revision), outputTarget: 'light-dom', scopePolicyStamp: 'input-value-test',
                    nodes: [{
                        kind: 'element', namespace: null, tag: 'form', attributes: [], renderNodeId: 'form',
                        children: types.map((type, index) => ({
                            kind: 'element', namespace: null, tag: 'input', renderNodeId: `input-${type}`, children: [],
                            attributes: [{ name: 'type', value: type },
                                ...(values[index] === null ? [] : [{ name: 'value', value: values[index] as string }])],
                        })),
                    }],
                };
                if (patches) {
                    const result = applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(previous, plan), renderPlanIdentity(plan), document);
                    expect(result.status).toBe('applied');
                    expect(result.diagnostics).toEqual([]);
                } else expect(applyRenderPlanToRange(bounds, plan, document).diagnostics).toEqual([]);
                previous = plan;
            };
            const nativeValue = (type: string, value: string | null) => {
                const native = document.createElement('input');
                native.type = type;
                if (value !== null) native.setAttribute('value', value);
                return native.value;
            };
            render(initial);
            const inputs = Array.from(canvasElement.querySelectorAll('input'));
            inputs.forEach((input, index) => { input.value = dirty[index]; });
            inputs[5].checked = true;
            inputs[6].checked = true;
            render(initial);
            expect(inputs.map(input => input.value)).toEqual(dirty);
            const check = (values: Array<string | null>) => {
                expect(Array.from(canvasElement.querySelectorAll('input'))).toEqual(inputs);
                inputs.forEach((input, index) => {
                    expect(input.value, `${input.type} live value`).toBe(nativeValue(input.type, values[index]));
                    expect(input.getAttribute('value')).toBe(values[index]);
                    expect(input.defaultValue).toBe(values[index] ?? '');
                });
                expect(inputs[5].checked).toBe(true);
                expect(inputs[6].checked).toBe(true);
            };
            render(next);
            check(next);
            render(initial);
            check(initial);
            const empty = types.map(() => '');
            render(empty);
            check(empty);
            render(next);
            check(next);
            const absent = types.map(() => null);
            render(absent);
            check(absent);
            // An unbound dirty input is unaffected by later renders; reset still uses the native default.
            inputs[0].value = 'unbound draft';
            render(absent);
            expect(inputs[0].value).toBe('unbound draft');
            canvasElement.querySelector('form')?.reset();
            expect(inputs[0].value).toBe('');
            render(next);
            inputs[0].value = 'another draft';
            canvasElement.querySelector('form')?.reset();
            expect(inputs[0].value).toBe('replacement');
        },
    };
}

export const DirectNativeInputValues = nativeInputValues(false);
export const PatchedNativeInputValues = nativeInputValues(true);
