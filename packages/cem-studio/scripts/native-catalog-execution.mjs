import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

/** Named acceptance policy; this does not select a parser, importer or runtime command. */
export const CATALOG_NATIVE_DIAGNOSTICS_POLICY = Object.freeze(JSON.parse(readFileSync(
    new URL('./fixtures/catalog-native-diagnostics.json', import.meta.url), 'utf8')));

export function nativeCatalogDiagnosticLimit(project, configuration, diagnosticEvidence) {
    const rows = diagnosticEvidence === 'companion-source-diagnostics'
        ? CATALOG_NATIVE_DIAGNOSTICS_POLICY.inspectorDiagnosticLimits
        : CATALOG_NATIVE_DIAGNOSTICS_POLICY.actualSourceLimits;
    return rows.find(row => row.project === project && row.configuration === configuration);
}

/** Execute the native command and retain diagnostics through named native JSON exports. */
export async function executeNativeCatalogCommand(cli, args, directory, expectedExit = 0, encoding = 'utf8', diagnosticReportPath) {
    const operation = args[0];
    const reportIndex = args.indexOf('--report-json');
    const reportPath = reportIndex < 0 ? diagnosticReportPath ?? resolve(directory, 'catalog-command.report.json') : args[reportIndex + 1];
    const reported = operation !== 'trace';
    const invocation = reported && reportIndex < 0 ? [...args, '--report-json', reportPath] : args;
    const actual = execute(cli, invocation, directory, expectedExit, encoding);
    const report = operation === 'trace' ? JSON.parse(actual.stdout).report
        : JSON.parse(await readFile(reportPath, 'utf8'));
    assert.ok(Array.isArray(report?.diagnostics), `${operation} must publish its native diagnostics`);
    return { ...actual, operation, report, diagnostics: report.diagnostics, diagnosticEvidence: 'command-report' };
}

/** Receipts name observed native outcomes; a default zero Parse exit is not conformance. */
export function nativeCatalogReceipt(project, configuration, outcome, expected = {}) {
    const hard = outcome.diagnostics.filter(item => ['error', 'fatal'].includes(item.severity));
    const warnings = outcome.diagnostics.filter(item => item.severity === 'warning');
    const limit = nativeCatalogDiagnosticLimit(project, configuration, outcome.diagnosticEvidence);
    if (limit) {
        assert.equal(outcome.operation, limit.operation, `${project}/${configuration} diagnostic operation drifted`);
        assert.equal(outcome.diagnosticEvidence, limit.diagnosticEvidence, `${project}/${configuration} diagnostic evidence drifted`);
        assert.equal(outcome.exitCode, limit.exitCode);
        assert.deepEqual(outcome.diagnostics.map(item => diagnosticFacts(item, limit.sourceMapFacts)), limit.diagnostics,
            `${project}/${configuration} pinned native diagnostic evidence drifted`);
    } else {
        assert.equal(hard.length, expected.errorCount ?? 0, `${project}/${configuration} native hard diagnostics drifted`);
        assert.notEqual(expected.knownLimit, true, `${project}/${configuration} has no pinned native diagnostic limit`);
    }
    if (expected.warningCount !== undefined) assert.equal(warnings.length, expected.warningCount, `${project}/${configuration} native warnings drifted`);
    if (expected.errorCode) assert.ok(hard.some(item => item.code === expected.errorCode), `${project}/${configuration} misses ${expected.errorCode}`);
    if (expected.warningCode) assert.ok(warnings.some(item => item.code === expected.warningCode), `${project}/${configuration} misses ${expected.warningCode}`);
    const receipt = { project, configuration, host: 'native', executed: true,
        operation: outcome.operation, diagnosticEvidence: outcome.diagnosticEvidence,
        status: limit ? 'known-limit' : outcome.exitCode !== 0 ? 'intentional-diagnostic' : 'passed',
        exitCode: outcome.exitCode, diagnosticCodes: hard.map(item => item.code),
        outputSha256: createHash('sha256').update(outcome.stdout).digest('hex') };
    if (outcome.operation === 'inspect') assert.equal(outcome.diagnosticEvidence, 'command-report');
    if (limit) {
        Object.assign(receipt, { diagnosticOffsets: hard.map(item => item.byteOffset),
            diagnosticSource: [...new Set(hard.map(item => item.uri))].length === 1 ? hard[0].uri : undefined });
    }
    return receipt;
}

function diagnosticFacts(item, sourceMapFacts) {
    const facts = Object.fromEntries(['code', 'severity', 'uri', 'byteOffset', 'line', 'column'].map(key => [key, item[key] ?? null]));
    if (sourceMapFacts) facts.sourceMapFacts = item.sourceMap === undefined || item.sourceMap === null
        ? null : item.sourceMap.frames.map(({ span, transform }) => ({ span, transform }));
    return facts;
}

/** Compare a pinned original-operation limit through an owned Studio source binding. */
export function assertWorkerNativeDiagnosticLimit(project, configuration, diagnostics, projectRootUri) {
    const root = new URL(projectRootUri);
    assert.equal(root.protocol, 'studio:'); assert.equal(root.pathname, '/');
    assert.equal(root.username, ''); assert.equal(root.password, '');
    const limit = nativeCatalogDiagnosticLimit(project, configuration, 'command-report');
    assert.ok(limit, 'Only an explicitly pinned native operation can be a known limit');
    assert.equal(diagnostics.length, limit.diagnostics.length);
    const relativeFacts = diagnostics.map((item, index) => {
        assert.equal(item.uri, `${projectRootUri}${limit.diagnostics[index].uri}`);
        const facts = diagnosticFacts(item, limit.sourceMapFacts);
        const normalizeOwnedUris = value => typeof value === 'string' && value.startsWith(projectRootUri)
            ? value.slice(projectRootUri.length) : Array.isArray(value) ? value.map(normalizeOwnedUris)
                : value && typeof value === 'object' ? Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, normalizeOwnedUris(entry)])) : value;
        return normalizeOwnedUris(facts);
    });
    assert.deepEqual(relativeFacts, limit.diagnostics, 'Owned worker operation diagnostic facts drifted');
    return diagnostics.filter(item => ['error', 'fatal'].includes(item.severity)).map(item => item.byteOffset);
}

function execute(cli, args, cwd, expectedExit, encoding) {
    const result = spawnSync(cli, args, { cwd, encoding, maxBuffer: 16 * 1024 * 1024 });
    if (result.error) throw result.error;
    assert.equal(result.status, expectedExit, `${cli} ${args.join(' ')}: ${result.stderr || result.stdout}`);
    return { exitCode: result.status, stdout: result.stdout };
}
