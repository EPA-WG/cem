import { readFile } from 'node:fs/promises';
import { beforeAll, expect, it } from 'vitest';
import { elementReferenceFixtureInputs, ELEMENT_REFERENCE_TEMPLATE } from './element-reference-lifecycle.fixtures.js';
import { processRetainedCemMlTemplate, retainCemMlTemplateSource, disposeRetainedCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import { serializeRenderPlanToHtmlFixture } from './edge-ssr-host-fixture.js';
import { advanceEdgeRenderState, InMemoryEdgeRenderStateStore, readEdgeRenderStateContents } from './projection.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- execute the shared native ingress in a Node SSR host without DOM globals.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
beforeAll(async () => { await wasm.default({ module_or_path: await readFile(new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) }); });
it('publishes native relationship IDs through SSR/Edge and retains the committed revision on incomplete replacement', async () => {
    expect(typeof document).toBe('undefined');
    const artifact = await retainCemMlTemplateSource(ELEMENT_REFERENCE_TEMPLATE);
    const references = elementReferenceFixtureInputs();
    const identity = { producedTag: 'reference-card', instanceId: 'persisted-server', templateArtifactId: 'reference-template', dataRevision: '1', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope' };
    const store = new InMemoryEdgeRenderStateStore();
    try {
        const first = await processRetainedCemMlTemplate(artifact.artifactId, { source: ELEMENT_REFERENCE_TEMPLATE, identity, data: { tone: 'server' }, elementReferenceInputs: references });
        expect(first.diagnostics).toEqual([]);
        const html = serializeRenderPlanToHtmlFixture(first.renderPlan);
        expect(html).toContain('commandfor="persisted-server-ref-1"');
        expect(html).toContain('id="persisted-server-ref-1"');
        const committed = advanceEdgeRenderState(store, { renderPlan: first.renderPlan, renderedHtml: html });
        if (!committed.ok) throw new Error(committed.reason);
        await expect(processRetainedCemMlTemplate(artifact.artifactId, { source: ELEMENT_REFERENCE_TEMPLATE, identity: { ...identity, dataRevision: '2' }, data: { tone: 'bad' }, elementReferenceInputs: { ...references, grants: [] }, previousRenderPlan: first.renderPlan })).rejects.toThrow();
        expect(store.readRecord(committed.record.stateKey)).toEqual(committed.record);
        const retained = readEdgeRenderStateContents(store, committed.record);
        expect(retained.ok).toBe(true);
        if (retained.ok) expect(retained.contents.renderedHtml).toBe(html);
        const next = await processRetainedCemMlTemplate(artifact.artifactId, { source: ELEMENT_REFERENCE_TEMPLATE, identity: { ...identity, dataRevision: '3' }, data: { tone: 'edge' }, elementReferenceInputs: references, previousRenderPlan: first.renderPlan });
        const nextHtml = serializeRenderPlanToHtmlFixture(next.renderPlan);
        expect(nextHtml).toContain('commandfor="persisted-server-ref-1"');
        expect(nextHtml).toContain('edge');
        const updated = advanceEdgeRenderState(store, { renderPlan: next.renderPlan, renderedHtml: nextHtml }, { expectedEtag: committed.record.etag });
        expect(updated.ok).toBe(true);
        const authored = elementReferenceFixtureInputs('{dialog @id=public-part | Authored ID}');
        const explicit = await processRetainedCemMlTemplate(artifact.artifactId, { source: ELEMENT_REFERENCE_TEMPLATE, identity, data: {}, elementReferenceInputs: authored });
        expect(serializeRenderPlanToHtmlFixture(explicit.renderPlan)).toContain('commandfor="public-part"');
    } finally { disposeRetainedCemMlTemplate(artifact.artifactId); }
});
