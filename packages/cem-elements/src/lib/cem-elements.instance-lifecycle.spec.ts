import { describe, expect, it } from 'vitest';

import { analyzeInstanceLifecycleShape } from './cem-elements.js';

describe('cem-element instance lifecycle shape', () => {
    it('distinguishes live payload, an explicit payload envelope, and resume state', () => {
        expect(
            analyzeInstanceLifecycleShape({
                markedIslandCount: 0,
                unmarkedTemplateCount: 0,
                meaningfulChildCount: 2,
            }),
        ).toEqual({ mode: 'capture-live-payload', diagnostics: [] });

        expect(
            analyzeInstanceLifecycleShape({
                markedIslandCount: 0,
                unmarkedTemplateCount: 1,
                meaningfulChildCount: 1,
            }),
        ).toEqual({ mode: 'capture-template-payload', diagnostics: [] });

        expect(
            analyzeInstanceLifecycleShape({
                markedIslandCount: 1,
                unmarkedTemplateCount: 1,
                meaningfulChildCount: 4,
            }),
        ).toEqual({ mode: 'resume-island', diagnostics: [] });
    });

    it('rejects ambiguous first-load payload and duplicate islands', () => {
        expect(
            analyzeInstanceLifecycleShape({
                markedIslandCount: 0,
                unmarkedTemplateCount: 1,
                meaningfulChildCount: 2,
            }),
        ).toEqual({
            mode: 'invalid',
            diagnostics: ['cem-element.instance_payload_mixed'],
        });

        expect(
            analyzeInstanceLifecycleShape({
                markedIslandCount: 2,
                unmarkedTemplateCount: 0,
                meaningfulChildCount: 2,
            }),
        ).toEqual({
            mode: 'invalid',
            diagnostics: ['cem-element.instance_island_count'],
        });
    });

    it('never reclassifies siblings as payload after a marked island exists', () => {
        const result = analyzeInstanceLifecycleShape({
            markedIslandCount: 1,
            unmarkedTemplateCount: 0,
            meaningfulChildCount: 12,
        });

        expect(result.mode).toBe('resume-island');
        expect(result.diagnostics).toEqual([]);
    });
});

