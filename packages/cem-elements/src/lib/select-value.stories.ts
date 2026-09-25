import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import {
    applyPatchFramesToRange, applyRenderPlanToRange, diffRenderPlansToPatchFrames,
    materializeRenderPlan, mergeRenderedFragmentIntoRange, renderPlanIdentity, type RenderPlan, type RenderPlanNode,
} from './projection.js';

const meta: Meta = { title: 'CEM Elements/Select Value Refresh', tags: ['test'] };
export default meta;
type Story = StoryObj;

function nativeSelectValues(mode: 'plan' | 'patch' | 'fragment', branchIdentities = false): Story {
    return {
        render: () => document.createElement('section'),
        play: ({ canvasElement }) => {
            const bounds = { start: document.createComment('start'), end: document.createComment('end') };
            canvasElement.append(bounds.start, bounds.end);
            let previous: RenderPlan | null = null;
            let revision = 0;
            const modes = ['single', 'multiple', 'listbox'];
            const render = (selected: string[], label = 'Release') => {
                const plan: RenderPlan = {
                    producedTag: 'select-value-test', instanceId: 'select-value-test', templateArtifactId: 'select-value-test',
                    dataRevision: String(++revision), outputTarget: 'light-dom', scopePolicyStamp: 'select-value-test',
                    nodes: [{
                        kind: 'element', namespace: null, tag: 'form', attributes: [], renderNodeId: 'form',
                        children: modes.map(kind => ({
                            kind: 'element', namespace: null, tag: 'select', renderNodeId: kind,
                            attributes: [{ name: 'aria-label', value: `${label} ${kind}` },
                                ...(kind === 'multiple' ? [{ name: 'multiple', value: '' }] : []),
                                ...(kind === 'listbox' ? [{ name: 'size', value: '4' }] : [])],
                            children: [{
                                kind: 'element', namespace: null, tag: 'optgroup', renderNodeId: `${kind}-disabled-group`,
                                attributes: [{ name: 'disabled', value: '' }, { name: 'label', value: 'Unavailable' }],
                                children: [{
                                    kind: 'element', namespace: null, tag: 'option', renderNodeId: `${kind}-group-disabled`,
                                    attributes: [{ name: 'value', value: 'group-disabled' }], children: [],
                                }],
                            }, {
                                kind: 'element', namespace: null, tag: 'optgroup', renderNodeId: `${kind}-group`,
                                attributes: [{ name: 'label', value: 'Releases' }],
                                children: ['disabled', 'a', 'b', 'c'].map(value => ({
                                    kind: 'element', namespace: null, tag: 'option', renderNodeId: `${kind}-${value}${branchIdentities ? selected.includes(value) ? '-default' : '-ordinary' : ''}`,
                                    attributes: [{ name: 'value', value },
                                        ...(value === 'disabled' ? [{ name: 'disabled', value: '' }] : []),
                                        ...(selected.includes(value) ? [{ name: 'selected', value: '' }] : [])],
                                    children: [{ kind: 'text', text: value }],
                                } satisfies RenderPlanNode)),
                            }],
                        })),
                    }],
                };
                if (mode === 'fragment') mergeRenderedFragmentIntoRange(bounds, materializeRenderPlan(plan, document));
                else if (mode === 'patch') {
                    const result = applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(previous, plan), renderPlanIdentity(plan), document);
                    expect(result.status).toBe('applied');
                    expect(result.diagnostics).toEqual([]);
                } else expect(applyRenderPlanToRange(bounds, plan, document).diagnostics).toEqual([]);
                previous = plan;
            };
            render(['a']);
            const selects = Array.from(canvasElement.querySelectorAll('select'));
            const options = Array.from(canvasElement.querySelectorAll('option'));
            const values = () => selects.map(select => Array.from(select.selectedOptions, option => option.value));
            let events = 0;
            for (const select of selects) {
                select.addEventListener('input', () => { events++; });
                select.addEventListener('change', () => { events++; });
                // Every option is dirty, including the next authored selection.
                for (const option of select.options) option.selected = option.value === 'b';
            }
            selects[0].focus();
            render(['a'], 'Unrelated label update');
            expect(values()).toEqual([['b'], ['b'], ['b']]);
            render(['c']);
            expect(values()).toEqual([['c'], ['c'], ['c']]);
            selects.forEach((select, index) => expect(canvasElement.querySelectorAll('select')[index]).toBe(select));
            if (!branchIdentities) options.forEach((option, index) => expect(canvasElement.querySelectorAll('option')[index]).toBe(option));
            expect(document.activeElement).toBe(selects[0]);
            render(['a', 'c']);
            expect(values()).toEqual([['c'], ['a', 'c'], ['c']]);
            render(['disabled']);
            expect(values()).toEqual([['disabled'], ['disabled'], ['disabled']]);
            render([]);
            expect(values()).toEqual([['a'], [], []]);
            render(['a']);
            expect(values()).toEqual([['a'], ['a'], ['a']]);
            selects.forEach(select => { select.value = 'b'; });
            render(['a']);
            expect(values()).toEqual([['b'], ['b'], ['b']]);
            canvasElement.querySelector('form')?.reset();
            expect(values()).toEqual([['a'], ['a'], ['a']]);
            expect(events).toBe(0);
        },
    };
}

export const DirectNativeSelectValues = nativeSelectValues('plan');
export const PatchedNativeSelectValues = nativeSelectValues('patch');

export const MergedNativeSelectValues = nativeSelectValues('fragment');

// Conditional branches can replace option identities instead of patching attrs.
export const DirectConditionalOptions = nativeSelectValues('plan', true);
export const PatchedConditionalOptions = nativeSelectValues('patch', true);
export const MergedConditionalOptions = nativeSelectValues('fragment', true);

function controlledSelectValues(mode: 'plan' | 'patch' | 'fragment'): Story {
    return {
        render: () => document.createElement('section'),
        play: ({ canvasElement }) => {
            const bounds = { start: document.createComment('start'), end: document.createComment('end') };
            canvasElement.append(bounds.start, bounds.end);
            let previous: RenderPlan | null = null;
            let revision = 0;
            const render = (value: string | undefined, options = ['a', 'b', 'c'], branch = '') => {
                const plan: RenderPlan = {
                    producedTag: 'controlled-select', instanceId: 'controlled-select', templateArtifactId: 'controlled-select',
                    dataRevision: String(++revision), outputTarget: 'light-dom', scopePolicyStamp: 'controlled-select',
                    nodes: [{ kind: 'element', namespace: null, tag: 'form', attributes: [], renderNodeId: 'form',
                        children: ['controlled', 'uncontrolled', 'owned-child'].map(kind => ({
                            kind: 'element', namespace: null, tag: kind === 'owned-child' ? 'article' : 'select', renderNodeId: kind,
                            attributes: kind === 'controlled' && value !== undefined ? [{ name: 'value', value }] : [],
                            children: kind === 'owned-child' ? [] : options.map(option => ({
                                kind: 'element', namespace: null, tag: 'option', renderNodeId: `${kind}-${option}-${branch}`,
                                attributes: [{ name: 'value', value: option }, ...(option === 'a' ? [{ name: 'selected', value: '' }] : [])],
                                children: [{ kind: 'text', text: option }],
                            })),
                        })),
                    }],
                };
                const mergeOptions = { preserveElementChildren: (element: Element) => element.localName === 'article' };
                if (mode === 'fragment') mergeRenderedFragmentIntoRange(bounds, materializeRenderPlan(plan, document), mergeOptions);
                else if (mode === 'patch') expect(applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(previous, plan), renderPlanIdentity(plan), document, mergeOptions).status).toBe('applied');
                else applyRenderPlanToRange(bounds, plan, document, mergeOptions);
                previous = plan;
            };
            render('c');
            const [controlled, uncontrolled] = Array.from(canvasElement.querySelectorAll('select'));
            expect(controlled.value).toBe('c');
            const nested = document.createElement('select');
            nested.innerHTML = '<option value="a">a</option><option value="b">b</option>';
            nested.setAttribute('value', 'a');
            nested.value = 'b';
            canvasElement.querySelector('article')?.replaceChildren(nested);
            let events = 0;
            controlled.addEventListener('input', () => { events++; });
            controlled.addEventListener('change', () => { events++; });
            controlled.focus();
            controlled.value = 'b';
            uncontrolled.value = 'b';
            // A superseded intermediate render leaves the final plan unchanged.
            render('c');
            expect(controlled.value).toBe('c');
            expect(uncontrolled.value).toBe('b');
            expect(nested.value).toBe('b');
            expect(document.activeElement).toBe(controlled);
            render('b', ['a', 'b', 'c'], 'replacement');
            expect(canvasElement.querySelector('select')).toBe(controlled);
            expect(controlled.value).toBe('b');
            render('late', ['a']);
            expect(controlled.selectedIndex).toBe(-1);
            render('late', ['a', 'late']);
            expect(controlled.value).toBe('late');
            render('', ['a', '']);
            expect(controlled.selectedIndex).toBe(1);
            render('', ['a']);
            expect(controlled.selectedIndex).toBe(-1);
            render(undefined);
            expect(controlled.value).toBe('a');
            controlled.value = 'b';
            render(undefined);
            expect(controlled.value).toBe('b');
            render('c');
            canvasElement.querySelector('form')?.reset();
            expect(controlled.value).toBe('a');
            render('c');
            expect(controlled.value).toBe('c');
            expect(events).toBe(0);
        },
    };
}

export const DirectControlledSelect = controlledSelectValues('plan');
export const PatchedControlledSelect = controlledSelectValues('patch');
export const MergedControlledSelect = controlledSelectValues('fragment');


function boundSelect(mode: 'dom' | 'worker' | 'wasm'): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const declarationTag = `cem-select-value-${mode}`;
            const tag = `story-select-value-${mode}`;
            const runtime = new CemElementRuntime({ declarationTag });
            runtime.install(window);
            const declaration = document.createElement(declarationTag);
            declaration.setAttribute('tag', tag);
            const template = document.createElement('template');
            if (mode === 'dom') {
                template.innerHTML = '<slice name="choice">c</slice>'
                    + '<select value="{$datadom.slices.choice}"><option value="a" selected>a</option><option value="b">b</option><option value="c">c</option></select>'
                    + '<select><option value="a" selected>a</option><option value="b">b</option></select>';
            } else {
                template.type = 'text/cem-ml';
                template.textContent = (mode === 'wasm'
                    ? '{location-element @slice=route @href="https://example.test/selects"}' : '') + `
{slice @name=choice | c}
{select @value="{$datadom.slices.choice}" |
    {option @value=a @selected=true | a}{option @value=b | b}{option @value=c | c}}
{select | {option @value=a @selected=true | a}{option @value=b | b}}`;
            }
            declaration.append(template);
            canvasElement.append(declaration);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const instance = document.createElement(tag);
            canvasElement.append(instance);
            await runtime.whenRenderSettled(instance);
            const [controlled, uncontrolled] = Array.from(instance.querySelectorAll('select'));
            expect(controlled.value).toBe('c');
            controlled.value = 'b';
            uncontrolled.value = 'b';
            controlled.focus();
            // The unused slice schedules a render whose DOM plan is unchanged.
            runtime.setInstanceSlices(instance, { tick: 'unrelated render' });
            await runtime.whenRenderSettled(instance);
            expect(controlled.value).toBe('c');
            expect(uncontrolled.value).toBe('b');
            controlled.value = 'b';
            runtime.setInstanceSlices(instance, { choice: 'b' });
            runtime.setInstanceSlices(instance, { choice: 'c' });
            await runtime.whenRenderSettled(instance);
            expect(controlled.value).toBe('c');
            expect(uncontrolled.value).toBe('b');
            expect(instance.querySelector('select')).toBe(controlled);
            expect(document.activeElement).toBe(controlled);
            expect(runtime.diagnosticsFor(declaration)).toEqual([]);
            expect(runtime.diagnosticsFor(instance)).toEqual([]);
        },
    };
}

export const DomBoundSelect = boundSelect('dom');
export const WorkerBoundSelect = boundSelect('worker');
export const WasmBoundSelect = boundSelect('wasm');
