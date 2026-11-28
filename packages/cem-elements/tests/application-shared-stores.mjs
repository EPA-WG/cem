import assert from 'node:assert/strict';

// Install grants from the shell, then resolve the alias in each recipient's own runtime.
async function provision(page, mode) {
    await page.evaluate(async mode => {
        const { grantCemApplicationSharedStore, getCemApplicationSharedStore } = await import('/runtime/index.js');
        const shell = document.getElementById('shell');
        const editor = id => {
            const container = document.getElementById(id);
            return (mode === 'light' ? container : container.querySelector('iframe').contentDocument).querySelector('cem-application');
        };
        const options = { storeId: 'store-17', scope: 'preferences', read: true, write: true };
        const a = editor('A'), b = editor('B');
        for (const recipient of [a, b]) {
            const session = { ...options, kind: 'session', scope: 'workspace' };
            grantCemApplicationSharedStore(shell, recipient, session);
            grantCemApplicationSharedStore(shell, recipient, { ...session, scope: 'summary', keys: ['draft'], write: false });
        }
        grantCemApplicationSharedStore(shell, a, options);
        grantCemApplicationSharedStore(shell, b, { ...options, scope: 'second-writer' });
        grantCemApplicationSharedStore(shell, b, { ...options, scope: 'preview', write: false, keys: ['format'] });
        grantCemApplicationSharedStore(shell, a, { ...options, scope: 'format-only', keys: ['format'] });
        grantCemApplicationSharedStore(a, a, { ...options, scope: 'own' });

        // Revocation acts on just this mount and just this issuance of the alias.
        window.rebindPreferences = write => grantCemApplicationSharedStore(shell, a, { ...options, write });
        window.testGrantAuthority = () => {
            function refusal(fn) { try { fn(); return 'allowed'; } catch (error) { return error.name; } }
            return [
                refusal(() => getCemApplicationSharedStore(b, 'preferences')),
                refusal(() => getCemApplicationSharedStore(shell, 'preferences')),
                refusal(() => grantCemApplicationSharedStore(a, b, options)),
                refusal(() => grantCemApplicationSharedStore(a, shell, options)),
                refusal(() => grantCemApplicationSharedStore(a, a, options)),
            ];
        };
    }, mode);
}

function binding(frame, id, scope) {
    return frame.evaluateHandle(async ({ id, scope }) => {
        const { getCemApplicationSharedStore } = await import('/runtime/index.js');
        const container = document.getElementById(id) ?? document;
        return getCemApplicationSharedStore(container.querySelector('cem-application'), scope);
    }, { id, scope });
}

// Preserve native error names across both Playwright transports.
async function call(store, method, ...args) {
    const result = await store.evaluate(async (store, { method, args }) => {
        try { return { value: await store[method](...args) }; }
        catch (error) { return { error: error.name, message: error.message }; }
    }, { method, args });
    if (result.error) throw Object.assign(new Error(result.message), { name: result.error });
    return result.value;
}

async function race(a, b, key) {
    const [left, right] = await Promise.all([call(a, 'read', key), call(b, 'read', key)]);
    assert.deepEqual(left, right);
    const results = await Promise.allSettled([
        call(a, 'setItem', key, 'A', left.version), call(b, 'setItem', key, 'B', right.version),
    ]);
    const winner = results.findIndex(result => result.status === 'fulfilled');
    assert.notEqual(winner, -1, 'one conditional writer commits');
    assert.equal(results[1 - winner].reason?.name, 'ConstraintError', 'the stale writer receives a conflict');
    assert.equal(await call(b, 'getItem', key), ['A', 'B'][winner]);
}

export async function verifySharedStores(page, mode, editors, other, peers) {
    await provision(page, mode);
    await provision(other, mode === 'light' ? 'iframe' : 'light');
    const [a, b, remote, preview, limited, own] = await Promise.all([
        binding(editors.A, 'A', 'preferences'), binding(editors.B, 'B', 'second-writer'),
        binding(peers.A, 'A', 'preferences'), binding(editors.B, 'B', 'preview'),
        binding(editors.A, 'A', 'format-only'), binding(editors.A, 'A', 'own'),
    ]);
    const denied = { name: 'NotAllowedError' }, conflict = { name: 'ConstraintError' };
    assert.deepEqual(await page.evaluate(() => window.testGrantAuthority()), Array(5).fill('NotAllowedError'));
    await race(a, b, 'same-host-race');
    await race(a, remote, 'format');
    await Promise.all([call(a, 'setItem', 'format', 'plain'), call(remote, 'setItem', 'format', 'markdown')]);
    const committed = await call(b, 'read', 'format');
    assert.ok(['plain', 'markdown'].includes(committed.value));
    assert.deepEqual(await call(remote, 'read', 'format'), committed, 'both tabs see the same last unconditional commit');
    await Promise.all([call(a, 'setItem', '', ''), call(remote, 'setItem', 'notes', 'mine')]);
    assert.deepEqual(await call(b, 'keys'), ['', 'format', 'notes', 'same-host-race']);
    assert.equal(await call(b, 'getItem', ''), '');
    assert.equal(await call(own, 'getItem', 'notes'), null, 'same store ID declared by a different app is private to it');

    await call(limited, 'setItem', 'format', 'text/plain');
    const before = await call(preview, 'read', 'format');
    assert.equal(before.value, 'text/plain');
    assert.deepEqual(await call(preview, 'keys'), ['format']);
    await assert.rejects(call(preview, 'setItem', 'format', before.value), denied);
    await assert.rejects(call(preview, 'setItem', 'format', 'other', before.version), denied);
    await assert.rejects(call(preview, 'removeItem', 'format'), denied);
    await assert.rejects(call(preview, 'clear'), denied);
    await assert.rejects(call(preview, 'getItem', 'notes'), denied);
    await assert.rejects(call(limited, 'setItem', 'notes', 'other'), denied);
    await assert.rejects(call(limited, 'clear'), denied);
    await call(a, 'setItem', 'format', before.value);
    assert.deepEqual(await call(remote, 'read', 'format'), before, 'denials and no-ops preserve the version');
    await assert.rejects(call(a, 'setItem', 'notes', 'other', before.version), conflict);
    await assert.rejects(call(own, 'setItem', 'format', 'other', before.version), conflict);

    await call(limited, 'removeItem', 'format', before.version);
    const deleted = await call(a, 'read', 'format');
    assert.equal(deleted.value, null);
    await call(remote, 'setItem', 'format', before.value, deleted.version);
    await assert.rejects(call(a, 'removeItem', 'format', before.version), conflict);
    const recreated = await call(a, 'read', 'format');
    await call(remote, 'clear');
    assert.deepEqual(await call(a, 'keys'), []);
    await assert.rejects(call(a, 'setItem', 'format', 'stale', recreated.version), conflict);
    const absent = await call(a, 'read', 'format');
    await call(remote, 'clear');
    await call(a, 'removeItem', 'format');
    assert.deepEqual(await call(remote, 'read', 'format'), absent, 'empty clear and absent removal preserve the version');
    await call(a, 'setItem', 'format', 'after clear', absent.version);
    assert.equal(await call(remote, 'getItem', 'format'), 'after clear');

    // Start an actual provider operation before retiring its grant. Observe its outcome and
    // the durable value from another live tab, including when the recipient is in an iframe.
    const fenced = await page.evaluate(async mode => {
        const { getCemApplicationSharedStore } = await import('/runtime/index.js');
        const container = document.getElementById('A');
        const app = (mode === 'light' ? container : container.querySelector('iframe').contentDocument).querySelector('cem-application');
        const writer = window.rebindPreferences(true);
        const stale = getCemApplicationSharedStore(app, 'preferences');
        // Queue the real writer behind an overlapping IndexedDB transaction. Signal as
        // soon as its transaction exists, before the browser can deliver any request.
        const original = IDBDatabase.prototype.transaction;
        let queued;
        const started = new Promise(resolve => { queued = resolve; });
        IDBDatabase.prototype.transaction = function(table, mode, ...options) {
            if (mode !== 'readwrite') return original.call(this, table, mode, ...options);
            const ahead = original.call(this, table, mode);
            ahead.objectStore(table).count();
            const tx = original.call(this, table, mode, ...options);
            queued();
            return tx;
        };
        let pending;
        try {
            pending = stale.setItem('format', 'retired');
            await Promise.race([started, pending.then(() => { throw Error('Expected a queued write'); })]);
        } finally { IDBDatabase.prototype.transaction = original; }
        const readOnly = window.rebindPreferences(false);
        writer.revoke();
        const current = getCemApplicationSharedStore(app, 'preferences');
        async function outcome(promise) { try { await promise; return 'allowed'; } catch (error) { return error.name; } }
        const results = [await outcome(pending), await outcome(stale.read('format')),
            await outcome(current.setItem('format', 'retired')), await current.getItem('format')];
        results.push(await outcome(current.removeItem('absent')), await outcome(current.clear()));
        const pendingRead = current.getItem('format');
        readOnly.revoke();
        results.push(await outcome(pendingRead));
        return results;
    }, mode);
    assert.deepEqual(fenced, ['InvalidStateError', 'InvalidStateError', 'NotAllowedError', 'after clear', 'NotAllowedError', 'NotAllowedError', 'InvalidStateError']);
    assert.equal(await call(remote, 'getItem', 'format'), 'after clear');
}

export async function verifySessionStores(page, mode, setup) {
    async function mount(tab) {
        const editors = await setup(tab, mode, ['A', 'B']);
        const ungranted = await editors.A.evaluate(async () => {
            const { getCemApplicationSharedStore } = await import('/runtime/index.js');
            const app = (document.getElementById('A') ?? document).querySelector('cem-application');
            try { getCemApplicationSharedStore(app, 'workspace'); } catch (error) { return error.name; }
        });
        assert.equal(ungranted, 'NotAllowedError', 'each mount needs its own grant');
        await provision(tab, mode);
        const [a, b, preview, durable] = await Promise.all([
            binding(editors.A, 'A', 'workspace'), binding(editors.B, 'B', 'workspace'),
            binding(editors.B, 'B', 'summary'), binding(editors.A, 'A', 'preferences'),
        ]);
        return { a, b, preview, durable };
    }

    await page.reload();
    const original = await mount(page);
    const simultaneous = await original.a.evaluate(async store => {
        const { version } = await store.read('counter');
        const writes = await Promise.allSettled([
            store.setItem('counter', 'one', version), store.setItem('counter', 'two', version),
        ]);
        return writes.map(write => write.status === 'fulfilled' ? 'committed' : write.reason.name);
    });
    assert.deepEqual(simultaneous.sort(), ['ConstraintError', 'committed']);
    await call(original.durable, 'setItem', 'draft', 'durable');
    assert.equal(await call(original.a, 'getItem', 'draft'), null, 'same issuer and ID, separate storage kinds');
    await call(original.a, 'setItem', 'draft', 'first tab');
    const saved = await call(original.preview, 'read', 'draft');
    assert.equal(saved.value, 'first tab');
    await race(original.a, original.b, 'race');
    assert.deepEqual(await call(original.preview, 'keys'), ['draft']);
    await assert.rejects(call(original.preview, 'setItem', 'draft', 'denied'), { name: 'NotAllowedError' });

    const independent = await page.context().newPage();
    await independent.goto(page.url());
    const second = await mount(independent);
    assert.deepEqual(await call(second.b, 'keys'), [], 'fresh tab starts empty');
    assert.equal(await call(second.durable, 'getItem', 'draft'), 'durable');
    await assert.rejects(call(second.a, 'setItem', 'draft', 'foreign', saved.version), { name: 'ConstraintError' });
    await call(second.a, 'setItem', 'draft', 'second tab');
    const secondSaved = await call(second.b, 'read', 'draft');

    await page.reload();
    const restored = await mount(page);
    assert.deepEqual(await call(restored.preview, 'read', 'draft'), saved, 'reload restores the value and its version');
    await call(restored.b, 'setItem', 'draft', 'after reload', saved.version);
    assert.equal(await call(second.b, 'getItem', 'draft'), 'second tab');

    // Native opener sessions start from a snapshot. Fresh grants can read it, but
    // clearing or editing the copy cannot change the opener's partition or version.
    const opened = page.waitForEvent('popup');
    await page.evaluate(() => { window.open(location.href, '_blank'); });
    const popup = await opened;
    await popup.waitForLoadState();
    const copy = await mount(popup);
    const openerValue = await call(restored.a, 'read', 'draft');
    assert.equal(await call(copy.preview, 'getItem', 'draft'), openerValue.value);
    await call(copy.b, 'clear');
    assert.deepEqual(await call(copy.a, 'keys'), []);
    assert.deepEqual(await call(restored.preview, 'read', 'draft'), openerValue);
    await popup.close();

    await independent.reload();
    const secondReload = await mount(independent);
    assert.deepEqual(await call(secondReload.b, 'read', 'draft'), secondSaved);
    await call(restored.b, 'clear');
    await assert.rejects(call(restored.a, 'setItem', 'draft', 'stale', openerValue.version), { name: 'ConstraintError' });
    assert.equal(await call(secondReload.preview, 'getItem', 'draft'), 'second tab');
    assert.equal(await call(secondReload.durable, 'getItem', 'draft'), 'durable');
    await independent.close();
}
