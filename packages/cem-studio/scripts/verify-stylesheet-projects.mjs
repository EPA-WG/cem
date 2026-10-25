import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { verifySelectorProject } from './verify-selector-project.mjs';

/** Native styles, owned modules and ordinary shared output previews. */
export async function verifyStylesheetProjects({ page, context, url }) {
    const evidence = {};
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), preview = current.projectPreview;
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty,
            text: current.projection?.output?.text, diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics,
            buildStatus: preview.attempt?.status, runId: preview.projection?.buildRun.id, entry: preview.entryArtifact,
            html: preview.projection?.output?.text, artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            origins: preview.projection?.buildRun.artifacts.flatMap(item => item.provenance.map(frame => frame.uri)) };
    });
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); };
    const run = async (project, configuration) => {
        await open({ project, configuration: configuration.id, build: 'build-html' });
        const current = await state(); assert.equal(current.editable, true, JSON.stringify(current));
        const name = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name, exact: true }).isVisible()) {
            const label = configuration.kind === 'validation' ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await click(name); return state();
    };
    const build = async project => { await open({ project, build: 'build-html' }); await click('Build project'); return state(); };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        return (await state()).route.project;
    };
    const rendered = async (id, edited) => {
        const expected = id === 'css-stylesheet' ? edited ? 'rgb(128, 48, 24)' : 'rgb(24, 92, 128)' : edited ? 'rgb(102, 51, 0)' : 'rgb(0, 51, 102)';
        const selector = id === 'css-stylesheet' ? '.catalog-card' : '.demo-card';
        await page.waitForFunction(({ selector, expected }) => {
            const doc = document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument;
            const card = doc?.querySelector(selector);
            return card && doc.defaultView.getComputedStyle(card.querySelector('h2')).color === expected;
        }, { selector, expected });
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin'); assert.equal(await frame.getAttribute('allow'), '');
        const visual = await frame.evaluate((element, selector) => {
            const doc = element.contentDocument, card = doc.querySelector(selector), style = node => doc.defaultView.getComputedStyle(node);
            const outside = doc.querySelector('main > h2');
            return { color: style(card.querySelector('h2')).color, padding: style(card).paddingTop, weight: style(card.querySelector('h2')).fontWeight,
                outside: outside ? style(outside).color : null, width: card.getBoundingClientRect().width, scripts: doc.querySelectorAll('script,iframe,object,embed').length };
        }, selector);
        assert.equal(visual.color, expected); assert.ok(visual.width > 0); assert.equal(visual.scripts, 0);
        if (id === 'css-stylesheet') { assert.notEqual(visual.outside, expected); assert.equal(visual.padding, '8px'); }
        else { assert.equal(visual.padding, edited ? '16px' : '12px'); assert.equal(visual.weight, '700'); }
    };

    for (const id of ['css-stylesheet', 'scss-stylesheet']) {
        const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
        const resource = name => definition.sources.find(item => item.id === name);
        const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
        const repair = definition.configurations.find(item => item.repair), graph = definition.configurations.find(item => item.kind === 'transformation-graph');
        const projection = definition.configurations.find(item => item.expectedOutput), primary = projection.source;
        const template = graph.dependencies.find(name => resource(name).role === 'template');
        const mutable = id === 'scss-stylesheet' ? 'scss-tokens' : primary;
        const originalIds = [...new Set([primary, mutable, repair.source, template])], originals = {};
        const expectedHtml = await fixture(graph.expectedGraphOutputs[graph.entryArtifact]);
        await page.goto(`${url}/?collection=content-types-documents-ui`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        for (const name of originalIds) {
            await open({ collection: 'content-types-documents-ui', sample: id, file: name, build: 'build-html' });
            assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); originals[name] = await readonly.evaluate(element => element.value);
        }
        await open({ collection: 'content-types-documents-ui', sample: id, build: 'build-html' }); await click('Build project');
        const sample = await state(); assert.equal(sample.buildStatus, 'succeeded', JSON.stringify(sample));
        assert.equal(sample.html, expectedHtml); assert.equal(sample.entry, graph.entryArtifact); assert.ok(sample.artifacts.some(item => item.path.endsWith('.css'))); await rendered(id, false);
        const project = await clone(`Navigator ${definition.name}`); assert.ok(project);
        for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph' && !item.repair)) {
            const result = await run(project, configuration); assert.equal(result.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(result));
            if (configuration.expectedOutput) assert.equal(result.text, await fixture(configuration.expectedOutput));
            for (const text of configuration.expectedText ?? []) assert.ok(result.text.includes(text), `${id}/${configuration.id}`);
        }
        const failed = await run(project, repair), expected = repair.expectedValidation;
        const diagnostic = failed.diagnostics?.find(item => item.code === expected.errorCode); assert.ok(diagnostic, JSON.stringify(failed));
        assert.equal(diagnostic.severity, 'error'); assert.equal(diagnostic.uri, `studio://${project}/${resource(repair.source).path}`);
        for (const [key, value] of Object.entries(expected.location)) assert.equal(diagnostic[key] ?? null, value);
        await open({ project, file: repair.source, build: 'build-html' });
        const repaired = originals[repair.source].replace(repair.repair.find, repair.repair.replace); assert.notEqual(repaired, originals[repair.source]); await save(repaired);
        assert.equal((await run(project, repair)).status, 'valid');
        console.log(`[cem-studio:${id}] catalog, native configurations, exact output and source repair passed`);

        const before = await run(project, projection);
        const mainEdit = id === 'css-stylesheet' ? originals[primary].replace('rgb(24, 92, 128)', 'rgb(128, 48, 24)') : originals[primary].replace('0.25rem', '0.5rem');
        assert.notEqual(mainEdit, originals[primary]); await editor.fill(mainEdit); await settle();
        await click('Run configuration'); assert.equal((await state()).text, before.text); assert.equal((await state()).dirty, true);
        await click('Save and validate'); await click('Run configuration'); assert.notEqual((await state()).text, before.text);
        let moduleEdit;
        if (id === 'scss-stylesheet') {
            await open({ project, file: mutable, build: 'build-html' });
            moduleEdit = originals[mutable].replace('#036', '#630').replace('0.75rem', '1rem'); assert.notEqual(moduleEdit, originals[mutable]); await save(moduleEdit);
        }
        const built = await build(project); assert.equal(built.buildStatus, 'succeeded', JSON.stringify(built)); await rendered(id, true);
        assert.ok(built.origins.some(uri => uri.endsWith(`/${resource(mutable).path}`))); assert.ok(built.origins.some(uri => uri.endsWith(`/${resource(template).path}`)));
        await open({ project, file: primary, build: 'build-html' }); await save(originals[repair.source]);
        const refused = await build(project); assert.equal(refused.buildStatus, 'failed'); assert.equal(refused.runId, built.runId); assert.deepEqual(refused.artifacts, built.artifacts);
        await open({ project, file: primary, build: 'build-html' }); await save(mainEdit);
        assert.equal((await build(project)).buildStatus, 'succeeded'); await rendered(id, true);
        console.log(`[cem-studio:${id}] persisted source/module edits, scoped CSS, provenance and last-good output passed`);

        const copy = await clone(`Portable ${definition.name}`); assert.ok(copy && copy !== project);
        await open({ project: copy, file: template, build: 'build-html' });
        const caption = id === 'css-stylesheet' ? 'Scoped card stylesheet' : 'SCSS card preview';
        const editedTemplate = originals[template].replace(caption, 'Independent saved stylesheet'); assert.notEqual(editedTemplate, originals[template]); await save(editedTemplate);
        const copied = await build(copy); assert.equal(copied.buildStatus, 'succeeded'); assert.ok(copied.html.includes('Independent saved stylesheet'));
        assert.equal((await build(project)).html, built.html);
        await context.setOffline(true);
        try {
            await page.reload(); await ready();
            const offline = await build(copy); assert.equal(offline.buildStatus, 'succeeded'); assert.equal(offline.html, copied.html); await rendered(id, true);
            assert.equal((await run(copy, projection)).status, 'projected');
            for (const [name, expectedText] of [[primary, mainEdit], [repair.source, repaired], ...(moduleEdit ? [[mutable, moduleEdit]] : [])]) {
                await open({ project: copy, file: name, build: 'build-html' }); assert.equal(await editor.evaluate(element => element.value), expectedText);
            }
            for (const name of originalIds) {
                await open({ collection: 'content-types-documents-ui', sample: id, file: name, build: 'build-html' }); assert.equal(await readonly.evaluate(element => element.value), originals[name]);
            }
        } finally { await context.setOffline(false); }
        console.log(`[cem-studio:${id}] independent native HTML/CSS builds and saved offline sources passed`);
        evidence[id] = { catalog: true, readonlyOriginals: true, nativeCommands: true, exactOutput: true, sourceRepair: true,
            staticEntry: true, cssArtifact: true, visibleStyles: true, nativeProvenance: true, unsavedSourceGuard: true,
            retainedFailure: true, independentCopies: true, offline: true, ...(moduleEdit ? { ownedModuleFreshness: true } : { scopeIsolation: true }) };
    }
    evidence['css-selector'] = await verifySelectorProject({ page, context, url });
    return evidence;
}
