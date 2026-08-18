import { describe, expect, it, vi } from 'vitest';

import type { DataIslandSnapshot } from '../../cem-elements.js';

vi.mock('./cem-ql-render.js', () => ({
    compileCemMlTemplate: vi.fn(async () => []),
    processCemMlTemplate: vi.fn(async (input: {
        data: Record<string, unknown>;
        identity: {
            producedTag: string;
            instanceId: string;
            templateArtifactId: string;
            dataRevision: string;
            outputTarget: 'light-dom';
            scopePolicyStamp: string;
        };
    }) => ({
        diagnostics: [],
        renderPlan: {
            ...input.identity,
            nodes: [{
                kind: 'element' as const,
                namespace: null,
                tag: 'span',
                attributes: [],
                renderNodeId: `${input.identity.producedTag}-1`,
                children: [{
                    kind: 'text' as const,
                    text: String(input.data.label ?? ''),
                    sourceMapRef: { fidelity: 'author-byte-exact' as const, frame: 'cem:8' },
                }],
            }],
        },
    })),
}));

import { CemProcessingEngine } from './processing-engine.js';

describe('Phase 3A retained processing engine', () => {
    it('compiles and diffs the same canonical CEM-ML semantics for worker and fallback hosts', async () => {
        const workerEngine = new CemProcessingEngine();
        const fallbackEngine = new CemProcessingEngine();
        const compileInput = {
            language: 'cem-ml' as const,
            producedTag: 'cem-worker-card',
            templateArtifactId: 'template-worker-card-1',
            registrationIdentity: 'cem-registration-v1:worker-card',
            source: '{article @class="card" | {span | {$label}}}',
            sourceRef: { kind: 'inline' as const, value: 'cem-worker-card' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: 'scope-policy-v1',
            sourceMapMode: 'dev' as const,
        };
        const [workerArtifact, fallbackArtifact] = await Promise.all([
            workerEngine.compile(compileInput),
            fallbackEngine.compile(compileInput),
        ]);
        expect(workerArtifact).toEqual(fallbackArtifact);

        const snapshot = snapshotFixture('1');
        const renderInput = {
            artifact: workerArtifact.artifact,
            revision: {
                instanceId: snapshot.instanceId,
                dataRevision: snapshot.dataRevision,
                templateArtifactId: snapshot.templateArtifactId,
                scopePolicyStamp: snapshot.scopePolicyStamp,
                outputTarget: snapshot.outputTarget,
            },
            snapshot,
            data: { label: 'Worker output' },
            scopeUid: 'worker-card-scope',
            instanceScopeUid: 'worker-card-instance-scope',
            previousRenderPlan: null,
        };
        const [workerRender, fallbackRender] = await Promise.all([
            workerEngine.renderDiff(renderInput),
            fallbackEngine.renderDiff({ ...renderInput, artifact: fallbackArtifact.artifact }),
        ]);

        expect(workerRender).toEqual(fallbackRender);
        expect(workerRender.frames.at(0)).toEqual(expect.objectContaining({ type: 'begin' }));
        expect(workerRender.frames.at(-1)).toEqual(expect.objectContaining({ type: 'commit' }));
        expect(workerRender.frames.some(
            (frame) => frame.type === 'ops' && frame.ops.some((operation) => operation.op === 'replaceScope')
        )).toBe(true);
    });

    it('retains the prior plan and emits a targeted text patch on the next revision', async () => {
        const engine = new CemProcessingEngine();
        const compiled = await engine.compile({
            language: 'cem-ml',
            producedTag: 'cem-worker-label',
            templateArtifactId: 'template-worker-label-1',
            registrationIdentity: 'cem-registration-v1:worker-label',
            source: '{span | {$label}}',
            sourceRef: { kind: 'inline', value: 'cem-worker-label' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: 'scope-policy-v1',
            sourceMapMode: 'dev',
        });
        const firstSnapshot = snapshotFixture('1', 'template-worker-label-1', 'cem-worker-label');
        const first = await engine.renderDiff({
            artifact: compiled.artifact,
            revision: revision(firstSnapshot),
            snapshot: firstSnapshot,
            data: { label: 'Before' },
            scopeUid: 'worker-label-scope',
            instanceScopeUid: 'worker-label-instance-scope',
            previousRenderPlan: null,
        });
        const secondSnapshot = snapshotFixture('2', 'template-worker-label-1', 'cem-worker-label');
        const second = await engine.renderDiff({
            artifact: compiled.artifact,
            revision: revision(secondSnapshot),
            snapshot: secondSnapshot,
            data: { label: 'After' },
            scopeUid: 'worker-label-scope',
            instanceScopeUid: 'worker-label-instance-scope',
            previousRenderPlan: first.nextRenderPlan,
        });

        expect(second.frames.flatMap((frame) => frame.type === 'ops' ? frame.ops : [])).toContainEqual(
            expect.objectContaining({ op: 'setText', value: 'After' })
        );
    });
});

function revision(snapshot: DataIslandSnapshot) {
    return {
        instanceId: snapshot.instanceId,
        dataRevision: snapshot.dataRevision,
        templateArtifactId: snapshot.templateArtifactId,
        scopePolicyStamp: snapshot.scopePolicyStamp,
        outputTarget: snapshot.outputTarget,
    };
}

function snapshotFixture(
    dataRevision: string,
    templateArtifactId = 'template-worker-card-1',
    producedTag = 'cem-worker-card'
): DataIslandSnapshot {
    return {
        version: '1.2.0',
        instanceId: `${producedTag}-instance-1`,
        producedTag,
        declarationTag: 'cem-element',
        templateArtifactId,
        dataRevision,
        outputTarget: 'light-dom',
        scopePolicyStamp: 'scope-policy-v1',
        privacyPolicyStamp: 'privacy-v1',
        sourceMapMode: 'dev',
        hostAttributes: {},
        dataset: {},
        payload: {
            roots: [],
            byKey: {},
            choices: [],
            options: [],
            data: [],
            dataByValue: {},
            optionsByValue: {},
            elementsByAttribute: {},
            slots: {},
            text: '',
        },
        slices: {},
        validationState: {},
        eventPayloads: {},
    };
}
