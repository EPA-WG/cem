// Read-only benchmark counters. No runtime instrumentation or memory-pressure policy.
import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

export function processStat(source) {
    const end = source.lastIndexOf(') ');
    const pid = Number(source.slice(0, source.indexOf(' ')));
    assert(Number.isSafeInteger(pid) && pid > 0 && end > 0, 'invalid process identity');
    const fields = source.slice(end + 2).trim().split(/\s+/);
    const ppid = Number(fields[1]), startTicks = fields[19];
    assert(Number.isSafeInteger(ppid) && ppid >= 0 && /^\d+$/.test(startTicks), 'invalid process lifetime');
    return { pid, ppid, startTicks };
}

export function processFamily(records, rootPid) {
    const byPid = new Map(records.map(record => [record.pid, record]));
    assert.equal(byPid.size, records.length, 'duplicate process identity');
    assert(byPid.has(rootPid), 'browser process is unavailable');
    const family = [byPid.get(rootPid)], visited = new Set([rootPid]);
    for (const parent of family) for (const child of records) {
        if (child.ppid === parent.pid && !visited.has(child.pid)) {
            family.push(child); visited.add(child.pid);
        }
    }
    return family.sort((a, b) => a.pid - b.pid);
}

export function smapsRollup(source) {
    const counters = new Map();
    for (const line of source.split('\n')) {
        const match = /^(Rss|Pss|Private_Clean|Private_Dirty|SwapPss|Shared_Hugetlb|Private_Hugetlb):\s+(\d+) kB$/.exec(line.trim());
        if (!match) continue;
        assert(!counters.has(match[1]), 'duplicate memory counter');
        const bytes = Number(match[2]) * 1024;
        assert(Number.isSafeInteger(bytes), 'invalid memory counter');
        counters.set(match[1], bytes);
    }
    for (const field of ['Rss', 'Pss', 'Private_Clean', 'Private_Dirty', 'SwapPss']) {
        assert(counters.has(field), `unavailable memory counter: ${field}`);
    }
    return {
        pssBytes: counters.get('Pss'), rssBytes: counters.get('Rss'),
        privateBytes: counters.get('Private_Clean') + counters.get('Private_Dirty'),
        swapPssBytes: counters.get('SwapPss'),
        explicitHugePageBytes: counters.has('Shared_Hugetlb') && counters.has('Private_Hugetlb')
            ? counters.get('Shared_Hugetlb') + counters.get('Private_Hugetlb') : null,
    };
}

export async function browserProcessMemory(rootPid, { procRoot = '/proc', attempts = 3, read = readFile } = {}) {
    const started = performance.now();
    const snapshot = async () => {
        const entries = await readdir(procRoot);
        const records = await Promise.all(entries.filter(name => /^\d+$/.test(name)).map(async name => {
            try { return processStat(await read(join(procRoot, name, 'stat'), 'utf8')); }
            catch (error) { if (['ENOENT', 'ESRCH'].includes(error.code)) return null; throw error; }
        }));
        return processFamily(records.filter(Boolean), rootPid);
    };
    let reason;
    for (let attempt = 1; attempt <= attempts; attempt++) {
        try {
            const family = await snapshot();
            const processes = await Promise.all(family.map(async identity => ({ ...identity,
                ...smapsRollup(await read(join(procRoot, String(identity.pid), 'smaps_rollup'), 'utf8')),
            })));
            assert.deepEqual(await snapshot(), family, 'process family or PID lifetime changed during sampling');
            const sum = field => processes.reduce((total, process) => total + process[field], 0);
            return { status: 'available', method: 'Linux smaps_rollup PSS across the browser process family',
                pssBytes: sum('pssBytes'), privateBytes: sum('privateBytes'), swapPssBytes: sum('swapPssBytes'),
                rssSumBytes: sum('rssBytes'), // Diagnostic only: shared pages can be counted repeatedly.
                explicitHugePageBytes: processes.every(process => process.explicitHugePageBytes !== null) ? sum('explicitHugePageBytes') : null,
                processCount: processes.length, attempts: attempt, durationMs: performance.now() - started, processes };
        } catch (error) { reason = error.message; }
    }
    return { status: 'unavailable', reason, attempts, durationMs: performance.now() - started };
}

export function distribution(values) {
    assert(values.length > 0 && values.every(Number.isFinite), 'finite, nonempty observations required');
    const sorted = [...values].sort((a, b) => a - b), middle = Math.floor(sorted.length / 2);
    return { samples: sorted.length, min: sorted[0], median: sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2,
        max: sorted.at(-1) };
}

export function summarizeStartup(results) {
    const groups = new Map();
    for (const row of results) {
        const key = `${row.kind}/${row.mode}`;
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(row);
    }
    const metric = (rows, value) => {
        const values = rows.map(value);
        return values.every(Number.isFinite) ? { status: 'available', ...distribution(values) }
            : { status: 'unavailable', samples: rows.length, availableSamples: values.filter(Number.isFinite).length,
                reason: 'one or more observations unavailable; no partial-sample aggregate' };
    };
    const capacity = native => native?.status === 'available' ? native.linearMemoryBytes ?? native.sample?.sample?.linearMemoryBytes : undefined;
    const live = native => native?.status === 'available' ? native.sample?.sample?.allocatorLiveBytes : undefined;
    return [...groups.values()].map(rows => ({ kind: rows[0].kind, mode: rows[0].mode, samples: rows.length,
        browserLaunchMs: metric(rows, row => row.browserLaunchMs), readyMs: metric(rows, row => row.readyMs),
        firstUseMs: metric(rows, row => row.firstUseMs),
        blankPssBytes: metric(rows, row => row.blankMemory?.pssBytes),
        readyPssBytes: metric(rows, row => row.coldMemory.browserFamily.pssBytes),
        afterPssBytes: metric(rows, row => row.afterMemory.browserFamily.pssBytes),
        afterMainJsBytes: metric(rows, row => row.afterMemory.mainThreadJsHeapUsedBytes),
        readyNativeCapacityBytes: metric(rows, row => capacity(row.coldMemory.native)),
        afterNativeCapacityBytes: metric(rows, row => capacity(row.afterMemory.native)),
        readyNativeLiveBytes: metric(rows, row => live(row.coldMemory.native)),
        afterNativeLiveBytes: metric(rows, row => live(row.afterMemory.native)),
        readyWorkers: metric(rows, row => row.coldMemory.activeWorkers),
        afterWorkers: metric(rows, row => row.afterMemory.activeWorkers),
        wasmRequests: metric(rows, row => row.traffic.wasmRequests),
    }));
}

export function renderStartupReport(report) {
    const number = (metric, divisor = 1) => metric.status !== 'available' ? 'unavailable'
        : `${(metric.median / divisor).toFixed(divisor === 1 ? 0 : 1)} [${(metric.min / divisor).toFixed(divisor === 1 ? 0 : 1)}–${(metric.max / divisor).toFixed(divisor === 1 ? 0 : 1)}]`;
    const mib = 1024 ** 2, suffix=report.recordSuffix??'';
    return `# Runtime cold-start and memory measurements

Generated with \`yarn nx run cem_ql:measure:browser-startup -- --runs=${report.runs}${suffix?' --record=code-sharing':''} --write-doc\`.
Committed per-run evidence: [cem-components-wasm-startup${suffix}.json](cem-components-wasm-startup${suffix}.json).
Full local observations, phase traffic, process identities and native samples:
\`dist/reports/components-wasm/browser-startup${suffix}.json\`.

## Method and scope

- The Nx target stages a separate benchmark deployment from already built bindings, canonical declarations and CSS. It refuses mismatched package/core glue and binary bytes. No demo/Studio output is rebuilt or overwritten. Each observation launches a fresh Chromium process, temporary profile and isolated context. Browser/network/code caches are fresh; OS file caches and the local fixture server can be warm. Browser launch is recorded separately from navigation-to-readiness.
- ${report.runs} observations per workload/mode; tables show median [minimum–maximum], not a percentile or performance budget. Workload/mode order rotates between runs. No CPU/network throttling is applied.
- Selected entry measures the pinned compiled cart/JSON API in one dedicated worker or one main-thread heap, with the first guarded response rendered natively. It has no DOM/control host. Full controls, JSON/CSV data view and editor use the existing complete runtime and canonical components; these are separate workloads, not a reduced whole-application comparison.
- Readiness/first use use the original control/render assertions. Timings include browser automation and local HTTP overhead. Memory snapshots run outside timed intervals; they still add diagnostic work between readiness and first use. No explicit GC, heap enumeration or production instrumentation is installed.
- Combined resident memory is the sum of Linux \`smaps_rollup\` **PSS** for the browser and its surviving descendants, including renderer/worker/WASM, GPU and network processes. PSS apportions shared resident pages ([kernel definition](https://docs.kernel.org/filesystems/proc.html)). Process identities and membership must match before/after each snapshot; missing/unstable counters are unavailable. Sampling spans multiple reads, not an atomic peak or an application-only allocation total. Node/Playwright/server memory, detached non-descendants and explicit huge pages outside PSS are excluded. Per-process private bytes, swap PSS and huge-page counters remain in the raw report.
- Main-thread JS heap and available native allocator/linear-memory capacity are separate diagnostics. **Do not add them to PSS** or treat reserved WASM capacity as resident/allocated bytes. Worker count means live dedicated workers observed by Playwright, separate from creation/job counters and OS processes.
- The local server uses no-store responses. Decoded and wire bodies, unique assets and repeated transfers are recorded separately against served-file/inventory hashes. No compressed-CDN, offline/all-capabilities or isolated npm-consumer claim is made.

Chromium: ${report.chromium}; Node: ${report.node}; platform: ${report.platform}.

| Artifact | SHA-256 |
| --- | --- |
| Full reference WASM | \`${report.coreSha256}\` |
| Selected cart WASM | \`${report.selectedSha256}\` |

## Repeated observations

Times are milliseconds. Worker counts are live dedicated workers, not native heaps or OS process counts.

| Workload | Mode | Samples | Browser launch ms | Ready ms | First use ms | Live workers ready / after | WASM requests |
| --- | --- | ---: | --- | --- | --- | --- | --- |
${report.summary.map(row => `| ${row.kind} | ${row.mode} | ${row.samples} | ${number(row.browserLaunchMs)} | ${number(row.readyMs)} | ${number(row.firstUseMs)} | ${number(row.readyWorkers)} / ${number(row.afterWorkers)} | ${number(row.wasmRequests)} |`).join('\n')}

## Combined resident memory and separate native diagnostics

All sizes below are MiB. PSS includes host and native resident pages across the whole browser family. The blank-page snapshot precedes application navigation. Native counters cover only the selected entry heap or the selected fallback facade; their scope is smaller than the browser family. Worker allocator/capacity counters remain unavailable in the full host. The selected ABI exposes capacity but not allocator-live bytes.

| Workload | Mode | Blank PSS | Ready PSS | After use PSS | Main JS after use | Native capacity ready / after | Native allocator live ready / after |
| --- | --- | --- | --- | --- | --- | --- | --- |
${report.summary.map(row => `| ${row.kind} | ${row.mode} | ${number(row.blankPssBytes, mib)} | ${number(row.readyPssBytes, mib)} | ${number(row.afterPssBytes, mib)} | ${number(row.afterMainJsBytes, mib)} | ${number(row.readyNativeCapacityBytes, mib)} / ${number(row.afterNativeCapacityBytes, mib)} | ${number(row.readyNativeLiveBytes, mib)} / ${number(row.afterNativeLiveBytes, mib)} |`).join('\n')}

## Remaining measurements

Keep the [historical single-context baseline](cem-components-wasm-baseline.md) separate. These observations do not establish causation from binary-size reduction, an OOM threshold, a peak or a complete reduced live-component distribution. ${suffix?'The measured code-sharing path acquires WASM once per workload. Measure':'Diagnose repeated full-WASM acquisition, then measure'} all-capabilities/offline totals and clean npm consumers before selecting fixed budgets. Full dynamic/native link closure and optional-module ABI/loader work remain separate gates.
`;
}
