import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { expect } from 'playwright/test';

const demo = '/examples/web/cem-application/';
const runtime = '/packages/cem-elements/dist/';
const css = '/packages/cem-elements/demo/demo.css';
const repo = new URL('../../../', import.meta.url);

/** Run the published example itself, including its trusted host and ordinary location-element controls. */
export async function verifyBrowserPolicy(context, origin, mode) {
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.route(origin + '/**', async route => {
            const pathname = new URL(route.request().url()).pathname;
            if (!(pathname.startsWith(demo) || pathname.startsWith(runtime) || pathname === css)) return route.fallback();
            await route.fulfill({ path: fileURLToPath(new URL('.' + pathname, repo)) });
        });
        await page.goto(origin + demo + 'index.html?host=' + mode);
        const card = name => page.locator('section').filter({ has: page.getByRole('heading', { name, exact: true }) });
        const writer = card('Workspace editor'), notes = card('Private notes'), guest = card('Uninvited editor');
        const personal = card('Personal editor'), review = card('Reviewer');
        const route = card => (mode === 'iframe' ? card.frameLocator('iframe') : card).locator('workspace-route');
        const read = async (card, value) => {
            await card.getByRole('button', { name: 'Read draft' }).click();
            await expect(card.locator('form output')).toHaveText(value);
        };
        const write = async (card, value, result = 'Saved') => {
            await card.getByRole('textbox', { name: 'Draft' }).fill(value);
            await card.getByRole('button', { name: 'Save draft' }).click();
            await expect(card.locator('form output')).toHaveText(result);
        };
        for (const kind of ['local', 'session']) {
            await page.getByLabel('Storage lifetime').selectOption(kind);
            await read(writer, 'Empty');
            await write(writer, 'First ' + kind);
            await read(review, 'First ' + kind);
            await write(review, 'Second', 'Write unavailable');
            await read(writer, 'First ' + kind);
            await read(personal, 'Empty');
            await write(notes, 'Personal ' + kind);
            await read(writer, 'First ' + kind);
            await read(guest, kind === 'local' ? 'Read unavailable' : 'Empty');
        }
        const reviewRoot = mode === 'iframe' ? review.frameLocator('iframe').locator('cem-application') : review.locator('cem-application');
        await reviewRoot.evaluate(app => {
            const doc = app.ownerDocument;
            const declaration = doc.createElement('cem-element');
            declaration.setAttribute('tag', 'reviewer-local-draft');
            declaration.innerHTML = `<template type="text/cem-ml">
{local-storage @key=draft @slice=draft @type=text @live=true @value=unauthorized}
{input @aria-label="Local draft" @value="{$datadom.slices.draft}" @slice=draft @slice-event=input @slice-value="$target.value"}
{output | {$datadom.slices.draft}}
</template>`;
            app.append(declaration, doc.createElement('reviewer-local-draft'));
        });
        const reader = reviewRoot.locator('reviewer-local-draft');
        await expect(reader.locator('output')).toHaveText('First local');
        await reader.getByLabel('Local draft').fill('attempted');
        await expect(reader.getByLabel('Local draft')).toHaveValue('First local');
        await page.getByLabel('Storage lifetime').selectOption('local');
        await read(writer, 'First local');
        await write(writer, 'Revised');
        await expect(reader.locator('output')).toHaveText('Revised');
        await page.getByLabel('Storage lifetime').selectOption('session');
        await page.getByRole('button', { name: 'Read as nested preview' }).click();
        await expect(page.locator('#nested-status')).toHaveText('Nested preview has no export');
        await route(writer).getByRole('button').click();
        for (const card of [writer, notes, review]) await expect(route(card).locator('output')).toHaveText('#editor');
        await route(review).getByRole('button').click();
        await expect(route(writer).locator('output')).toHaveText('#editor');
        await route(personal).getByRole('button').click();
        await expect(route(personal).locator('output')).toHaveText('#personal');
        await route(notes).getByRole('button').click();
        await expect(route(review).locator('output')).toHaveText('#notes');
        assert.equal(new URL(page.url()).hash, '');
        await page.getByRole('button', { name: 'Withdraw access' }).click();
        await read(review, 'Read unavailable');
        await page.getByRole('button', { name: 'Read retained local handle' }).click();
        await expect(page.locator('#grant-status')).toHaveText('Retained handle refused');
        await page.getByRole('button', { name: 'Bind separate group' }).click();
        await read(review, 'Empty');
        await expect(route(review).locator('output')).toHaveText('');
        await page.getByRole('button', { name: 'Read retained local handle' }).click();
        await expect(page.locator('#grant-status')).toHaveText('Retained handle refused');
        await read(writer, 'First session');

        const boundary = mode === 'iframe' ? personal.frameLocator('iframe').locator('cem-application') : personal.locator('cem-application');
        const result = await boundary.evaluate(async root => {
            const { getCemApplicationStorage: storage, grantCemApplicationBrowserServices: grant } =
                await import('/packages/cem-elements/dist/index.js');
            const create = (parent, policy) => {
                const app = parent.ownerDocument.createElement('cem-application');
                app.innerHTML = policy;
                return parent.appendChild(app);
            };
            const refuses = fn => { try { fn(); return false; } catch { return true; } };
            const a = create(root, '<browser-policy scope="x"></browser-policy>');
            a.setAttribute('shared', '');
            const b = create(root, '<browser-policy scope="other" session-storage="private"></browser-policy>');
            b.setAttribute('shared', '');
            const config = { scope: 'x', groupId: 'same', 'local-storage': 'read-write' };
            const fallback = storage(a), session = storage(a, 'session');
            fallback.setItem('draft', 'private');
            const first = grant(root, a, config);
            const ownSession = storage(b, 'session');
            const second = grant(root, b, { ...config, scope: 'other', 'session-storage': 'read-write' });
            const old = storage(a);
            old.setItem('draft', 'group');
            let events = 0;
            old.subscribe('draft', () => events++);
            storage(b).setItem('draft', 'changed');
            const retired = grant(root, a, { ...config, groupId: 'replacement', 'local-storage': 'read' });
            first.revoke();
            storage(b).setItem('draft', 'after rebind');
            const nested = create(a, '<browser-policy scope="x" local-storage="shared"></browser-policy>');
            const bad = create(root, '');
            bad.setAttribute('private', ''); bad.setAttribute('shared', '');
            const typo = create(root, '<browser-policy loction="shared"></browser-policy>');
            const checks = {
                oldPrivateRefused: refuses(() => fallback.getItem('draft')), siblingRetainsGroup: storage(b).getItem('draft'),
                oldGrantRefused: refuses(() => old.removeItem('draft')), events, newGroupEmpty: storage(a).getItem('draft'),
                unexportedSession: storage(a, 'session') === session,
                explicitPrivate: storage(b, 'session') === ownSession,
                scopeMismatchRefused: refuses(() => grant(root, b, config)), childRefused: refuses(() => grant(a, root, config)),
                skippedParentRefused: refuses(() => grant(root, nested, config)), nestedDenied: refuses(() => storage(nested)),
                conflictsDenied: refuses(() => storage(bad)), typoDenied: refuses(() => storage(typo)),
            };
            second.revoke(); retired.revoke();
            return checks;
        });
        assert.deepEqual(result, { oldPrivateRefused: true, siblingRetainsGroup: 'after rebind', oldGrantRefused: true,
            events: 1, newGroupEmpty: null, unexportedSession: true, childRefused: true, skippedParentRefused: true,
            nestedDenied: true, conflictsDenied: true, typoDenied: true, explicitPrivate: true, scopeMismatchRefused: true });
        await page.reload();
        await page.getByLabel('Storage lifetime').selectOption('local');
        await read(review, 'Revised');
        await page.getByLabel('Storage lifetime').selectOption('session');
        await read(review, 'First session');
        await expect(route(review).locator('output')).toHaveText('');
        assert.deepEqual(errors, []);
    } finally { await page.close(); }
}
