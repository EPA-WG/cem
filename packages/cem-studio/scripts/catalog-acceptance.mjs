import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';
import { nativeCatalogDiagnosticLimit } from './native-catalog-execution.mjs';

/** Read named project/control envelopes; native source parsing is never duplicated. */
export async function catalogAcceptanceInputs(root) {
    const json = async path => JSON.parse(await readFile(resolve(root, path), 'utf8'));
    const generated = 'packages/cem-studio/generated/feature-tour';
    const index = await json(`${generated}/index.json`);
    const definitions = new Map(await Promise.all((await readdir(resolve(root, 'examples/projects'), { withFileTypes: true }))
        .filter(item => item.isDirectory()).map(async item => { const definition = await json(`examples/projects/${item.name}/example.json`); return [definition.id, definition]; })));
    const projects = await Promise.all(index.projects.map(async record => {
        const bytes = await readFile(resolve(root, generated, record.archive.slice(2)));
        assert.equal(createHash('sha256').update(bytes).digest('hex'), record.sha256, `${record.id}: catalog archive hash drifted`);
        const bundle = parseCemStudioProjectBundle(new Uint8Array(bytes));
        const definition = definitions.get(record.id);
        const configurations = bundle.project.entries.filter(entry => entry.kind !== 'subproject').map(entry => {
            const authored = definition?.configurations.find(item => item.id === entry.id);
            return { ...authored, id: entry.id, kind: entry.kind, source: authored?.source ?? entry.resourceIds[0],
                command: bundle.project.resources.find(item => item.id === entry.runConfigResourceId)?.schema === 'https://cem.dev/ns/cli/command/1',
                binary: bundle.project.resources.find(item => item.id === entry.resourceIds[0])?.contentType.endsWith('+cem-bin') ?? false };
        });
        const folders = new Set();
        for (const resource of bundle.project.resources) {
            const parts = resource.path.split('/');
            for (let length = 1; length < parts.length; length++) folders.add(`${parts.slice(0, length).join('/')}/`);
        }
        if (bundle.project.entries.length) folders.add('@configurations');
        for (const entry of bundle.project.entries.filter(item => item.kind === 'subproject')) folders.add(`@configurations/${entry.id}`);
        return { id: record.id, record, definition, bundle, resources: bundle.project.resources.map(item => item.id), configurations,
            collectionIds: record.collections?.length ? record.collections : ['examples'], folders: [...folders] };
    }));
    return { index, projects, identity: createHash('sha256').update(JSON.stringify({ index,
        definitions: [...definitions.entries()].sort(([a], [b]) => a.localeCompare(b)) })).digest('hex') };
}

export const loginLiveDiagnosticOffsets = Object.freeze([196, 657, 1147, 1187]);

export async function catalogAcceptanceStamp(root) {
    const output = resolve(root, 'packages/cem-studio/dist/static'), paths = [];
    const visit = async (path = '') => {
        for (const item of await readdir(resolve(output, path), { withFileTypes: true })) {
            const relative = path ? `${path}/${item.name}` : item.name;
            if (item.isDirectory()) await visit(relative); else paths.push(relative);
        }
    };
    await visit();
    const files = await Promise.all(paths.sort().map(async path => {
        const bytes = await readFile(resolve(output, path));
        return { path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') };
    }));
    return { catalogIdentity: (await catalogAcceptanceInputs(root)).identity,
        deployedSha256: createHash('sha256').update(JSON.stringify(files)).digest('hex') };
}

export async function catalogCensusVerifierIdentity(root) {
    const paths = ['scripts/verify-navigation.mjs', 'scripts/verify-catalog-projects.mjs', 'scripts/catalog-acceptance.mjs',
        'scripts/native-catalog-execution.mjs', 'scripts/fixtures/catalog-native-diagnostics.json',
        'scripts/studio-browser-diagnostics.mjs', 'src/file-system-provider.js'];
    const hash = createHash('sha256');
    for (const path of paths) { hash.update(`${path}\0`); hash.update(await readFile(resolve(root, 'packages/cem-studio', path))); }
    return hash.digest('hex');
}

export function assertAcceptanceStamp(actual, expected, label) {
    for (const field of ['catalogIdentity', 'deployedSha256']) {
        assert.match(expected[field], /^[0-9a-f]{64}$/);
        assert.equal(actual[field], expected[field], `${label}: stale ${field}`);
    }
}

/** Compare an explicitly requested native query JSON export, never execution input. */
export function nativeQueryRangeEvidence(text, ranges, sourceUri) {
    const report = JSON.parse(text);
    assert.equal(report.language, 'cem-ql');
    assert.deepEqual(report.result.diagnostics, []); assert.equal(report.result.error, null);
    const selected = report.result.items.map(item => {
        assert.equal(item.kind, 'native'); assert.equal(item.representation, 'cem.ql.imported-cem-ast');
        assert.equal(typeof item.identity, 'string'); assert.ok(item.identity.length > 0);
        const boundary = item.sourceMap.frames.find(frame => frame.transform.kind === 'SourceBoundary');
        assert.equal(boundary?.transform.uri, sourceUri);
        // Arc-backed identity includes an owner-local allocator address. It is
        // neither a portable address nor a stable result-byte promise. Keep
        // every other exported field, including complete original source maps.
        const { identity: _identity, ...facts } = item;
        return facts;
    });
    assert.deepEqual(selected.map(item => item.sourceMap.frames.find(frame => frame.transform.kind === 'SourceBoundary').span.ranges), ranges);
    return { ...report, result: { ...report.result, items: selected } };
}
const key = row => `${row.project}/${row.configuration}`;
const sameMembers = (actual, expected, label) => {
    assert.equal(new Set(actual).size, actual.length, `${label}: duplicate evidence`);
    assert.deepEqual([...actual].sort(), [...expected].sort(), `${label}: missing or stale evidence`);
};

/** Validate actual execution receipts separately from navigation and byte delivery. */
export function assertCatalogConfigurationEvidence(projects, receipts, { host, phase } = {}) {
    const expected = projects.flatMap(project => project.configurations.map(configuration => ({ project: project.id, ...configuration })));
    sameMembers(receipts.map(key), expected.map(row => `${row.project}/${row.id}`), `${host}/${phase ?? 'native'} configurations`);
    for (const receipt of receipts) {
        assert.equal(receipt.host, host);
        if (phase) assert.equal(receipt.phase, phase);
        const configuration = expected.find(row => row.project === receipt.project && row.id === receipt.configuration);
        const label = key(receipt);
        if (host === 'worker' && configuration.expectedNativeRanges) assert.equal(receipt.outputComparison, 'original-source-facts-without-owner-local-identity', label);
        else assert.equal(receipt.outputComparison, undefined, `${label}: undeclared output comparison exception`);
        if (host === 'native' && configuration.argv?.[0] === 'inspect') {
            assert.equal(receipt.executed, true, label);
            assert.equal(receipt.status, 'verified-output', label);
            assert.equal(receipt.exitCode, 0, label);
            assert.equal(receipt.diagnosticEvidence, 'companion-source-diagnostics', label);
            assert.equal(receipt.diagnosticCodes, null, 'Inspect does not publish original-view diagnostic evidence');
            const limit = nativeCatalogDiagnosticLimit(receipt.project, receipt.configuration, 'companion-source-diagnostics');
            const hard = limit?.diagnostics.filter(item => ['error', 'fatal'].includes(item.severity)) ?? [];
            assert.equal(receipt.diagnosticProbe.status, limit ? 'known-limit' : 'passed', label);
            assert.equal(receipt.diagnosticProbe.exitCode, 0, label);
            assert.deepEqual(receipt.diagnosticProbe.diagnosticCodes, hard.map(item => item.code), label);
            assert.deepEqual(receipt.diagnosticProbe.diagnosticOffsets, hard.map(item => item.byteOffset), label);
            assert.deepEqual(receipt.diagnosticProbe.diagnosticUris, hard.map(item => item.uri), label);
            continue;
        }
        assert.ok(Array.isArray(receipt.diagnosticCodes), `${label}: missing hard diagnostic receipt`);
        const nativeLimit = (host === 'native' || host === 'worker' && label === 'supplier-pipeline/catalog-source')
            && nativeCatalogDiagnosticLimit(receipt.project, receipt.configuration, 'command-report');
        if (host === 'worker' && configuration.binary) {
            assert.equal(receipt.status, 'policy-refusal', label);
            assert.equal(receipt.executed, false, label);
            assert.equal(receipt.exitCode, null, label);
            assert.deepEqual(receipt.diagnosticCodes, []);
        } else if (nativeLimit) {
            assert.equal(receipt.executed, true, label);
            assert.equal(receipt.status, 'known-limit', label);
            assert.equal(receipt.exitCode, nativeLimit.exitCode, label);
            const hard = nativeLimit.diagnostics.filter(item => ['error', 'fatal'].includes(item.severity));
            assert.deepEqual(receipt.diagnosticCodes, hard.map(item => item.code), label);
            assert.deepEqual(receipt.diagnosticOffsets, hard.map(item => item.byteOffset), label);
        } else {
            const exit = configuration.expectedExit ?? 0;
            assert.equal(receipt.exitCode, exit, label);
            assert.equal(receipt.status, exit ? 'intentional-diagnostic' : 'passed', label);
            if (exit) {
                assert.ok(receipt.diagnosticCodes.length > 0, `${label}: expected hard diagnostics`);
                if (configuration.expectedValidation?.errorCode) assert.ok(receipt.diagnosticCodes.includes(configuration.expectedValidation.errorCode), label);
            } else assert.deepEqual(receipt.diagnosticCodes, [], `${label}: unexpected hard diagnostics`);
            assert.equal(receipt.executed, true, label);
        }
    }
    return { configurations: receipts.length, executed: receipts.filter(row => row.status !== 'policy-refusal').length,
        passed: receipts.filter(row => row.status === 'passed').length,
        verifiedOutputs: receipts.filter(row => row.status === 'verified-output').length,
        intentionalDiagnostics: receipts.filter(row => row.status === 'intentional-diagnostic').length,
        knownLimits: receipts.filter(row => row.status === 'known-limit').length,
        companionDiagnosticLimits: receipts.filter(row => row.diagnosticProbe?.status === 'known-limit').length,
        policyRefusals: receipts.filter(row => row.status === 'policy-refusal').length };
}

export function assertCatalogNavigationEvidence(projects, evidence) {
    const receipts = evidence.projects;
    sameMembers(receipts.map(row => row.id), projects.map(row => row.id), 'catalog navigation projects');
    sameMembers(evidence.unchangedOriginalProjectIds, projects.map(row => row.id), 'unchanged catalog originals');
    assert.equal(evidence.draft.persistedBytesUnchanged, true, 'offline draft changed its saved bytes');
    sameMembers(receipts.map(row => row.savedProjectId), receipts.map(row => row.savedProjectId), 'independent project IDs');
    sameMembers(receipts.map(row => row.rootUri), receipts.map(row => row.rootUri), 'independent project roots');
    for (const project of projects) {
        const receipt = receipts.find(row => row.id === project.id);
        sameMembers(receipt.resourceIds, project.resources, `${project.id} resource routes`);
        sameMembers(receipt.configurationIds, project.configurations.map(row => row.id), `${project.id} configuration routes`);
        assert.ok(receipt.savedProjectId && receipt.savedProjectId !== project.id, `${project.id} independent clone`);
        assert.equal(receipt.rootUri, `studio://${receipt.savedProjectId}/`);
        assert.equal(receipt.offline.archive, true);
        sameMembers(receipt.offline.resourceIds, project.resources, `${project.id} offline resource routes`);
        sameMembers(receipt.offline.configurationIds, project.configurations.map(row => row.id), `${project.id} offline configuration routes`);
        sameMembers(receipt.collectionIds, project.collectionIds ?? [], `${project.id} catalog memberships`);
        sameMembers(receipt.offline.collectionIds, project.collectionIds ?? [], `${project.id} offline memberships`);
        sameMembers(receipt.folderPaths, project.folders ?? [], `${project.id} folder routes`);
        sameMembers(receipt.offline.folderPaths, project.folders ?? [], `${project.id} offline folder routes`);
        sameMembers(receipt.configurations.map(row => row.id), project.configurations.map(row => row.id), `${project.id} source admission receipts`);
        for (const configuration of project.configurations) {
            const row = receipt.configurations.find(row => row.id === configuration.id);
            assert.equal(row.sourceId, configuration.source, `${project.id}/${configuration.id} source`);
            assert.equal(row.admission, configuration.binary ? 'read-only-binary' : configuration.command ? 'editable-command' : 'editable-validation');
            if (configuration.binary) {
                assert.equal(row.runEnabled, false);
                assert.ok(row.actions.includes('download-resource'));
            }
            else {
                assert.equal(row.runEnabled, !!configuration.command);
                const action = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
                assert.ok(row.actions.includes(`available:${action}`));
            }
        }
        for (const action of ['open-original-resources', 'open-original-configurations', 'clone-selected', 'open-saved-resources', 'open-saved-configurations']) assert.ok(receipt.actions.includes(action), `${project.id}/${action}`);
    }
    return { projects: receipts.length, resources: receipts.reduce((count, row) => count + row.resourceIds.length, 0),
        configurations: receipts.reduce((count, row) => count + row.configurationIds.length, 0) };
}
