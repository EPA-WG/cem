import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { createInteractiveCartService } from '../../cem-studio/scripts/verify-interactive-cart.mjs';

// An explicit trusted host fixture. Application content is still script-free;
// every declaration/source is original text admitted by the native adapters.
const fixture = new URL('../fixtures/component-application/shopping-cart/', import.meta.url);
const source = await readFile(new URL('profile.json', fixture), 'utf8');
const profile = JSON.parse(source);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const wasmPath = '/packages/cem_ql/dist/wasm-selected-application/';
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
const server = await createServer({ configFile: false, publicDir: false, optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null },
    plugins: [{ name: 'cart-service', configureServer(server) { server.middlewares.use((request, response, next) => {
        if (!request.url.startsWith('/graphql')) return next();
        void service.handle(request, response).catch(error => { response.statusCode = 500; response.end(String(error)); });
    }); } }] });
let browser;
try {
    await server.listen();
    const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
    browser = await chromium.launch({ executablePath: process.env.CEM_CHROME });
    for (const mode of ['worker', 'fallback', 'privacy']) {
        service.reset(); service.requests.length = 0;
        const page = await browser.newPage();
        await page.route(`${origin}/fixture`, route => route.fulfill({ contentType: 'text/html', body: '<!doctype html><iframe title="Cart"></iframe>' }));
        await page.goto(`${origin}/fixture`);
        await page.evaluate(async ({ origin, source, sourceUri, resources, profile, wasmHash, wasmPath, mode }) => {
            const binding = await import('/packages/cem-elements/src/lib/internal/runtime-support/native-runtime.ts');
            let heap = binding.isolateCemFallbackGeneration(); await heap.ready();
            let embedding = heap.claimOwnership(), previousAdmission;
            const [{ mountCemLivePreview }, { createCemGraphqlPreviewHost }, { CemCheckpointPublicationCoordinator },
                { CemNativeRecoveryScope, CemNativeGenerationGroup }, { resumeCemCheckpointOwners }] = await Promise.all([
                import('/packages/cem-elements/src/lib/live-preview.ts'), import('/packages/cem-elements/src/lib/graphql-preview.ts'),
                import('/packages/cem-elements/src/lib/checkpoint-publication.ts'),
                import('/packages/cem-elements/src/lib/internal/runtime-support/native-generation.ts'),
                import('/packages/cem-elements/src/lib/checkpoint-resume.ts'),
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
            const grant = { id: profile.id, endpoint: '/graphql', factoryUrl: wasmPath + 'cem_ql_factory.js', wasmUrl: wasmPath + 'cem_ql_bg.wasm',
                wasmSha256: wasmHash, sourceSha256: await crypto.subtle.digest('SHA-256', new TextEncoder().encode(source))
                    .then(bytes => Array.from(new Uint8Array(bytes), byte => byte.toString(16).padStart(2, '0')).join('')),
                resources: await Promise.all(resources.filter(resource => resource.path).map(async resource => ({ path: resource.path,
                    sha256: await crypto.subtle.digest('SHA-256', new TextEncoder().encode(resource.text))
                        .then(bytes => Array.from(new Uint8Array(bytes), byte => byte.toString(16).padStart(2, '0')).join('')) }))),
                initialRead: profile.graphql.initialRead, actions: profile.graphql.actions,
                bindings: { cartId: profile.graphql.initialRead.variables.id } };
            const frame = document.querySelector('iframe'), errors = [];
            const host = createCemGraphqlPreviewHost(origin + '/', [grant]);
            let application;
            const controller = mountCemLivePreview(frame, { source, sourceUri, resources,
                declarations: profile.components.map(tag => ({ tag, uri: `app://cart/components/${tag}.xhtml#${tag}` })),
                application: { id: profile.id }, onError: message => errors.push(message), runtimeOptions: {
                    checkpointPublication: coordinator, recoveryScope: scope,
                    processingWorkerFactory: () => { if (mode === 'fallback') throw Error('owned fallback');
                        return new Worker(`${origin}/packages/cem-elements/dist/lib/internal/runtime-support/processing-worker.js`, { type: 'module' }); },
                    applicationHost: async (...args) => { application = await host(...args); return application; },
                } });
            await controller.ready;
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
        }, { origin, source, sourceUri, resources, profile, wasmHash, wasmPath, mode });
        const frame = page.frameLocator('iframe');
        const quantity = frame.getByRole('spinbutton', { name: 'Quantity' }).first();
        assert.equal(await quantity.inputValue(), '2');
        await quantity.fill('3'); await quantity.press('Tab');
        await frame.getByText('USD 27.00', { exact: true }).waitFor();
        const before = structuredClone(service.requests);
        if (mode === 'privacy') {
            await page.evaluate(() => window.cartRecoveryFixture.refusePrivacy());
            assert.deepEqual(service.requests, before);
            await page.evaluate(() => window.cartRecoveryFixture.dispose());
            await page.close();
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
        console.log(`Live application ${mode}: original response/rendering group, missing-peer/stale-frame refusal, retained controls and zero recovery requests passed.`);
    }
} finally { await browser?.close(); await server.close(); }
