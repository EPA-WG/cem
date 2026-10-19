import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { EXPERIMENTAL_ARIA_REFERENCE_PROFILE } from './aria-reference-profile.js';
import { applyRenderPlanToRange, diffRenderPlansToPatchFrames, preparePatchFramesForRange,
    renderPlanIdentity, renderRevisionKey, type RenderPlan, type RenderPlanNode } from './projection.js';

export default { title: 'CEM Elements/Committed DOM Reuse', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const element = (tag: string, id: string, attributes: Record<string, string> = {}, children: RenderPlanNode[] = []): RenderPlanNode =>
    ({ kind: 'element', tag, namespace: null, renderNodeId: id, attributes: Object.entries(attributes).map(([name, value]) => ({ name, value })), children });
const text = (id: string, value: string): RenderPlanNode => ({ kind: 'text', renderNodeId: id, text: value });
const plan = (revision: string, value = 'Original'): RenderPlan => ({ producedTag: 'reuse-form', instanceId: 'form',
    templateArtifactId: 'template', dataRevision: revision, outputTarget: 'light-dom', scopePolicyStamp: 'policy',
    nodes: [element('form', 'form', {}, [element('input', 'input', { value }),
        element('textarea', 'textarea', {}, [text('textarea-text', value)]),
        element('input', 'check', { type: 'checkbox' }),
        element('select', 'select', {}, ['a', 'b'].map(v => element('option', v, { value: v }, [text(`${v}-text`, v)]))),
        element('div', 'editor', { contenteditable: 'true' }, [text('editor-text', value)]),
        element('p', 'label', {}, [text('label-text', revision)])])] });
function retainedControls(mode: 'plan' | 'patch'): Story {
    return { render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        let previous: RenderPlan | null = null;
        const publish = (next: RenderPlan) => {
            if (mode === 'plan') applyRenderPlanToRange(bounds, next, document);
            else expect(preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(previous, next), renderPlanIdentity(next), document)
                .commit(renderPlanIdentity(next)).status).toBe('applied');
            previous = structuredClone(next);
        };
        publish(plan('1'));
        const mutations = new MutationObserver(() => undefined);
        mutations.observe(host, { subtree: true, attributes: true, childList: true, characterData: true });
        publish(plan('1'));
        expect(mutations.takeRecords()).toEqual([]);
        mutations.disconnect();
        const nodes = Array.from(host.querySelectorAll('input,textarea,select,[contenteditable]'));
        const [input, area, check, select, editor] = nodes as [HTMLInputElement, HTMLTextAreaElement, HTMLInputElement, HTMLSelectElement, HTMLElement];
        input.value = 'User text'; area.value = 'User area'; check.checked = true; select.value = 'b';
        editor.firstChild!.textContent = 'User editor';
        area.focus(); area.setSelectionRange(2, 6, 'backward');
        for (let i = 2; i < 5; i++) publish(plan(String(i)));
        expect(Array.from(host.querySelectorAll('input,textarea,select,[contenteditable]'))).toEqual(nodes);
        expect([input.value, area.value, check.checked, select.value, editor.textContent]).toEqual(['User text', 'User area', true, 'b', 'User editor']);
        expect(document.activeElement).toBe(area);
        expect([area.selectionStart, area.selectionEnd, area.selectionDirection]).toEqual([2, 6, 'backward']);
        editor.focus();
        const selection = document.getSelection()!;
        selection.setBaseAndExtent(editor.firstChild!, 1, editor.firstChild!, 5);
        publish(plan('5'));
        expect(document.activeElement).toBe(editor);
        expect(selection.toString()).toBe('ser ');
        publish(plan('6', 'Model update'));
        expect([input.value, area.value, editor.textContent]).toEqual(['Model update', 'Model update', 'Model update']);
        expect(host.querySelector('input')).toBe(input);
        host.querySelector('form')!.reset();
        expect([input.value, area.value, check.checked, select.value]).toEqual(['Model update', 'Model update', false, 'a']);
    } };
}
export const DirectProjection = retainedControls('plan');
export const PatchProjection = retainedControls('patch');
export const CompleteOwnershipAndRevision: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('1');
        for (const changed of [{ instanceId: 'other' }, { scopePolicyStamp: 'other' }, { templateArtifactId: 'other' },
            { producedTag: 'other-form' }, { ariaReferenceProfile: EXPERIMENTAL_ARIA_REFERENCE_PROFILE }]) {
            applyRenderPlanToRange(bounds, before, document);
            const input = host.querySelector('input')!; input.value = 'User edit'; input.focus();
            const next = { ...before, ...changed };
            expect(diffRenderPlansToPatchFrames(before, next).flatMap(f => f.type === 'ops' ? f.ops : []).some(op => op.op === 'replaceScope')).toBe(true);
            applyRenderPlanToRange(bounds, next, document);
            expect(host.querySelector('input')).not.toBe(input);
            expect(host.querySelector('input')!.value).toBe('Original');
            expect(document.activeElement).not.toBe(host.querySelector('input'));
        }
        const a = { ...renderPlanIdentity(before), instanceId: 'a:b', dataRevision: 'c' };
        const b = { ...a, instanceId: 'a', dataRevision: 'b:c' };
        expect(renderRevisionKey(a)).not.toBe(renderRevisionKey(b));
        const frames = diffRenderPlansToPatchFrames(null, { ...before, ...a });
        expect(preparePatchFramesForRange(bounds, frames, a, document).commit(b).status).toBe('stale');
    },
};

export const ConsumedTransactionsReleaseInputs: Story = {
    render: () => '<section></section>', play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { cdp } = await import('vitest/browser');
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('1'), after = plan('2');
        applyRenderPlanToRange(bounds, before, document);
        const prepare = (cancel: boolean) => {
            const witness = { node: document.createElement('aside') };
            const transaction = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), renderPlanIdentity(after), document,
                { preserveNode: node => node === witness.node });
            if (cancel) transaction.cancel();
            else expect(transaction.commit(renderPlanIdentity(after)).status).toBe('applied');
            return { transaction, witness: new WeakRef(witness) };
        };
        const committed = prepare(false), cancelled = prepare(true);
        await new Promise(resolve => setTimeout(resolve, 0));
        await cdp().send('HeapProfiler.collectGarbage');
        expect(committed.witness.deref() === undefined).toBe(true);
        expect(cancelled.witness.deref() === undefined).toBe(true);
        expect(committed.transaction.check(renderPlanIdentity(after)).status).toBe('aborted');
        expect(cancelled.transaction.check(renderPlanIdentity(after)).status).toBe('aborted');
    },
};

export const PendingCandidatesPreserveLatestEdits: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('1'), after = plan('2');
        applyRenderPlanToRange(bounds, before, document);
        const area = host.querySelector('textarea')!;
        const candidate = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), renderPlanIdentity(after), document);
        area.value = 'Typed while pending'; area.focus(); area.setSelectionRange(3, 8);
        expect(candidate.commit(renderPlanIdentity(after)).status).toBe('applied');
        expect(host.querySelector('textarea')).toBe(area);
        expect(area.value).toBe('Typed while pending');
        expect(document.activeElement).toBe(area);
        expect([area.selectionStart, area.selectionEnd]).toEqual([3, 8]);
        const changed = plan('3', 'Cancelled model');
        const cancelled = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(after, changed), renderPlanIdentity(changed), document);
        cancelled.cancel();
        expect(cancelled.commit(renderPlanIdentity(changed)).status).toBe('aborted');
        const stale = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(after, changed), renderPlanIdentity(changed), document);
        expect(stale.commit({ ...renderPlanIdentity(changed), renderAttempt: 1 }).status).toBe('stale');
        expect(area.value).toBe('Typed while pending');
        expect([area.selectionStart, area.selectionEnd]).toEqual([3, 8]);
    },
};

export const CustomInputsKeepFreshProvenance: Story = {
    render: () => '<section></section>', play: ({ canvasElement }) => {
        const tag = 'reuse-authored-inputs';
        if (!customElements.get(tag)) customElements.define(tag, class extends HTMLElement {});
        const host = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('1');
        const child = element('p', 'paragraph', {}, [text('text', 'Authored input')]);
        child.sourceMapRef = { fidelity: 'author-byte-exact', frame: 'before' };
        before.nodes = [element(tag, 'custom', {}, [child])];
        applyRenderPlanToRange(bounds, before, document);
        const paragraph = host.querySelector('p')!;
        const after = structuredClone(before);
        const custom = after.nodes[0];
        if (custom.kind !== 'element') throw new Error('missing custom input');
        custom.children[0].sourceMapRef = { fidelity: 'author-byte-exact', frame: 'after' };
        applyRenderPlanToRange(bounds, after, document);
        expect(host.querySelector('p')).toBe(paragraph);
        expect(paragraph.getAttribute('data-cem-source-frame')).toBe('after');
    },
};
