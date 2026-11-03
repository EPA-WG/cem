import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createHash, webcrypto } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';

const source = await readFile(new URL('../src/service-worker.js', import.meta.url), 'utf8');
const scope = 'https://fixture.test/studio/';
const scopeHash = createHash('sha256').update(scope).digest('hex');
const revision = 'a'.repeat(64);
const family = `cem-studio:deployment:${scopeHash}:`;
const namespace = `${family}${revision}:0.1.0:`;
const url = path => new URL(path, scope).href;
const inventory = { schemaVersion: 2, commonVersion: '0.1.0', groups: [
    { id: 'shell', urls: ['./index.html', './cache-inventory.json'] },
    { id: 'runtime', moduleMap: './module-map.json' },
    { id: 'samples', strategy: 'cache-first', catalog: './samples/index.json' },
] };

function harness(selectedRevision = revision, profile = false) {
    const stores = new Map(), network = [];
    function store(name) {
        if (!stores.has(name)) stores.set(name, new Map());
        return stores.get(name);
    }
    const caches = {
        keys: async () => [...stores.keys()],
        delete: async name => stores.delete(name),
        open: async name => ({ match: async request => store(name).get(typeof request === 'string' ? request : request.url)?.clone() }),
        match: async request => {
            for (const values of stores.values()) {
                const response = values.get(typeof request === 'string' ? request : request.url);
                if (response) return response.clone();
            }
        },
    };
    const add = (name, path, value) => store(name).set(url(path), typeof value === 'string' ? new Response(value) : Response.json(value));
    add('cem-studio:0.1.0:runtime', './assets/module.js', 'legacy-stale');
    add('cem-studio:deployment:other-scope:' + revision + ':0.1.0:runtime', './assets/module.js', 'other-scope');
    const selectedNamespace = profile ? `cem-studio:profile:${scopeHash}:schema-assets:${revision}:0.1.0:` : namespace;
    add(`${selectedNamespace}shell`, './cache-inventory.json', { ...inventory,
        ...(profile ? { deploymentProfile: 'schema-assets', deploymentDigest: revision } : {}) });
    add(`${selectedNamespace}shell`, './index.html', '<base href="./" data-cem-studio-scope><main>Current shell</main>');
    add(`${selectedNamespace}runtime`, './module-map.json', { imports: { test: { path: './assets/module.js' } } });
    add(`${selectedNamespace}runtime`, './assets/module.js', 'current-code');
    add(`${selectedNamespace}samples`, './samples/index.json', { schemaVersion: 1, commonVersion: '0.1.0', cacheUrls: ['./sample.cem'] });
    const global = { caches, crypto: webcrypto, TextEncoder, URL, Uint8Array, Request, Response, Headers,
        self: { location: { href: url('./service-worker.js') }, registration: { scope }, addEventListener: () => undefined, clients: { claim: () => undefined } },
        fetch: async request => { network.push(typeof request === 'string' ? request : request.url); throw Error('Network intentionally unavailable or newer deployment'); } };
    runInNewContext(source.replace(/^import[^\n]+\n/, `const deploymentRevision = '${selectedRevision}';\n`)
        + '\nglobalThis.api = { cachedDeploymentResponse, deploymentPlan, activateDeployment };', global);
    return { api: global.api, stores, add, network };
}

test('only this deployment/scope supplies cached bytes, including after an offline worker restart', async () => {
    const host = harness();
    const response = await host.api.cachedDeploymentResponse(new Request(url('./assets/module.js')));
    assert.equal(await response.text(), 'current-code');
    const navigation = new Request(url('./project/'));
    Object.defineProperty(navigation, 'mode', { value: 'navigate' });
    const fallback = await host.api.cachedDeploymentResponse(navigation);
    assert.match(await fallback.text(), /Current shell/);
    assert.deepEqual(host.network, [url('./project/')]);
});

test('activation retires old revisions of this scope while preserving other scopes and legacy clients', async () => {
    const host = harness(), older = `${family}${'b'.repeat(64)}:0.1.0:runtime`;
    host.add(older, './assets/module.js', 'older-code');
    await host.api.activateDeployment();
    assert.equal(host.stores.has(older), false);
    assert.equal(host.stores.has(`${namespace}runtime`), true);
    assert.equal(host.stores.has('cem-studio:0.1.0:runtime'), true);
    assert.equal(host.stores.has('cem-studio:deployment:other-scope:' + revision + ':0.1.0:runtime'), true);
    assert.deepEqual(host.network, []);
});

test('invalid build identities fail closed before reading deployment metadata', async () => {
    const host = harness('invalid');
    await assert.rejects(host.api.deploymentPlan(), /deployment revision is invalid/);
    assert.deepEqual(host.network, []);
});

test('optional profiles retain their explicit deployment identity and refuse a mismatched worker revision', async () => {
    const host = harness(revision, true);
    assert.equal(await (await host.api.cachedDeploymentResponse(new Request(url('./assets/module.js')))).text(), 'current-code');
    assert.equal((await host.api.deploymentPlan()).profile, true);
    assert.deepEqual(host.network, []);
    const invalid = harness(revision, true);
    invalid.add(`cem-studio:profile:${scopeHash}:schema-assets:${revision}:0.1.0:shell`, './cache-inventory.json',
        { ...inventory, deploymentProfile: 'schema-assets', deploymentDigest: 'b'.repeat(64) });
    await assert.rejects(invalid.api.deploymentPlan(), /profile revision is invalid/);
});
