import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { promisify } from 'node:util';
import { resolve } from 'node:path';

// A native process owns preparation and both evidence hosts for the entire run.
// Only named HTML delivery and individual scalar predicates cross this fixture
// channel. No AST snapshot or execution seal is reconstructed in JavaScript.
export async function verifyRecordedOutput(browser, workspaceRoot) {
    await promisify(execFile)('cargo', ['build', '-p', 'cem-ql', '--example', 'recorded_output_probe', '--quiet'], {
        cwd: workspaceRoot, env: { ...process.env, CARGO_TARGET_DIR: 'dist/target' }, maxBuffer: 4 * 1024 * 1024,
    });
    const cases = [];
    for (const mode of ['unchanged', 'text', 'extra-node', 'cssom', 'disabled', 'move-style', 'environment', 'callback-write']) {
        const context = await browser.newContext();
        const page = await context.newPage();
        let markup = '', result, stderr = '';
        await page.route('**/*', route => route.fulfill({ contentType: 'text/html; charset=utf-8', body: `<!doctype html>${markup}` }));
        const process = spawn(resolve(workspaceRoot, 'dist/target/debug/examples/recorded_output_probe'), [], { cwd: workspaceRoot, stdio: ['pipe', 'pipe', 'pipe'] });
        process.stderr.on('data', chunk => { stderr += chunk; });
        const done = new Promise((resolveDone, reject) => { process.on('error', reject); process.on('close', code => code === 0 ? resolveDone() : reject(new Error(`${mode}: native probe exit ${code}: ${stderr}`))); });
        // Attach rejection handling while the request stream is still active.
        done.catch(() => { /* The awaited completion below reports native failures. */ });
        const timer = setTimeout(() => process.kill(), 60_000);
        try {
            for await (const line of createInterface({ input: process.stdout })) {
                const request = JSON.parse(line);
                if (request.kind === 'result') { result = request; continue; }
                let reply;
                if (request.kind === 'output') {
                    markup = request.html;
                    await page.goto('https://output.test/docs/whole.html');
                    await page.evaluate(() => {
                        globalThis.wholeProbe = { slots: new Map(), identities: new WeakMap(), sheets: new WeakMap(), next: 1, nextSheet: 1, revision: 1, undo: () => { /* Unchanged case has nothing to restore. */ }, mutateOnSheet: false };
                    });
                    reply = true;
                } else if (request.kind === 'ready') {
                    assert.equal(request.sheets, 3);
                    reply = await page.evaluate(mode => {
                        const state = globalThis.wholeProbe;
                        const card = getComputedStyle(document.querySelector('cem-card > p'));
                        const inner = getComputedStyle(document.querySelector('cem-inner > p'));
                        const outside = getComputedStyle(document.querySelector('body > p'));
                        if (card.color !== 'rgb(1, 2, 3)' || card.fontWeight !== '700' || inner.backgroundColor !== 'rgb(4, 5, 6)' || outside.color === card.color || outside.fontWeight === '700') throw new Error('native private/shared scopes did not apply to the intended output');
                        const text = document.querySelector('cem-card > p').firstChild;
                        const original = text.nodeValue;
                        const sheet = document.querySelector('style').sheet;
                        if (mode === 'text') { text.nodeValue = 'Changed'; state.undo = () => { text.nodeValue = original; }; }
                        if (mode === 'extra-node') { const node = document.createElement('p'); document.body.append(node); state.undo = () => node.remove(); }
                        if (mode === 'cssom') { sheet.insertRule('p{color:purple}', 0); state.undo = () => sheet.deleteRule(0); }
                        if (mode === 'disabled') { sheet.disabled = true; state.undo = () => { sheet.disabled = false; }; }
                        if (mode === 'move-style') { const style = document.querySelector('style'); const next = style.nextSibling; document.body.append(style); state.undo = () => document.head.insertBefore(style, next); }
                        if (mode === 'environment') { state.revision++; state.undo = () => { state.revision--; }; }
                        if (mode === 'callback-write') { state.mutateOnSheet = true; state.undo = () => { text.nodeValue = original; }; }
                        return mode === 'unchanged';
                    }, mode);
                } else {
                    reply = await page.evaluate(request => {
                        const state = globalThis.wholeProbe;
                        try {
                            if (request.kind === 'restore') { state.undo(); return true; }
                            if (request.kind === 'freeze') { const base = document.querySelector('base[href]'); if (base) base.setAttribute('href', base.getAttribute('href')); return document.baseURI; }
                            if (request.kind === 'revision') {
                                if (document.adoptedStyleSheets.length || [...document.querySelectorAll('*')].some(node => node.shadowRoot)) throw new Error('unsupported installation universe');
                                return `revision:${state.revision}`;
                            }
                            if (request.kind === 'sheet') {
                                const node = state.slots.get(request.owner);
                                const sheet = node?.sheet;
                                if (!sheet || sheet.ownerNode !== node) throw new Error('missing sheet');
                                const parser = document.implementation.createHTMLDocument('');
                                const expected = parser.createElement('style'); expected.textContent = request.text; parser.head.append(expected);
                                const cssom = sheet => Array.from(sheet.cssRules, rule => rule.cssText).join('\n');
                                const matches = cssom(sheet) === cssom(expected.sheet);
                                let installation = state.sheets.get(sheet);
                                if (!installation) { installation = state.nextSheet++; state.sheets.set(sheet, installation); }
                                if (state.mutateOnSheet) { state.mutateOnSheet = false; document.querySelector('cem-card > p').firstChild.nodeValue = 'Callback change'; }
                                return { selected: !sheet.disabled, matches, installation };
                            }
                            if (request.kind !== 'slot') throw new Error('unknown native predicate');
                            const children = node => Array.from(node.childNodes).filter(n => n.nodeType !== 10);
                            const attributes = node => node.nodeType === 1 ? Array.from(node.attributes).filter(a => !(node === document.documentElement && a.name === 'xmlns' && a.value === 'http://www.w3.org/1999/xhtml')) : [];
                            if (request.parent === null) state.slots.clear();
                            const parent = state.slots.get(request.parent);
                            const node = request.parent === null ? document : request.nodeKind === 2 ? attributes(parent)[request.index] : children(parent)[request.index];
                            if (!node || node.nodeType !== request.nodeKind || state.slots.has(request.id) || children(node).length !== request.children || attributes(node).length !== request.attributes) throw new Error('topology mismatch');
                            if ((node.nodeType === 1 || node.nodeType === 2) && ((node.namespaceURI ?? '') !== request.namespace || node.localName !== request.name)) throw new Error('name mismatch');
                            if ([2, 3, 8].includes(node.nodeType) && node.nodeValue !== request.value) throw new Error('value mismatch');
                            let identity = state.identities.get(node);
                            if (!identity) { identity = state.next++; state.identities.set(node, identity); }
                            state.slots.set(request.id, node);
                            return identity;
                        } catch (error) { return { error: String(error) }; }
                    }, request);
                }
                process.stdin.write(`${JSON.stringify(reply)}\n`);
            }
            await done;
            assert.ok(result, `${mode}: missing native result`);
            assert.equal(result.valid, mode === 'unchanged');
            assert.ok(result.blockers.includes('atomic-host-transaction-required'));
            assert.ok(result.blockers.includes('css-runtime-dependency-closure-required'));
            cases.push(mode);
        } finally { clearTimeout(timer); process.kill(); await context.close(); }
    }
    console.log(`Verified ${cases.length} native whole-output cases against Chromium ${browser.version()}.`);
    return { cases, chromiumVersion: browser.version() };
}
