import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Build an ordinary sample and its saved clone through the deployed Studio. */
export async function verifyShoppingCart({ page, context, url }) {
    const collection = 'content-types-documents-ui', build = 'build-cart';
    const sample = 'shopping-cart';
    const expected = await readFile(new URL('../../../examples/projects/shopping-cart/expected/cart.html', import.meta.url), 'utf8');
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const open = async route => {
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route);
        await settle();
    };
    const buildAndPreview = async (html, line, total) => {
        await click('Build project');
        const built = await page.evaluate(() => {
            const current = globalThis.__cemStudioApplication.workbench.snapshot().projectPreview;
            return {
                attempt: current.attempt, path: current.artifactPath, html: current.projection?.output?.text,
                inputs: current.projection?.buildRun.inputs.map(input => input.resourceId),
                origins: current.projection?.buildRun.artifacts[0].provenance.map(frame => frame.uri),
            };
        });
        assert.equal(built.attempt.status, 'succeeded', JSON.stringify(built.attempt));
        assert.equal(built.path, 'generated/cart.html');
        assert.equal(built.html, html);
        assert.ok(built.inputs.includes('cart-response'));
        assert.ok(built.origins.some(uri => uri.endsWith('/templates/snapshot.cemt')));
        const iframe = page.locator('cem-media-preview[mode=artifact-tree] iframe');
        const frame = iframe.contentFrame();
        await frame.getByRole('heading', { name: 'Shopping cart', exact: true }).waitFor();
        assert.equal(await frame.getByRole('listitem').textContent(), line);
        assert.equal(await frame.getByText(`Total ${total}`, { exact: true }).count(), 1);
        assert.equal(await iframe.getAttribute('sandbox'), 'allow-same-origin');
        assert.equal(await frame.locator('script, img, button, input').count(), 0);
    };
    await page.goto(`${url}/?collection=${collection}`);
    await ready();
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: 'Shopping cart', exact: true }).click();
    await settle();
    assert.equal(new URL(page.url()).searchParams.get('sample'), sample);
    await open({ collection, sample, build });
    await buildAndPreview(expected, 'Tea × 2', 'USD 18.00');

    await click('Clone selected');
    await page.getByRole('dialog').getByRole('textbox').fill('Shopping cart clone');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click();
    await settle();
    const project = new URL(page.url()).searchParams.get('project');
    assert.ok(project);
    await open({ project, file: 'cart-response' });
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const original = await editor.evaluate(element => element.value);
    assert.equal(JSON.parse(original).data.cart.lines[0].quantity, 2);
    const edited = original.replace('"Tea"', '"Tea <img>"').replace('"quantity":2', '"quantity":4')
        .replace('"totalMinor":1800', '"totalMinor":3600').replace('USD 18.00', 'USD 36.00');
    await editor.fill(edited);
    await settle();
    await click('Save and validate');
    await open({ project, build });
    await buildAndPreview(expected.replace('Tea × 2', 'Tea &lt;img&gt; × 4').replace('USD 18.00', 'USD 36.00'), 'Tea <img> × 4', 'USD 36.00');

    await page.evaluate(() => navigator.serviceWorker.ready.then(() => null));
    await page.waitForFunction(() => navigator.serviceWorker.controller);
    await context.setOffline(true);
    try {
        await page.goto(`${url}/?${new URLSearchParams({ collection, sample, build })}`);
        await ready();
        await buildAndPreview(expected, 'Tea × 2', 'USD 18.00');
    } finally {
        await context.setOffline(false);
    }
    console.log('Studio Shopping cart passed: sample and clone builds, exact sandboxed preview, saved edits and offline sample rebuild.');
}
