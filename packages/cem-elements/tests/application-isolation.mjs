// Run against the built cem-elements package. All persistence and navigation go
// through the production declarative controls in a fresh Playwright context.
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, firefox } from 'playwright';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const origin = 'http://127.0.0.1';
const failures = [];
for (const engine of [chromium, firefox]) {
    const browser = await engine.launch();
    try {
        for (const mode of ['light', 'iframe']) {
            const context = await browser.newContext();
            try {
                await context.route(origin + '/**', serve);
                await verify(context, mode);
                console.log(engine.name(), mode, 'passed');
            } catch (error) {
                failures.push(new Error(engine.name() + ' ' + mode, { cause: error }));
            } finally { await context.close(); }
        }
    } finally { await browser.close(); }
}
if (failures.length) throw new AggregateError(failures, 'Application resource isolation');

async function serve(route) {
    const url = new URL(route.request().url());
    if (url.pathname.startsWith('/runtime/')) {
        const file = path.resolve(dist, decodeURIComponent(url.pathname.slice('/runtime/'.length)));
        assert.ok(file.startsWith(dist));
        return route.fulfill({ path: file, contentType: file.endsWith('.wasm') ? 'application/wasm' : 'text/javascript' });
    }
    const root = url.pathname === '/editor' ? '<main></main>' : '<cem-application id="shell"></cem-application>';
    return route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>CEM application test</title>' + root });
}

function read(frame, id, live = false) {
    return frame.evaluate(({ id, live }) => window.apps.get(id).read(live), { id, live });
}
function write(frame, id, value, method, src) {
    return frame.evaluate(({ id, value, method, src }) => window.apps.get(id).write(value, method, src),
        { id, value, method, src });
}

async function verify(context, mode) {
    const page = await context.newPage();
    await page.goto(origin + '/#host');
    await page.evaluate(() => localStorage.setItem('draft', 'host'));
    // Exercise the host service before any cem-element is registered.
    const direct = await page.evaluate(async () => {
        const { getCemApplicationStorage } = await import('/runtime/index.js');
        const shell = document.getElementById('shell');
        const create = () => shell.appendChild(document.createElement('cem-application'));
        const a = create(), b = create();
        const local = getCemApplicationStorage(a), session = getCemApplicationStorage(a, 'session');
        local.setItem('draft', 'local A');
        session.setItem('draft', 'session A');
        const values = [local.getItem('draft'), session.getItem('draft'), getCemApplicationStorage(b).getItem('draft'),
            getCemApplicationStorage(b, 'session').getItem('draft'), localStorage.getItem('draft'), sessionStorage.getItem('draft')];
        local.removeItem('draft');
        values.push(local.getItem('draft'), session.getItem('draft'));
        a.remove(); b.remove();
        let retired = false;
        try { local.setItem('draft', 'stale'); } catch { retired = true; }
        return { values, retired };
    });
    assert.deepEqual(direct, { values: ['local A', 'session A', null, null, 'host', null, null, 'session A'], retired: true });
    const { A, B } = await setup(page, mode, ['A', 'B']);
    assert.equal((await read(A, 'A')).draft, null, 'empty private draft');
    await A.evaluate(() => window.apps.get('A').storage.setItem('draft', 'service A'));
    assert.equal((await read(A, 'A', true)).draft, 'service A', 'the element reads the app service’s write');
    assert.equal((await read(B, 'B', true)).draft, null, 'a sibling cannot see the service write');

    await write(A, 'A', 'alpha', 'history.replaceState', '#A');
    await write(B, 'B', 'beta', 'location.hash', 'B');
    for (const [id, frame, draft] of [['A', A, 'alpha'], ['B', B, 'beta']]) {
        for (const live of [false, true]) {
            const state = await read(frame, id, live);
            assert.equal(state.draft, draft, id + ' private draft');
            assert.equal(state.hash, '#' + id, id + ' private URL');
        }
        if (mode === 'iframe') assert.equal(new URL(frame.url()).hash, '', 'frame document keeps its own URL');
    }
    assert.equal(await A.evaluate(() => window.apps.get('A').storage.getItem('draft')), 'alpha', 'the app service reads the element’s write');
    assert.deepEqual(await page.evaluate(() => [localStorage.getItem('draft'), location.hash]), ['host', '#host']);

    // Reopen both named apps in reverse order and the other host representation.
    // Both paths end with the same container-local ID, "editor".
    const other = await context.newPage();
    await other.goto(origin + '/#another-tab');
    const peers = await setup(other, mode === 'light' ? 'iframe' : 'light', ['B', 'A']);
    assert.equal((await read(peers.A, 'A')).draft, 'alpha', 'A recovered its own partition');
    assert.equal((await read(peers.B, 'B')).draft, 'beta', 'B recovered its own partition');
    assert.equal((await read(peers.A, 'A')).hash, '', 'fresh instance has an independent URL');

    await write(A, 'A', 'updated', 'location.assign', './notes?edit=1#updated');
    await peers.A.waitForFunction(() => window.apps.get('A').snapshot().draft === 'updated');
    assert.equal((await read(peers.B, 'B', true)).draft, 'beta', 'external write stays in A’s partition');
    const updated = await read(A, 'A', true);
    assert.deepEqual([updated.pathname, updated.search, updated.hash], ['/notes', '?edit=1', '#updated']);
    assert.equal((await read(B, 'B', true)).hash, '#B');
    assert.equal((await read(peers.A, 'A', true)).hash, '', 'navigation stays in the originating instance');
    assert.equal(new URL(page.url()).hash, '#host');
    const withdrawn = await page.evaluate(async mode => {
        const { getCemApplicationStorage } = await import('/runtime/index.js');
        const container = document.getElementById('A');
        const owner = mode === 'light' ? container : container.querySelector('iframe').contentDocument;
        const service = getCemApplicationStorage(owner.querySelector('cem-application'));
        const before = service.getItem('draft');
        container.remove();
        try { service.setItem('draft', 'closed'); } catch { return { before, refused: true }; }
        return { before, refused: false };
    }, mode);
    assert.deepEqual(withdrawn, { before: 'updated', refused: true });
    assert.equal((await read(peers.A, 'A')).draft, 'updated');
}

async function setup(page, mode, ids) {
    const frames = {};
    for (const id of ids) {
        await page.evaluate(async ({ mode, id }) => {
            const container = document.createElement('cem-application');
            container.id = id;
            document.querySelector('#shell').append(container);
            if (mode === 'iframe') {
                const iframe = document.createElement('iframe');
                iframe.name = id;
                const loaded = new Promise(resolve => { iframe.onload = resolve; });
                iframe.src = '/editor';
                container.append(iframe);
                await loaded;
            }
        }, { mode, id });
        const frame = mode === 'light' ? page : page.frame({ name: id });
        frames[id] = frame;
        await frame.evaluate(async ({ mode, id }) => {
            const { getCemApplicationStorage, installCemElementRuntime, mountCemView } = await import('/runtime/index.js');
            window.runtime ??= installCemElementRuntime(window, { processingWorkerFactory() { throw Error('use owned fallback'); } });
            window.apps ??= new Map();
            const app = document.createElement('cem-application');
            app.id = 'editor';
            (mode === 'light' ? document.getElementById(id) : document.querySelector('main')).append(app);

            const declarations = new Set();
            async function mount(suffix, markup) {
                const tag = 'verify-' + id.toLowerCase() + '-' + suffix;
                if (!declarations.has(tag)) {
                    const declaration = document.createElement('cem-element');
                    declaration.setAttribute('tag', tag);
                    declaration.innerHTML = '<template>' + markup + '<output></output></template>';
                    app.append(declaration);
                    await window.runtime.whenDeclarationSettled(declaration);
                    declarations.add(tag);
                }
                const container = document.createElement('section');
                app.append(container);
                const view = mountCemView(window.runtime, { root: container, tag });
                await view.whenSettled();
                return view;
            }
            const resources = '<local-storage slice="draft" key="draft" type="text" live="true"></local-storage>' +
                '<location-element slice="route" live="true"></location-element>';
            const reader = await mount('reader', resources);
            const snapshot = (view = reader) => {
                const { slices } = window.runtime.readInstanceSnapshot(view.element);
                return { draft: slices.draft, pathname: slices.route?.pathname, search: slices.route?.search, hash: slices.route?.hash };
            };
            let writers = 0;
            window.apps.set(id, {
                storage: getCemApplicationStorage(app), snapshot,
                async write(value, method, src) {
                    const writer = await mount('writer-' + ++writers, '<local-storage slice="write" key="draft" type="text" value="' +
                        value + '"></local-storage><location-element method="' + method + '" src="' + src + '" trigger="navigate"></location-element>');
                    writer.dispose();
                },
                async read(live) {
                    const view = live ? reader : await mount('reader', resources);
                    try {
                        await view.whenSettled();
                        return snapshot(view);
                    } finally { if (!live) view.dispose(); }
                },
            });
        }, { mode, id });
    }
    return frames;
}
