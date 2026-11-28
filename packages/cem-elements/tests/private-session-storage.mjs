import assert from 'node:assert/strict';

function stores(app) {
    return app.evaluateHandle(async app => {
        const { getCemApplicationStorage } = await import('/runtime/index.js');
        return { session: getCemApplicationStorage(app, 'session'), local: getCemApplicationStorage(app) };
    });
}

function drafts(editors) {
    return Promise.all([editors.A, editors.B].map(editor => editor.evaluate(({ session }) => session.getItem('draft'))));
}

export async function verifyPrivateSessions(context, url, mode, mountEditor) {
    async function mount(page, ids = ['A', 'B']) {
        const editors = {};
        for (const id of ids) {
            const { app } = await mountEditor(page, mode, id);
            editors[id] = await stores(app);
            await app.dispose();
        }
        return editors;
    }

    const first = await context.newPage();
    await first.goto(url);
    await first.evaluate(() => sessionStorage.setItem('draft', 'host'));
    let original = await mount(first);
    assert.deepEqual(await drafts(original), [null, null]);
    await original.A.evaluate(({ session, local }) => {
        session.setItem('draft', 'first A');
        local.setItem('draft', 'durable A');
    });
    await original.B.evaluate(({ session }) => session.setItem('draft', 'first B'));
    assert.equal(await first.evaluate(() => sessionStorage.getItem('draft')), 'host');

    const second = await context.newPage();
    await second.goto(url);
    let independent = await mount(second);
    assert.deepEqual(await drafts(independent), [null, null], 'fresh tab has fresh private sessions');
    assert.equal(await independent.A.evaluate(({ local }) => local.getItem('draft')), 'durable A');
    await independent.A.evaluate(({ session }) => session.setItem('draft', 'second A'));
    await independent.B.evaluate(({ session }) => session.setItem('draft', 'second B'));

    await first.reload();
    original = await mount(first, ['B', 'A']);
    assert.deepEqual(await drafts(original), ['first A', 'first B'], 'restore each editor under its own parent');
    await original.A.evaluate(({ session }) => session.removeItem('draft'));
    await second.reload();
    independent = await mount(second, ['B', 'A']);
    assert.deepEqual(await drafts(independent), ['second A', 'second B']);
    await independent.B.evaluate(({ session }) => session.removeItem('draft'));
    assert.deepEqual(await drafts(original), [null, 'first B'], 'a removal stays in its tab');

    if (mode === 'iframe') {
        // Retain the old handle in the still-live host while only its editor frame reloads.
        const frame = second.locator('iframe[name="A"]');
        const retired = await frame.evaluateHandle(async iframe => {
            const { getCemApplicationStorage } = await import('/runtime/index.js');
            return getCemApplicationStorage(iframe.contentDocument.querySelector('#editor'), 'session');
        });
        assert.equal(await retired.evaluate(session => session.getItem('draft')), 'second A');
        await frame.evaluate(iframe => new Promise(resolve => {
            iframe.addEventListener('load', () => resolve(), { once: true });
            iframe.contentWindow.location.reload();
        }));
        const refused = await retired.evaluate(session => {
            try { session.setItem('draft', 'retired'); return false; } catch { return true; }
        });
        assert.ok(refused, 'a host-held handle cannot write after its editor document reloads');
        const remounted = await second.frameLocator('iframe[name="A"]').locator('#editor').elementHandle();
        independent.A = await stores(remounted);
        assert.deepEqual(await drafts(independent), ['second A', null], 'frame reload reattaches to its tab and namespace');
    }

    const opened = first.waitForEvent('popup');
    await first.evaluate(() => { window.open(location.href, '_blank'); });
    const copy = await opened;
    await copy.waitForLoadState();
    const copied = await mount(copy);
    assert.deepEqual(await drafts(copied), [null, 'first B'], 'an opener-created tab gets the initial snapshot');
    await copied.B.evaluate(({ session }) => session.setItem('draft', 'copy B'));
    await original.B.evaluate(({ session }) => session.removeItem('draft'));
    await copy.reload();
    assert.deepEqual(await drafts(await mount(copy)), [null, 'copy B']);
    assert.deepEqual(await drafts(original), [null, null]);
    await copy.close();

    // The admitted page owns the session provider even when the consumer is in a frame.
    // Withdraw it after binding and verify both operations fail without changing either store.
    const fresh = await mountEditor(second, mode, 'C');
    await second.evaluate(() => {
        const descriptor = Object.getOwnPropertyDescriptor(window, 'sessionStorage');
        window.restoreSessionProvider = () => Object.defineProperty(window, 'sessionStorage', descriptor);
        Object.defineProperty(window, 'sessionStorage', { configurable: true, value: undefined });
    });
    try {
        await assert.rejects(independent.A.evaluate(({ session }) => session.getItem('draft')), /cem\.application\.storage-unavailable/);
        await assert.rejects(independent.A.evaluate(({ session }) => session.setItem('draft', 'blocked')), /cem\.application\.storage-unavailable/);
        await assert.rejects(stores(fresh.app), /cem\.application\.storage-unavailable/);
    } finally { await second.evaluate(() => window.restoreSessionProvider()); }
    const recovered = await stores(fresh.app);
    assert.equal(await recovered.evaluate(({ session }) => session.getItem('draft')), null);
    assert.deepEqual(await drafts(independent), ['second A', null]);
    assert.equal(await independent.A.evaluate(({ local }) => local.getItem('draft')), 'durable A');
    await second.close();
    await first.close();
}
