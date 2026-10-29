import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import type { DataIslandSnapshot } from './cem-elements.js';
import { preparePatchFramesForRange } from './projection.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import type { CemProcessingCompileInput, CemProcessingRenderDiffInput } from './internal/runtime-support/processing-host.js';
import { cemMlTemplateArtifactPayloadKey, compileCemMlTemplateArtifact, retainCemMlTemplateArtifact, disposeRetainedCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';

export default { title: 'CEM Elements/Pinned Template Artifacts', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function scenario(fallback: boolean): Story {
    return { render: () => '<section aria-label="Pinned binary template transport"></section>', play: async ({ canvasElement }) => {
        // Test-only build stage. Deployment inputs below contain no source text.
        const source = '{style | ``` :scope { display: block; } ```}{input @value=Original}{p | {$label}}';
        const payloadKey = await cemMlTemplateArtifactPayloadKey(source, 'prod');
        const bytes = await compileCemMlTemplateArtifact(source, ['label'], 'prod');
        const buffer = bytes.slice().buffer as ArrayBuffer;
        const build = await retainCemMlTemplateArtifact(buffer, '', source, ['label'], 'prod');
        if (!build.contentHash) throw Error('Build stage omitted the artifact hash');
        const contentHash = build.contentHash; disposeRetainedCemMlTemplate(build.artifactId);
        const scopePolicyStamp = 'pinned-policy', templateArtifactId = 'pinned-view';
        const input: CemProcessingCompileInput = { language: 'cem-ml', producedTag: 'cem-pinned-view', templateArtifactId,
            registrationIdentity: 'pinned-registration', resolverIdentity: 'pinned-deployment', scopePolicyStamp, sourceMapMode: 'prod',
            sourceRef: { kind: 'url', value: 'https://example.test/assets/view.bin' }, hostBindings: ['label'],
            source: { kind: 'pinned-template-artifact-v1', artifact: { kind: 'template-artifact', cacheKey: contentHash,
                payloadKey, formatVersion: 'cem-template-artifact/1', policyStamp: scopePolicyStamp, bytes: buffer } } };
        expect(JSON.stringify(input)).not.toContain(source);
        const canvas = canvasElement.querySelector('section');
        if (!canvas) throw Error('Missing fixture region');
        // Re-admit the same portable bytes under two independently owned hosts.
        for (let owner = 0; owner < 2; owner++) {
            const scope = createCemDeclarationScope({ document });
            const host = cemProcessingHostForScope(scope, {
                workerScriptUrl: new URL('./internal/runtime-support/processing-worker.js', import.meta.url),
                ...(fallback ? { workerFactory: () => { throw Error('fixture selects fallback'); } } : {}),
                poolPolicy: { workerCount: 1, maxWorkers: 1 },
            });
            const bounds = { start: document.createComment('start'), end: document.createComment('end') };
            canvas.append(bounds.start, bounds.end);
            try {
                const corrupt = structuredClone(input);
                if (corrupt.source.kind !== 'pinned-template-artifact-v1') throw Error('fixture');
                corrupt.source.artifact.cacheKey = `cem-bin/1+blake3:${'0'.repeat(64)}`;
                await expect(host.compile(corrupt).result).rejects.toThrow('hash_mismatch');
                const pending = host.compile(input);
                const compiled = await pending.result;
                expect(compiled.diagnostics).toEqual(fallback ? [expect.objectContaining({ code: 'cem.processing_host.worker_startup_fallback', severity: 'warning' })] : []); expect(compiled.stylesheets).toHaveLength(1);
                const snapshot: DataIslandSnapshot = { instanceId: `pinned-${owner}`, producedTag: 'cem-pinned-view', declarationTag: 'cem-element',
                    templateArtifactId, dataRevision: '1', outputTarget: 'light-dom', scopePolicyStamp, privacyPolicyStamp: 'fixture', sourceMapMode: 'prod',
                    hostAttributes: {}, dataset: {}, slices: {}, validationState: {}, eventPayloads: {},
                    payload: { roots: [], byKey: {}, choices: [], options: [], data: [], dataByValue: {}, optionsByValue: {}, elementsByAttribute: {}, slots: {}, text: '' } };
                const request: CemProcessingRenderDiffInput = { artifact: compiled.artifact, snapshot,
                    revision: { instanceId: snapshot.instanceId, templateArtifactId, dataRevision: '1', scopePolicyStamp, outputTarget: 'light-dom' },
                    data: { label: 'First' }, scopeUid: `pinned-${owner}` };
                const first = await host.renderDiff(request).result;
                expect(preparePatchFramesForRange(bounds, first.frames, first.revision, document).commit(first.revision).status).toBe('applied');
                expect(canvas.textContent).toContain('First');
                const control = canvas.querySelector('input');
                if (!control) throw Error('Missing rendered input');
                control.value = 'User edit'; control.focus(); control.setSelectionRange(1, 4);
                const changed = await host.renderDiff({ ...request, data: { label: 'Changed <img>' }, previousRenderPlan: first.nextRenderPlan }).result;
                expect(preparePatchFramesForRange(bounds, changed.frames, changed.revision, document).commit(changed.revision).status).toBe('applied');
                expect(canvas.textContent).toContain('Changed <img>'); expect(canvas.querySelector('img')).toBeNull();
                expect(canvas.querySelector('input')).toBe(control); expect(control.value).toBe('User edit');
                expect(document.activeElement).toBe(control); expect([control.selectionStart, control.selectionEnd]).toEqual([1, 4]);
                expect(host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { await host.dispose({ reason: 'scope-disposed' }).result; scope.dispose(); canvas.replaceChildren(); }
        }
    } };
}
export const WorkerAdmission: Story = scenario(false);
export const FallbackAdmission: Story = scenario(true);
