import assert from 'node:assert/strict';
import { launchPublicationBrowser } from './publication-browser.mjs';

// Exercise the same composed publication locally and after owner deployment.
export async function verifyComponentGalleries(origin, manifest) {
    const browser = await launchPublicationBrowser();
    const report = { origin, galleryCount: manifest.galleryCount, galleryRoutes: [], propertyRoutes: [], examples: [] };
    try {
        const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
        const page = await context.newPage();
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        page.on('response', response => {
            if (response.url().startsWith(origin + '/') && response.status() >= 400
                && /\.(?:js|mjs|xhtml|wasm|css)(?:\?|$)/u.test(response.url())) errors.push(`HTTP ${response.status()}: ${response.url()}`);
        });
        await page.goto(origin + '/guides/declarative-amsterdam-2026/');
        const link = page.locator('nav[aria-label=Primary]').getByRole('link', { name: 'CEM Components', exact: true });
        assert.equal(await link.getAttribute('href'), '/packages/cem-components/');
        await link.click();
        await page.locator('cem-gallery-nav h1').waitFor({ timeout: 60_000 });
        assert.equal(new URL(page.url()).origin, origin);
        assert.equal(new URL(page.url()).pathname, '/packages/cem-components/');
        const routes = await page.locator('nav[aria-label=Components] a[href]').evaluateAll(nodes => nodes
            .filter(node => /-gallery\.html$/u.test(new URL(node.href).pathname))
            .map(node => new URL(node.href).pathname.replace(/\.html$/u, '')).sort());
        assert.deepEqual(routes, [...manifest.galleryRoutes].sort(), 'Component index must link to every gallery on this Site');
        for (const route of routes) {
            const response = await context.request.get(origin + route);
            assert.equal(response.status(), 200, route);
            assert.equal(new URL(response.url()).origin, origin);
            const tag = route.split('/').at(-1).replace(/-gallery$/u, '');
            assert.ok((await response.text()).includes(`heading="${tag}: Full examples and variation matrix"`), `${route}: wrong header`);
            report.galleryRoutes.push(route);
            const propertyRoute = route.replace(/-gallery$/u, '');
            const property = await context.request.get(origin + propertyRoute);
            assert.equal(property.status(), 200, propertyRoute);
            assert.ok((await property.text()).includes('<cem-gallery-page'), `${propertyRoute}: missing shared page`);
            report.propertyRoutes.push(propertyRoute);
        }
        for (const tag of ['cem-icon', 'cem-icon-button', 'cem-suggestions']) {
            const route = `/packages/cem-components/playgrounds/${tag}-gallery`;
            await page.goto(origin + route, { waitUntil: 'domcontentloaded' });
            const owner = page.locator('cem-gallery-page');
            const control = page.locator('cem-gallery-nav cem-theme-switch');
            await control.locator('cem-menu button').last().waitFor({ state: 'attached', timeout: 60_000 });
            const heading = page.locator('cem-gallery-nav h1');
            assert.equal(await heading.textContent(), `${tag}: Full examples and variation matrix`);
            assert.ok(await heading.isVisible());
            assert.ok(await page.locator('cem-gallery-nav [part=theme-label]').isVisible());
            const fixed = await owner.locator('[data-gallery-theme]').evaluateAll(nodes => nodes.map(node => [node.className, getComputedStyle(node).backgroundColor]));
            await control.locator('[part=trigger] button').click();
            await control.getByRole('menuitemradio', { name: 'Presentation', exact: true }).click();
            await presentationReady(page);
            assert.deepEqual(await owner.locator('[data-gallery-theme]').evaluateAll(nodes => nodes.map(node => [node.className, getComputedStyle(node).backgroundColor])), fixed);
            await page.reload();
            await presentationReady(page);
            await control.locator('[part=trigger] button').waitFor();
            assert.ok(await heading.isVisible());
            report.examples.push({ route, heading: await heading.textContent(), theme: await owner.getAttribute('data-theme'),
                background: await owner.evaluate(node => getComputedStyle(node).backgroundColor), restored: true });
        }
        assert.deepEqual(errors, [], 'Combined Site/gallery browser errors');
        report.status = 'passed';
        await context.close();
        return report;
    } finally { await browser.close(); }
}

async function presentationReady(page) {
    await page.waitForFunction(() => {
        const owner = document.querySelector('cem-gallery-page');
        return owner?.getAttribute('data-theme') === 'cem-theme-projector'
            && document.querySelector('cem-gallery-nav cem-theme-switch')?.getAttribute('value') === 'cem-theme-projector'
            && localStorage.getItem('cem-playground-theme') === 'cem-theme-projector'
            && getComputedStyle(owner).backgroundColor === 'rgb(255, 255, 255)';
    });
}
