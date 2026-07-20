import { describe, expect, it } from 'vitest';

import { analyzeDeclarationShape } from './cem-elements.js';

describe('cem-element declaration shape contract', () => {
    it('accepts exactly one inline template with no live declaration content', () => {
        const result = analyzeDeclarationShape({
            tag: 'story-inline-shape',
            src: null,
            directTemplateCount: 1,
            directLiveNodeCount: 0,
        });

        expect(result.ok).toBe(true);
        expect(result.tag).toBe('story-inline-shape');
        expect(result.src).toBeNull();
        expect(result.diagnostics).toEqual([]);
    });

    it('keeps src declarations exclusive from inline templates', () => {
        const srcOnly = analyzeDeclarationShape({
            tag: 'story-src-shape',
            src: './templates.html#story-src-shape',
            directTemplateCount: 0,
            directLiveNodeCount: 0,
        });
        expect(srcOnly.ok).toBe(true);
        expect(srcOnly.src).toBe('./templates.html#story-src-shape');

        const conflict = analyzeDeclarationShape({
            tag: 'story-src-conflict',
            src: './templates.html#story-src-conflict',
            directTemplateCount: 1,
            directLiveNodeCount: 0,
        });
        expect(codes(conflict)).toEqual(['cem-element.src_inline_template_conflict']);
        expect(conflict.diagnostics[0]?.sourceMapRef).toEqual({
            fidelity: 'declaration-only',
            frame: 'decl:story-src-conflict',
        });
    });

    it('rejects missing, duplicate, and live-content inline declaration shapes', () => {
        expect(
            codes(
                analyzeDeclarationShape({
                    tag: 'story-missing-template',
                    src: null,
                    directTemplateCount: 0,
                    directLiveNodeCount: 0,
                }),
            ),
        ).toEqual(['cem-element.inline_template_missing']);

        expect(
            codes(
                analyzeDeclarationShape({
                    tag: 'story-duplicate-template',
                    src: null,
                    directTemplateCount: 2,
                    directLiveNodeCount: 0,
                }),
            ),
        ).toEqual(['cem-element.inline_template_count']);

        expect(
            codes(
                analyzeDeclarationShape({
                    tag: 'story-live-content',
                    src: null,
                    directTemplateCount: 1,
                    directLiveNodeCount: 1,
                }),
            ),
        ).toEqual(['cem-element.declaration_live_content']);
    });

    it('records tag diagnostics before inline shape diagnostics', () => {
        const result = analyzeDeclarationShape({
            tag: 'Bad-Tag',
            src: null,
            directTemplateCount: 0,
            directLiveNodeCount: 1,
        });

        expect(codes(result)).toEqual([
            'cem-element.tag_invalid',
            'cem-element.inline_template_missing',
            'cem-element.declaration_live_content',
        ]);
        expect(result.ok).toBe(false);
        expect(result.diagnostics[0]?.sourceMapRef).toEqual({
            fidelity: 'declaration-only',
            frame: 'decl:Bad-Tag',
        });
    });
});

function codes(result: ReturnType<typeof analyzeDeclarationShape>): string[] {
    return result.diagnostics.map((diagnostic) => diagnostic.code);
}
