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
