import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

export function publicationOrigin(value) {
    if (value === undefined) return undefined;
    const url = new URL(value);
    assert.ok(url.protocol === 'https:' && url.pathname === '/' && !url.username && !url.password
        && !url.search && !url.hash && !url.port, 'Public Site acceptance requires an HTTPS origin without credentials, path, query, fragment or custom port');
    return url.origin;
}

export async function verifyPublishedAssets(origin, inventory, { fetchImpl = fetch, routes = [] } = {}) {
    assert.equal(publicationOrigin(origin), origin);
    const files = []; let cursor = 0;
    const outcomes = await Promise.allSettled(Array.from({ length: 4 }, async () => {
        while (cursor < inventory.files.length) {
            const expected = inventory.files[cursor++];
            const entry = { path: expected.path }; files.push(entry);
            try {
                const route = routes.find(route => route.output === expected.path && route.kind === 'page')?.route ?? expected.path;
                entry.requestPath = route;
                const response = await fetchImpl(new URL(route, origin + '/'), {
                    cache: 'no-store', redirect: 'error', signal: AbortSignal.timeout(30_000),
                });
                entry.status = response.status;
                assert.equal(response.status, 200, `Public Site asset unavailable: ${expected.path}`);
                const bytes = Buffer.from(await response.arrayBuffer());
                entry.bytes = bytes.length; entry.sha256 = createHash('sha256').update(bytes).digest('hex');
                assert.ok(entry.bytes === expected.bytes && entry.sha256 === expected.sha256, `Public Site asset differs from the verified build: ${expected.path}`);
            } catch (error) { entry.error = error.message; throw error; }
        }
    }));
    const failures = outcomes.filter(outcome => outcome.status === 'rejected');
    if (failures.length) throw new AggregateError(failures.map(outcome => outcome.reason), 'Public Site byte verification failed', { cause: { files } });
    return { origin, aggregateSha256: inventory.aggregateSha256, files: files.sort((a, b) => a.path.localeCompare(b.path)) };
}
