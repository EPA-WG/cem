import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Ordinary native projects rendered by the shared bounded artifact viewer. */
export async function verifySvgMathmlProjects({ page, context, url }) {
    const evidence = {};
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), preview = current.projectPreview;
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty, text: current.projection?.output?.text,
            diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics,
            buildStatus: preview.attempt?.status, runId: preview.projection?.buildRun.id, entry: preview.entryArtifact,
            html: preview.projection?.output?.text, artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            origins: preview.projection?.buildRun.artifacts.flatMap(item => item.provenance.map(frame => frame.uri)) };
    });
    const save = async value => { await editor.fill(value); await settle(); await click('Save and validate'); };
    const run = async (project, configuration, kind) => {
        await open({ project, configuration, build: 'build-html' });
        const current = await state(); assert.equal(current.editable, true, JSON.stringify(current));
        if (kind === 'validation') {
            const validate = page.getByRole('button', { name: 'Validate persisted revision', exact: true });
            if (!await validate.isVisible()) { await page.locator('cem-expansion[label="Editor details"] > [part~=root] [part~=header]').first().click(); await settle(); }
            await validate.click(); await settle(); return state();
        }
        const button = page.getByRole('button', { name: 'Run configuration', exact: true });
        if (!await button.isVisible()) { await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle(); }
        await button.click(); await settle(); return state();
    };
    const build = async project => { await open({ project, build: 'build-html' }); await click('Build project'); return state(); };
    const values = current => {
        const result = JSON.parse(current.text);
        assert.equal(result.language, 'xpath'); return result.result.sequence.items.map(item => item.value.lexicalValue);
    };
    const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
    const rendered = async family => {
        await page.waitForFunction(family => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector(family), family);
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin');
        assert.equal(await frame.getAttribute('allow'), '');
        return frame.evaluate((element, family) => {
            const doc = element.contentDocument, root = doc.querySelector(family), rect = root.getBoundingClientRect();
            const fraction = doc.querySelector('mfrac')?.getBoundingClientRect();
            return { namespace: root.namespaceURI, role: root.getAttribute('role'), label: root.getAttribute('aria-label'),
                title: root.querySelector('title')?.textContent, description: root.querySelector('desc')?.textContent,
                labels: [...root.querySelectorAll('text')].map(node => node.textContent), rectWidth: root.querySelector('rect')?.getAttribute('width'),
                variable: root.querySelector('mi')?.textContent, fraction: fraction && { width: fraction.width, height: fraction.height },
                superscripts: root.querySelectorAll('msup').length, width: rect.width, height: rect.height,
                scripts: doc.querySelectorAll('script,iframe,object,embed,foreignObject,annotation-xml').length,
                eventAttributes: [...doc.querySelectorAll('*')].flatMap(node => [...node.attributes]).filter(attr => /^on/i.test(attr.name)).length };
        }, family);
    };
    const assertGeometry = (visual, family, edited = false) => {
        assert.equal(visual.namespace, family === 'svg' ? 'http://www.w3.org/2000/svg' : 'http://www.w3.org/1998/Math/MathML');
        assert.ok(visual.width > 0 && visual.height > 0, JSON.stringify(visual));
        assert.equal(visual.scripts, 0); assert.equal(visual.eventAttributes, 0);
        if (family === 'svg') {
            assert.equal(visual.role, 'img'); assert.ok(visual.title && visual.description);
            assert.ok(visual.labels.includes(edited ? 'Café source' : 'Source'));
            assert.equal(visual.rectWidth, edited ? '72' : '80');
        } else {
            assert.equal(visual.variable, edited ? 'y' : 'x'); assert.ok(visual.label?.includes(edited ? 'y squared' : 'x squared'));
            assert.ok(visual.fraction?.width > 0 && visual.fraction?.height > 0); assert.equal(visual.superscripts, 1);
        }
    };

    for (const [id, family] of [['svg-diagram', 'svg'], ['mathml-formula', 'math']]) {
        const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
        const config = name => definition.configurations.find(item => item.id === name);
        const resource = name => definition.sources.find(item => item.id === name);
        const query = definition.configurations.find(item => item.expectedSemanticContract), primary = query.source;
        const repair = definition.configurations.find(item => item.repair);
        const graph = config('build-html');
        const template = graph.dependencies.find(name => resource(name).role === 'template');
        const fixture = async name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
        const expected = JSON.parse(await fixture(query.expectedSemanticContract));
        const html = await fixture(graph.expectedGraphOutputs[graph.entryArtifact]);
        await page.goto(`${url}/?collection=content-types-documents-ui`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        const originals = {};
        for (const name of [primary, repair.source, template]) {
            await open({ collection: 'content-types-documents-ui', sample: id, file: name, build: 'build-html' });
            assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); originals[name] = await readonly.evaluate(element => element.value);
        }
        await open({ collection: 'content-types-documents-ui', sample: id, build: 'build-html' }); await click('Build project');
        const sample = await state(); assert.equal(sample.buildStatus, 'succeeded', JSON.stringify(sample));
        assert.equal(sample.entry, graph.entryArtifact); assert.equal(sample.html, html); assertGeometry(await rendered(family), family);
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(`Navigator ${definition.name}`);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project);
        for (const operation of definition.configurations.filter(item => item.kind !== 'transformation-graph' && !item.repair)) {
            const result = await run(project, operation.id, operation.kind);
            assert.equal(result.status, operation.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(result));
            if (operation.expectedOutput) assert.equal(result.text, await fixture(operation.expectedOutput));
            if (operation.expectedSemanticContract) assert.deepEqual(values(result), expected);
        }
        await open({ project, configuration: repair.id, build: 'build-html' });
        const validate = page.getByRole('button', { name: 'Validate persisted revision', exact: true });
        if (!await validate.isVisible()) { await page.locator('cem-expansion[label="Editor details"] > [part~=root] [part~=header]').first().click(); await settle(); }
        await validate.click(); await settle();
        const failed = await state(), expectedFailure = repair.expectedValidation;
        const diagnostic = failed.diagnostics?.find(item => item.code === expectedFailure.errorCode);
        assert.ok(diagnostic, JSON.stringify(failed)); assert.equal(diagnostic.severity, 'error');
        assert.equal(diagnostic.uri, `studio://${project}/${resource(repair.source).path}`);
        for (const [key, value] of Object.entries(expectedFailure.location)) assert.equal(diagnostic[key] ?? null, value);
        await open({ project, file: repair.source, build: 'build-html' });
        const repaired = originals[repair.source].replace(repair.repair.find, repair.repair.replace); assert.notEqual(repaired, originals[repair.source]);
        await save(repaired);
        await open({ project, configuration: repair.id, build: 'build-html' }); await validate.click(); await settle(); assert.equal((await state()).status, 'valid');
        console.log(`[cem-studio:${id}] catalog, exact native commands and namespace repair passed`);

        await run(project, query.id);
        const edited = family === 'svg' ? originals[primary].replace('>Source<', '>Café source<').replace('width="80"', 'width="72"')
            : originals[primary].replace('>x<', '>y<').replace('x squared', 'y squared');
        assert.notEqual(edited, originals[primary]); await editor.fill(edited); await settle();
        await click('Run configuration'); assert.deepEqual(values(await state()), expected); assert.equal((await state()).dirty, true);
        await click('Save and validate'); await click('Run configuration'); assert.notDeepEqual(values(await state()), expected);
        const built = await build(project); assert.equal(built.buildStatus, 'succeeded', JSON.stringify(built));
        assert.notEqual(built.html, html); assertGeometry(await rendered(family), family, true);
        assert.ok(built.origins.some(uri => uri.endsWith(`/${resource(template).path}`)));
        assert.ok(built.origins.some(uri => uri.endsWith(`/${resource(primary).path}`)));
        const successfulRun = built.runId;
        await open({ project, file: primary, build: 'build-html' }); await save(originals[repair.source]);
        const refused = await build(project); assert.equal(refused.buildStatus, 'failed'); assert.equal(refused.runId, successfulRun); assert.equal(refused.html, built.html);
        await open({ project, file: primary, build: 'build-html' }); await save(edited);
        assert.equal((await build(project)).buildStatus, 'succeeded'); assertGeometry(await rendered(family), family, true);
        console.log(`[cem-studio:${id}] saved data, visible namespace geometry, provenance and retained failed-build output passed`);
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(`Portable ${definition.name}`);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const copy = (await state()).route.project; assert.ok(copy && copy !== project);
        await open({ project: copy, file: template, build: 'build-html' });
        const caption = family === 'svg' ? 'Rendered from retained SVG data.' : 'Rendered from retained Presentation MathML.';
        const editedTemplate = originals[template].replace(caption, 'Rendered from an independent saved copy.'); assert.notEqual(editedTemplate, originals[template]); await save(editedTemplate);
        const copied = await build(copy); assert.equal(copied.buildStatus, 'succeeded'); assert.ok(copied.html.includes('Rendered from an independent saved copy.'));
        assert.equal((await build(project)).html, built.html);
        await context.setOffline(true);
        try {
            await page.reload(); await ready();
            const offline = await build(copy); assert.equal(offline.buildStatus, 'succeeded'); assert.equal(offline.html, copied.html); assertGeometry(await rendered(family), family, true);
            assert.equal((await run(copy, query.id)).status, 'projected');
            await open({ project: copy, file: primary, build: 'build-html' }); assert.equal(await editor.evaluate(element => element.value), edited);
            await open({ project: copy, file: repair.source, build: 'build-html' }); assert.equal(await editor.evaluate(element => element.value), repaired);
            for (const name of [primary, repair.source, template]) {
                await open({ collection: 'content-types-documents-ui', sample: id, file: name, build: 'build-html' });
                assert.equal(await readonly.evaluate(element => element.value), originals[name]);
            }
        } finally { await context.setOffline(false); }
        console.log(`[cem-studio:${id}] independent copy and offline native commands, math/vector rendering and original sources passed`);
        evidence[id] = { catalog: true, readonlyOriginals: true, nativeCommands: true, exactFormatter: true, namespaceQuery: true,
            nativeRepair: true, staticEntry: true, foreignNamespace: true, visibleGeometry: true, nativeProvenance: true,
            unsavedSourceGuard: true, retainedFailure: true, independentCopies: true, offline: true };
    }
    return evidence;
}
