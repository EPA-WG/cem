import assert from 'node:assert/strict';
import { binding, call } from './application-shared-stores.mjs';

function grant(page, id, options) {
    return page.evaluateHandle(({ id, options }) => window.grantNamedStore(id, options), { id, options });
}

function collect(store) {
    return store.evaluateHandle(async store => {
        const events = [];
        const stop = await store.subscribe(event => events.push(event));
        return { events, stop };
    });
}

async function through(frame, collector, value) {
    await frame.waitForFunction(([log, value]) => log.events.some(e => e.newValue === value), [collector, value],
        { timeout: 10000 });
    return collector.evaluate(log => log.events);
}

function change(scope, key, oldValue, newValue) {
    const type = key === null ? 'clear' : newValue === null ? 'remove' : 'set';
    return { scope, type, key, oldValue, newValue };
}

export async function verifySubscriptions(page, editors, other, peers) {
    await verifyDurableUpgrade(page, editors);
    for (const kind of ['local', 'session']) {
        const options = { kind, storeId: 'announcements', read: true, write: false };
        const writer = { ...options, scope: 'edit', write: true };
        const preview = { ...options, scope: 'preview', keys: ['format', 'barrier'], subscribe: true };
        const audit = { ...options, scope: 'all', subscribe: true };
        await grant(page, 'A', writer);
        await grant(page, 'A', audit);
        await grant(page, 'B', audit);
        const lease = await grant(page, 'B', preview);
        await grant(other, 'A', writer);
        await grant(other, 'B', audit);
        const a = await binding(editors.A, 'A', 'edit');
        const remote = await binding(peers.A, 'A', 'edit');
        const restricted = await binding(editors.B, 'B', 'preview');
        const full = await binding(editors.B, 'B', 'all');
        const remoteLog = await collect(await binding(peers.B, 'B', 'all'));
        const selfLog = await collect(await binding(editors.A, 'A', 'all'));
        await assert.rejects(collect(a), /subscription is not granted/);
        await assert.rejects(grant(page, 'A', { ...preview, read: false, write: true }), /Invalid shared store grant/);

        // Subscription starts at the current commit. A private key stays invisible even
        // when a whole-store clear removes it. A later clear touches a readable key.
        await call(a, 'setItem', 'secret', 'unpublished');
        const subsetLog = await collect(restricted);
        const fullLog = await collect(full);
        await call(a, 'clear');
        await call(a, 'clear');
        await call(a, 'removeItem', 'format');
        await call(a, 'setItem', 'format', 'start');
        await call(a, 'setItem', 'format', 'start');
        await assert.rejects(call(a, 'setItem', 'format', 'conflict', 'stale'), { name: 'ConstraintError' });
        const secondWriter = kind === 'local' ? remote : a;
        await Promise.all([call(a, 'setItem', 'format', 'left'), call(secondWriter, 'setItem', 'format', 'right')]);
        const last = await call(a, 'getItem', 'format');
        const first = last === 'left' ? 'right' : 'left';
        await call(a, 'removeItem', 'format');
        await call(a, 'setItem', 'secret', 'hidden');
        await call(a, 'setItem', 'format', '');
        await call(a, 'clear');
        await call(a, 'setItem', 'barrier', 'done');

        const common = [['format', null, 'start'], ['format', 'start', first], ['format', first, last],
            ['format', last, null]];
        const end = [['format', null, ''], [null, null, null], ['barrier', null, 'done']];
        const expected = (scope, rows) => rows.map(args => change(scope, ...args));
        const fullRows = [[null, null, null], ...common, ['secret', null, 'hidden'], ...end];
        assert.deepEqual(await through(editors.B, subsetLog, 'done'), expected('preview', [...common, ...end]));
        assert.deepEqual(await through(editors.B, fullLog, 'done'), expected('all', fullRows));

        await call(remote, 'setItem', 'barrier', 'remote');
        const remoteEvents = await through(peers.B, remoteLog, 'remote');
        if (kind === 'local') {
            assert.deepEqual(remoteEvents, expected('all', [['secret', null, 'unpublished'], ...fullRows, ['barrier', 'done', 'remote']]));
            await through(editors.B, fullLog, 'remote');
            const rightWrite = common.find(row => row[2] === 'right');
            assert.deepEqual(await through(editors.A, selfLog, 'remote'), expected('all', [rightWrite, ['barrier', 'done', 'remote']]));
        } else {
            assert.deepEqual(remoteEvents, [change('all', 'barrier', null, 'remote')]);
            assert.deepEqual(await selfLog.evaluate(log => log.events), []);
        }

        await fullLog.evaluate(log => log.stop());
        // Two callbacks are enrolled on the same grant before commit. Whichever is
        // delivered first asks the host to revoke it. The other must never run.
        await page.evaluate(lease => { window.revokePreview = () => lease.revoke(); }, lease);
        const cut = await restricted.evaluateHandle(async store => {
            const seen = [];
            const receive = event => { seen.push(event); window.top.revokePreview(); };
            await store.subscribe(receive);
            await store.subscribe(receive);
            return seen;
        });
        await call(a, 'setItem', 'format', 'cut');
        await editors.B.waitForFunction(log => log.length > 0, cut);
        await assert.rejects(collect(restricted), /expired/);
        await grant(page, 'B', { ...preview, keys: ['barrier'] });
        const replacement = await binding(editors.B, 'B', 'preview');
        const resumed = await collect(replacement);
        await call(a, 'setItem', 'format', 'after revoke');
        await call(a, 'setItem', 'barrier', 'resumed');
        const oldBarrier = kind === 'local' ? 'remote' : 'done';
        assert.deepEqual(await through(editors.B, resumed, 'resumed'), [change('preview', 'barrier', oldBarrier, 'resumed')]);
        assert.deepEqual(await cut.jsonValue(), [change('preview', 'format', null, 'cut')]);
        assert.deepEqual(await fullLog.evaluate(log => log.events), expected('all',
            kind === 'local' ? [...fullRows, ['barrier', 'done', 'remote']] : fullRows));
        // Replacing a live grant also withdraws its observers. A marker on another
        // grant establishes that delivery passed the replacement point.
        await grant(page, 'B', { ...preview, subscribe: false });
        const witness = await collect(full);
        await call(a, 'setItem', 'barrier', 'after downgrade');
        await through(editors.B, witness, 'after downgrade');
        assert.equal(await resumed.evaluate(log => log.events.length), 1);
        for (const log of [witness, subsetLog, remoteLog, selfLog]) await log.evaluate(log => log.stop());
        await grant(page, 'B', preview);
        const recovering = await binding(editors.B, 'B', 'preview');
        if (kind === 'local') await verifyJournalRecovery(a, recovering, editors.B);
        else await verifySessionReconciliation(page, a, recovering, editors.B);
    }
}

async function verifyDurableUpgrade(page, editors) {
    const storeId = 'before-subscriptions';
    await page.evaluate(async storeId => {
        // The previous release stored the epoch and entries in version 1 of this database.
        const url = new URL(location.href);
        url.hash = ''; url.search = '';
        const path = [['page', url.href], ['id', 'shell']];
        const opening = indexedDB.open('cem.application.shared/1:' + JSON.stringify([path, storeId]), 1);
        opening.onupgradeneeded = () => {
            const entries = opening.result.createObjectStore('entries');
            entries.createIndex('present', 'present');
            entries.add('old epoch', 0);
            entries.add({ value: 'existing', stamp: 'old stamp', present: 1 }, 'format');
        };
        await new Promise((resolve, reject) => {
            opening.onsuccess = () => { opening.result.close(); resolve(); };
            opening.onerror = () => reject(opening.error);
        });
    }, storeId);
    const options = { storeId, scope: 'upgrade', read: true, write: true };
    await grant(page, 'A', options);
    await grant(page, 'B', { ...options, write: false, subscribe: true });
    const writer = await binding(editors.A, 'A', 'upgrade');
    const log = await collect(await binding(editors.B, 'B', 'upgrade'));
    const version = JSON.stringify(['old epoch', 'format', 'old stamp']);
    assert.deepEqual(await call(writer, 'read', 'format'), { value: 'existing', version });
    await call(writer, 'setItem', 'format', 'upgraded', version);
    assert.deepEqual(await through(editors.B, log, 'upgraded'), [change('upgrade', 'format', 'existing', 'upgraded')]);
    await log.evaluate(log => log.stop());
}

async function verifyJournalRecovery(writer, reader, frame) {
    // Make the broadcast transport unavailable during enrollment and manually tick
    // the fallback poll. Writes still go through the public application store.
    const log = await reader.evaluateHandle(async store => {
        const host = window.top;
        const originalChannel = host.BroadcastChannel, originalInterval = host.setInterval;
        let poll;
        try {
            host.BroadcastChannel = class { constructor() { throw Error('Unavailable transport'); } };
            host.setInterval = fn => { poll = fn; return -1; };
            const events = [];
            const stop = await store.subscribe(event => events.push(event));
            return { events, poll, stop };
        } finally { host.BroadcastChannel = originalChannel; host.setInterval = originalInterval; }
    });
    await writer.evaluate(async store => {
        for (let i = 0; i < 65; i++) await store.setItem('secret', String(i));
        await store.setItem('format', 'caught up');
    });
    assert.equal(await log.evaluate(log => log.events.length), 0);
    await log.evaluate(log => log.poll());
    await frame.waitForFunction(log => log.events.length > 0, log);
    const reset = { ...change('preview', null, null, null), type: 'invalidate',
        snapshot: [['barrier', 'after downgrade'], ['format', 'caught up']] };
    assert.deepEqual(await log.evaluate(log => log.events), [reset]);
    await call(writer, 'setItem', 'format', 'next commit');
    await log.evaluate(log => log.poll());
    assert.deepEqual(await through(frame, log, 'next commit'), [reset, change('preview', 'format', 'caught up', 'next commit')]);
    await log.evaluate(log => log.stop());
}

async function verifySessionReconciliation(page, writer, reader, frame) {
    const log = await collect(reader);
    // A native clear in another document of this test tab is reported to the page
    // provider. The subscriber receives its readable snapshot, then normal commits.
    await page.evaluate(() => {
        const child = document.body.appendChild(document.createElement('iframe'));
        try { child.contentWindow.sessionStorage.clear(); } finally { child.remove(); }
    });
    await frame.waitForFunction(log => log.events.length > 0, log);
    await call(writer, 'setItem', 'format', 'new session value');
    assert.deepEqual(await through(frame, log, 'new session value'), [
        { ...change('preview', null, null, null), type: 'invalidate', snapshot: [] },
        change('preview', 'format', null, 'new session value'),
    ]);
    await log.evaluate(log => log.stop());
}
