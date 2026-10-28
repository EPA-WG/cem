import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';

// Diagnostic only: fill remains CDP insertion, paste remains trusted clipboard
// input. Static DOM/style isolation below is never an editor implementation.
const workspace = resolve(import.meta.dirname, '../../..');
const url = new URL(process.argv[2] ?? 'http://127.0.0.1:8081/');
const output = resolve(workspace, 'dist/reports/cem-studio/editor-input');
const sourceBytes = await readFile(resolve(workspace, 'examples/projects/native-projections/projections/dispatch.ast.json'));
const source = new TextDecoder('utf-8', { fatal: true }).decode(sourceBytes);
const declaration = await readFile(resolve(workspace, 'packages/cem-components/src/components/cem-source-editor/cem-source-editor.xhtml'), 'utf8');
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const report = { schemaVersion: 1, browser: browser.version(), node: process.version,
    source: { bytes: sourceBytes.length, utf16Units: source.length, lines: source.split('\n').length,
        sha256: createHash('sha256').update(sourceBytes).digest('hex') }, cases: [], passed: false };
await mkdir(output, { recursive: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, permissions: ['clipboard-read', 'clipboard-write'] });
const studioSelector = 'cem-studio-workbench-controls cem-source-editor [role=textbox]';
const settle = async page => {
    await page.waitForFunction(() => globalThis.__cemStudioApplication?.workbenchView);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
};
async function studio() {
    const page = await context.newPage(); page.setDefaultTimeout(30_000);
    await page.goto(new URL('studio/?sample=native-projections', url).href); await settle(page);
    await page.getByRole('button', { name: 'Clone selected', exact: true }).click();
    await page.getByRole('dialog').getByRole('textbox').fill('Editor input measurement');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle(page);
    const project = new URL(page.url()).searchParams.get('project'); assert.ok(project);
    await page.evaluate(project => globalThis.__cemStudioApplication.workbench.openEntity(`project=${project}&file=ast-export`), project);
    await settle(page);
    // Set up a distinct draft through the ordinary service, before native input.
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.updateDraft('initial'));
    await settle(page); return page;
}
async function measure(page, selector, name, kind, owned) {
    const control = page.locator(selector);
    await page.evaluate(({ selector, owned }) => {
        const control = document.querySelector(selector), host = owned ? control.closest('cem-source-editor, driver-editor') : control;
        const state = { control, host, before: 0, inputs: 0, publications: 0, trusted: true };
        globalThis.__cemEditorInput = state;
        window.addEventListener('beforeinput', event => { if (event.target === control) state.before++; }, true);
        window.addEventListener('input', event => { if (event.target === control) { state.inputs++; state.trusted &&= event.isTrusted; } }, true);
        host.addEventListener('input', () => state.publications++);
    }, { selector, owned });
    await control.focus();
    if (kind === 'paste') { await page.evaluate(source => navigator.clipboard.writeText(source), source); await control.press('ControlOrMeta+A'); }
    const session = process.env.CEM_EDITOR_INPUT_PROFILE === '1' ? await context.newCDPSession(page) : undefined;
    if (session) { await session.send('Profiler.enable'); await session.send('Profiler.start'); }
    const started = performance.now();
    try {
        if (kind === 'paste') await control.press('ControlOrMeta+V'); else await control.fill(source);
    } finally {
        if (session) {
            const { profile } = await session.send('Profiler.stop'); await session.detach();
            await writeFile(resolve(output, `${name}.cpuprofile`), JSON.stringify(profile));
        }
    }
    const milliseconds = performance.now() - started;
    const observed = await page.evaluate(() => {
        const { control, host, before, inputs, publications, trusted } = globalThis.__cemEditorInput;
        return { before, inputs, publications, trusted, nodes: control.childNodes.length, elements: document.querySelectorAll('*').length, styleSheets: document.styleSheets.length,
            value: control.value ?? control.innerText, focused: document.activeElement === control,
            selection: typeof control.selectionStart === 'number' ? [control.selectionStart, control.selectionEnd] : undefined,
            sameControl: host === control || host.querySelector('[role=textbox]') === control };
    });
    assert.equal(observed.value, source, `${name}: exact source`);
    assert.equal(observed.focused, true); assert.equal(observed.sameControl, true); assert.equal(observed.trusted, true);
    if (owned) {
        assert.equal(observed.publications, 1, `${name}: one final value publication`);
        assert.deepEqual(observed.selection, [source.length, source.length]);
    }
    delete observed.value;
    const row = { name, kind, milliseconds, ...observed }; report.cases.push(row);
    // Native clipboard acceptance includes undo/redo. Driver fragmentation and
    // timing are observations, not assertions that require a browser defect.
    if (kind === 'paste') {
        await control.press('ControlOrMeta+z');
        assert.equal(await control.evaluate(element => element.value), 'initial');
        await control.press('ControlOrMeta+y');
        assert.equal(await control.evaluate(element => element.value), source);
        row.undoRedo = true;
    }
    console.log(JSON.stringify(row));
}
try {
    const plain = await context.newPage(); plain.setDefaultTimeout(30_000); await plain.goto(url.href);
    await plain.evaluate(() => {
        const control = document.createElement('pre'); control.setAttribute('contenteditable', 'plaintext-only');
        control.textContent = 'initial'; Object.assign(control.style, { whiteSpace: 'pre', height: '500px', overflow: 'auto' }); document.body.append(control);
    });
    await measure(plain, 'pre[contenteditable]', 'native-fill', 'fill', false); await plain.close();

    const shared = await context.newPage(); shared.setDefaultTimeout(30_000); await shared.goto(url.href);
    await shared.evaluate(async declaration => {
        const { CemElementRuntime, createCemDeclarationScope } = await import('/studio/assets/@epa-wg/cem-elements/dist/index.js');
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationTag: 'driver-declaration', declarationScope: scope }); runtime.install(window);
        const template = document.createElement('template'); template.innerHTML = declaration;
        const source = template.content.querySelector('cem-element'), owner = document.createElement('driver-declaration');
        for (const attribute of source.attributes) if (attribute.name !== 'tag') owner.setAttribute(attribute.name, attribute.value);
        owner.setAttribute('tag', 'driver-editor'); owner.append(...source.childNodes); document.body.append(owner);
        await runtime.whenDeclarationSettled(owner);
        const host = document.createElement('driver-editor'); host.setAttribute('value', 'initial'); host.setAttribute('highlight', 'off');
        document.body.append(host); await runtime.whenRenderSettled(host);
    }, declaration);
    await measure(shared, 'driver-editor [role=textbox]', 'shared-fill', 'fill', true); await shared.close();

    const page = await studio();
    const snapshot = await page.evaluate(() => {
        const root = document.documentElement.cloneNode(true);
        for (const script of root.querySelectorAll('script')) script.remove();
        const base = document.createElement('base'); base.href = location.href; root.querySelector('head').prepend(base);
        return '<!doctype html>' + root.outerHTML;
    });
    for (const styles of [true, false]) {
        const isolated = await context.newPage(); isolated.setDefaultTimeout(30_000); await isolated.goto(url.href); await isolated.setContent(snapshot);
        if (!styles) await isolated.evaluate(selector => {
            for (const sheet of document.styleSheets) sheet.disabled = true;
            Object.assign(document.querySelector(selector).style, { display: 'block', whiteSpace: 'pre', height: '500px', width: '800px', overflow: 'auto' });
        }, studioSelector);
        await measure(isolated, studioSelector, styles ? 'static-studio-fill' : 'static-studio-no-styles-fill', 'fill', false);
        await isolated.close();
    }
    await measure(page, studioSelector, 'studio-fill', 'fill', true); await page.close();
    const paste = await studio(); await measure(paste, studioSelector, 'studio-paste', 'paste', true); await paste.close();
    report.passed = true;
} finally {
    await writeFile(resolve(output, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
    await context.close(); await browser.close();
}
