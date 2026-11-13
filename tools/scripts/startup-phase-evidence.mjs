import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

export function startupPhaseEvidence(report, rawSha256) {
    assert(report.phaseCohort && !report.summary.length, 'only diagnostic phase records may be projected');
    const { results, methods, ...metadata } = report;
    return { ...metadata, contract: 'cem-startup-phase-evidence/1', rawSha256,
        limits: ['Diagnostic timings are excluded from ordinary latency/PSS distributions.',
            'Visibility witnesses supplement the unchanged original role/locator assertions.',
            'API spans overlap and are not additive CPU counters; worker-internal API calls are not captured.'],
        sourceHashes: Object.fromEntries(results.flatMap(row => row.responses.filter(response => /\.(?:m?js)(?:\?|$)/.test(response.path))
            .map(response => [response.path, response.sha256]))),
        results: results.map(({ kind, mode, run, readyMs, firstUseMs, startupEvidence, workerCounters, traffic }) => {
            assert.equal(startupEvidence.overflow, false);
            assert(Number.isFinite(startupEvidence.visibleAt) && startupEvidence.assertionAt >= startupEvidence.visibleAt);
            const { resources, ...phases } = startupEvidence;
            return { kind, mode, run, readyMs, firstUseMs, workerCounters, traffic,
                startupEvidence: { ...phases, resourceCount: resources.length,
                    lastResourceEnd: Math.max(0, ...resources.map(resource => resource.responseEnd)),
                    lastScriptEnd: Math.max(0, ...resources.filter(resource => /\.(?:m?js)$/.test(resource.path)).map(resource => resource.responseEnd)),
                    wasmResources: resources.filter(resource => resource.path.endsWith('.wasm')) } };
        }) };
}

if (process.argv[1] && resolve(process.argv[1]) === import.meta.filename) {
    const root = resolve(import.meta.dirname, '../..');
    const hash = bytes => createHash('sha256').update(bytes).digest('hex');
    const playwright = { version: JSON.parse(await readFile(resolve(root, 'node_modules/playwright-core/package.json'))).version,
        serverBundleSha256: hash(await readFile(resolve(root, 'node_modules/playwright-core/lib/coreBundle.js'))) };
    const projectorSha256 = hash(await readFile(import.meta.filename));
    for (const cohort of ['before-sharing', 'code-sharing', 'before-sharing-observe-only', 'code-sharing-observe-only']) {
        const bytes = await readFile(resolve(root, `dist/reports/components-wasm/browser-startup-phases-${cohort}.json`));
        const evidence = { ...startupPhaseEvidence(JSON.parse(bytes), hash(bytes)), playwright, projectorSha256 };
        const path = `docs/cem-components-wasm-startup-phases-${cohort}.json`;
        await writeFile(resolve(root, path), `${JSON.stringify(evidence, null, 2)}\n`);
        console.log(`Projected ${evidence.results.length} phase observations to ${path}`);
    }
}
