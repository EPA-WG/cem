import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** A loopback GraphQL service fixture. Product and UI code never runs here. */
export async function createInteractiveCartService() {
    const root = new URL('../../cem_ql/fixtures/component-application/shopping-cart/', import.meta.url);
    const document = await readFile(new URL('operations.graphql', root), 'utf8');
    const initial = JSON.parse(await readFile(new URL('cart.json', root), 'utf8')).data.cart;
    const cart = structuredClone(initial);
    const requests = [];
    let malformed = false;
    return {
        requests,
        failNext() { malformed = true; },
        reset() { Object.assign(cart, structuredClone(initial)); },
        async handle(request, response) {
            const url = new URL(request.url, 'http://localhost');
            if (url.pathname !== '/graphql') return false;
            let payload;
            if (request.method === 'GET') payload = {
                query: url.searchParams.get('query'), operationName: url.searchParams.get('operationName'),
                variables: JSON.parse(url.searchParams.get('variables')),
            };
            else {
                const chunks = [];
                for await (const chunk of request) chunks.push(chunk);
                payload = JSON.parse(Buffer.concat(chunks).toString('utf8'));
            }
            assert.equal(payload.query, document);
            const { operationName: operation, variables } = payload;
            requests.push({ method: request.method, operation, variables });
            if (operation === 'Cart') {
                assert.equal(request.method, 'GET'); assert.equal(variables.id, cart.id);
            } else {
                assert.equal(request.method, 'POST'); assert.equal(variables.cartId, cart.id);
                const index = cart.lines.findIndex(line => line.id === variables.lineId);
                assert.ok(index >= 0);
                if (operation === 'SetCartLineQuantity') {
                    assert.ok(Number.isInteger(variables.quantity) && variables.quantity > 0);
                    cart.lines[index].quantity = variables.quantity;
                } else {
                    assert.equal(operation, 'RemoveCartLine');
                    cart.lines.splice(index, 1);
                }
                cart.totalMinor = cart.lines.reduce((total, line) => total + line.quantity * line.unitPriceMinor, 0);
                cart.displayTotal = `USD ${(cart.totalMinor / 100).toFixed(2)}`;
            }
            const body = malformed ? { data: { cart: { id: cart.id } } } : { data: { cart } };
            malformed = false;
            response.writeHead(200, { 'content-type': 'application/graphql-response+json', 'cache-control': 'no-store' });
            response.end(JSON.stringify(body));
            return true;
        },
    };
}

export async function verifyInteractiveCart({ page, url, service }) {
    const route = { collection: 'content-types-documents-ui', sample: 'shopping-cart', build: 'live-cart' };
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const open = async value => {
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), value);
        await settle();
    };
    const build = async () => {
        await page.getByRole('button', { name: 'Build project', exact: true }).click(); await settle();
        const diagnostics = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.projection.diagnostics);
        assert.deepEqual(diagnostics.filter(diagnostic => ['error', 'fatal'].includes(diagnostic.severity)), []);
    };
    await page.goto(`${url}/?${new URLSearchParams(route)}`);
    await page.waitForFunction(() => globalThis.__cemStudioApplication);
    await settle();
    await build();
    const iframe = page.locator('cem-media-preview[mode=live] iframe[part~=live-document]');
    const frame = iframe.contentFrame();
    const quantity = frame.getByRole('spinbutton', { name: 'Quantity' });
    await quantity.waitFor();
    assert.equal(await quantity.inputValue(), '2');
    assert.equal(await iframe.getAttribute('sandbox'), 'allow-same-origin');
    assert.equal(await frame.locator('script').count(), 0);
    const count = service.requests.length;
    await quantity.fill('3');
    await quantity.press('Tab');
    await frame.getByText('USD 27.00', { exact: true }).waitFor();
    assert.equal(service.requests.length, count + 1);
    assert.equal(service.requests.at(-1).operation, 'SetCartLineQuantity');
    await frame.getByRole('button', { name: 'Remove', exact: true }).click();
    await frame.getByText('Your cart is empty', { exact: true }).waitFor();
    assert.equal(service.requests.length, count + 2);
    assert.equal(service.requests.at(-1).operation, 'RemoveCartLine');

    // A mounted application never grants authored code or declaration authority.
    await frame.locator('cem-card').evaluate(card => card.dispatchEvent(new MouseEvent('click', { bubbles: true })));
    assert.equal(service.requests.length, count + 2);
    await page.evaluate(async url => {
        const { createCemGraphqlPreviewHost } = await import(`${url}/assets/@epa-wg/cem-elements/dist/lib/graphql-preview.js`);
        const deployment = await fetch(`${url}/build.json`).then(response => response.json());
        const grant = deployment.previewApplications[0];
        const bundle = await globalThis.__cemStudioApplication.exampleProjects.examples.find(record => record.id === 'shopping-cart').load();
        const sourceUri = new URL('application.json', bundle.project.rootUri).href;
        const request = { id: grant.id, sourceUri, source: new TextDecoder().decode(bundle.contents['cart-application']),
            resources: bundle.project.resources.map(resource => ({ uri: new URL(resource.path, bundle.project.rootUri).href,
                contentType: resource.contentType, text: new TextDecoder().decode(bundle.contents[resource.id]) })) };
        for (const [badGrant, badRequest, code] of [
            [{ ...grant, wasmSha256: '0'.repeat(64) }, request, 'runtime_changed'],
            [grant, { ...request, resources: request.resources.filter(resource => !resource.uri.endsWith('/schema.graphql')) }, 'resource_changed'],
        ]) {
            let refused;
            try { await createCemGraphqlPreviewHost(url, [badGrant])(badRequest, new AbortController().signal); }
            catch (error) { refused = String(error); }
            if (!refused?.includes(code)) throw Error(`Expected application refusal ${code}, received ${refused}`);
        }
    }, url);
    assert.equal(service.requests.length, count + 2);
    service.reset();
    await open({ ...route, build: 'build-cart' });
    await open(route); await build();
    await quantity.waitFor();
    assert.equal(await quantity.inputValue(), '2');
    service.failNext();
    await quantity.fill('4'); await quantity.press('Tab');
    await page.getByText('cem.graphql.field.missing', { exact: false }).waitFor();
    await frame.getByText('USD 18.00', { exact: true }).waitFor();
    const afterWrites = service.requests.length;
    await frame.getByRole('button', { name: 'Remove', exact: true }).click();
    await page.getByText('cem.checkpoint.recovery.unknown', { exact: false }).waitFor();
    assert.equal(service.requests.length, afterWrites);
    await build();
    await quantity.waitFor();
    assert.equal(await quantity.inputValue(), '4');
    assert.equal(service.requests.length, afterWrites + 1);
    assert.equal(service.requests.at(-1).method, 'GET');
    await open({ ...route, build: 'build-cart' });
    await page.getByRole('button', { name: 'Clone selected', exact: true }).click(); await settle();
    await page.getByRole('dialog').getByRole('textbox').fill('Cart source pin check');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    const project = new URL(page.url()).searchParams.get('project');
    await open({ project, file: 'cart-application' });
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const source = await editor.evaluate(element => element.value);
    await editor.fill(source + '\n'); await settle();
    await page.getByRole('button', { name: 'Save and validate', exact: true }).click(); await settle();
    await open({ project, build: 'live-cart' }); await build();
    await page.getByText('Error: cem.preview.application.source_changed', { exact: true }).waitFor();
    assert.equal(service.requests.length, afterWrites + 1);
    console.log('Studio interactive cart passed: parent-hosted GET and mutations, script-free frame, disposal and source pin refusal.');
}
