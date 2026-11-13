// Delivery/control metadata only. Schema/document bodies are hashed as bytes,
// never interpreted here or substituted for native schema/source admission.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { lstat, readFile, readdir, realpath } from 'node:fs/promises';
import { resolve, sep } from 'node:path';
import { gzipSync } from 'node:zlib';
import { readDemoInventory } from './demo-deployment.mjs';

export const bodyHash = bytes => createHash('sha256').update(bytes).digest('hex');
export function bodyTotals(files) {
    const urls = new Map(), contents = new Map();
    for (const file of files) {
        assert(Number.isSafeInteger(file.bytes) && file.bytes >= 0 && /^[a-f0-9]{64}$/.test(file.sha256));
        if (urls.has(file.path)) assert.deepEqual(urls.get(file.path), file, 'same URL has inconsistent bytes');
        if (contents.has(file.sha256)) assert.equal(contents.get(file.sha256), file.bytes, 'same content hash has inconsistent size');
        urls.set(file.path, file); contents.set(file.sha256, file.bytes);
    }
    return { entries: files.length, bodyBytes: files.reduce((sum, file) => sum + file.bytes, 0),
        uniqueUrls: urls.size, uniqueUrlBytes: [...urls.values()].reduce((sum, file) => sum + file.bytes, 0),
        uniqueContents: contents.size, uniqueContentBytes: [...contents.values()].reduce((sum, size) => sum + size, 0) };
}

export async function studioOfflinePlan(root) {
    root = await realpath(root);
    const origin = new URL('https://footprint.invalid/profile/');
    const pathOf = (reference, base = origin) => {
        assert(typeof reference === 'string', 'asset reference must be a URL string');
        const url = new URL(reference, base);
        assert(url.origin === origin.origin && url.pathname.startsWith(origin.pathname) && !url.search && !url.hash, 'asset is outside deployment scope');
        const path = decodeURIComponent(url.pathname.slice(origin.pathname.length));
        assert(path && !/[\\\x00-\x1f]/.test(path) && !path.split('/').some(segment => segment === '..' || segment === '.'), 'unsafe asset path');
        return path;
    };
    const bytes = async path => {
        const file = resolve(root, path), actual = await realpath(file);
        assert(actual.startsWith(root + sep) && (await lstat(file)).isFile(), 'asset must be an in-scope regular file');
        return readFile(file);
    };
    const json = async path => JSON.parse(await bytes(path));
    const inventory = await json('cache-inventory.json'), build = await json('build.json');
    assert.equal(inventory.schemaVersion, 2); assert.equal(inventory.commonVersion, build.commonVersion);
    assert.equal(inventory.deploymentProfile, build.deploymentProfile); assert.equal(inventory.deploymentDigest, build.deploymentDigest);
    const groups = [], identities = new Set();
    for (const group of inventory.groups) {
        assert(!identities.has(group.id), 'duplicate cache group'); identities.add(group.id);
        let paths;
        if (group.id === 'shell') paths = group.urls.map(path => pathOf(path));
        else if (group.id === 'runtime') {
            const mapPath = pathOf(group.moduleMap), map = await json(mapPath);
            paths = [mapPath, ...[...Object.values(map.imports), ...Object.values(map.resources)].map(entry => pathOf(entry.path))];
        } else if (group.id === 'samples') {
            const path = pathOf(group.catalog), catalog = await json(path);
            assert.equal(catalog.schemaVersion, 1); assert.equal(catalog.commonVersion, build.commonVersion);
            paths = [path, ...catalog.cacheUrls.map(reference => pathOf(reference, new URL(path, origin)))];
        } else if (group.id === 'schema-assets') {
            const path = pathOf(group.inventory), pack = await json(path), base = new URL('./', new URL(path, origin));
            assert.equal(build.deploymentProfile, 'schema-assets'); assert.equal(pack.algorithm, 'sha256');
            assert.equal(pack.commonVersion, build.commonVersion); assert.equal(pack.indexSha256, build.schemaAssetIndexDigest);
            assert.equal(bodyHash(await bytes(pathOf('index.json', base))), pack.indexSha256);
            const expected = [...pack.files.map(file => file.path), 'integrity.json'];
            assert.equal(new Set(expected).size, expected.length, 'duplicate pack path');
            assert.deepEqual([...pack.offlineUrls].sort(), expected.sort(), 'pack inventory is incomplete');
            for (const file of pack.files) {
                const content = await bytes(pathOf(file.path, base));
                assert.equal(content.length, file.bytes); assert.equal(bodyHash(content), file.sha256, 'pack asset differs');
            }
            paths = pack.offlineUrls.map(reference => pathOf(reference, base));
        } else throw Error(`unknown cache group: ${group.id}`);
        const files = [];
        for (const path of [...new Set(paths)].sort()) {
            const content = await bytes(path);
            files.push({ path, bytes: content.length, sha256: bodyHash(content) });
        }
        groups.push({ id: group.id, files, totals: bodyTotals(files) });
    }
    for (const required of ['shell', 'runtime', 'samples']) assert(identities.has(required), 'incomplete cache plan');
    const delivery = [];
    const walk = async prefix => {
        for (const entry of await readdir(resolve(root, prefix), { withFileTypes: true })) {
            const path = prefix ? prefix + '/' + entry.name : entry.name;
            if (entry.isDirectory()) await walk(path);
            else {
                assert(entry.isFile(), 'deployment contains a non-regular asset');
                const content = await bytes(path);
                delivery.push({ path, bytes: content.length, sha256: bodyHash(content), gzipBytes: gzipSync(content, { level: 9 }).length });
            }
        }
    };
    await walk(''); delivery.sort((a, b) => a.path.localeCompare(b.path));
    const offline = groups.flatMap(group => group.files);
    return { build, groups, delivery, deliveryTotals: { ...bodyTotals(delivery), perFileGzipBytes: delivery.reduce((sum, file) => sum + file.gzipBytes, 0) },
        offlineTotals: bodyTotals(offline), uncachedPaths: delivery.filter(file => !offline.some(item => item.path === file.path)).map(file => file.path),
        identity: bodyHash(JSON.stringify({ build, groups, delivery })) };
}

export function verifyOfflineBodies(plan, caches, fetched) {
    assert.equal(caches.length, plan.groups.length, 'unexpected cache groups');
    for (const group of plan.groups) {
        const candidates = caches.filter(cache => cache.name.endsWith(':' + group.id)
            || group.id === 'schema-assets' && /:schema-assets:[a-f0-9]{64}$/.test(cache.name));
        assert.equal(candidates.length, 1, 'cache group identity mismatch');
        assert.deepEqual([...candidates[0].files].sort((a, b) => a.path.localeCompare(b.path)),
            [...group.files].sort((a, b) => a.path.localeCompare(b.path)), 'cached body differs from build');
    }
    const expected = [...new Map(plan.groups.flatMap(group => group.files).map(file => [file.path, file])).values()];
    assert.deepEqual([...fetched].sort((a, b) => a.path.localeCompare(b.path)), expected.sort((a, b) => a.path.localeCompare(b.path)), 'offline response differs from build');
    return bodyTotals(caches.flatMap(cache => cache.files));
}

export async function catalogOfflinePlan(root, catalog) {
    root = await realpath(root);
    const inventory = await readDemoInventory(root);
    assert(inventory, 'catalog deployment inventory is missing');
    const delivery = [];
    const add = async path => {
        const file = resolve(root, path);
        assert((await realpath(file)).startsWith(root + sep) && (await lstat(file)).isFile(), 'catalog asset escapes scope');
        const content = await readFile(file);
        const entry = { path, bytes: content.length, sha256: bodyHash(content) };
        delivery.push({ ...entry, gzipBytes: gzipSync(content, { level: 9 }).length });
        return entry;
    };
    for (const expected of inventory.files) assert.deepEqual(await add(expected.path), expected, 'catalog inventory drift');
    await add('demo-deployment.json'); delivery.sort((a, b) => a.path.localeCompare(b.path));
    for (const component of catalog.components) assert(delivery.some(file => file.path === component.implementation.source), 'canonical declaration omitted');
    const files = delivery.map(({ path, bytes, sha256 }) => ({ path, bytes, sha256 }));
    return { build: { buildIdentity: 'full-source-component-catalog-audit', componentTags: catalog.components.map(component => component.tag) },
        groups: [{ id: 'catalog', files, totals: bodyTotals(files) }], delivery,
        deliveryTotals: { ...bodyTotals(delivery), perFileGzipBytes: delivery.reduce((sum, file) => sum + file.gzipBytes, 0) },
        offlineTotals: bodyTotals(files), uncachedPaths: [], identity: bodyHash(JSON.stringify({ delivery, catalog })) };
}
