import { expect, it } from 'vitest';
import { createCemNativeSuggestionsPublication, type CemNativeSuggestionsComponentFrame } from './native-suggestions-publication.js';
import type { CemProcessingRenderDiffResult } from './internal/runtime-support/processing-host.js';

it('keeps external native consumers enrolled through revoked in-flight frames and retires settled bindings', async () => {
    let enrolled = 0, finish!: (result: CemProcessingRenderDiffResult) => void;
    const publication = createCemNativeSuggestionsPublication({ query: '', queryRevision: 1 }, () => true,
        async () => { throw new Error('unused source operation'); }, async () => undefined, 'publication',
        () => new Promise(resolve => { finish = resolve; }), () => { throw new Error('unused row source'); }, undefined, () => {
            enrolled++; let active = true;
            return () => { if (active) { active = false; enrolled--; } };
        });
    const consumer = { instanceId: 'editor', revision: '1', scopePolicyStamp: 'policy', current: () => true };
    const binding = publication.bind(consumer);
    const frame = { compile: { scopePolicyStamp: 'policy', templateArtifactId: 'template' }, render: {
        revision: { instanceId: 'editor', dataRevision: '1', scopePolicyStamp: 'policy', templateArtifactId: 'template' },
    } } as CemNativeSuggestionsComponentFrame;
    expect(enrolled).toBe(1);
    const pending = binding.renderComponent(frame); expect(enrolled).toBe(2);
    binding.release(); expect(enrolled).toBe(1);
    finish({ suggestionPlacements: [] } as unknown as CemProcessingRenderDiffResult);
    await expect(pending).rejects.toThrow('superseded'); expect(enrolled).toBe(0);
    const next = publication.bind(consumer); expect(enrolled).toBe(1);
    await publication.release(); expect(enrolled).toBe(0);
    next.release(); next.release(); expect(enrolled).toBe(0);
});
