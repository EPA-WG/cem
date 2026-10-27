import assert from 'node:assert/strict';
import { test } from 'node:test';
import { catalogAcceptanceInputs, assertCatalogConfigurationEvidence, assertCatalogNavigationEvidence, assertAcceptanceStamp, nativeQueryRangeEvidence } from './catalog-acceptance.mjs';
import { nativeCatalogReceipt, nativeCatalogDiagnosticLimit, assertWorkerNativeDiagnosticLimit } from './native-catalog-execution.mjs';
import { createCemStudioPreview } from '../src/preview.js';
import { resolve } from 'node:path';

const fixture = () => ({
    projects: [{ id: 'demo', resources: ['source'], configurations: [{ id: 'inspect', source: 'source', command: true }, { id: 'repair', source: 'source', command: true, kind: 'validation', expectedExit: 1, expectedValidation: { errorCode: 'demo.invalid' } }, { id: 'binary', source: 'source', binary: true }] }],
    receipts: [
        { project: 'demo', configuration: 'inspect', host: 'worker', phase: 'online', status: 'passed', exitCode: 0, diagnosticCodes: [], executed: true },
        { project: 'demo', configuration: 'repair', host: 'worker', phase: 'online', status: 'intentional-diagnostic', exitCode: 1, diagnosticCodes: ['demo.invalid'], executed: true },
        { project: 'demo', configuration: 'binary', host: 'worker', phase: 'online', status: 'policy-refusal', exitCode: null, diagnosticCodes: [], executed: false },
    ],
});
const check = value => assertCatalogConfigurationEvidence(value.projects, value.receipts, { host: 'worker', phase: 'online' });

test('all current catalog resources retain their declared text or compiled-binary presentation', async () => {
    const { projects } = await catalogAcceptanceInputs(resolve(import.meta.dirname, '../../..'));
    for (const project of projects) for (const resource of project.bundle.project.resources) {
        const preview = createCemStudioPreview({ bytes: project.bundle.contents[resource.id], contentType: resource.contentType });
        assert.equal(preview.kind === 'download', resource.contentType.split(';')[0].endsWith('+cem-bin'), `${project.id}/${resource.id}: ${resource.contentType}`);
    }
});

test('accounts for real execution, intentional diagnostics and a separate binary admission refusal', () => {
    assert.deepEqual(check(fixture()), { configurations: 3, executed: 2, passed: 1, verifiedOutputs: 0, intentionalDiagnostics: 1, knownLimits: 0, companionDiagnosticLimits: 0, policyRefusals: 1 });
});
for (const [name, mutate] of [
    ['missing configuration', data => data.receipts.pop()],
    ['duplicate configuration', data => data.receipts.push(data.receipts[0])],
    ['stale configuration', data => { data.receipts[0].configuration = 'old'; }],
    ['wrong host', data => { data.receipts[0].host = 'native'; }],
    ['wrong phase', data => { data.receipts[0].phase = 'offline'; }],
    ['missing execution witness', data => { delete data.receipts[0].executed; }],
    ['unexpected hard diagnostic despite exit zero', data => { data.receipts[0].diagnosticCodes = ['native.error']; }],
    ['unbounded known-limit claim', data => { data.receipts[0].status = 'known-limit'; }],
    ['missing expected diagnostic', data => { data.receipts[1].diagnosticCodes = []; }],
    ['wrong intentional diagnostic', data => { data.receipts[1].diagnosticCodes = ['other']; }],
    ['binary claimed executed', data => { data.receipts[2].executed = true; }],
    ['binary claimed passing', data => { data.receipts[2].status = 'passed'; data.receipts[2].exitCode = 0; }],
]) test(`rejects ${name}`, () => { const data = fixture(); mutate(data); assert.throws(() => check(data)); });

test('native binary header validation does not imply worker source execution', () => {
    const data = fixture();
    data.receipts = data.receipts.map(row => ({ ...row, host: 'native', ...(row.configuration === 'binary' ? { status: 'passed', exitCode: 0, executed: true } : {}) }));
    assert.equal(assertCatalogConfigurationEvidence(data.projects, data.receipts, { host: 'native' }).executed, 3);
});
test('Login element-template acceptance has no generic-source diagnostic exception', () => {
    const projects = [{ id: 'login', configurations: [{ id: 'live-view' }] }];
    assert.equal(nativeCatalogDiagnosticLimit('login', 'live-view', 'command-report'), undefined);
    const receipt = { project: 'login', configuration: 'live-view', host: 'native', status: 'passed', exitCode: 0, executed: true, diagnosticCodes: [] };
    assert.equal(assertCatalogConfigurationEvidence(projects, [receipt], { host: 'native' }).knownLimits, 0);
    const legacy = { ...receipt, status: 'known-limit', diagnosticCodes: Array(4).fill('cem.ql.compile_failed'), diagnosticOffsets: [196, 657, 1147, 1187] };
    assert.throws(() => assertCatalogConfigurationEvidence(projects, [legacy], { host: 'native' }));
    assert.throws(() => nativeCatalogReceipt('login', 'live-view', { operation: 'parse', diagnosticEvidence: 'command-report', exitCode: 0,
        diagnostics: legacy.diagnosticCodes.map((code, index) => ({ code, severity: 'error', byteOffset: legacy.diagnosticOffsets[index] })) }));
    const warnings = Array.from({ length: 15 }, () => ({ severity: 'warning', code: 'cem.schema.unknown_html_attribute' }));
    const passed = nativeCatalogReceipt('login', 'live-view', { operation: 'parse', diagnosticEvidence: 'command-report', exitCode: 0, stdout: 'native CEM', diagnostics: warnings },
        { errorCount: 0, warningCount: 15, warningCode: 'cem.schema.unknown_html_attribute' });
    assert.equal(passed.status, 'passed');
});
test('native success requires absence of hard diagnostics, independently of the process exit', () => {
    assert.throws(() => nativeCatalogReceipt('demo', 'parse', { exitCode: 0, stdout: 'partial native output',
        diagnostics: [{ severity: 'error', code: 'native.parse_failed' }] }));
});
test('native Inspect requires diagnostics from the original selected operation', () => {
    const projects = [{ id: 'demo', configurations: [{ id: 'inspect', argv: ['inspect'] }] }];
    const receipt = { project: 'demo', configuration: 'inspect', host: 'native', executed: true, exitCode: 0,
        status: 'passed', diagnosticCodes: [], diagnosticEvidence: 'command-report' };
    assert.equal(assertCatalogConfigurationEvidence(projects, [receipt], { host: 'native' }).passed, 1);
    assert.throws(() => assertCatalogConfigurationEvidence(projects, [{ ...receipt, diagnosticCodes: null }], { host: 'native' }));
    assert.throws(() => assertCatalogConfigurationEvidence(projects, [{ ...receipt, diagnosticEvidence: 'companion-source-diagnostics' }], { host: 'native' }));
    assert.throws(() => assertCatalogConfigurationEvidence(projects, [{ ...receipt, diagnosticProbe: {} }], { host: 'native' }));
});
test('native warning counts remain checked without relabeling successful source validation', () => {
    const outcome = { exitCode: 0, stdout: 'native output', diagnostics: [{ severity: 'warning', code: 'native.warning' }] };
    assert.equal(nativeCatalogReceipt('demo', 'validate', outcome, { warningCount: 1, warningCode: 'native.warning' }).status, 'passed');
    assert.throws(() => nativeCatalogReceipt('demo', 'validate', outcome, { warningCount: 0 }));
});
test('delivery and navigation must belong to the same catalog and exact deployed bytes', () => {
    const stamp = { catalogIdentity: 'a'.repeat(64), deployedSha256: 'b'.repeat(64) };
    assertAcceptanceStamp(stamp, stamp, 'delivery');
    assert.throws(() => assertAcceptanceStamp({}, stamp, 'delivery'));
    assert.throws(() => assertAcceptanceStamp({ ...stamp, deployedSha256: 'c'.repeat(64) }, stamp, 'delivery'));
    assert.throws(() => assertAcceptanceStamp({ ...stamp, catalogIdentity: 'c'.repeat(64) }, stamp, 'delivery'));
});
test('fresh native query owners preserve every source fact while opaque identities remain local', () => {
    const value = identity => ({ language: 'cem-ql', result: { diagnostics: [], error: null, items: [{ kind: 'native', representation: 'cem.ql.imported-cem-ast',
        identity, sourceMap: { frames: [{ source_id: 1, span: { kind: 'Single', ranges: { start: 2, len: 4 } }, transform: { kind: 'SourceBoundary', uri: 'studio://copy/input.json' } }] } }] } });
    const evidence = report => nativeQueryRangeEvidence(JSON.stringify(report), [{ start: 2, len: 4 }], 'studio://copy/input.json');
    assert.deepEqual(evidence(value('first owner')), evidence(value('fresh owner')));
    const changed = value('fresh owner'); changed.result.items[0].sourceMap.frames[0].span.ranges.len = 3;
    assert.throws(() => evidence(changed));
    assert.throws(() => nativeQueryRangeEvidence(JSON.stringify(value('fresh')), [{ start: 2, len: 4 }], 'studio://other/input.json'));
    const extra = value('fresh'); extra.result.items[0].newFact = 'retained';
    assert.notDeepEqual(evidence(extra), evidence(value('fresh')));
});
test('retired source diagnostic exceptions cannot classify fresh native results', () => {
    assert.equal(nativeCatalogDiagnosticLimit('supplier-pipeline', 'catalog-source', 'command-report'), undefined);
    for (const [project, configuration] of [['html-cem-card', 'inspect-component'], ['svg-diagram', 'inspect-svg'],
        ['mathml-formula', 'inspect-mathml'], ['xslt-transform', 'inspect-template']]) {
        assert.equal(nativeCatalogDiagnosticLimit(project, configuration, 'command-report'), undefined);
        assert.equal(nativeCatalogDiagnosticLimit(project, configuration, 'companion-source-diagnostics'), undefined);
    }
});
test('the remaining control-source diagnostic limit retains exact original coordinates and owned URIs', () => {
    const root = 'studio://copy/', limit = nativeCatalogDiagnosticLimit('portable-config', 'inspect-project-cem', 'command-report');
    const diagnostics = limit.diagnostics.map(facts => ({ ...facts, uri: `${root}${facts.uri}` }));
    const check = (items, uri = root) => assertWorkerNativeDiagnosticLimit('portable-config', 'inspect-project-cem', items, uri);
    assert.ok(check(diagnostics).length > 0);
    assert.throws(() => check(diagnostics, 'studio://other/'));
    const changed = structuredClone(diagnostics); changed[0].byteOffset++;
    assert.throws(() => check(changed));
    assert.throws(() => check([...diagnostics, diagnostics[0]]));
});
test('requires complete routes, clones, offline observations and unchanged originals', () => {
    const projects = fixture().projects;
    const receipt = { id: 'demo', resourceIds: ['source'], configurationIds: ['inspect', 'repair', 'binary'],
        savedProjectId: 'copy', rootUri: 'studio://copy/', offline: { archive: true, resourceIds: ['source'], configurationIds: ['inspect', 'repair', 'binary'] },
        collectionIds: [], folderPaths: [], configurations: [
            { id: 'inspect', sourceId: 'source', admission: 'editable-command', runEnabled: true, actions: ['available:Run configuration'] },
            { id: 'repair', sourceId: 'source', admission: 'editable-command', runEnabled: true, actions: ['available:Validate persisted revision'] },
            { id: 'binary', sourceId: 'source', admission: 'read-only-binary', runEnabled: false, actions: ['download-resource'] },
        ],
        actions: ['open-original-resources', 'open-original-configurations', 'clone-selected', 'open-saved-resources', 'open-saved-configurations'] };
    Object.assign(receipt.offline, { collectionIds: [], folderPaths: [] });
    const evidence = { projects: [receipt], unchangedOriginalProjectIds: ['demo'], draft: { persistedBytesUnchanged: true } };
    assert.deepEqual(assertCatalogNavigationEvidence(projects, evidence), { projects: 1, resources: 1, configurations: 3 });
    for (const change of [{ resourceIds: [] }, { configurationIds: ['inspect'] }, { offline: { ...receipt.offline, archive: false } }, { savedProjectId: 'demo' }, { actions: [] },
        { configurations: [] }, { collectionIds: ['stale'] }, { configurations: receipt.configurations.map(row => ({ ...row, sourceId: 'other' })) },
        { configurations: receipt.configurations.map(row => row.id === 'binary' ? { ...row, admission: 'editable-command', runEnabled: true } : row) }]) {
        assert.throws(() => assertCatalogNavigationEvidence(projects, { ...evidence, projects: [{ ...receipt, ...change }] }));
    }
    assert.throws(() => assertCatalogNavigationEvidence(projects, { ...evidence, unchangedOriginalProjectIds: [] }));
});
