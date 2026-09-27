import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { applyRenderPlanToRange, preparePatchFramesForRange, diffRenderPlansToPatchFrames,
    renderPlanIdentity, type RenderPlan } from './projection.js';

export default { title: 'CEM Elements/Prepared Patch Transactions', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const plan = (text: string, revision: string): RenderPlan => ({ producedTag: 'prepared-card', instanceId: 'card',
    templateArtifactId: 'template', dataRevision: revision, outputTarget: 'light-dom', scopePolicyStamp: 'policy',
    nodes: [{ kind: 'element', namespace: null, tag: 'p', attributes: [], renderNodeId: 'paragraph',
        children: [{ kind: 'text', text, renderNodeId: 'text' }] }] });

export const AdmissionAndPublication: Story = {
    render: () => '<section></section>',
    play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section');
        if (!host) throw new Error('missing patch fixture');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('Before', '1'); const after = plan('After', '2');
        const revision = renderPlanIdentity(after);
        const reset = () => applyRenderPlanToRange(bounds, before, document);
        const prepare = () => preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), revision, document);
        reset();
        const paragraph = host.querySelector('p');
        const frames = diffRenderPlansToPatchFrames(before, after);
        const candidate = preparePatchFramesForRange(bounds, frames, revision, document);
        frames.splice(0); // The transaction owns a snapshot of transport frames.
        expect(candidate.check(revision).status).toBe('ready');
        expect(host.textContent).toBe('Before');
        expect(candidate.commit(revision).status).toBe('applied');
        expect(host.textContent).toBe('After');
        expect(host.querySelector('p')).toBe(paragraph);
        expect(candidate.commit(revision).status).toBe('aborted');
        reset();
        const stale = prepare();
        expect(stale.commit(renderPlanIdentity(plan('Later', '3'))).status).toBe('stale');
        expect(host.textContent).toBe('Before');
        expect(stale.commit(revision).status).toBe('aborted');
        const cancelled = prepare(); cancelled.cancel();
        expect(cancelled.check(revision).status).toBe('aborted');
        expect(cancelled.commit(revision).status).toBe('aborted');
        expect(host.textContent).toBe('Before');
        const incomplete = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after).slice(0, -1), revision, document);
        expect(incomplete.commit(revision).status).toBe('aborted');
        expect(host.textContent).toBe('Before');
    },
};

export const RevalidateRangeAndTargets: Story = {
    render: () => '<section></section>',
    play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section');
        if (!host) throw new Error('missing patch fixture');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('Before', '1'); const after = plan('After', '2');
        const revision = renderPlanIdentity(after);
        const reset = () => applyRenderPlanToRange(bounds, before, document);
        const frames = () => diffRenderPlansToPatchFrames(before, after, { textNodeIdsAvailable: false });
        reset();
        const changed = preparePatchFramesForRange(bounds, frames(), revision, document);
        const paragraph = host.querySelector('p');
        if (!paragraph) throw new Error('missing paragraph');
        paragraph.replaceWith(paragraph.cloneNode(true)); // Same serialized ID, different DOM owner.
        expect(changed.commit(revision).status).toBe('aborted');
        expect(host.textContent).toBe('Before');
        reset();
        const moved = preparePatchFramesForRange(bounds, frames(), revision, document);
        const other = document.createElement('div'); host.append(other);
        while (bounds.start.nextSibling && bounds.start.nextSibling !== other) other.append(bounds.start.nextSibling);
        other.prepend(bounds.start);
        expect(moved.commit(revision).status).toBe('aborted');
        expect(other.textContent).toBe('Before');
        host.append(bounds.start, ...Array.from(other.childNodes).filter(node => node !== bounds.start), bounds.end);
        other.remove(); reset();
        const reversed = preparePatchFramesForRange({ start: bounds.end, end: bounds.start }, frames(), revision, document);
        expect(reversed.commit(revision).status).toBe('aborted');
        const removed = preparePatchFramesForRange(bounds, frames(), revision, document);
        bounds.end.remove();
        expect(removed.commit(revision).status).toBe('aborted');
        expect(host.textContent).toBe('Before');
    },
};

export const ScopeReplacementAdmission: Story = {
    render: () => '<section></section>',
    play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('section');
        if (!host) throw new Error('missing replacement fixture');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const before = plan('Before', '1');
        const after = { ...plan('After', '2'), templateArtifactId: 'new-template' };
        applyRenderPlanToRange(bounds, before, document);
        const frames = diffRenderPlansToPatchFrames(before, after);
        const revision = renderPlanIdentity(after);
        expect(frames.flatMap(frame => frame.type === 'ops' ? frame.ops : []).some(op => op.op === 'replaceScope')).toBe(true);
        const foreign = preparePatchFramesForRange(bounds, frames, revision, document.implementation.createHTMLDocument());
        expect(foreign.commit(revision).status).toBe('aborted');
        const mixed = structuredClone(frames);
        const batch = mixed.find(frame => frame.type === 'ops');
        if (!batch || batch.type !== 'ops') throw new Error('missing operation batch');
        batch.ops.push({ op: 'setText', target: { id: 'text' }, value: 'Partial' });
        expect(preparePatchFramesForRange(bounds, mixed, revision, document).commit(revision).status).toBe('aborted');
        expect(host.textContent).toBe('Before');
        const candidate = preparePatchFramesForRange(bounds, frames, revision, document);
        expect(candidate.check(revision).status).toBe('ready');
        expect(host.textContent).toBe('Before');
        expect(candidate.commit(revision).status).toBe('applied');
        expect(host.textContent).toBe('After');
    },
};
