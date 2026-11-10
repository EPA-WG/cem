import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const file = name => readFile(new URL(`../fixtures/component-application/shopping-cart/${name}`, import.meta.url), 'utf8');
const [profile, document, initial, updated, removed] = await Promise.all([
    file('profile.json').then(JSON.parse), file('operations.graphql'), file('cart.json'), file('updated.json'), file('removed.json'),
]);
const server = await createServer({
    configFile: false, publicDir: false, optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null },
});
let browser;
try {
    await server.listen();
    const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
    browser = await chromium.launch({ executablePath: process.env.CEM_CHROME });
    const page = await browser.newPage();
    const requests = [];
    const reads = [];
    const firstResponse = Promise.withResolvers();
    const expected = [
        { query: document, operationName: 'SetCartLineQuantity', variables: { cartId: 'cart-1', lineId: 'tea', quantity: 3 } },
        { query: document, operationName: 'RemoveCartLine', variables: { cartId: 'cart-1', lineId: 'tea' } },
        { query: document, operationName: 'SetCartLineQuantity', variables: { cartId: 'cart-1', lineId: 'coffee', quantity: 2 } },
    ];
    await page.route(`${origin}/fixture`, route => route.fulfill({
        contentType: 'text/html', body: '<!doctype html><main></main>',
    }));
    await page.route(url => url.origin === origin && url.pathname === profile.graphql.endpoint, async route => {
        if (route.request().method() === 'GET') {
            reads.push(Object.fromEntries(new URL(route.request().url()).searchParams));
            return route.fulfill({ contentType: 'application/graphql-response+json', body: initial });
        }
        requests.push({ method: route.request().method(), payload: route.request().postDataJSON() });
        const index = requests.length - 1;
        if (index === 0) await firstResponse.promise;
        return route.fulfill({
            contentType: 'application/graphql-response+json; charset=utf-8',
            body: [updated, removed, '{"data":{"cart":{"id":"cart-1"}}}'][index] ?? '{}',
        });
    });
    await page.goto(`${origin}/fixture`);
    await page.evaluate(async profile => {
        const assets = '/packages/cem_ql/dist/wasm-selected-application/';
        const { createCemComponentWasmBindings } = await import(assets + 'cem_ql_factory.js');
        const { CemElementRuntime } = await import('/packages/cem-elements/src/lib/cem-elements.ts');
        const { bindCemGraphqlActions, readCemGraphqlQuery } = await import('/packages/cem-elements/src/lib/graphql-actions.ts');
        const { CemCheckpointPublicationCoordinator } = await import('/packages/cem-elements/src/lib/checkpoint-publication.ts');
        const module = await WebAssembly.compileStreaming(fetch(assets + 'cem_ql_bg.wasm'));
        // Use the canonical compiled CEM widgets for native input and click.
        // The entry, variables and responses go through the selected runtime.
        const bundle = '/dist/artifacts/cem_ql/selected-application/';
        const { declarations } = await fetch(bundle + 'requirements.json').then(response => response.json());
        const runtime = new CemElementRuntime({
            retainedStylesheets: {},
            loadSrcDocument: () => { throw Error('Unexpected source declaration load'); },
            processingWorkerFactory: () => { throw Error('fixture fallback'); },
        });
        runtime.install(window);
        await Promise.all(profile.components.map(async tag => {
            const [metadataJson, templateBytes] = await Promise.all([
                fetch(`${bundle}${tag}.json`).then(response => response.text()),
                fetch(`${bundle}${tag}.bin`).then(response => response.arrayBuffer()),
            ]);
            const declaration = document.createElement(runtime.declarationTag);
            declaration.setAttribute('tag', tag);
            const pending = runtime.registerCompiledDeclaration(declaration, {
                metadataJson, templateBytes, metadataHash: declarations[tag].metadataHash,
            });
            document.body.append(declaration);
            await pending;
        }));

        const root = document.querySelector('main');
        const errors = [];
        const journal = new CemCheckpointPublicationCoordinator({
            runtimeId: 'cart-browser-fixture', generation: 1, policyStamp: 'fixture',
            allowOwner: () => true, seal: candidate => candidate,
        });
        let saved;
        let revision = 0;
        const owner = journal.register('cart', () => '<template>cart</template>');
        const publish = response => {
            root.innerHTML = response.html;
            saved = response;
            owner.published({ instanceId: 'cart', declarationIdentity: profile.id, dataRevision: String(++revision) });
            return true;
        };
        let guard, unbind;
        const attach = async () => {
            const retiredMemory = guard?.memory();
            unbind?.();
            guard?.release();
            guard = createCemComponentWasmBindings();
            await guard.initialize(module);
            if (guard.memory() === retiredMemory) throw Error('Expected a fresh selected WASM heap');
            if (!saved) {
                const options = { baseUrl: location.href, endpoint: profile.graphql.endpoint, guard };
                for (const [operation, variables, expected] of [
                    ['RemoveCartLine', { cartId: 'cart-1', lineId: 'tea' }, 'cem.graphql.operation.kind'],
                    ['Cart', {}, 'cem.graphql.field.missing'],
                ]) {
                    let refused;
                    try { await readCemGraphqlQuery({ ...options, operation, variables }); }
                    catch (error) { refused = String(error); }
                    if (!refused?.includes(expected)) throw Error(`Initial read accepted invalid input: ${operation}`);
                }
                publish(await readCemGraphqlQuery({ ...options, ...profile.graphql.initialRead }));
            } else {
                publish({ ...saved, html: guard.renderResponse(saved.operation, saved.bytes, saved.uri) });
            }
            unbind = bindCemGraphqlActions({
                root, endpoint: profile.graphql.endpoint, actions: profile.graphql.actions,
                bindings: { cartId: profile.graphql.initialRead.variables.id }, guard, owner, publish,
                reportError: error => errors.push(String(error)),
            });
        };
        await attach();
        window.cartFixture = { journal, errors, attach };
    }, profile);

    const line = id => page.locator(`[data-line-id="${id}"]:visible`);
    const quantity = id => line(id).getByRole('spinbutton', { name: 'Quantity' });
    const remove = id => line(id).getByRole('button', { name: 'Remove' });
    const change = async (id, value) => {
        await quantity(id).fill(value);
        await quantity(id).press('Tab');
    };
    const noReplay = async id => {
        await quantity(id).dispatchEvent('change', { bubbles: true });
        await remove(id).dispatchEvent('click', { bubbles: true });
    };
    await noReplay('tea');
    assert.equal(await quantity('tea').inputValue(), '2');
    assert.deepEqual(reads, [{ query: document, operationName: 'Cart', variables: '{"id":"cart-1"}' }]);
    await change('tea', '1.5');
    await page.waitForFunction(() => window.cartFixture.errors.some(error => /cem\.graphql/.test(error)));
    assert.deepEqual(requests, []);
    assert.equal(await page.evaluate(() => window.cartFixture.journal.inspectRecovery().unknownOperations), 0);

    try {
        await Promise.all([
            page.waitForRequest(`${origin}${profile.graphql.endpoint}`),
            change('tea', '3'),
        ]);
        assert.equal(await page.evaluate(() => window.cartFixture.journal.inspectRecovery().unknownOperations), 1);
        await remove('tea').click();
        assert.equal(requests.length, 1);
    } finally {
        firstResponse.resolve();
    }
    await quantity('coffee').waitFor();
    assert.equal(await quantity('tea').inputValue(), '3');
    assert.equal(await page.locator('main img').count(), 0);
    assert.equal(await page.evaluate(async () => {
        const { journal, attach } = window.cartFixture;
        await journal.capture();
        const outcome = journal.current().receipt.operations[0].outcome;
        await attach();
        return outcome;
    }), 'acknowledged');
    await noReplay('tea');
    assert.equal(requests.length, 1);

    await remove('tea').click();
    await line('tea').waitFor({ state: 'detached' });
    assert.equal(await quantity('coffee').inputValue(), '1');
    await change('coffee', '2');
    await page.waitForFunction(() => window.cartFixture.errors.filter(error => /cem\.graphql/.test(error)).length === 2);
    assert.deepEqual(await page.evaluate(async () => {
        const { journal, attach } = window.cartFixture;
        let recovery;
        try { journal.recoveryEnrollment(); } catch (error) { recovery = error.message; }
        await attach();
        return { recovery, unknown: journal.inspectRecovery().unknownOperations };
    }), { recovery: 'cem.checkpoint.recovery.unknown', unknown: 1 });
    assert.equal(await quantity('coffee').inputValue(), '1');
    await noReplay('coffee');
    await remove('coffee').click();
    await page.waitForFunction(() => window.cartFixture.errors.some(error => /cem\.checkpoint\.recovery\.unknown/.test(error)));
    assert.deepEqual(requests, expected.map(payload => ({ method: 'POST', payload })));
    assert.equal(reads.length, 1);
    console.log('Cart browser passed: guarded initial GET, trusted mutations, journal fencing and no request replay on rebind.');
} finally {
    await browser?.close();
    await server.close();
}
