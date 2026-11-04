import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { hasEscapedPageBridge } from './publication-html.mjs';

const root = resolve(import.meta.dirname, '../../..');
const manifest = JSON.parse(await readFile(resolve(root, 'apps/cem-site/site.routes.json'), 'utf8'));
for (const fixture of [
    { route: '/guides/cem-ml/cli/', sourceHref: '../cem-elements/README.md#optional-native-schema-source-loading',
        destination: 'https://github.com/EPA-WG/cem/blob/develop/packages/cem-elements/README.md#optional-native-schema-source-loading' },
    { route: '/reference/cem-ml/wasm-runtime/', sourceHref: '../../docs/cem-data-import-principle.md#passive-projection-source-imports',
        destination: 'https://github.com/EPA-WG/cem/blob/develop/docs/cem-data-import-principle.md#passive-projection-source-imports' },
]) test(`${fixture.route} native publication preserves repository ownership and fragment`, async () => {
    const entry = manifest.entries.find(entry => entry.route === fixture.route);
    assert.equal(entry.relativeLinkPolicy, 'canonical-source');
    const source = await readFile(resolve(root, entry.source), 'utf8');
    assert.ok(source.includes('](' + fixture.sourceHref + ')'), 'Fixture no longer exercises the package-relative link');
    const output = await readFile(resolve(root, 'dist/apps/cem-site', entry.output), 'utf8');
    const hrefs = [...output.matchAll(/href="([^"]+)"/g)].map(match => match[1]);
    const links = hrefs.filter(href => new URL(href, 'https://site.invalid' + entry.route).href === fixture.destination);
    assert.equal(links.length, 1, 'Expected one owner-correct native link with its original fragment');
    assert.ok(links[0].startsWith(entry.canonicalSourceBase), 'Link bypassed the native canonical-source base');
    assert.ok(!hrefs.includes(fixture.sourceHref), 'Package link still resolves relative to the deployed site');
    assert.ok(hrefs.includes('/') && hrefs.includes('/guides/'), 'Shared site navigation must remain site-relative');
    const scripts = [...output.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gu)];
    assert.equal(scripts.length, 2, 'Reference pages load only the shared snippet viewer and its import map');
    assert.ok(scripts.some(([, attrs]) => attrs.includes('src="/packages/cem-demo-element/dist/index.js"')));
    assert.ok(scripts.some(([, attrs, body]) => attrs.includes('type="importmap"') && JSON.parse(body).imports['@epa-wg/cem-ml/wasm']));
});

test('native Amsterdam source excerpt is a code example, not an escaped page bridge', async () => {
    const output = await readFile(resolve(root, 'dist/apps/cem-site/guides/declarative-amsterdam-2026/index.html'), 'utf8');
    assert.ok(output.includes('&lt;h1'));
    assert.equal(hasEscapedPageBridge(output), false);
    assert.equal(hasEscapedPageBridge('<main><code>&lt;h1&gt;</code></main>'), false);
});
test('escaped page content outside code still fails, including a page with a legitimate example', () => {
    assert.equal(hasEscapedPageBridge('<main>&lt;h1&gt;A broken bridge&lt;/h1&gt;</main>'), true);
    assert.equal(hasEscapedPageBridge('<pre><code>&lt;h1&gt;Example</code></pre><main>&lt;h1&gt;Broken</main>'), true);
    assert.equal(hasEscapedPageBridge('<cem-demo-element demo="false" source="&lt;h1&gt;Code&lt;/h1&gt;"></cem-demo-element>'), false);
    assert.equal(hasEscapedPageBridge('<cem-demo-element demo="false" source="&lt;h1&gt;Code">&lt;h1&gt;Broken body</cem-demo-element>'), true);
});

test('both runtime routes publish the existing shared transport/schema/declaration helpers byte-for-byte', async () => {
    const sourceMap = JSON.parse(await readFile(resolve(root, 'apps/cem-site/runtime/source.module-map.json'), 'utf8'));
    const destinationMap = JSON.parse(await readFile(resolve(root, 'apps/cem-site/runtime/destination.module-map.json'), 'utf8'));
    for (const file of ['compiled-declaration-assets.js', 'http-resource-transport.js', 'schema-asset-pack.js',
        'internal/runtime-support/schema-asset-protocol.js', 'internal/runtime-support/vendor/schema-asset-protocol.js']) {
        const key = '@epa-wg/cem-elements/dist/lib/' + file;
        assert.equal(sourceMap.resources[key].path, '../../../packages/cem-elements/dist/lib/' + file);
        const source = await readFile(resolve(root, 'packages/cem-elements/dist/lib', file));
        for (const runtime of manifest.runtimes) for (const route of runtime.routes) {
            const path = new URL(destinationMap.resources[key].path, 'https://site.invalid' + route).pathname;
            assert.deepEqual(await readFile(resolve(root, 'dist/apps/cem-site', '.' + path)), source, `${route}: ${file}`);
        }
    }
});
