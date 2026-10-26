import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import { catalogAcceptanceInputs, assertCatalogConfigurationEvidence, nativeQueryRangeEvidence } from './catalog-acceptance.mjs';
import { assertWorkerSupplierDiagnosticLimit } from './native-catalog-execution.mjs';

/** Exercise the public owned-workbench command path in the deployed worker. */
export async function verifyCatalogConfigurations({ page, context, url }) {
    const { projects, identity } = await catalogAcceptanceInputs(resolve(import.meta.dirname, '../../..'));
    const bounded = async (operation, label) => {
        let timer;
        try { return await Promise.race([operation, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`Catalog command ${label} exceeded 30 seconds`)), 30_000); })]); }
        finally { clearTimeout(timer); }
    };
    const evaluate = (fn, argument) => bounded(page.evaluate(fn, argument), 'observation');
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled()); };
    await page.goto(`${url}/`); await ready();
    const copies = new Map();
    for (const project of projects) {
        const id = `catalog-command-${randomUUID()}`;
        await evaluate(async ({ sample, id }) => {
            const app = globalThis.__cemStudioApplication;
            const bundle = structuredClone(await app.exampleProjects.examples.find(item => item.id === sample).load());
            bundle.project = { ...bundle.project, id, rootUri: `studio://${id}/`, name: `Command acceptance ${sample}`, revision: 1 };
            await app.repository.execute({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
                operation: 'import-project', parameters: { bundle, mode: 'create' } });
        }, { sample: project.id, id });
        copies.set(project.id, id);
    }
    const run = async (project, configuration) => bounded(page.evaluate(async ({ projectId, configuration, module }) => {
        const app = globalThis.__cemStudioApplication;
        const { createCemStudioProjectWorkbench } = await import(module);
        const workbench = await createCemStudioProjectWorkbench({ repository: app.repository, validator: app.validator,
            selection: { projectId, entryId: configuration } });
        try {
            const admission = workbench.snapshot();
            if (admission.editable === false) return { executed: false, editable: admission.editable, status: admission.status,
                resourceId: admission.resourceId, byteLength: admission.byteLength };
            const operation = admission.runnable === false ? 'validate-source' : 'configured-command';
            if (operation === 'validate-source') await workbench.validatePersisted();
            else await workbench.runPersistedOperation();
            const current = workbench.snapshot(), projection = current.projection;
            return { executed: true, operation, status: current.status, error: current.error, dirty: current.dirty,
                exitCode: projection?.exitCode ?? current.validation?.exitCode, diagnostics: projection?.diagnostics ?? current.validation?.diagnostics,
                output: projection && { contentType: projection.output.contentType, text: projection.output.text, bytes: Array.from(projection.output.bytes) },
                outputs: projection?.outputs.map(output => ({ path: projection.buildRun.artifacts.find(item => item.uri === output.uri)?.path,
                    bytes: Array.from(output.bytes), contentType: output.contentType })), previewKind: projection?.preview?.kind,
                summary: projection?.summary, expectedMatches: projection?.expected?.matches };
        } finally { workbench.dispose(); }
    }, { projectId: copies.get(project.id), configuration: configuration.id,
        module: `${url}/assets/@epa-wg/cem-studio/workbench.js` }), `${project.id}/${configuration.id}`);
    const receipts = { online: [], offline: [] };
    const signatures = new Map();
    for (const phase of ['online', 'offline']) {
        if (phase === 'offline') { await context.setOffline(true); await page.reload(); await ready(); }
        try {
            for (const project of projects) {
                console.log(`[cem-studio:catalog-commands] ${phase} ${project.id}: ${project.configurations.length} configurations`);
                const expectedBytes = id => Buffer.from(project.bundle.contents[id]);
                const expectedText = id => expectedBytes(id).toString('utf8');
                for (const configuration of project.configurations) {
                    const label = `${project.id}/${configuration.id}`, outcome = await run(project, configuration);
                    if (configuration.binary) {
                        assert.equal(outcome.executed, false, label); assert.equal(outcome.editable, false, label);
                        assert.equal(outcome.status, 'read-only', label); assert.ok(outcome.byteLength > 0, label);
                        receipts[phase].push({ project: project.id, configuration: configuration.id, host: 'worker', phase,
                            status: 'policy-refusal', executed: false, exitCode: null, diagnosticCodes: [] });
                        continue;
                    }
                    assert.equal(outcome.executed, true, `${label}: command unexpectedly refused`);
                    assert.equal(outcome.error, undefined, label); assert.equal(outcome.dirty, false, label);
                    assert.ok(outcome.output || outcome.operation === 'validate-source', `${label}: missing native output`);
                    const diagnostics = outcome.diagnostics ?? [], hard = diagnostics.filter(item => ['error', 'fatal'].includes(item.severity));
                    const receipt = { project: project.id, configuration: configuration.id, host: 'worker', phase, executed: true,
                        status: label === 'supplier-pipeline/catalog-source' ? 'known-limit' : (configuration.expectedExit ?? 0) ? 'intentional-diagnostic' : 'passed',
                        exitCode: outcome.exitCode, diagnosticCodes: hard.map(item => item.code), operation: outcome.operation,
                        ...(outcome.output && { outputSha256: digest(Buffer.from(outcome.output.bytes)) }) };
                    if (label === 'login/live-view') {
                        assert.equal(outcome.previewKind, 'live-cem', label);
                        assert.deepEqual(hard, [], 'The worker supplies the live processing bindings; CLI generic Parse remains a separate limit');
                    }
                    if (label === 'supplier-pipeline/catalog-source') {
                        receipt.diagnosticOffsets = assertWorkerSupplierDiagnosticLimit(diagnostics, `studio://${copies.get(project.id)}/`);
                    }
                    if (configuration.expectedValidation) {
                        const expected = configuration.expectedValidation;
                        assert.equal(hard.length, expected.errorCount, label);
                        assert.equal(diagnostics.filter(item => item.severity === 'warning').length, expected.warningCount, label);
                        if (expected.errorCode) assert.ok(hard.some(item => item.code === expected.errorCode), label);
                        if (expected.warningCode) assert.ok(diagnostics.some(item => item.code === expected.warningCode), label);
                    }
                    for (const text of configuration.expectedText ?? []) assert.ok(outcome.output.text.includes(text), `${label}: missing ${text}`);
                    if (configuration.expectedOutput || configuration.expectedOutputBytes) {
                        const expected = configuration.expectedOutput ?? configuration.expectedOutputBytes;
                        if (configuration.expectedOutputMode === 'json-value') assert.deepEqual(JSON.parse(outcome.output.text), JSON.parse(expectedText(expected)), label);
                        else assert.deepEqual(Buffer.from(outcome.output.bytes), expectedBytes(expected), `${label}: native fixture drifted`);
                    }
                    if (configuration.expectedSemanticContract) {
                        // Explicit query JSON is a requested export, never an AST execution handoff.
                        const report = JSON.parse(outcome.output.text);
                        const values = report.language === 'css-selector'
                            ? report.matches.map(({ localName, namespaceUri, documentOrder, nodeId }) => ({ localName, namespaceUri, documentOrder, nodeId }))
                            : report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
                        assert.deepEqual(values, JSON.parse(expectedText(configuration.expectedSemanticContract)), label);
                    }
                    for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs ?? {})) {
                        const output = outcome.outputs.find(item => item.path === path);
                        assert.ok(output, `${label}: missing build artifact ${path}`);
                        // CLI indents abstract JSON; worker publication is compact.
                        // Compare the complete explicitly exported value, retaining
                        // the worker's actual bytes for the offline equality check.
                        if (output.contentType.split(';')[0] === 'application/json' || output.contentType.split(';')[0].endsWith('+json')) {
                            assert.deepEqual(JSON.parse(Buffer.from(output.bytes).toString('utf8')), JSON.parse(expectedText(expected)), `${label}/${path}`);
                        } else assert.deepEqual(Buffer.from(output.bytes), expectedBytes(expected), `${label}/${path}: build fixture drifted`);
                    }
                    if (configuration.expectedGraphOutput) {
                        const output = outcome.outputs.find(item => item.path === configuration.expectedGraphOutput);
                        assert.ok(output, label); assert.deepEqual(Buffer.from(output.bytes), expectedBytes('login-view-expected'), label);
                    }
                    let nativeRanges;
                    if (configuration.expectedNativeRanges) {
                        const source = project.bundle.project.resources.find(resource => resource.id === configuration.source);
                        nativeRanges = nativeQueryRangeEvidence(outcome.output.text, JSON.parse(expectedText(configuration.expectedNativeRanges)),
                            `studio://${copies.get(project.id)}/${source.path}`);
                        receipt.outputComparison = 'original-source-facts-without-owner-local-identity';
                    }
                    const signature = { exitCode: outcome.exitCode, diagnostics,
                        outputs: outcome.outputs?.map(item => ({ path: item.path, contentType: item.contentType,
                            ...(nativeRanges && item.path === 'cem-stdio://stdout' ? { nativeRanges } : { sha256: digest(Buffer.from(item.bytes)) }) })) };
                    if (phase === 'online') signatures.set(label, signature);
                    else assert.deepEqual(signature, signatures.get(label), `${label}: offline command evidence drifted`);
                    assertCatalogConfigurationEvidence([{ id: project.id, configurations: [configuration] }], [receipt], { host: 'worker', phase });
                    receipts[phase].push(receipt);
                }
            }
        } finally { if (phase === 'offline') await context.setOffline(false); }
    }
    const online = assertCatalogConfigurationEvidence(projects, receipts.online, { host: 'worker', phase: 'online' });
    const offline = assertCatalogConfigurationEvidence(projects, receipts.offline, { host: 'worker', phase: 'offline' });
    for (const project of projects) {
        const hashes = await evaluate(async ({ sample, projectId }) => {
            const app = globalThis.__cemStudioApplication;
            const original = await app.exampleProjects.examples.find(item => item.id === sample).load();
            const saved = (await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
                operation: 'export-project', parameters: { projectId } })).value;
            const hashes = async bundle => Promise.all(bundle.project.resources.map(async resource => {
                const value = bundle.contents[resource.id], bytes = value instanceof Uint8Array ? value : new Uint8Array(value);
                return [resource.id, [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(byte => byte.toString(16).padStart(2, '0')).join('')];
            }));
            return { original: await hashes(original), saved: await hashes(saved) };
        }, { sample: project.id, projectId: copies.get(project.id) });
        const expected = project.bundle.project.resources.map(resource => [resource.id, resource.sha256]).sort();
        assert.deepEqual(hashes.original.sort(), expected); assert.deepEqual(hashes.saved.sort(), expected);
    }
    return { identity, online, offline, receipts, offlineOutputsVerified: true,
        nativeOwnerIdentitiesAreNotPortable: true, originalsAndCopiesUnchanged: true };
}

function digest(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
