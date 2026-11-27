import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, firefox } from 'playwright';

const repo = fileURLToPath(new URL('../../../', import.meta.url));
const dist = path.join(repo, 'packages/cem-elements/dist');
const temp = await mkdtemp(path.join(tmpdir(), 'cem-two-editor-'));
const host = await readFile(new URL('./two-editor-host.mjs', import.meta.url));
const shell = '<!doctype html><meta charset=utf-8><button id=file text-key="file.menu" popovertarget="commands">File</button>'
    + '<div popover id=commands><button id=save command-id="file.save-as"></button></div>'
    + '<style>iframe,section {display:block;width:600px;height:240px}dialog{margin:0}</style>';
const frame = '<!doctype html><meta charset=utf-8><main></main><script type=module>import {editor} from "/fixture.mjs";window.makeEditor=editor</script>';
const server = createServer(async (req, res) => {
    try {
        const url = new URL(req.url, 'http://localhost').pathname;
        if (url === '/' || url === '/editor.html') { res.setHeader('Content-Type', 'text/html'); res.end(url === '/' ? shell : frame); return; }
        res.setHeader('Content-Type', url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
        if (url === '/fixture.mjs') { res.end(host); return; }
        assert.ok(url.startsWith('/runtime/'));
        const file = path.resolve(dist, url.slice('/runtime/'.length));
        assert.ok(file.startsWith(dist + path.sep));
        res.end(await readFile(file));
    } catch { res.writeHead(404).end(); }
});
const xmlPath = path.join(temp, 'manifest.xml');
try {
    const cli = path.join(repo, 'dist/target/cem_ml_cli/debug/cem-ml' + (process.platform === 'win32' ? '.exe' : ''));
    execFileSync(cli, ['convert', path.join(repo, 'packages/cem-components/docs/examples/file-save-as.v1.cem'),
        '--from-format', 'cem', '--schema', 'https://cem.dev/ns/cem-ml/1', '--to-format', 'xml',
        '--out', xmlPath, '--report', path.join(temp, 'convert.json'), '--output-color-type', 'none']);
    const xml = await readFile(xmlPath, 'utf8');
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    for (const engine of [chromium, firefox]) {
        const browser = await engine.launch();
        try {
            for (const mode of ['light', 'iframe']) {
                for (const scenario of ['save', 'revoke', 'late-receipt']) {
                    const page = await browser.newPage();
                    try { await run(page, engine.name(), mode, scenario, xml); }
                    finally { await page.close(); }
                    console.log(engine.name(), mode, scenario, 'passed');
                }
            }
        } finally { await browser.close(); }
    }
} finally {
    if (server.listening) await new Promise(resolve => server.close(resolve));
    await rm(temp, { recursive: true, force: true });
}

async function run(page, browser, mode, scenario, xml) {
    page.setDefaultTimeout(10000);
    const errors = [], writes = [];
    page.on('pageerror', error => errors.push(error.message));
    const destination = [browser, mode, scenario, 'private-destination.txt'].join('-');
    await page.exposeFunction('commitTestFile', async request => {
        writes.push(request);
        assert.equal(request.parameters.filePath, destination);
        await writeFile(path.join(temp, destination), request.document.content, { flag: 'wx' });
    });
    await page.addInitScript(() => {
        const projections = [];
        document.addEventListener('formdata', event => projections.push([...event.formData.entries()]));
        document.addEventListener('input', event => projections.push({ data: event.data, value: event.target.value }));
        document.addEventListener('keydown', event => { if (event.key.toLowerCase() === 's') window.keyClaimed = event.defaultPrevented; });
        window.projections = projections;
    });
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(async ([mode, xml, useGeneratedForm]) => {
        window.fixture = await (await import('/fixture.mjs')).start(mode, xml, useGeneratedForm);
    }, [mode, xml, scenario === 'late-receipt']);
    const scope = id => mode === 'light' ? page.locator('#' + id) : page.frameLocator('#' + id);
    const draft = id => scope(id).getByRole('textbox', { name: 'Draft' });
    const editorFrame = async id => mode === 'light' ? page : (await (await page.locator('#' + id).elementHandle()).contentFrame());
    const a = await editorFrame('A'), b = await editorFrame('B');
    const dialog = id => scope(id).getByRole('dialog');
    const pathInput = dialog('A').getByRole('textbox');
    const open = async () => {
        await draft('A').focus();
        await page.locator('#file').click();
        // A host menu pins its content owner even if focus is subsequently moved.
        await draft('B').focus();
        assert.equal(await page.locator('#save').isEnabled(), true);
        await page.locator('#save').click();
        assert.equal(await dialog('A').isVisible(), true);
    };
    const inspect = () => page.evaluate(() => window.fixture.inspect());
    async function privateChannels() {
        const state = await inspect();
        const projections = await Promise.all([a, b, page].map(frame => frame.evaluate(() => window.projections)));
        assert.ok(!JSON.stringify([state, projections, errors]).includes(destination), 'private destination escaped the form');
        return state;
    }
    async function shortcut(frame, id, claimed) {
        await draft(id).focus();
        await frame.evaluate(() => { window.keyClaimed = null; });
        await page.keyboard.press('Control+Shift+s');
        assert.equal(await frame.evaluate(() => window.keyClaimed), claimed);
    }
    await draft('A').fill('Pinned draft A');
    await draft('B').fill('Distinct draft B');
    await open();
    await pathInput.fill(destination);
    await dialog('A').getByRole('combobox').selectOption('text/markdown');
    await draft('B').fill('B edited during A’s form');
    assert.equal(await dialog('A').isVisible(), true);
    await draft('A').fill('A edited after pin');
    await pathInput.focus();
    assert.deepEqual(await pathInput.evaluate(input => [...new FormData(input.form)]), []);
    const editing = await privateChannels();
    assert.deepEqual(Object.keys(editing.snapshots.A.snapshot.formData), ['draft']);
    assert.deepEqual(Object.keys(editing.snapshots.A.snapshot.validationState), ['draft']);
    assert.equal(editing.snapshots.A.snapshot.formData.draft.body, 'A edited after pin');
    assert.ok(JSON.stringify(editing.snapshots.B.snapshot.eventPayloads).includes('B edited during'));

    if (scenario !== 'late-receipt') {
        await page.evaluate(() => window.fixture.beginFrench());
        if (scenario === 'revoke') {
            assert.equal(await page.locator('#save').textContent(), 'Save As…');
            await page.evaluate(() => window.fixture.grant('A', false));
            assert.equal(await dialog('A').count(), 0);
            await draft('A').focus(); await page.locator('#file').click();
            assert.equal(await page.locator('#save').isDisabled(), true);
            await page.locator('#save').dispatchEvent('click');
            await page.keyboard.press('Escape');
            await shortcut(a, 'A', false);
        }
        await page.evaluate(() => window.fixture.apply());
        await page.waitForFunction(() => document.querySelector('#file').textContent === 'Fichier');
        if (scenario === 'save') {
            assert.ok(await dialog('A').locator('[submit-role=accept]').isDisabled());
            assert.equal(await pathInput.inputValue(), destination);
            assert.equal(await pathInput.evaluate(input => input.ownerDocument.activeElement === input), true);
            assert.equal(await dialog('A').getAttribute('aria-label'), 'Enregistrer sous');
            assert.equal(await page.locator('#save').textContent(), 'Enregistrer le document sous…');
            await shortcut(b, 'B', false);
            assert.equal(await page.locator('#save').getAttribute('aria-keyshortcuts'), null);
        }
        await page.evaluate(() => window.fixture.confirm());
        if (scenario === 'revoke') {
            await shortcut(a, 'A', false);
            await page.locator('#file').click(); assert.equal(await page.locator('#save').isDisabled(), true);
            await page.locator('#save').dispatchEvent('click');
            const state = await privateChannels();
            assert.deepEqual([writes.length, state.checkpoint.receipt.operations.length, state.results.length], [0, 0, 0]);
            assert.deepEqual(errors, []);
            return;
        }
        assert.equal(await page.locator('#save').getAttribute('aria-keyshortcuts'), 'Control+Shift+S');
        await shortcut(b, 'B', true);
        // Cancel B's independent invocation. The private form opened for A stays live.
        await dialog('B').getByRole('button', { name: 'Annuler' }).click();
    } else await page.evaluate(() => window.fixture.holdReceipt());
    await dialog('A').locator('[submit-role=accept]').click();
    await page.waitForFunction(async () => (await window.fixture.inspect()).writes.length === 1);
    if (scenario === 'late-receipt') {
        assert.equal((await inspect()).checkpoint.receipt.operations[0].outcome, 'unknown');
        await page.evaluate(() => window.fixture.replace('A'));
        await page.evaluate(() => window.fixture.releaseReceipt());
    }
    await page.waitForFunction(async () => (await window.fixture.inspect()).checkpoint.receipt.operations[0]?.outcome === 'acknowledged');
    const state = await inspect();
    assert.ok(!JSON.stringify([state, errors]).includes(destination));
    assert.equal(state.checkpoint.receipt.operations.length, 1);
    assert.equal(state.writes.length, 1); assert.equal(writes.length, 1);
    assert.deepEqual(writes[0].document, { id: 'A', revision: '2', content: 'Pinned draft A' });
    assert.equal(writes[0].parameters.contentType, 'text/markdown');
    assert.equal(await readFile(path.join(temp, destination), 'utf8'), 'Pinned draft A');
    assert.equal(await draft('B').inputValue(), 'B edited during A’s form');
    if (scenario === 'save') {
        assert.deepEqual(state.results.map(({ id, status }) => [id, status]), [['B', 'cancelled'], ['A', 'succeeded']]);
        assert.deepEqual(state.store, { lastContentType: 'text/markdown' });
        await privateChannels();
    } else assert.deepEqual(state.results, []);
    assert.deepEqual(errors, []);
}
