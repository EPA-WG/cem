import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

// Compare native API output with real browser execution of the same fixture
// inputs. JSON here is named test control/scalar evidence, never a document AST.
export async function verifyHtmlBaseParity(browser, workspaceRoot) {
    const cases = JSON.parse(await readFile(
        resolve(workspaceRoot, 'packages/cem_ml/tests/fixtures/html-base.json'), 'utf8',
    ));
    const native = await emitNativeHtmlBaseEvidence(workspaceRoot);
    const chromiumVersion = browser.version();
    assert.deepEqual(native.map(({ name }) => name), cases.map(({ name }) => name));
    const results = [];
    const observations = [];
    for (const [index, fixture] of cases.entries()) {
        const context = await browser.newContext();
        try {
            const page = await context.newPage();
            await page.route('**/*', (route) => route.fulfill({
                status: 200,
                headers: {
                    'content-type': 'text/html; charset=utf-8',
                    ...(fixture.policy === 'deny' ? { 'content-security-policy': "base-uri 'none'" } : {}),
                },
                body: fixture.context === 'document'
                    ? fixture.markup : '<html><head></head><body></body></html>',
            }));
            if (fixture.context === 'document') {
                await page.goto(fixture.url);
            } else if (fixture.about_base_url) {
                await page.goto('https://parent.test/frame-owner.html');
                await page.evaluate(async ({ context: kind, about_base_url, markup }) => {
                    const base = document.createElement('base');
                    base.href = about_base_url;
                    document.head.append(base);
                    const frame = document.createElement('iframe');
                    if (kind === 'srcdoc') {
                        const loaded = new Promise((resolveLoad) => frame.addEventListener('load', resolveLoad, { once: true }));
                        frame.srcdoc = markup;
                        document.body.append(frame);
                        await loaded;
                    } else {
                        document.body.append(frame);
                        frame.contentDocument.open();
                        frame.contentDocument.write(markup);
                        frame.contentDocument.close();
                    }
                }, fixture);
            } else {
                assert.equal(fixture.context, 'blank');
                assert.equal(fixture.url, 'about:blank');
                await page.setContent(fixture.markup);
            }
            const actual = await page.evaluate((embedded) => {
                const doc = embedded ? document.querySelector('iframe').contentDocument : document;
                return {
                    effective_url: doc.baseURI,
                    selected_href: Array.from(doc.getElementsByTagNameNS('http://www.w3.org/1999/xhtml', 'base'))
                        .find((base) => base.hasAttribute('href'))?.getAttribute('href') ?? null,
                    encoding: doc.characterSet,
                };
            }, Boolean(fixture.about_base_url));
            assert.equal(actual.encoding, 'UTF-8', `${fixture.name}: execution encoding`);
            assert.equal(actual.selected_href, native[index].selected_href, `${fixture.name}: selected native slot`);
            const divergence = fixture.chromium_divergence;
            if (divergence) {
                // These are measured differences from the standard, not parity.
                // Any browser upgrade or behavior change requires explicit review.
                assert.equal(chromiumVersion, divergence.version, `${fixture.name}: review pinned Chromium divergence`);
                assert.equal(actual.effective_url, divergence.effective_url, `${fixture.name}: measured Chromium divergence`);
                assert.notEqual(actual.effective_url, native[index].effective_url, `${fixture.name}: remove resolved divergence`);
            } else {
                assert.equal(actual.effective_url, native[index].effective_url, `${fixture.name}: Chromium/native base URI`);
            }
            assert.equal(actual.effective_url, native[index].execution_url, `${fixture.name}: explicit Chromium compatibility`);
            observations.push({
                name: fixture.name, base_uri: actual.effective_url, selected_href: actual.selected_href,
                engine: 'chromium', version: chromiumVersion,
            });
            results.push({
                ...native[index], chromium_url: actual.effective_url,
                parity: divergence ? 'known-divergence' : 'matched',
            });
        } finally {
            await context.close();
        }
    }
    const matched = results.filter(({ parity }) => parity === 'matched').length;
    const divergent = results.length - matched;
    const confirmed = await emitNativeHtmlBaseEvidence(workspaceRoot, '--confirm-observations', observations);
    assert.deepEqual(confirmed, native, 'actual browser observations must confirm the native prepared evidence');
    console.log(`Checked ${results.length} native HTML base cases against Chromium ${chromiumVersion}: ${matched} standard matches, ${divergent} explicit compatibility cases, ${confirmed.length} native confirmations.`);
    return {
        schemaVersion: 1, profile: 'retained-utf8-html-snapshot', chromiumVersion,
        matched, divergent, compatibilityConfirmed: confirmed.length, cases: results,
    };
}

export function emitNativeHtmlBaseEvidence(workspaceRoot, mode, input) {
    return new Promise((resolveResult, rejectResult) => {
        const args = ['run', '--quiet', '-p', 'cem-ml', '--example', 'html-base-parity-emit'];
        if (mode) args.push('--', mode);
        const child = execFile('cargo', args, {
            cwd: workspaceRoot,
            env: { ...process.env, CARGO_TARGET_DIR: resolve(workspaceRoot, 'dist/target') },
            timeout: 300_000, maxBuffer: 4 * 1024 * 1024,
        }, (error, stdout) => {
            if (error) { rejectResult(error); return; }
            try { resolveResult(JSON.parse(stdout)); } catch (parseError) { rejectResult(parseError); }
        });
        child.stdin.on('error', rejectResult);
        child.stdin.end(input === undefined ? '' : JSON.stringify(input));
    });
}
