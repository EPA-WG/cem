import { describe, expect, it } from 'vitest';

import {
    CEM_DECLARATION_REGISTRATION_CONTRACT,
    analyzeDeclarationRegistration,
    analyzeDeclarationRegistrationIdentity,
    analyzeDeclarationShape,
} from './cem-elements.js';

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

describe('cem-element declaration registration contract', () => {
    it('content-addresses source, language, tag, and an explicit browser behavior identity', () => {
        const base = {
            tag: 'cem-button',
            resolvedTemplateSource: '{button | Save}',
            templateLanguage: 'cem-ml' as const,
            hasBehavior: false,
        };
        const identity = analyzeDeclarationRegistrationIdentity(base);

        expect(identity.diagnostics).toEqual([]);
        expect(identity.registrationIdentity).toMatch(/^cem-registration-v1:/);
        expect(analyzeDeclarationRegistrationIdentity(base)).toEqual(identity);

        const identities = [
            identity.registrationIdentity,
            analyzeDeclarationRegistrationIdentity({ ...base, tag: 'cem-link' }).registrationIdentity,
            analyzeDeclarationRegistrationIdentity({
                ...base,
                resolvedTemplateSource: '{button | Delete}',
            }).registrationIdentity,
            analyzeDeclarationRegistrationIdentity({ ...base, templateLanguage: 'dom' }).registrationIdentity,
            analyzeDeclarationRegistrationIdentity({
                ...base,
                hasBehavior: true,
                behaviorIdentity: 'button-behavior-v1',
            }).registrationIdentity,
            analyzeDeclarationRegistrationIdentity({
                ...base,
                hasBehavior: true,
                behaviorIdentity: 'button-behavior-v2',
            }).registrationIdentity,
        ];
        expect(new Set(identities).size).toBe(identities.length);
    });

    it('requires a non-empty host identity for behavior-bearing declarations', () => {
        const missing = analyzeDeclarationRegistrationIdentity({
            tag: 'cem-button',
            resolvedTemplateSource: '{button | Save}',
            templateLanguage: 'cem-ml',
            hasBehavior: true,
        });
        expect(missing.registrationIdentity).toBeNull();
        expect(codes(missing)).toEqual(['cem-element.behavior_identity_required']);

        const blank = analyzeDeclarationRegistrationIdentity({
            tag: 'cem-button',
            resolvedTemplateSource: '{button | Save}',
            templateLanguage: 'cem-ml',
            hasBehavior: true,
            behaviorIdentity: '   ',
        });
        expect(blank.registrationIdentity).toBeNull();
        expect(codes(blank)).toEqual(['cem-element.behavior_identity_required']);
    });

    it('separates scoped inherited declaration lookup from document-global browser registration', () => {
        expect(CEM_DECLARATION_REGISTRATION_CONTRACT).toEqual({
            logicalRegistry: 'scoped-inherited',
            browserRegistry: 'document-global',
            scopedBrowserRegistryRequired: false,
            publicTagUniqueness: 'document-global',
            sameScopeDuplicate: 'error',
            compatibleInheritedDeclaration: 'reuse',
            incompatibleInheritedDeclaration: 'error',
            incompatibleBrowserDefinition: 'error',
        });

        expect(
            analyzeDeclarationRegistration({
                tag: 'cem-button',
                registrationIdentity: 'sha256:button-v1',
            }),
        ).toEqual({
            action: 'define-browser-tag',
            diagnostics: [],
        });

        // The `cem-` prefix is reserved for @epa-wg/cem-components by its
        // package contract, not required of every third-party declaration.
        expect(
            analyzeDeclarationRegistration({
                tag: 'story-button',
                registrationIdentity: 'sha256:story-button-v1',
            }).action,
        ).toBe('define-browser-tag');
    });

    it('rejects same-scope duplicates even when their registration identities match', () => {
        const result = analyzeDeclarationRegistration({
            tag: 'cem-button',
            registrationIdentity: 'sha256:button-v1',
            sameScope: { registrationIdentity: 'sha256:button-v1' },
        });

        expect(result.action).toBe('reject');
        expect(codes(result)).toEqual(['cem-element.registry_same_scope_duplicate']);
    });

    it('reuses an identical inherited declaration without defining the browser tag again', () => {
        const result = analyzeDeclarationRegistration({
            tag: 'cem-button',
            registrationIdentity: 'sha256:button-v1',
            inherited: { registrationIdentity: 'sha256:button-v1' },
            browser: {
                owner: 'cem-element',
                registrationIdentity: 'sha256:button-v1',
            },
        });

        expect(result).toEqual({
            action: 'reuse-inherited',
            diagnostics: [],
        });
    });

    it('rejects incompatible inherited shadowing before browser mutation', () => {
        const result = analyzeDeclarationRegistration({
            tag: 'cem-button',
            registrationIdentity: 'sha256:button-v2',
            inherited: { registrationIdentity: 'sha256:button-v1' },
            browser: {
                owner: 'cem-element',
                registrationIdentity: 'sha256:button-v1',
            },
        });

        expect(result.action).toBe('reject');
        expect(codes(result)).toEqual(['cem-element.registry_inherited_collision']);
    });

    it.each(['legacy-custom-element', 'foreign'] as const)(
        'rejects a document-global browser tag owned by %s',
        (owner) => {
            const result = analyzeDeclarationRegistration({
                tag: 'cem-button',
                registrationIdentity: 'sha256:button-v1',
                browser: { owner },
            });

            expect(result.action).toBe('reject');
            expect(codes(result)).toEqual(['cem-element.browser_tag_collision']);
        },
    );

    it('reuses only the same CEM registration identity across runtime instances', () => {
        expect(
            analyzeDeclarationRegistration({
                tag: 'cem-button',
                registrationIdentity: 'sha256:button-v1',
                browser: {
                    owner: 'cem-element',
                    registrationIdentity: 'sha256:button-v1',
                },
            }),
        ).toEqual({
            action: 'reuse-browser-tag',
            diagnostics: [],
        });

        const incompatible = analyzeDeclarationRegistration({
            tag: 'cem-button',
            registrationIdentity: 'sha256:button-v2',
            browser: {
                owner: 'cem-element',
                registrationIdentity: 'sha256:button-v1',
            },
        });
        expect(incompatible.action).toBe('reject');
        expect(codes(incompatible)).toEqual(['cem-element.browser_tag_collision']);
    });
});

function codes(result: { diagnostics: Array<{ code: string }> }): string[] {
    return result.diagnostics.map((diagnostic) => diagnostic.code);
}
