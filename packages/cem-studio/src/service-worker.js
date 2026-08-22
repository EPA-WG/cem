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
        } else if (group?.id === 'samples' && group.strategy === 'cache-first' && Array.isArray(group.urls)) {
            groups.push({
                cacheName: cacheName(inventory.commonVersion, group.id),
                urls: uniqueUrls(group.urls),
            });
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

function uniqueUrls(paths) {
    return [...new Set(paths.map((path) => new URL(path, self.registration.scope).href))];
}

function cacheName(version, group) {
    return `${CACHE_NAMESPACE}:${version}:${group}`;
}
