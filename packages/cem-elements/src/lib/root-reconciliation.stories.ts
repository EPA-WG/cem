import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { applyRenderPlanToRange, diffRenderPlansToPatchFrames, preparePatchFramesForRange,
    renderPlanIdentity, type RenderPlan, type RenderPlanNode } from './projection.js';
import { validateLivePreviewFrames } from './live-preview-policy.js';

export default { title: 'CEM Elements/Root Reconciliation', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const text = (id: string, value: string): RenderPlanNode => ({ kind: 'text', renderNodeId: id, text: value });
const element = (tag: string, id: string, attributes: Record<string, string> = {}, children: RenderPlanNode[] = []): RenderPlanNode =>
    ({ kind: 'element', tag, namespace: null, renderNodeId: id, attributes: Object.entries(attributes).map(([name, value]) => ({ name, value })), children });
const controls = () => [element('input', 'input', { value: 'Original' }), element('textarea', 'area', {}, [text('area-text', 'Original')]),
    element('input', 'check', { type: 'checkbox' }), element('select', 'select', {}, ['a', 'b'].map(v => element('option', v, { value: v }, [text(`${v}-text`, v)]))),
    element('div', 'editor', { contenteditable: 'true' }, [text('editor-text', 'Original')])];
const plan = (nodes: RenderPlanNode[], revision: number): RenderPlan => ({ producedTag: 'root-form', instanceId: 'instance', templateArtifactId: 'template',
    dataRevision: String(revision), scopePolicyStamp: 'policy', outputTarget: 'light-dom', nodes });
function roots(mode: 'direct' | 'patch' | 'serialized' | 'fallback'): Story {
    return { render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        if (mode === 'fallback') Object.defineProperty(host, 'moveBefore', { value: undefined });
        const outside = document.createElement('button'), preserved = document.createElement('aside');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(outside, bounds.start, preserved, bounds.end);
        const options = { preserveNode: (node: Node) => node === preserved };
        let previous: RenderPlan | null = null, revision = 0;
        const publish = (nodes: RenderPlanNode[]) => {
            const next = plan(nodes, ++revision);
            if (mode === 'direct') expect(applyRenderPlanToRange(bounds, next, document, options).mode).toBe('patch');
            else {
                const frames = diffRenderPlansToPatchFrames(previous, next, { textNodeIdsAvailable: mode !== 'serialized' });
                expect(preparePatchFramesForRange(bounds, frames, renderPlanIdentity(next), document, options).commit(renderPlanIdentity(next)).status).toBe('applied');
            }
            previous = next;
        };
        publish(controls());
        const input = host.querySelector('input')!, area = host.querySelector('textarea')!, check = host.querySelector<HTMLInputElement>('[type=checkbox]')!;
        const select = host.querySelector('select')!, editor = host.querySelector<HTMLElement>('[contenteditable]')!;
        input.value = 'User input'; area.value = 'User area'; check.checked = true; select.value = 'b'; editor.textContent = 'User editor';
        area.focus(); area.setSelectionRange(1, 5, 'backward');
        const assertEdits = () => {
            expect([host.querySelector('input:not([type])'), host.querySelector('textarea'), host.querySelector('[type=checkbox]'), host.querySelector('select'), host.querySelector('[contenteditable]')])
                .toEqual([input, area, check, select, editor]);
            expect([input.value, area.value, check.checked, select.value, editor.textContent]).toEqual(['User input', 'User area', true, 'b', 'User editor']);
        };
        const added = () => [text('heading', 'Heading'), { kind: 'comment', renderNodeId: 'comment', text: 'Comment' } as RenderPlanNode,
            element('p', 'notice', {}, [text('notice-text', 'Notice')])];
        publish([...added(), ...controls()]); assertEdits();
        expect(document.activeElement).toBe(area);
        expect([area.selectionStart, area.selectionEnd, area.selectionDirection]).toEqual([1, 5, 'backward']);
        editor.focus(); document.getSelection()!.setBaseAndExtent(editor.firstChild!, 1, editor.firstChild!, 5);
        publish([...controls().reverse(), ...added()]); assertEdits();
        expect(document.activeElement).toBe(editor); expect(document.getSelection()!.toString()).toBe('ser ');
        publish(controls()); assertEdits();
        // Changed root text/comments in restored output also reconcile the range.
        publish([...added(), ...controls()]);
        const changed = added(); if (changed[0].kind === 'text') changed[0].text = 'Updated heading';
        publish([...changed, ...controls()]); assertEdits(); expect(host.textContent).toContain('Updated heading');
        publish([]);
        expect(host.querySelector('input')).toBeNull(); expect(host.querySelector('[contenteditable]')).toBeNull();
        expect(Array.from(host.childNodes)).toEqual([outside, bounds.start, preserved, bounds.end]);
        publish([]); publish(controls());
        expect(host.querySelector('input')).not.toBe(input); expect(host.querySelector('input')!.value).toBe('Original');
        expect(host.contains(outside)).toBe(true); expect(host.contains(preserved)).toBe(true);
    } };
}
export const DirectRoots = roots('direct');
export const PatchRoots = roots('patch');
export const SerializedRoots = roots('serialized');
export const FallbackMoves = roots('fallback');

export const EmptyAdmissionAndPreviewPolicy: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan(controls(), 1), empty = plan([], 2);
        applyRenderPlanToRange(bounds, before, document);
        const area = host.querySelector('textarea')!; area.value = 'Keep this'; area.focus();
        const frames = diffRenderPlansToPatchFrames(before, empty);
        expect(frames.some(f => f.type === 'ops')).toBe(true);
        const prepare = () => preparePatchFramesForRange(bounds, frames, renderPlanIdentity(empty), document);
        const cancelled = prepare(); cancelled.cancel(); expect(cancelled.commit(renderPlanIdentity(empty)).status).toBe('aborted');
        expect(prepare().commit({ ...renderPlanIdentity(empty), dataRevision: 'newer' }).status).toBe('stale');
        expect(host.querySelector('textarea')).toBe(area); expect(area.value).toBe('Keep this');
        const changed = prepare(); area.replaceWith(area.cloneNode(true));
        expect(changed.commit(renderPlanIdentity(empty)).status).toBe('aborted');
        const unsafe = plan([...controls(), element('script', 'unsafe')], 3);
        expect(() => validateLivePreviewFrames(diffRenderPlansToPatchFrames(before, unsafe))).toThrow('cem.preview.element_denied');
        const incompatible = structuredClone(frames);
        if (incompatible[0].type === 'begin') incompatible[0].renderEngineVersion = '1.6.0';
        expect(preparePatchFramesForRange(bounds, incompatible, renderPlanIdentity(empty), document).commit(renderPlanIdentity(empty)).status).toBe('aborted');
        const duplicate = diffRenderPlansToPatchFrames(before, plan([...controls(), element('p', 'extra')], 2));
        const rootBatch = duplicate.find(f => f.type === 'ops');
        const rootOp = rootBatch?.type === 'ops' ? rootBatch.ops[0] : undefined;
        if (rootOp?.op !== 'reconcileRoots') throw new Error('missing root operation');
        rootOp.children.push(structuredClone(rootOp.children[0]));
        expect(preparePatchFramesForRange(bounds, duplicate, renderPlanIdentity(empty), document).commit(renderPlanIdentity(empty)).status).toBe('aborted');
        const mixed = structuredClone(frames); const batch = mixed.find(f => f.type === 'ops');
        if (!batch || batch.type !== 'ops') throw new Error('missing empty operation');
        batch.ops.push({ op: 'setText', target: { id: 'area-text' }, value: 'Partial' });
        expect(preparePatchFramesForRange(bounds, mixed, renderPlanIdentity(empty), document).commit(renderPlanIdentity(empty)).status).toBe('aborted');
        const foreignEmpty = { ...empty, templateArtifactId: 'replacement' };
        expect(preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, foreignEmpty), renderPlanIdentity(foreignEmpty), document)
            .commit(renderPlanIdentity(foreignEmpty)).status).toBe('applied');
        expect(host.querySelector('textarea')).toBeNull();
    },
};

export const StableTextRootIdentity: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan([text('stable', 'Stable text'), ...controls()], 1);
        applyRenderPlanToRange(bounds, before, document);
        const stable = bounds.start.nextSibling;
        const after = plan([text('new', 'Added text'), ...before.nodes], 2);
        expect(preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), renderPlanIdentity(after), document)
            .commit(renderPlanIdentity(after)).status).toBe('applied');
        expect(bounds.start.nextSibling?.nextSibling).toBe(stable);
        expect(stable?.textContent).toBe('Stable text');
    },
};

export const CorruptionStillRecovers: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        const retained = document.createElement('aside');
        host.append(bounds.start, retained, bounds.end);
        const options = { preserveNode: (node: Node) => node === retained };
        const before = plan(controls(), 1); applyRenderPlanToRange(bounds, before, document, options);
        const input = host.querySelector('input')!;
        input.setAttribute('data-cem-render-node-id', 'corrupted');
        (input as HTMLInputElement & { cemRenderNodeId?: string }).cemRenderNodeId = 'corrupted';
        const next = plan([...controls(), element('p', 'extra')], 2);
        const result = applyRenderPlanToRange(bounds, next, document, options);
        expect(result.mode).toBe('replaceScope');
        expect(result.diagnostics[0]?.reason).toBe('recovery');
        expect(host.querySelector('input')).not.toBe(input);
        expect(host.contains(retained)).toBe(true);
        const empty = plan([], 3);
        expect(preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(next, empty), renderPlanIdentity(empty), document, options)
            .commit(renderPlanIdentity(empty)).status).toBe('applied');
        expect(Array.from(host.childNodes)).toEqual([bounds.start, retained, bounds.end]);
    },
};

export const CapturedInputPatchInvalidatesOwner: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const root = canvasElement.querySelector('section')!;
        const tag = 'cem-captured-input-owner-fixture';
        if (!customElements.get(tag)) customElements.define(tag, class extends HTMLElement {});
        const bounds = {start:document.createComment('start'),end:document.createComment('end')};
        root.append(bounds.start,bounds.end);
        const original = plan([element(tag,'owner',{},[element('input','field',{value:'Original'})])],1);
        applyRenderPlanToRange(bounds,original,document);
        const owner = root.querySelector(tag)!, field = owner.querySelector('input')!;
        const inert = document.createElement('template');
        const payload = document.createElement('section'); inert.content.append(payload);
        payload.append(field);
        owner.append(document.createElement('p'));
        const options = {preserveElementChildren:(node:Element)=>node===owner,
            authoredChildContainer:(node:Element)=>node===owner?payload:undefined};
        const disabled = plan([element(tag,'owner',{},[element('input','field',{value:'Original',disabled:'true'})])],2);
        expect(preparePatchFramesForRange(bounds,diffRenderPlansToPatchFrames(original,disabled),renderPlanIdentity(disabled),document,options)
            .commit(renderPlanIdentity(disabled)).status).toBe('applied');
        expect(field.disabled).toBe(true);
        applyRenderPlanToRange(bounds,{...original,dataRevision:'3'},document,options);
        expect(field.disabled).toBe(false);
        expect(payload.querySelector('input')).toBe(field);
        expect(owner.querySelector('p')).not.toBeNull();
    },
};
