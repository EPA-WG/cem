import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { emitNativeHtmlBaseEvidence } from './verify-html-base.mjs';

// Execution-host fixture only: drive actual browser events and capture scalar
// observations at each boundary. This is not a production DOM observer or parser.
export async function verifyHtmlBaseFreezeEvidence(browser, workspaceRoot) {
    const cases = JSON.parse(await readFile(
        resolve(workspaceRoot, 'packages/cem_ml/tests/fixtures/html-base-freezes.json'), 'utf8',
    ));
    const context = await browser.newContext();
    try {
        const page = await context.newPage();
        await page.route('**/*', (route) => route.fulfill({
            contentType: 'text/html; charset=utf-8', body: cases[0].markup,
        }));
        await page.goto(cases[0].url);
        const observed = [];
        for (const step of cases) {
            const record = await page.evaluate(({ name, action, href, url }) => {
                const firstBase = () => Array.from(document.getElementsByTagNameNS('http://www.w3.org/1999/xhtml', 'base'))
                    .find((base) => base.hasAttribute('href'));
                const state = globalThis.fixtureBaseCapture ??= {
                    epoch: 0, revision: 0, policy: 'allow:1', freezePolicy: '', fallback: '',
                };
                if (action === 'history') history.pushState(null, '', url);
                else if (action === 'href') firstBase().setAttribute('href', href);
                else if (action === 'remove') firstBase().remove();
                else if (action === 'policy') {
                    const meta = document.createElement('meta');
                    meta.httpEquiv = 'Content-Security-Policy';
                    meta.content = "base-uri 'none'";
                    document.head.append(meta);
                    state.policy = 'deny:2';
                } else if (action !== 'load') throw new Error(`unknown action ${action}`);
                state.revision += 1;
                const selected = firstBase();
                if (['load', 'href', 'remove'].includes(action) || (action === 'history' && !selected)) {
                    state.epoch += 1;
                    state.fallback = document.URL;
                    state.freezePolicy = state.policy;
                }
                return {
                    name, url: document.URL, selected_href: selected?.getAttribute('href') ?? null,
                    base_uri: document.baseURI, freeze_fallback: state.fallback,
                    freeze_policy: state.freezePolicy, current_policy: state.policy,
                    epoch: state.epoch, revision: String(state.revision),
                };
            }, step);
            observed.push({ ...record, engine: 'chromium', version: browser.version() });
        }
        const confirmed = await emitNativeHtmlBaseEvidence(workspaceRoot, '--verify-freeze-trace', observed);
        assert.deepEqual(confirmed.map(({ name }) => name), cases.map(({ name }) => name));
        assert.equal(confirmed.filter(({ captured }) => captured).length, 5);
        assert.deepEqual(confirmed.filter(({ read_set_valid }) => !read_set_valid).map(({ name }) => name), [
            'history-retains-freeze', 'late-policy-retains-freeze',
            'history-without-base',
        ]);
        assert.equal(confirmed.at(-1).effective_url, null);
        assert.match(confirmed.at(-1).rejection, /disagrees with the selected compatibility profile/);
        console.log('Verified eight Chromium freeze transitions: five captures, two stale read sets, and rejection of the unsupported stale fallback after CSP/removal/history.');
        return { schemaVersion: 1, chromiumVersion: browser.version(), transitions: confirmed };
    } finally {
        await context.close();
    }
}
