import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright';
import { prepareComponentBenchmark } from './component-benchmark-deployment.mjs';
import { processStat, processFamily, smapsRollup, browserProcessMemory, distribution, summarizeStartup } from './browser-runtime-metrics.mjs';

const rollup = 'Rss: 20 kB\nPss: 8 kB\nPrivate_Clean: 2 kB\nPrivate_Dirty: 3 kB\nSwapPss: 0 kB\nShared_Hugetlb: 0 kB\nPrivate_Hugetlb: 0 kB\n';
const stat = (pid, parent, ticks = '17') => `${pid} (worker (name)) S ${parent} ${Array(17).fill('0').join(' ')} ${ticks} 0\n`;

test('process ownership uses root ancestry and lifetime, not global browser names', () => {
    assert.deepEqual(processStat(stat(12, 10)), { pid: 12, ppid: 10, startTicks: '17' });
    const family = processFamily([processStat(stat(99, 1)), processStat(stat(12, 10)), processStat(stat(10, 1)), processStat(stat(13, 12))], 10);
    assert.deepEqual(family.map(process => process.pid), [10, 12, 13]);
    assert.throws(() => processFamily([family[0], family[0]], 10), /duplicate/);
    assert.throws(() => processFamily(family, 88), /unavailable/);
    assert.throws(() => processStat('12 (truncated)'), /invalid/);
});

test('resident accounting keeps proportional/private bytes distinct from duplicate RSS and missing values', () => {
    assert.deepEqual(smapsRollup(rollup), { pssBytes: 8192, rssBytes: 20480, privateBytes: 5120, swapPssBytes: 0, explicitHugePageBytes: 0 });
    assert.throws(() => smapsRollup(rollup.replace('Pss: 8 kB\n', '')), /unavailable/);
    assert.throws(() => smapsRollup(rollup + 'Pss: 8 kB\n'), /duplicate/);
    assert.throws(() => smapsRollup(rollup.replace('Pss: 8', 'Pss: -1')), /unavailable/);
});

test('sampling refuses lost counters and PID reuse rather than publishing partial or zero totals', async () => {
    const root = await mkdtemp(join(tmpdir(), 'cem-browser-metrics-'));
    try {
        for (const [pid, parent] of [[10, 1], [12, 10], [99, 1]]) {
            await mkdir(join(root, String(pid)));
            await writeFile(join(root, String(pid), 'stat'), stat(pid, parent));
            await writeFile(join(root, String(pid), 'smaps_rollup'), rollup);
        }
        const ready = await browserProcessMemory(10, { procRoot: root });
        assert.equal(ready.status, 'available');
        assert.equal(ready.processCount, 2);
        assert.equal(ready.pssBytes, 16384);
        assert.equal(ready.rssSumBytes, 40960);
        await rm(join(root, '12', 'smaps_rollup'));
        const missing = await browserProcessMemory(10, { procRoot: root });
        assert.equal(missing.status, 'unavailable');
        assert.equal(missing.pssBytes, undefined);
        assert.equal(missing.attempts, 3);
        await writeFile(join(root, '12', 'smaps_rollup'), rollup);
        let reads = 0;
        const changed = await browserProcessMemory(10, { procRoot: root, read: async (path, ...args) => {
            if (path === join(root, '12', 'stat')) return stat(12, 10, String(++reads));
            return readFile(path, ...args);
        } });
        assert.equal(changed.status, 'unavailable');
        assert.match(changed.reason, /PID lifetime changed/);
        assert.equal(changed.pssBytes, undefined);
    } finally { await rm(root, { recursive: true, force: true }); }
});

test('descriptive summaries retain all samples and do not turn unavailable memory into zero', () => {
    assert.deepEqual(distribution([9, 1, 5]), { samples: 3, min: 1, median: 5, max: 9 });
    assert.equal(distribution([1, 5]).median, 3);
    assert.throws(() => distribution([NaN]), /finite/);
    const rows = [1, 2, 3].map(index => ({ kind: 'selected', mode: 'worker', browserLaunchMs: index,
        readyMs: index * 10, firstUseMs: index, coldMemory: { browserFamily: { pssBytes: index === 2 ? undefined : 100 }, activeWorkers: 1 },
        afterMemory: { browserFamily: { pssBytes: 120 }, activeWorkers: 1 }, traffic: { wasmRequests: 1 } }));
    const [result] = summarizeStartup(rows);
    assert.equal(result.samples, 3);
    assert.equal(result.readyMs.median, 20);
    assert.equal(result.readyPssBytes.status, 'unavailable');
    assert.equal(result.readyPssBytes.availableSamples, 2);
    assert.equal(result.afterPssBytes.median, 120);
});

test('matched benchmark assembly deduplicates runtime assets and refuses drift before publication', async () => {
    const temporary = await mkdtemp(join(tmpdir(), 'cem-benchmark-deployment-'));
    const root = join(temporary, 'input'), destination = join(temporary, 'output');
    const core = 'packages/cem_ql/dist/wasm', vendor = 'packages/cem-elements/dist/lib/internal/runtime-support/vendor';
    try {
        for (const path of [core, vendor, 'packages/cem-components/dist', 'packages/cem-components/src/components', 'packages/cem-theme/dist/lib/css']) {
            await mkdir(join(root, path), { recursive: true });
        }
        for (const path of [core, vendor]) for (const name of ['cem_ql.js', 'cem_ql_bg.wasm', 'cem_ql_factory.js', 'cem_ql_code.js']) {
            await writeFile(join(root, path, name), name);
        }
        const prepared = await prepareComponentBenchmark(root, destination);
        const inventory = prepared.inventory.files;
        assert.equal(inventory.filter(file => file.path.endsWith('.wasm')).length, 1);
        assert.equal(inventory.find(file => file.path.endsWith('.wasm')).path, core + '/cem_ql_bg.wasm');
        const shim = await readFile(join(destination, vendor, 'cem_ql_factory.js'), 'utf8');
        assert.match(shim, /\.\.\/cem_ql\/dist\/wasm\/cem_ql_factory.js/);
        const before = await readFile(join(destination, 'demo-deployment.json'));
        await writeFile(join(root, vendor, 'cem_ql_bg.wasm'), 'foreign');
        await assert.rejects(prepareComponentBenchmark(root, destination), /mixed benchmark glue\/binary cohort/);
        assert.deepEqual(await readFile(join(destination, 'demo-deployment.json')), before);
    } finally { await rm(temporary, { recursive: true, force: true }); }
});

test('real browser family includes touched worker WASM pages and observes worker termination', { skip: process.platform !== 'linux' }, async () => {
    const server = await chromium.launchServer({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
    const browser = await chromium.connect(server.wsEndpoint());
    try {
        const context = await browser.newContext(), page = await context.newPage();
        const before = await browserProcessMemory(server.process().pid);
        assert.equal(before.status, 'available', before.reason);
        assert.equal(page.workers().length, 0);
        await page.evaluate(() => new Promise(resolve => {
            const script = `globalThis.memory = new WebAssembly.Memory({initial:512});
                const pages = new Uint8Array(memory.buffer); for(let n=0;n<pages.length;n+=4096) pages[n]=1;
                postMessage(memory.buffer.byteLength);`;
            window.memoryWorker = new Worker(URL.createObjectURL(new Blob([script], { type: 'text/javascript' })));
            memoryWorker.onmessage = event => resolve(event.data);
        })).then(capacity => assert.equal(capacity, 32 * 1024 ** 2));
        assert.equal(page.workers().length, 1);
        const after = await browserProcessMemory(server.process().pid);
        assert.equal(after.status, 'available', after.reason);
        assert(after.pssBytes - before.pssBytes > 24 * 1024 ** 2, 'touched WASM pages must appear in family resident memory');
        await page.evaluate(() => memoryWorker.terminate());
        await page.waitForFunction(() => true);
        await new Promise(done => setImmediate(done));
        if (page.workers().length) await page.workers()[0].waitForEvent('close');
        assert.equal(page.workers().length, 0);
        await context.close();
    } finally { await browser.close(); await server.close(); }
});
