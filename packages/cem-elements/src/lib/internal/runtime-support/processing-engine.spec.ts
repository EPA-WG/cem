import { describe, expect, it, vi } from 'vitest';

import type { DataIslandSnapshot } from '../../cem-elements.js';

vi.mock('./cem-ql-render.js', () => {
    let nextArtifactId = 1;
    const sources = new Map<number, string>();
    return {
    cemMlTemplateArtifactPayloadKey: vi.fn(async (source: string, sourceMapMode: 'dev' | 'prod') => ({
        contentType: 'cem-template-artifact' as const,
        sourceHash: `cem-bin/1+blake3:source-${source.length}`,
        cemMlVersion: '0.1.0',
        cemQlVersion: '0.1.0',
        sourceMapMode,
    })),
    compileCemMlTemplateArtifact: vi.fn(async (source: string) => new TextEncoder().encode(source)),
    retainCemMlTemplateSource: vi.fn(async (source: string) => {
        const artifactId = nextArtifactId++;
        sources.set(artifactId, source);
        return { artifactId, diagnostics: [] };
    }),
    retainCemMlTemplateArtifact: vi.fn(async (bytes: ArrayBuffer) => {
        const artifactId = nextArtifactId++;
        const source = new TextDecoder().decode(bytes);
        sources.set(artifactId, source);
        return {
            artifactId,
            contentHash: `cem-bin/1+blake3:fixture-${source.length}`,
            formatVersion: 'cem-template-artifact/1',
            diagnostics: [],
        };
    }),
    disposeRetainedCemMlTemplate: vi.fn(() => true),
    processRetainedCemMlTemplate: vi.fn(async (artifactId: number, input: {
        source: string;
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
            nodes: [
                ...(sources.get(artifactId)?.includes('http-request') ? [{
                    kind: 'element' as const,
                    namespace: null,
                    tag: 'http-request',
                    attributes: [
                        { name: 'slice', value: 'page' },
                        { name: 'url', value: './records.json' },
                        { name: 'method', value: 'get' },
                        { name: 'header-accept', value: 'application/json' },
                        { name: 'content-type', value: 'application/json' },
                    ],
                    renderNodeId: `${input.identity.producedTag}-control-1`,
                    children: [],
                    sourceMapRef: { fidelity: 'author-byte-exact' as const, frame: 'cem:0' },
                }] : []),
                ...(sources.get(artifactId)?.includes('repository-query') ? [{
                    kind: 'element' as const,
                    namespace: null,
                    tag: 'repository-query',
                    attributes: [
                        { name: 'slice', value: 'projects' },
                        { name: 'repository', value: 'studio-projects' },
                        { name: 'operation', value: 'list-projects' },
                        { name: 'parameters', value: '{"includeTrash":false}' },
                        { name: 'live', value: 'true' },
                        { name: 'cursor', value: '12' },
                    ],
                    renderNodeId: `${input.identity.producedTag}-repository-1`,
                    children: [],
                    sourceMapRef: { fidelity: 'author-byte-exact' as const, frame: 'cem:1' },
                }] : []),
                ...(sources.get(artifactId)?.includes('storage-status') ? [{
                    kind: 'element' as const,
                    namespace: null,
                    tag: 'storage-status',
                    attributes: [
                        { name: 'slice', value: 'storage' },
                        { name: 'repository', value: 'studio-projects' },
                        { name: 'live', value: 'true' },
                    ],
                    renderNodeId: `${input.identity.producedTag}-storage-1`,
                    children: [],
                    sourceMapRef: { fidelity: 'author-byte-exact' as const, frame: 'cem:2' },
                }] : []),
                {
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
                },
            ],
        },
    })),
    };
});

import { CemProcessingEngine } from './processing-engine.js';
import {
    compileCemMlTemplateArtifact,
    retainCemMlTemplateArtifact,
    retainCemMlTemplateSource,
} from './cem-ql-render.js';
import { createCemProcessingTextSource } from './processing-host.js';

describe('Phase 3A retained processing engine', () => {
    it('compiles and diffs the same canonical CEM-ML semantics for worker and fallback hosts', async () => {
        const workerEngine = new CemProcessingEngine();
        const fallbackEngine = new CemProcessingEngine();
        const compileInput = {
            language: 'cem-ml' as const,
            producedTag: 'cem-worker-card',
            templateArtifactId: 'template-worker-card-1',
            registrationIdentity: 'cem-registration-v1:worker-card',
            source: createCemProcessingTextSource(
                '{http-request @slice=page @url=./records.json}{article @class="card" | {span | {$label}}}',
                8
            ),
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
        expect(workerRender.resourceControls).toHaveLength(1);
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
            source: createCemProcessingTextSource('{span | {$label}}'),
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

    it('keys URI artifacts by source and resolver identity, not transport chunk boundaries', async () => {
        const engine = new CemProcessingEngine();
        const source = '{span | URI source}';
        const common = {
            language: 'cem-ml' as const,
            producedTag: 'cem-uri-card',
            registrationIdentity: 'cem-registration-v1:uri-card',
            sourceRef: { kind: 'specifier' as const, value: '@scope/cards/card.cem' },
            resolverIdentity: 'module-map:v1',
            scopePolicyStamp: 'scope-policy-v1',
            sourceMapMode: 'dev' as const,
        };
        const first = await engine.compile({
            ...common,
            templateArtifactId: 'template-uri-card-1',
            source: createCemProcessingTextSource(source, 2),
        });
        const rechunked = await engine.compile({
            ...common,
            templateArtifactId: 'template-uri-card-2',
            source: createCemProcessingTextSource(source, 9),
        });
        const anotherResolver = await engine.compile({
            ...common,
            templateArtifactId: 'template-uri-card-3',
            resolverIdentity: 'module-map:v2',
            source: createCemProcessingTextSource(source, 9),
        });

        expect(rechunked.artifact.cacheKey).toBe(first.artifact.cacheKey);
        expect(anotherResolver.artifact.cacheKey).not.toBe(first.artifact.cacheKey);
    });

    it('reuses compiled content by address and evicts the least-recently-used entry at the bound', async () => {
        const compile = vi.mocked(retainCemMlTemplateSource);
        compile.mockClear();
        const engine = new CemProcessingEngine({ maxArtifactEntries: 2 });
        const input = (templateArtifactId: string, source: string) => ({
            language: 'cem-ml' as const,
            producedTag: 'cem-cache-card',
            templateArtifactId,
            registrationIdentity: 'cem-registration-v1:cache-card',
            source: createCemProcessingTextSource(source),
            sourceRef: { kind: 'inline' as const, value: 'cem-cache-card' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: 'scope-policy-v1',
            sourceMapMode: 'dev' as const,
        });

        await engine.compile(input('artifact-a-1', '{span | A}'));
        await engine.compile(input('artifact-b-1', '{span | B}'));
        await engine.compile(input('artifact-a-2', '{span | A}'));
        await engine.compile(input('artifact-c-1', '{span | C}'));
        await engine.compile(input('artifact-b-2', '{span | B}'));

        expect(compile).toHaveBeenCalledTimes(4);
    });

    it('imports matching precompiled template bytes without source compilation', async () => {
        const compile = vi.mocked(compileCemMlTemplateArtifact);
        const retain = vi.mocked(retainCemMlTemplateArtifact);
        compile.mockClear();
        retain.mockClear();
        const engine = new CemProcessingEngine();
        const source = '{span | precompiled}';
        const bytes = new TextEncoder().encode(source).buffer as ArrayBuffer;

        const result = await engine.compile({
            language: 'cem-ml',
            producedTag: 'cem-precompiled-card',
            templateArtifactId: 'template-precompiled-card-1',
            registrationIdentity: 'cem-registration-v1:precompiled-card',
            source: createCemProcessingTextSource(source),
            sourceRef: { kind: 'inline', value: 'cem-precompiled-card' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: 'scope-policy-v1',
            sourceMapMode: 'dev',
            exportCompiledArtifact: true,
            precompiledArtifact: {
                kind: 'template-artifact',
                payloadKey: templatePayloadKey(source),
                cacheKey: 'cem-bin/1+blake3:fixture-20',
                formatVersion: 'cem-template-artifact/1',
                policyStamp: 'scope-policy-v1',
                bytes,
            },
        });

        expect(compile).not.toHaveBeenCalled();
        expect(retain).toHaveBeenCalledWith(
            bytes,
            'cem-bin/1+blake3:fixture-20',
            source,
            [],
            'dev'
        );
        expect(result.compiledArtifact).toBeUndefined();
    });

    it('rejects a policy-mismatched artifact and deterministically falls back to source', async () => {
        const compile = vi.mocked(compileCemMlTemplateArtifact);
        compile.mockClear();
        const engine = new CemProcessingEngine();
        const source = '{span | fallback}';

        const result = await engine.compile({
            language: 'cem-ml',
            producedTag: 'cem-rejected-card',
            templateArtifactId: 'template-rejected-card-1',
            registrationIdentity: 'cem-registration-v1:rejected-card',
            source: createCemProcessingTextSource(source),
            sourceRef: { kind: 'inline', value: 'cem-rejected-card' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: 'scope-policy-v2',
            sourceMapMode: 'dev',
            exportCompiledArtifact: true,
            precompiledArtifact: {
                kind: 'template-artifact',
                payloadKey: templatePayloadKey(source),
                cacheKey: 'cem-bin/1+blake3:fixture',
                formatVersion: 'cem-template-artifact/1',
                policyStamp: 'scope-policy-v1',
                bytes: new TextEncoder().encode(source).buffer as ArrayBuffer,
            },
        });

        expect(compile).toHaveBeenCalledWith(source, [], 'dev');
        expect(result.compiledArtifact).toEqual(expect.objectContaining({
            kind: 'template-artifact',
            policyStamp: 'scope-policy-v2',
        }));
        expect(result.diagnostics).toContainEqual(expect.objectContaining({
            code: 'cem.processing_host.precompiled_artifact_rejected',
            severity: 'warning',
        }));
    });

    it('falls back to a full replacement when an old content-addressed plan was evicted', async () => {
        const engine = new CemProcessingEngine({ maxRenderPlanEntries: 2 });
        const compiled = await engine.compile({
            language: 'cem-ml',
            producedTag: 'cem-plan-cache',
            templateArtifactId: 'template-plan-cache-1',
            registrationIdentity: 'cem-registration-v1:plan-cache',
            source: createCemProcessingTextSource('{span | {$label}}'),
            sourceRef: { kind: 'inline', value: 'cem-plan-cache' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: 'scope-policy-v1',
            sourceMapMode: 'dev',
        });
        const render = async (
            dataRevision: string,
            previousRenderPlan: Awaited<ReturnType<typeof engine.renderDiff>>['nextRenderPlan'] | null
        ) => {
            const snapshot = snapshotFixture(dataRevision, compiled.artifact.artifactId, 'cem-plan-cache');
            return engine.renderDiff({
                artifact: compiled.artifact,
                revision: revision(snapshot),
                snapshot,
                data: { label: dataRevision },
                scopeUid: 'plan-cache-scope',
                instanceScopeUid: 'plan-cache-instance-scope',
                previousRenderPlan,
            });
        };

        const first = await render('1', null);
        const second = await render('2', first.nextRenderPlan);
        await render('3', second.nextRenderPlan);
        const afterEviction = await render('4', first.nextRenderPlan);

        expect(afterEviction.frames.flatMap((frame) => frame.type === 'ops' ? frame.ops : [])).toContainEqual(
            expect.objectContaining({ op: 'replaceScope' })
        );
    });

    it('lowers interpolated HTTP declarations to clone-safe controls before retained-plan diffing', async () => {
        const engine = new CemProcessingEngine();
        const snapshot = snapshotFixture('1', 'template-worker-http-1', 'cem-worker-http');
        const compiled = await engine.compile({
            language: 'cem-ml',
            producedTag: 'cem-worker-http',
            templateArtifactId: snapshot.templateArtifactId,
            registrationIdentity: 'cem-registration-v1:worker-http',
            source: createCemProcessingTextSource(
                '{http-request @slice=page @url=./records.json}{span | {$label}}',
                7
            ),
            sourceRef: { kind: 'url', value: 'https://example.test/components/http.cem' },
            resolverIdentity: 'module-map:fixture-v1',
            scopePolicyStamp: snapshot.scopePolicyStamp,
            sourceMapMode: 'dev',
        });

        const rendered = await engine.renderDiff({
            artifact: compiled.artifact,
            revision: revision(snapshot),
            snapshot,
            data: { label: 'Loaded' },
            scopeUid: 'worker-http-scope',
            instanceScopeUid: 'worker-http-instance-scope',
            previousRenderPlan: null,
        });

        expect(rendered.resourceControls).toEqual([{
            kind: 'http-request',
            renderNodeId: 'cem-worker-http-control-1',
            sliceName: 'page',
            authoredUrl: './records.json',
            method: 'GET',
            headers: { accept: 'application/json' },
            expectedContentType: 'application/json',
            sourceMapRef: { fidelity: 'author-byte-exact', frame: 'cem:0' },
        }]);
        expect(JSON.stringify(rendered.frames)).not.toContain('http-request');
        expect(structuredClone(rendered)).toEqual(rendered);
    });

    it('lowers repository reads and storage status without mutation authority in the render plan', async () => {
        const engine = new CemProcessingEngine();
        const snapshot = snapshotFixture(
            '1',
            'template-worker-repository-1',
            'cem-worker-repository'
        );
        const compiled = await engine.compile({
            language: 'cem-ml',
            producedTag: 'cem-worker-repository',
            templateArtifactId: snapshot.templateArtifactId,
            registrationIdentity: 'cem-registration-v1:worker-repository',
            source: createCemProcessingTextSource(
                '{repository-query @slice=projects @repository=studio-projects @operation=list-projects}' +
                    '{storage-status @slice=storage @repository=studio-projects}' +
                    '{span | {$label}}',
                5
            ),
            sourceRef: { kind: 'inline', value: 'cem-worker-repository' },
            resolverIdentity: 'document:https://example.test/',
            scopePolicyStamp: snapshot.scopePolicyStamp,
            sourceMapMode: 'dev'
        });

        const rendered = await engine.renderDiff({
            artifact: compiled.artifact,
            revision: revision(snapshot),
            snapshot,
            data: { label: 'Loaded' },
            scopeUid: 'worker-repository-scope',
            instanceScopeUid: 'worker-repository-instance-scope',
            previousRenderPlan: null
        });

        expect(rendered.resourceControls).toEqual([
            {
                kind: 'repository-query',
                renderNodeId: 'cem-worker-repository-repository-1',
                sliceName: 'projects',
                repository: 'studio-projects',
                operation: 'list-projects',
                parameters: '{"includeTrash":false}',
                live: true,
                cursor: '12',
                sourceMapRef: { fidelity: 'author-byte-exact', frame: 'cem:1' }
            },
            {
                kind: 'storage-status',
                renderNodeId: 'cem-worker-repository-storage-1',
                sliceName: 'storage',
                repository: 'studio-projects',
                live: true,
                sourceMapRef: { fidelity: 'author-byte-exact', frame: 'cem:2' }
            }
        ]);
        expect(JSON.stringify(rendered.frames)).not.toMatch(
            /repository-query|storage-status|execute/
        );
        expect(structuredClone(rendered)).toEqual(rendered);
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

function templatePayloadKey(source: string) {
    return {
        contentType: 'cem-template-artifact' as const,
        sourceHash: `cem-bin/1+blake3:source-${source.length}`,
        cemMlVersion: '0.1.0',
        cemQlVersion: '0.1.0',
        sourceMapMode: 'dev' as const,
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
