import { describe, expect, it } from 'vitest';

import { generateScopeUid } from './cem-elements.js';
import {
    diffRenderPlansToPatchFrames,
    projectTemplate,
    resolveDeclarationStyleScope,
    resolveDeclarationStylesheetScopes,
    renderPlansHaveDomChanges,
    renderInstanceScopeUid,
    scopeCssText,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type RenderPlan,
} from './projection.js';
import {
    PROCESSING_BOUNDARY_TEMPLATE_SOURCE as TEMPLATE_SOURCE,
    processingBoundarySnapshotFixture as snapshotFixture,
} from './processing-boundary.fixtures.js';

describe('host processing boundary contracts', () => {
    it('resolves valid, absent, and invalid declaration style scope names', () => {
        expect(resolveDeclarationStyleScope(false, null)).toEqual({
            scope: null,
            valid: true,
        });
        expect(resolveDeclarationStyleScope(true, '  abc-lib  ')).toEqual({
            scope: 'abc-lib',
            valid: true,
        });
        for (const invalid of ['', '   ', 'abc lib', '1abc', 'abc.lib', '{$scope}']) {
            expect(resolveDeclarationStyleScope(true, invalid)).toEqual({
                scope: null,
                valid: false,
            });
        }
    });

    it.each([
        {
            name: 'unscoped bare style is private',
            declarationScope: null,
            stylesheetScopes: [null],
            expected: [{ kind: 'private', scope: null }],
        },
        {
            name: 'unscoped explicit style is invalid',
            declarationScope: null,
            stylesheetScopes: ['abc-lib'],
            expected: [{ kind: 'invalid', scope: 'abc-lib' }],
        },
        {
            name: 'scoped bare-only styles use the shared shorthand',
            declarationScope: 'abc-lib',
            stylesheetScopes: [null, null],
            expected: [
                { kind: 'shared', scope: 'abc-lib' },
                { kind: 'shared', scope: 'abc-lib' },
            ],
        },
        {
            name: 'scoped explicit-only style is shared',
            declarationScope: 'abc-lib',
            stylesheetScopes: ['abc-lib'],
            expected: [{ kind: 'shared', scope: 'abc-lib' }],
        },
        {
            name: 'matching explicit style makes coexisting bare styles private',
            declarationScope: 'abc-lib',
            stylesheetScopes: [null, 'abc-lib', null],
            expected: [
                { kind: 'private', scope: null },
                { kind: 'shared', scope: 'abc-lib' },
                { kind: 'private', scope: null },
            ],
        },
        {
            name: 'mismatched explicit style is invalid without changing bare shorthand',
            declarationScope: 'abc-lib',
            stylesheetScopes: [null, 'other-lib'],
            expected: [
                { kind: 'shared', scope: 'abc-lib' },
                { kind: 'invalid', scope: 'other-lib' },
            ],
        },
        {
            name: 'blank explicit style is invalid',
            declarationScope: 'abc-lib',
            stylesheetScopes: ['   '],
            expected: [{ kind: 'invalid', scope: '' }],
        },
    ])('$name', ({ declarationScope, stylesheetScopes, expected }) => {
        expect(resolveDeclarationStylesheetScopes(declarationScope, stylesheetScopes)).toEqual(expected);
    });

    it('generates scope UIDs from explicit seeds, blank seeds, and dynamic fallback seeds', () => {
        expect(
            generateScopeUid({
                producedTag: 'story-card',
                uidSeed: 'demo/scoped-css/card',
                occurrencePath: '0.2',
            }),
        ).toBe('cem-scope-story-card-udemoz2fscoped-cssz2fcard-p0-2');

        expect(
            generateScopeUid({
                producedTag: 'story-card',
                uidSeed: '',
                occurrencePath: '0.2',
                runtimeSeed: 'ignored',
            }),
        ).toBe('cem-scope-story-card-p0-2');

        expect(
            generateScopeUid({
                producedTag: 'story-card',
                uidSeed: null,
                occurrencePath: '0.2',
                runtimeSeed: 'worker-0-counter-7',
            }),
        ).toBe('cem-scope-story-card-uworker-0-counter-7-p0-2');
    });

    it('covers the accepted UID matrix for stable persisted output', () => {
        const stableInput = {
            producedTag: 'story-card',
            uidSeed: 'docs/public seed: card',
            occurrencePath: '0.2.1',
        };
        const first = generateScopeUid(stableInput);
        const repeated = generateScopeUid(stableInput);
        expect(repeated).toBe(first);
        expect(first).toBe('cem-scope-story-card-udocsz2fpublicz20seedz3az20card-p0-2-1');
        expect(first).not.toMatch(/[ /:@]/);

        expect(
            generateScopeUid({
                ...stableInput,
                occurrencePath: '0.2.2',
            }),
        ).not.toBe(first);

        expect(
            generateScopeUid({
                ...stableInput,
                runtimeSeed: 'worker-17-counter-99',
            }),
        ).toBe(first);

        const scheduledOutOfOrder = [
            generateScopeUid({ ...stableInput, occurrencePath: '0.3' }),
            generateScopeUid({ ...stableInput, occurrencePath: '0.1' }),
            generateScopeUid({ ...stableInput, occurrencePath: '0.2' }),
        ].sort();
        const scheduledInOrder = ['0.1', '0.2', '0.3']
            .map((occurrencePath) => generateScopeUid({ ...stableInput, occurrencePath }))
            .sort();
        expect(scheduledOutOfOrder).toEqual(scheduledInOrder);
    });

    it('rewrites scoped CSS with nesting, host aliases, keyframes, and suppressed globals', () => {
        const result = scopeCssText(
            [
                '@import url("./theme.css");',
                '@font-face { font-family: Demo; src: url("./demo.woff2"); }',
                ':host { display: block; }',
                ':global(.legacy) button, :root { color: red; }',
                '@keyframes pulse { from { opacity: 0; } to { opacity: 1; } }',
                'button { animation: pulse 1s ease; animation-name: pulse; }',
            ].join('\n'),
            'cem-scope-story-card-useed-p0',
        );

        expect(result.css).toContain(':where([data-cem-scope="cem-scope-story-card-useed-p0"]) {');
        expect(result.css).toContain('& { display: block; }');
        expect(result.css).toContain('&.legacy button, & { color: red; }');
        expect(result.css).toContain('@keyframes pulse-cem-scope-story-card-useed-p0');
        expect(result.css).toContain('animation: pulse-cem-scope-story-card-useed-p0 1s ease');
        expect(result.css).toContain('animation-name: pulse-cem-scope-story-card-useed-p0');
        expect(result.css).not.toContain('@import');
        expect(result.css).not.toContain('@font-face');
        expect(result.diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
            'cem.scoped_css.import_unsupported',
            'cem.scoped_css.global_construct_unsupported',
            'cem.scoped_css.global_alias',
        ]);
    });

    it('uses an explicit zero-specificity private boundary and suppresses every global at-rule', () => {
        const result = scopeCssText(
            [
                ':host([invalid]) { color: red; }',
                '@font-face { font-family: Demo; src: url(demo.woff2); }',
                '@property --demo { syntax: "<color>"; inherits: true; initial-value: red; }',
                '@counter-style demo { system: cyclic; symbols: x; }',
                '@font-palette-values --demo { font-family: Demo; }',
                '@page { margin: 1cm; }',
                '@namespace svg url(http://www.w3.org/2000/svg);',
            ].join('\n'),
            'private-style-id',
            { boundarySelector: ':where(cem-select)' },
        );

        expect(result.css).toContain(':where(cem-select) {');
        expect(result.css).toContain('&[invalid] { color: red; }');
        for (const atRule of ['font-face', 'property', 'counter-style', 'font-palette-values', 'page', 'namespace']) {
            expect(result.css).not.toContain(`@${atRule}`);
        }
        expect(
            result.diagnostics.filter(
                (diagnostic) => diagnostic.code === 'cem.scoped_css.global_construct_unsupported',
            ),
        ).toHaveLength(6);
    });

    it('stamps render identity separately and uses a zero-specificity tag boundary', () => {
        const plan = projectTemplate(
            [
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'section',
                    attributes: [],
                    children: [
                        {
                            kind: 'element',
                            namespace: null,
                            tag: 'style',
                            attributes: [],
                            children: [{ kind: 'text', text: 'button { color: green; }' }],
                        },
                        {
                            kind: 'element',
                            namespace: null,
                            tag: 'button',
                            attributes: [],
                            children: [{ kind: 'text', text: 'Save' }],
                        },
                    ],
                },
            ],
            { snapshot: snapshotFixture(), values: {} },
        );
        const scoped = scopeRenderPlan(plan, 'cem-scope-story-card-useed-p0');
        const root = scoped.renderPlan.nodes[0];
        expect(root.kind).toBe('element');
        if (root.kind !== 'element') return;

        expect(root.attributes).toContainEqual({
            name: 'data-cem-render-scope',
            value: 'cem-scope-story-card-useed-p0',
        });
        const style = root.children[0];
        expect(style.kind).toBe('element');
        if (style.kind !== 'element') return;
        expect(style.children[0]).toEqual({
            kind: 'text',
            text: ':where(boundary-card) {\n    button { color: green; }\n}',
            sourceMapRef: undefined,
        });
    });

    it('rewrites payload style nodes against an instance scope', () => {
        const plan: RenderPlan = {
            producedTag: 'story-card',
            instanceId: 'cem-instance-7',
            templateArtifactId: 'template-artifact-payload-style',
            dataRevision: '1',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'boundary-scope',
            nodes: [
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'button',
                    renderNodeId: 'story-card-1',
                    attributes: [],
                    children: [
                        {
                            kind: 'element',
                            namespace: null,
                            tag: 'style',
                            renderNodeId: 'payload-0',
                            attributes: [],
                            children: [{ kind: 'text', text: 'button { border-color: red; }' }],
                        },
                    ],
                },
            ],
        };
        const scopeUid = 'cem-scope-story-card-useed-p0';
        const instanceScopeUid = renderInstanceScopeUid(scopeUid, plan.instanceId);
        const scoped = scopeRenderPlan(plan, scopeUid, { instanceScopeUid });
        const root = scoped.renderPlan.nodes[0];
        expect(root.kind).toBe('element');
        if (root.kind !== 'element') return;
        const style = root.children[0];
        expect(style.kind).toBe('element');
        if (style.kind !== 'element') return;
        expect(style.children[0]).toEqual({
            kind: 'text',
            text: `story-card[data-cem-instance-scope="${instanceScopeUid}"] {\n    button { border-color: red; }\n}`,
            sourceMapRef: undefined,
        });
    });

    it('diagnoses duplicate generated render-plan and stylesheet IDs', () => {
        const plan: RenderPlan = {
            producedTag: 'story-card',
            instanceId: 'cem-instance-1',
            templateArtifactId: 'template-artifact-1',
            dataRevision: '1',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'boundary-scope',
            nodes: [
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'style',
                    renderNodeId: 'story-card-style',
                    attributes: [],
                    children: [{ kind: 'text', text: 'button { color: red; }' }],
                },
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'section',
                    renderNodeId: 'story-card-section',
                    attributes: [],
                    children: [
                        {
                            kind: 'element',
                            namespace: null,
                            tag: 'style',
                            renderNodeId: 'story-card-style',
                            attributes: [],
                            children: [{ kind: 'text', text: 'button { color: blue; }' }],
                        },
                    ],
                },
            ],
        };

        expect(validateRenderPlanGeneratedIds(plan).map((diagnostic) => diagnostic.code)).toEqual([
            'cem.render_plan.generated_render_node_id_duplicate',
            'cem.render_plan.generated_stylesheet_id_duplicate',
        ]);
    });

    it('detects visible render-plan DOM changes while ignoring revision-only changes', () => {
        const first = projectTemplate(TEMPLATE_SOURCE, {
            snapshot: snapshotFixture(),
            values: { label: 'Projected' },
        });
        const revisionOnly: RenderPlan = {
            ...first,
            dataRevision: '2',
        };
        expect(renderPlansHaveDomChanges(first, revisionOnly)).toBe(false);

        const changedAttribute = projectTemplate(TEMPLATE_SOURCE, {
            snapshot: snapshotFixture(),
            values: { label: 'Updated' },
        });
        expect(renderPlansHaveDomChanges(first, changedAttribute)).toBe(true);

        const changedText: RenderPlan = {
            ...first,
            nodes: [
                {
                    ...(first.nodes[0] as Extract<RenderPlan['nodes'][number], { kind: 'element' }>),
                    children: [{ kind: 'text', text: 'Updated' }],
                },
            ],
        };
        expect(renderPlansHaveDomChanges(first, changedText)).toBe(true);
    });

    it('reconciles conditional children without replacing a stable native sibling', () => {
        const input = {
            kind: 'element' as const,
            namespace: null,
            tag: 'input',
            renderNodeId: 'behavior-field-input',
            attributes: [{ name: 'aria-expanded', value: 'false' }],
            children: [],
        };
        const help = {
            kind: 'element' as const,
            namespace: null,
            tag: 'span',
            renderNodeId: 'behavior-field-help',
            attributes: [{ name: 'class', value: 'help' }],
            children: [{ kind: 'text' as const, text: 'Help' }],
        };
        const previous: RenderPlan = {
            producedTag: 'behavior-field',
            instanceId: 'behavior-field-1',
            templateArtifactId: 'behavior-field-template-1',
            dataRevision: '1',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'boundary-scope',
            nodes: [
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'div',
                    renderNodeId: 'behavior-field-root',
                    attributes: [],
                    children: [input, help],
                },
            ],
        };
        const next: RenderPlan = {
            ...previous,
            dataRevision: '2',
            nodes: [
                {
                    ...(previous.nodes[0] as Extract<RenderPlan['nodes'][number], { kind: 'element' }>),
                    children: [
                        { ...input, attributes: [{ name: 'aria-expanded', value: 'true' }] },
                        {
                            kind: 'element',
                            namespace: 'http://www.w3.org/2000/svg',
                            tag: 'svg',
                            renderNodeId: 'behavior-field-popup',
                            attributes: [{ name: 'role', value: 'listbox' }],
                            children: [],
                        },
                        help,
                    ],
                },
            ],
        };

        const frames = diffRenderPlansToPatchFrames(previous, next, { transactionId: 'conditional-child' });
        const ops = frames.flatMap((frame) => (frame.type === 'ops' ? frame.ops : []));

        expect(ops.map((operation) => operation.op)).toContain('reconcileChildren');
        expect(JSON.stringify(ops)).toContain('"namespace":"http://www.w3.org/2000/svg"');
        expect(ops).not.toContainEqual(
            expect.objectContaining({
                op: 'replace',
                target: { kind: 'render-node', id: 'behavior-field-root' },
            }),
        );
    });
});
