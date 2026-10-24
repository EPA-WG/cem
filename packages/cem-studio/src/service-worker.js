const CACHE_NAMESPACE = 'cem-studio';
const DEPLOYMENT_INVENTORY = new URL('./cache-inventory.json', self.location.href).href;
const SHELL_FALLBACK = new URL('./index.html', self.location.href).href;
let deploymentPlanPromise;

self.addEventListener('install', (event) => {
    event.waitUntil(cacheDeployment());
});

self.addEventListener('activate', (event) => {
    event.waitUntil(activateDeployment());
});

self.addEventListener('fetch', (event) => {
    if (event.request.method !== 'GET') return;
    const url = new URL(event.request.url);
    if (url.origin !== self.location.origin || !url.href.startsWith(self.registration.scope)) return;
    event.respondWith(cachedDeploymentResponse(event.request));
});

self.addEventListener('message', (event) => {
    if (event.data?.type === 'cem-studio-activate-update') {
        event.waitUntil(self.skipWaiting());
        return;
    }
    if (event.data?.type === 'cem-studio-deployment-inventory') {
        event.source?.postMessage({
            type: 'cem-studio-deployment-inventory',
            url: DEPLOYMENT_INVENTORY,
        });
    }
});

async function cacheDeployment() {
    const plan = await deploymentPlan();
    await Promise.all(
        plan.groups.map(async ({ cacheName, urls }) => {
            const cache = await caches.open(cacheName);
            await cache.addAll(urls);
        }),
    );
}

async function activateDeployment() {
    const plan = await deploymentPlan();
    const currentCaches = new Set(plan.groups.map(({ cacheName }) => cacheName));
    const existingCaches = await caches.keys();
    await Promise.all(
        existingCaches
            .filter((name) => name.startsWith(`${CACHE_NAMESPACE}:`) && !currentCaches.has(name))
            .map((name) => caches.delete(name)),
    );
    await self.clients.claim();
}

async function cachedDeploymentResponse(request) {
    const cached = await caches.match(request);
    if (cached) return cached;
    try {
        return await fetch(request);
    } catch (error) {
        if (request.mode === 'navigate') {
            const shell = await caches.match(SHELL_FALLBACK);
            if (shell) return scopedShellResponse(shell);
        }
        throw error;
    }
}

async function scopedShellResponse(shell) {
    const html = await shell.text();
    const scopedHtml = html.replace(
        /<base href="\.\/" data-cem-studio-scope\s*\/?>/,
        `<base href="${self.registration.scope}" data-cem-studio-scope>`,
    );
    const headers = new Headers(shell.headers);
    headers.delete('content-length');
    return new Response(scopedHtml, {
        status: shell.status,
        statusText: shell.statusText,
        headers,
    });
}

function deploymentPlan() {
    deploymentPlanPromise ??= loadDeploymentPlan();
    return deploymentPlanPromise;
}

async function loadDeploymentPlan() {
    const inventoryResponse = await fetch(DEPLOYMENT_INVENTORY, { cache: 'no-store' });
    if (!inventoryResponse.ok) {
        throw new Error(`CEM Studio cache inventory failed: ${inventoryResponse.status}`);
    }
    const inventory = await inventoryResponse.json();
    if (inventory?.schemaVersion !== 2 || typeof inventory.commonVersion !== 'string') {
        throw new Error('CEM Studio cache inventory is invalid');
    }

    const groups = [];
    for (const group of inventory.groups ?? []) {
        if (group?.id === 'shell' && Array.isArray(group.urls)) {
            groups.push({
                cacheName: cacheName(inventory.commonVersion, group.id),
                urls: uniqueUrls(group.urls),
            });
        } else if (group?.id === 'runtime' && typeof group.moduleMap === 'string') {
            const moduleMapUrl = new URL(group.moduleMap, self.registration.scope).href;
            const moduleMapResponse = await fetch(moduleMapUrl, { cache: 'no-store' });
            if (!moduleMapResponse.ok) {
                throw new Error(`CEM Studio module map failed: ${moduleMapResponse.status}`);
            }
            const moduleMap = await moduleMapResponse.json();
            const entries = [...Object.values(moduleMap.imports ?? {}), ...Object.values(moduleMap.resources ?? {})];
            const targets = entries.map((entry) => entry?.path).filter((path) => typeof path === 'string');
            groups.push({
                cacheName: cacheName(inventory.commonVersion, group.id),
                urls: uniqueUrls([group.moduleMap, ...targets]),
            });
        } else if (group?.id === 'samples' && group.strategy === 'cache-first' && typeof group.catalog === 'string') {
            const catalogUrl = new URL(group.catalog, self.registration.scope).href;
            const catalogResponse = await fetch(catalogUrl, { cache: 'no-store' });
            if (!catalogResponse.ok) {
                throw new Error(`CEM Studio sample catalog failed: ${catalogResponse.status}`);
            }
            const catalog = await catalogResponse.json();
            if (
                catalog?.schemaVersion !== 1
                || catalog.commonVersion !== inventory.commonVersion
                || !Array.isArray(catalog.cacheUrls)
            ) {
                throw new Error('CEM Studio sample catalog is invalid');
            }
            groups.push({
                cacheName: cacheName(inventory.commonVersion, group.id),
                urls: uniqueUrls([
                    catalogUrl,
                    ...catalog.cacheUrls.map((path) => new URL(path, catalogUrl).href),
                ]),
            });
        } else if (group?.id === 'schema-assets') {
            // Opt-in delivery only. The native-compiled bootstrap, not this
            // offline inventory, remains the schema/source trust anchor.
            if (group.strategy !== 'cache-first' || typeof group.inventory !== 'string') {
                throw new Error('CEM Studio schema-asset group is invalid');
            }
            const inventoryUrl = new URL(group.inventory, self.registration.scope);
            if (!inventoryUrl.href.startsWith(self.registration.scope) || inventoryUrl.origin !== self.location.origin) {
                throw new Error('CEM Studio schema-asset inventory is outside scope');
            }
            const response = await fetch(inventoryUrl, { cache: 'no-store', credentials: 'omit', redirect: 'error' });
            if (!response.ok || response.redirected) throw new Error('CEM Studio schema-asset inventory is unavailable');
            const pack = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(await schemaAssetControlBytes(response, 1024 * 1024)));
            if (pack?.formatVersion !== 1 || pack.algorithm !== 'sha256' || pack.commonVersion !== inventory.commonVersion
                || !/^[a-f0-9]{64}$/.test(pack.indexSha256) || !Array.isArray(pack.offlineUrls) || !Array.isArray(pack.files)
                || pack.files.length > 1030 || pack.offlineUrls.length !== pack.files.length + 1) {
                throw new Error('CEM Studio schema-asset inventory is invalid');
            }
            const files = pack.files.map(file => file?.path);
            if (new Set(files).size !== files.length || !files.includes('index.json')
                || !pack.offlineUrls.includes('integrity.json')
                || new Set(pack.offlineUrls).size !== pack.offlineUrls.length
                || files.some(path => !pack.offlineUrls.includes(path))) {
                throw new Error('CEM Studio schema-asset inventory is incomplete');
            }
            const base = new URL('./', inventoryUrl);
            const indexResponse = await fetch(new URL('index.json', base), { cache: 'no-store', credentials: 'omit', redirect: 'error' });
            if (!indexResponse.ok || indexResponse.redirected) throw new Error('CEM Studio schema-asset index is unavailable');
            // Explicit catalog DTO only: establish delivery completeness, not
            // native schema identity or source interpretation/activation.
            const indexBytes = await schemaAssetControlBytes(indexResponse, 4 * 1024 * 1024);
            const indexDigest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', indexBytes)), byte => byte.toString(16).padStart(2, '0')).join('');
            if (indexDigest !== pack.indexSha256) throw new Error('CEM Studio schema-asset delivery index differs');
            const index = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(indexBytes));
            if (index.formatVersion !== 'cem-schema-asset-pack/1' || !Array.isArray(index.sources) || index.sources.length > 1024
                || index.sources.some(source => !/^[a-f0-9]{64}$/.test(source?.digest))) {
                throw new Error('CEM Studio schema-asset catalog is invalid');
            }
            const expected = new Set(['index.json', 'index.sha256', 'bootstrap.json', 'measurements.json', 'integrity.json',
                ...index.sources.map(source => `sources/${source.digest}`)]);
            if (expected.size !== pack.offlineUrls.length || pack.offlineUrls.some(path => !expected.has(path))) {
                throw new Error('CEM Studio schema-asset inventory omits catalog sources');
            }
            const urls = pack.offlineUrls.map(path => {
                if (typeof path !== 'string' || !/^(?:[a-z0-9_.-]+\/)*[a-z0-9_.-]+$/.test(path)
                    || path.split('/').some(segment => segment === '.' || segment === '..')) {
                    throw new Error('CEM Studio schema-asset URL is invalid');
                }
                return new URL(path, base).href;
            });
            groups.push({ cacheName: `${cacheName(inventory.commonVersion, group.id)}:${pack.indexSha256}`, urls });
        }
    }
    if (!groups.some(({ urls }) => urls.includes(SHELL_FALLBACK))) {
        throw new Error('CEM Studio cache inventory does not include the shell fallback');
    }
    for (const requiredGroup of ['shell', 'runtime', 'samples']) {
        if (!groups.some(({ cacheName }) => cacheName.endsWith(`:${requiredGroup}`))) {
            throw new Error(`CEM Studio cache inventory is missing the ${requiredGroup} group`);
        }
    }
    return { groups };
}

async function schemaAssetControlBytes(response, capacity) {
    const reader = response.body?.getReader();
    if (!reader) throw new Error('CEM Studio schema-asset control body is missing');
    const chunks = [];
    let length = 0, complete = false;
    try {
        for (;;) {
            const chunk = await reader.read();
            if (chunk.done) { complete = true; break; }
            if (length + chunk.value.byteLength > capacity || chunks.length >= 65_536) {
                throw new Error('CEM Studio schema-asset control body is too large');
            }
            chunks.push(chunk.value); length += chunk.value.byteLength;
        }
    } finally {
        if (!complete) void reader.cancel().catch(() => undefined);
        reader.releaseLock();
    }
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
    return bytes;
}

function uniqueUrls(paths) {
    return [...new Set(paths.map((path) => new URL(path, self.registration.scope).href))];
}

function cacheName(version, group) {
    return `${CACHE_NAMESPACE}:${version}:${group}`;
}
