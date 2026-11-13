import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { createServer as createHttpServer } from 'node:http';
import { extname, resolve, sep } from 'node:path';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { createInteractiveCartService } from '../../cem-studio/scripts/verify-interactive-cart.mjs';

// An explicit trusted host fixture. Application content is still script-free;
// every declaration/source is original text admitted by the native adapters.
const fixture = new URL('../fixtures/component-application/shopping-cart/', import.meta.url);
const source = await readFile(new URL('profile.json', fixture), 'utf8');
const profile = JSON.parse(source);
const deployed = process.argv.includes('--offline');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const wasmPath = deployed ? '/applications/shopping-cart/' : '/packages/cem_ql/dist/wasm-selected-application/';
const wasmHash = hash(await readFile(new URL('../dist/wasm-selected-application/cem_ql_bg.wasm', import.meta.url)));
const sourceUri = 'app://cart/application.json';
const resources = await Promise.all([
    ['views/cart.cem', profile.entryTemplate], ['graphql/schema.graphql', profile.graphql.schema],
    ['graphql/operations.graphql', profile.graphql.operations],
].map(async ([path, file]) => ({ uri: new URL(path, sourceUri).href, path,
    text: await readFile(new URL(file, fixture), 'utf8'), contentType: 'text/plain' })));
for (const tag of profile.components) resources.push({ uri: `app://cart/components/${tag}.xhtml`, contentType: 'application/xhtml+xml',
    text: await readFile(new URL(`../../cem-components/src/components/${tag}/${tag}.xhtml`, import.meta.url), 'utf8') });
const service = await createInteractiveCartService();
let serviceMode = 'online';
const serviceAttempts = [];
async function handleService(request, response) {
    serviceAttempts.push(request.method);
    if (serviceMode === 'unavailable') {
        response.writeHead(503, { 'content-type': 'application/json', 'cache-control': 'no-store' });
        response.end('{"errors":[{"message":"Service unavailable"}]}'); return;
    }
    if (serviceMode === 'lost-reply' && request.method === 'POST') {
        // Apply the real fixture mutation, then lose the acknowledgment.
        await service.handle(request, { writeHead() {}, end() { request.socket.destroy(); } }); return;
    }
    await service.handle(request, response);
}
const server = deployed ? await deployedServer() : await createServer({ configFile: false, publicDir: false, optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null },
    plugins: [{ name: 'cart-service', configureServer(server) { server.middlewares.use((request, response, next) => {
        if (!request.url.startsWith('/graphql')) return next();
        void handleService(request, response).catch(error => { response.statusCode = 500; response.end(String(error)); });
    }); } }] });
let browser;
try {
    await server.listen();
    const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
    browser = await chromium.launch({ executablePath: process.env.CEM_CHROME });
    const context = await browser.newContext();
    const cases = deployed ? [
        { mode: 'worker', scenario: 'offline' }, { mode: 'fallback', scenario: 'offline' },
        { mode: 'worker', scenario: 'missing-runtime' }, { mode: 'worker', scenario: 'unavailable-initial' },
        { mode: 'worker', scenario: 'offline-initial' },
        { mode: 'worker', scenario: 'unavailable' }, { mode: 'fallback', scenario: 'unavailable' },
        { mode: 'worker', scenario: 'lost-reply' }, { mode: 'fallback', scenario: 'lost-reply' },
    ] : ['worker', 'fallback', 'privacy'].map(mode => ({ mode, scenario: 'online' }));
    const selected = process.env.CEM_RECOVERY_CASE;
    const selectedCases = selected ? cases.filter(test => `${test.mode}/${test.scenario}` === selected) : cases;
    assert.ok(selectedCases.length > 0, `Unknown recovery case: ${selected}`);
    for (const { mode, scenario } of selectedCases) {
        service.reset(); service.requests.length = 0; serviceAttempts.length = 0; serviceMode = 'online';
        const page = await context.newPage();
        const attempts = [];
        const observe = request => { if (new URL(request.url()).pathname === '/graphql') attempts.push(request.method()); };
        context.on('request', observe);
        if (!deployed) await page.route(`${origin}/fixture`, route => route.fulfill({ contentType: 'text/html', body: '<!doctype html><iframe title="Cart"></iframe>' }));
        await page.goto(`${origin}/fixture`);
        if (deployed) {
            await page.evaluate(async () => { await navigator.serviceWorker.register('/service-worker.js', { type: 'module' }); await navigator.serviceWorker.ready; });
            await page.waitForFunction(() => navigator.serviceWorker.controller);
        }
        if (scenario === 'unavailable-initial') serviceMode = 'unavailable';
        if (scenario === 'offline-initial') await context.setOffline(true);
        const initial = await page.evaluate(async ({ origin, source, sourceUri, resources, profile, wasmHash, wasmPath, mode, scenario, deployed }) => {
            const lib = deployed ? '/assets/@epa-wg/cem-elements/dist/lib/' : '/packages/cem-elements/src/lib/';
            const extension = deployed ? 'js' : 'ts';
            const binding = await import(`${lib}internal/runtime-support/native-runtime.${extension}`);
            let heap = binding.isolateCemFallbackGeneration(); await heap.ready();
            let embedding = heap.claimOwnership(), previousAdmission;
            const [{ mountCemLivePreview }, { createCemGraphqlPreviewHost }, { CemCheckpointPublicationCoordinator },
                { CemNativeRecoveryScope, CemNativeGenerationGroup }, { resumeCemCheckpointOwners }] = await Promise.all([
                import(`${lib}live-preview.${extension}`), import(`${lib}graphql-preview.${extension}`),
                import(`${lib}checkpoint-publication.${extension}`),
                import(`${lib}internal/runtime-support/native-generation.${extension}`),
                import(`${lib}checkpoint-resume.${extension}`),
            ]);
            const check = (value, message) => { if (!value) throw Error(message); };
            const refused = async (action, pattern) => {
                try { await action(); } catch (error) { check(pattern.test(String(error)), `unexpected refusal: ${String(error)}`); return; }
                throw Error(`Expected refusal: ${pattern}`);
            };
            const scope = new CemNativeRecoveryScope();
            const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
                policyStamp: 'cart-recovery', allowOwner: owner => mode !== 'privacy' || owner.instanceId !== 'application',
                seal: (candidate, receipt) => binding.sealCommittedCheckpoint(candidate, receipt) });
            const grant = { mutationIdempotency: 'server-keyed', id: profile.id, endpoint: '/graphql', factoryUrl: wasmPath + 'cem_ql_factory.js', wasmUrl: wasmPath + 'cem_ql_bg.wasm',
                wasmSha256: wasmHash, sourceSha256: await crypto.subtle.digest('SHA-256', new TextEncoder().encode(source))
                    .then(bytes => Array.from(new Uint8Array(bytes), byte => byte.toString(16).padStart(2, '0')).join('')),
                resources: await Promise.all(resources.filter(resource => resource.path).map(async resource => ({ path: resource.path,
                    sha256: await crypto.subtle.digest('SHA-256', new TextEncoder().encode(resource.text))
                        .then(bytes => Array.from(new Uint8Array(bytes), byte => byte.toString(16).padStart(2, '0')).join('')) }))),
                initialRead: profile.graphql.initialRead, actions: profile.graphql.actions,
                bindings: { cartId: profile.graphql.initialRead.variables.id } };
            const frame = document.querySelector('iframe'), errors = [];
            if (mode === 'worker' && scenario === 'online') await refused(() => createCemGraphqlPreviewHost(origin + '/',
                [{ ...grant, mutationIdempotency: undefined }]), /idempotency_required/);
            const host = createCemGraphqlPreviewHost(origin + '/', [grant]);
            let application;
            const controller = mountCemLivePreview(frame, { source, sourceUri, resources,
                declarations: profile.components.map(tag => ({ tag, uri: `app://cart/components/${tag}.xhtml#${tag}` })),
                application: { id: profile.id }, onError: message => errors.push(message), runtimeOptions: {
                    checkpointPublication: coordinator, recoveryScope: scope,
                    processingWorkerFactory: () => { if (mode === 'fallback') throw Error('owned fallback');
                        return new Worker(`${origin}${deployed ? lib : '/packages/cem-elements/dist/lib/'}internal/runtime-support/processing-worker.js`, { type: 'module' }); },
                    applicationHost: async (...args) => { application = await host(...args); return application; },
                } });
            try { await controller.ready; }
            catch (error) {
                if (!scenario.endsWith('-initial')) throw error;
                check(!controller.checkpoint() && !controller.applicationRecovery(), 'Failed initial read admitted a frame');
                await refused(() => application.guard.applicationIdentity(), /application.recovery.stale/);
                controller.dispose(); coordinator.dispose(); scope.dispose(); heap.releaseOwnership(embedding);
                return { failed: true, error: String(error) };
            }
            check(!scenario.endsWith('-initial'), 'Unavailable initial read fabricated a response');
            const capture = async () => {
                for (let attempt = 0; ; attempt++) {
                    try { await coordinator.capture(); return; }
                    catch (error) { if (attempt >= 100) throw Error(`${error}; ${JSON.stringify(coordinator.inspectRecovery())}; ${errors.join('; ')}`); await new Promise(resolve => setTimeout(resolve, 10)); }
                }
            };
            const recovery = async () => {
                await capture();
                const input = frame.contentDocument.querySelector('input'), previousDocument = frame.contentDocument;
                check(input, 'Settled frame has no quantity control');
                const consent = mode === 'fallback' ? heap.retirementConsent([embedding]) : undefined;
                const proof = await coordinator.closeRenderingGeneration(5000, consent); await capture();
                const ticket = coordinator.prepareResume(), response = controller.applicationRecovery();
                check(response, 'Missing selected source owner');
                const plan = await scope.prepare();
                await refused(() => new CemNativeGenerationGroup([response.participant()]), /allocation.coverage/);
                await refused(() => new CemNativeGenerationGroup([coordinator.generationParticipant()]), /allocation.coverage/);
                const appTicket = response.bindingTicket();
                await refused(() => application.guard.validateVariables('SetCartLineQuantity',
                    new TextEncoder().encode('{"cartId":"cart-1","lineId":"tea","quantity":9}'), origin + '/graphql#variables'),
                    /application.recovery.stale/);
                frame.setAttribute('allow', 'camera');
                await refused(() => plan.createGenerationGroup(), /enrollment-stale/);
                frame.setAttribute('allow', '');
                const group = plan.createGenerationGroup();
                const rendering = await coordinator.retireRenderingGeneration(proof, 5000, consent);
                const selected = response.retire();
                await refused(() => group.allocate([rendering]), /allocation.coverage/);
                const admission = await group.allocate([rendering, selected]);
                check(admission.generations.length === 2, 'Expected both actual native heaps');
                const sourceHandle = admission.generations.find(generation => generation.runtime.kind === 'cem-selected-application-generation')?.runtime;
                check(sourceHandle && Object.isFrozen(sourceHandle) && !('guard' in sourceHandle), 'Fresh source target must be an opaque original handle');
                const resumed = await resumeCemCheckpointOwners(admission, [ticket], 5000, [appTicket]);
                for (const owner of resumed.owners) {
                    await owner.owner.resume();
                    check(frame.contentDocument.querySelector('input') === input,
                        `Control replaced after ${owner.owner.element?.localName}; connected=${input.isConnected}`);
                }
                check(controller.checkpoint()?.(), 'Frame source witness was lost');
                check(frame.contentDocument === previousDocument && frame.contentDocument.querySelector('input') === input,
                    'Recovery replaced the original document/control');
                await capture();
                if (mode === 'fallback') {
                    heap = admission.generations.find(generation => typeof generation.runtime.claimOwnership === 'function').runtime;
                    embedding = heap.claimOwnership();
                }
                previousAdmission?.revoke(); previousAdmission = admission;
                return admission;
            };
            window.cartRecoveryFixture = { controller, coordinator, scope, recovery, errors,
                refuseUnknown: async () => { await capture(); await refused(() => scope.prepare(), /cem.checkpoint.recovery.unknown/); },
                refusePrivacy: async () => { await capture(); await refused(() => scope.prepare(), /cem.checkpoint.privacy/); },
                dispose: () => {
                    controller.dispose(); heap.releaseOwnership(embedding); previousAdmission?.revoke(); coordinator.dispose(); scope.dispose();
                } };
            return { failed: false };
        }, { origin, source, sourceUri, resources, profile, wasmHash, wasmPath, mode, scenario, deployed });
        if (scenario.endsWith('-initial')) {
            assert.equal(initial.failed, true);
            assert.equal(service.requests.length, 0);
            assert.ok(attempts.includes('GET') && !attempts.includes('POST'));
            await context.setOffline(false); await page.close(); context.off('request', observe);
            console.log(`Live application ${scenario}: no invented response, mutation or automatic retry.`); continue;
        }
        const frame = page.frameLocator('iframe');
        const quantity = frame.getByRole('spinbutton', { name: 'Quantity' }).first();
        assert.equal(await quantity.inputValue(), '2');
        await quantity.fill('3'); await quantity.press('Tab');
        await frame.getByText('USD 27.00', { exact: true }).waitFor().catch(async error => {
            const errors = await page.evaluate(() => window.cartRecoveryFixture.errors);
            throw Error(`${mode}/${scenario} initial mutation: ${JSON.stringify({ errors, attempts, requests: service.requests, value: await quantity.inputValue() })}`, { cause: error });
        });
        const before = structuredClone(service.requests);
        if (deployed) {
            const baselineAttempts = attempts.length, baselineService = serviceAttempts.length;
            let removed;
            if (scenario === 'missing-runtime') removed = await removeRuntimeAsset(page, origin);
            let uncachedRealm;
            if (scenario === 'missing-runtime') {
                uncachedRealm = await context.newPage();
                await uncachedRealm.goto(`${origin}/fixture`);
                await uncachedRealm.waitForFunction(() => navigator.serviceWorker.controller);
            }
            if (scenario === 'offline' || scenario === 'missing-runtime') await context.setOffline(true);
            if (scenario === 'unavailable' || scenario === 'lost-reply') serviceMode = 'unavailable';
            if (scenario === 'missing-runtime') {
                // The original host still owns verified immutable code; fresh
                // heaps need no second byte transfer. A new realm has no such
                // code and must refuse while the byte asset is unavailable.
                await page.evaluate(() => window.cartRecoveryFixture.recovery());
                await assert.rejects(uncachedRealm.evaluate(async () => {
                    const native = await import('/assets/@epa-wg/cem-elements/dist/lib/internal/runtime-support/native-runtime.js');
                    await native.default();
                }), /fetch|load|allocation|worker|code/i);
                await uncachedRealm.close();
                assert.equal(await quantity.inputValue(), '3');
                assert.equal(await frame.getByText('USD 27.00', { exact: true }).count(), 1);
            } else {
                await page.evaluate(() => window.cartRecoveryFixture.recovery());
                assert.equal(await quantity.inputValue(), '3');
            }
            assert.deepEqual(service.requests, before); assert.equal(attempts.length, baselineAttempts);
            assert.equal(serviceAttempts.length, baselineService);
            if (scenario === 'lost-reply') serviceMode = 'lost-reply';
            await quantity.fill('4'); await quantity.press('Tab');
            await page.waitForFunction(() => window.cartRecoveryFixture.errors.length > 0);
            await page.evaluate(() => window.cartRecoveryFixture.refuseUnknown());
            await context.setOffline(false); serviceMode = 'online';
            if (removed) await restoreRuntimeAsset(page, removed);
            const reconnect = attempts.length;
            await frame.getByRole('button', { name: 'Remove', exact: true }).click();
            await page.waitForFunction(() => window.cartRecoveryFixture.errors.length >= 2);
            assert.equal(attempts.length, reconnect);
            assert.equal(await frame.getByText('USD 27.00', { exact: true }).count(), 1);
            assert.equal(service.requests.length, before.length + (scenario === 'lost-reply' ? 1 : 0), JSON.stringify({ attempts, serviceAttempts, requests: service.requests }));
            if (scenario === 'lost-reply') {
                // Probe the service contract separately from the application.
                // The browser journal stays unknown even when an independent
                // caller can retrieve the service's committed result.
                const key = service.deliveries.at(-1).key;
                const payload = { query: resources.find(resource => resource.path === 'graphql/operations.graphql').text,
                    operationName: 'SetCartLineQuantity', variables: { cartId: 'cart-1', lineId: 'tea', quantity: 4 } };
                const post = body => fetch(origin + '/graphql', { method: 'POST', body: JSON.stringify(body),
                    headers: { 'content-type': 'application/json', 'idempotency-key': key } });
                const cached = await post(payload);
                assert.equal(cached.status, 200); assert.match(await cached.text(), /"quantity":4/);
                const conflict = await post({ ...payload, variables: { ...payload.variables, quantity: 99 } });
                assert.equal(conflict.status, 409); await conflict.text();
                assert.equal(service.requests.length, before.length + 1);
                const deliveries = service.deliveries.filter(delivery => delivery.key === key);
                assert.equal(deliveries.filter(delivery => !delivery.duplicate).length, 1);
                assert.ok(deliveries.filter(delivery => delivery.duplicate).length >= 2);
                await page.evaluate(() => window.cartRecoveryFixture.refuseUnknown());
            }
            await page.evaluate(() => window.cartRecoveryFixture.dispose());
            await page.close(); context.off('request', observe);
            console.log(`Live application ${mode}/${scenario}: accepted frame retained, no recovery/reconnect replay, unknown writes fenced; duplicate deliveries apply once.`);
            continue;
        }
        if (mode === 'privacy') {
            await page.evaluate(() => window.cartRecoveryFixture.refusePrivacy());
            assert.deepEqual(service.requests, before);
            await page.evaluate(() => window.cartRecoveryFixture.dispose());
            await page.close();
            context.off('request', observe);
            console.log('Live application privacy: original frame policy refuses response-source capture without requests or retirement.');
            continue;
        }
        await page.evaluate(async () => { window.cartRecoveryFixture.admission = await window.cartRecoveryFixture.recovery(); });
        assert.deepEqual(service.requests, before);
        assert.equal(await quantity.inputValue(), '3');
        await quantity.fill('4'); await quantity.press('Tab');
        await frame.getByText('USD 36.00', { exact: true }).waitFor();
        assert.deepEqual(service.requests.map(request => request.method), ['GET', 'POST', 'POST']);
        const second = structuredClone(service.requests);
        await page.evaluate(async () => { await window.cartRecoveryFixture.recovery(); });
        assert.deepEqual(service.requests, second);
        assert.equal(await quantity.inputValue(), '4');
        await quantity.fill('5'); await quantity.press('Tab');
        await frame.getByText('USD 45.00', { exact: true }).waitFor();
        assert.deepEqual(service.requests.map(request => request.method), ['GET', 'POST', 'POST', 'POST']);
        service.failNext();
        await quantity.fill('6'); await quantity.press('Tab');
        await page.waitForFunction(() => window.cartRecoveryFixture.errors.some(error => /cem\.graphql/.test(error)));
        const unknown = structuredClone(service.requests);
        await page.evaluate(() => window.cartRecoveryFixture.refuseUnknown());
        assert.deepEqual(service.requests, unknown);
        assert.equal(await quantity.inputValue(), '6');
        await page.evaluate(() => window.cartRecoveryFixture.dispose());
        await page.close();
        context.off('request', observe);
        console.log(`Live application ${mode}: original response/rendering group, missing-peer/stale-frame refusal, retained controls and zero recovery requests passed.`);
    }
} finally { await browser?.close(); await server.close(); }

async function deployedServer() {
    const root = resolve(import.meta.dirname, '../../cem-studio/dist/cart-preview');
    const security = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8'));
    const httpServer = createHttpServer(async (request, response) => {
        try {
            const url = new URL(request.url, 'http://localhost');
            if (url.pathname === '/graphql') return await handleService(request, response);
            const headers = { ...security.headers, 'cache-control': 'no-store', 'service-worker-allowed': '/' };
            if (url.pathname === '/fixture') {
                response.writeHead(200, { ...headers, 'content-type': 'text/html' });
                response.end('<!doctype html><iframe title="Cart"></iframe>'); return;
            }
            const file = resolve(root, '.' + decodeURIComponent(url.pathname));
            if (!file.startsWith(root + sep)) throw Error('Outside deployment');
            const bytes = await readFile(file);
            const type = { '.js': 'text/javascript', '.json': 'application/json', '.html': 'text/html',
                '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' }[extname(file)] ?? 'application/octet-stream';
            response.writeHead(200, { ...headers, 'content-type': type }); response.end(bytes);
        } catch { response.writeHead(404).end('Not found'); }
    });
    return { httpServer, listen: () => new Promise(resolve => httpServer.listen(0, '127.0.0.1', resolve)),
        close: () => new Promise((resolve, reject) => httpServer.close(error => error ? reject(error) : resolve())) };
}

async function removeRuntimeAsset(page, origin) {
    return page.evaluate(async origin => {
        const url = `${origin}/assets/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm`;
        const entries = [];
        for (const name of await caches.keys()) {
            const cache = await caches.open(name);
            if (await cache.match(url)) { await cache.delete(url); entries.push(name); }
        }
        if (!entries.length) throw Error('Expected the deployed worker WASM in the production cache');
        return { url, entries };
    }, origin);
}
async function restoreRuntimeAsset(page, removed) {
    await page.evaluate(async ({ url, entries }) => {
        for (const name of entries) await (await caches.open(name)).add(url);
    }, removed);
}
