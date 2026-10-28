import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Native projections remain ordinary resources, explicit exports and shared views. */
export async function verifyNativeProjectionProject({ page, context, url }) {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    const id = 'native-projections';
    const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
    const resource = name => definition.sources.find(item => item.id === name);
    const operation = name => definition.configurations.find(item => item.id === name);
    const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url));
    const originals = Object.fromEntries(await Promise.all(definition.sources.map(async item => [item.id, await fixture(item.id)])));
    const text = name => originals[name].toString('utf8');
    const bounded = async (operation, label) => {
        let timer;
        try {
            await Promise.race([
                operation,
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`Studio ${label} exceeded 30 seconds: ${page.url()}`)), 30_000); }),
            ]);
        } finally { clearTimeout(timer); }
    };
    const settle = () => bounded(page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled()), 'settling');
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await bounded(page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route), `opening ${new URLSearchParams(route)}`); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot();
        return { route: current.navigation.selection, status: current.status, editable: current.editable, runnable: current.runnable,
            dirty: current.dirty, resourceId: current.resourceId, navigationError: current.navigationError,
            sampleTextAvailable: current.navigation.sampleTextAvailable, sourcePreview: current.navigation.sourcePreview,
            text: current.projection?.output?.text, bytes: current.projection?.output?.bytes && Array.from(current.projection.output.bytes),
            previewKind: current.projection?.preview?.kind, outputType: current.projection?.output?.contentType,
            provenance: current.projection?.provenance, selection: current.selection,
            artifacts: current.projection?.buildRun.artifacts.map(artifact => ({ uri: artifact.uri, handleId: artifact.handleId,
                sourceMapId: artifact.sourceMapId, byteLength: artifact.byteLength, sourceMaps: artifact.sourceMaps, provenance: artifact.provenance })),
            diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics };
    });
    const sampleRoute = file => ({ collection: definition.collections[0], sample: id, ...(file ? { file } : {}) });
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    // Exercise a real bulk paste. CDP multiline insertText expands this buffer
    // into thousands of editing fragments instead of the browser paste path.
    const edit = async value => {
        await page.evaluate(value => navigator.clipboard.writeText(value), value);
        await editor.focus(); await editor.press('ControlOrMeta+A'); await editor.press('ControlOrMeta+V');
        assert.equal(await editor.evaluate(element => element.value), value);
        await settle();
    };
    const save = async value => { await edit(value); await click('Save and validate'); };
    const run = async (project, name) => {
        const configuration = operation(name); assert.ok(configuration);
        await open({ project, configuration: name }); assert.equal((await state()).editable, true);
        const button = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name: button, exact: true }).isVisible()) {
            await page.locator('cem-expansion[label="Editor details"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        await click(button); return state();
    };
    const resultsTab = async tab => {
        const expansion = page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first();
        if (await expansion.getAttribute('aria-expanded') === 'false') { await expansion.click(); await settle(); }
        await page.getByRole('tab', { name: 'Results', exact: true }).click(); await settle();
        await page.getByRole('tab', { name: tab, exact: true }).click(); await settle();
    };
    const downloadSource = async expected => {
        assert.ok(await page.getByRole('button', { name: 'Download resource', exact: true }).isVisible());
        const downloaded = await page.evaluate(async () => {
            let output;
            await globalThis.__cemStudioApplication.workbench.downloadSource(async value => {
                output = { filename: value.filename, contentType: value.contentType, bytes: Array.from(value.bytes) };
            });
            return output;
        });
        assert.ok(downloaded.filename); assert.deepEqual(Buffer.from(downloaded.bytes), originals[expected]);
        assert.equal(downloaded.contentType, resource(expected).contentType);
    };
    const binaryExport = async current => {
        assert.equal(current.previewKind, 'download'); await resultsTab('Operation output');
        assert.equal(await page.getByRole('button', { name: 'Copy exact output', exact: true }).isDisabled(), true);
        const transferred = await page.evaluate(async () => {
            const workbench = globalThis.__cemStudioApplication.workbench;
            let copied = false, refusal;
            try { await workbench.copyProjection(async () => { copied = true; }); } catch (error) { refusal = error.message; }
            let download;
            await workbench.downloadProjection(async value => { download = { contentType: value.contentType, bytes: Array.from(value.bytes) }; });
            return { copied, refusal, download };
        }); await settle();
        assert.equal(transferred.copied, false); assert.match(transferred.refusal, /downloaded as exact bytes/);
        assert.deepEqual(transferred.download.bytes, current.bytes); assert.equal(transferred.download.contentType, current.outputType);
    };
    const catalogBytes = () => page.evaluate(async sample => {
        const entry = globalThis.__cemStudioApplication.exampleProjects.examples.find(item => item.id === sample);
        const bundle = await entry.load();
        return Object.fromEntries(Object.entries(bundle.contents).map(([key, value]) => [key, Array.from(value)]));
    }, id);
    const assertOriginals = async () => {
        const actual = await catalogBytes();
        for (const item of definition.sources) assert.deepEqual(Buffer.from(actual[item.id]), originals[item.id], `Catalog source ${item.id} changed`);
    };
    const assertSuccess = async (current, configuration) => {
        assert.equal(current.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(current));
        assert.ok(!current.diagnostics?.some(item => item.severity === 'error'), JSON.stringify(current.diagnostics));
        if (configuration.kind === 'inspection') { assert.ok(current.text.startsWith('@doc cem-ml 1')); assert.ok(!current.text.includes('\u001b[')); }
        for (const expected of configuration.expectedText ?? []) assert.ok(current.text.includes(expected), `${configuration.id}: ${expected}`);
        const expected = configuration.expectedOutput ?? configuration.expectedOutputBytes;
        if (expected) {
            if (configuration.expectedOutputMode === 'json-value') assert.deepEqual(JSON.parse(current.text), JSON.parse(text(expected)), `${configuration.id} exported JSON drifted`);
            else assert.deepEqual(Buffer.from(current.bytes), originals[expected], `${configuration.id} exported bytes drifted`);
        }
    };
    const unicodeEvent = (current, source, value) => {
        // This is an explicitly requested JSON export, never execution input.
        const events = JSON.parse(current.text), event = events.find(item => item.kind === 'value' && item.value === value);
        assert.ok(event); const bytes = Buffer.from(source), range = event.byteRange;
        assert.equal(range.len, Buffer.byteLength(value)); assert.equal(bytes.subarray(range.start, range.start + range.len).toString('utf8'), value);
        return range;
    };

    for (const collection of definition.collections) {
        console.log(`[cem-studio:native-projections] catalog folder: ${collection}`);
        await page.goto(`${url}/?collection=${collection}`); await ready();
        const record = await page.evaluate(sample => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === sample), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, id);
    }
    await assertOriginals();
    for (const source of ['cem-source', 'dom-json']) {
        console.log(`[cem-studio:native-projections] readonly original: ${source}`);
        await open(sampleRoute(source)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        assert.equal(await readonly.evaluate(element => element.value), text(source));
    }
    for (const source of ['dom-binary', 'ast-binary', 'events-binary']) {
        await open(sampleRoute(source)); const current = await state();
        assert.equal(current.sampleTextAvailable, false); assert.equal(current.sourcePreview, undefined);
        assert.equal(await page.locator('cem-source-editor').count(), 0); await downloadSource(source);
    }
    await open(sampleRoute()); const project = await clone(`Navigator ${definition.name}`), repaired = {}, baseline = {};
    for (const configuration of definition.configurations) {
        console.log(`[cem-studio:native-projections] running ${configuration.id}`);
        const current = await run(project, configuration.id);
        console.log(`[cem-studio:native-projections] ${configuration.id}: ${current.status}`);
        if (!configuration.repair) {
            await assertSuccess(current, configuration); if (current.bytes) baseline[configuration.id] = current.bytes;
            if (configuration.expectedOutputBytes) await binaryExport(current);
        } else {
            const expected = configuration.expectedValidation, diagnostic = current.diagnostics?.find(item => item.code === expected.errorCode);
            assert.ok(diagnostic, JSON.stringify(current)); assert.equal(diagnostic.severity, 'error');
            assert.equal(current.diagnostics.filter(item => item.severity === 'error').length, expected.errorCount);
            assert.equal(diagnostic.uri, `studio://${project}/${resource(configuration.source).path}`);
            for (const [key, value] of Object.entries(expected.location)) assert.equal(diagnostic[key] ?? null, value);
            if (configuration.id === 'repair-source') {
                console.log('[cem-studio:native-projections] selecting original native diagnostic');
                await resultsTab('Diagnostics');
                console.log('[cem-studio:native-projections] diagnostic panel ready');
                await page.getByRole('listbox', { name: 'Diagnostics', exact: true }).selectOption(String(current.diagnostics.indexOf(diagnostic)));
                console.log('[cem-studio:native-projections] native diagnostic chosen'); await settle();
                const followed = await state();
                assert.equal(followed.navigationError, undefined); assert.equal(followed.resourceId, configuration.source);
                assert.equal(followed.selection.kind, 'diagnostic');
                assert.equal(followed.selection.byteStart, diagnostic.range.start); assert.equal(followed.selection.byteLength, diagnostic.range.len);
                const selected = await editor.evaluate(element => ({ value: element.value, start: element.selectionStart, end: element.selectionEnd }));
                assert.equal(selected.value, text(configuration.source));
                assert.equal(selected.value.slice(selected.start, selected.end), new TextDecoder('utf-8', { fatal: true }).decode(originals[configuration.source].subarray(diagnostic.range.start, diagnostic.range.start + diagnostic.range.len)));
            }
            repaired[configuration.source] = text(configuration.source).replace(configuration.repair.find, configuration.repair.replace);
            assert.notEqual(repaired[configuration.source], text(configuration.source));
            await open({ project, file: configuration.source }); await save(repaired[configuration.source]);
            assert.equal((await run(project, configuration.id)).status, 'valid');
        }
    }
    assert.equal(repaired['invalid-source'], text('cem-source'));
    console.log('[cem-studio:native-projections] catalog, all native presentations/exports, original repairs and exact binary transfers passed');

    for (const source of ['dom-binary', 'ast-binary', 'events-binary']) {
        await open({ project, file: source }); const current = await state();
        assert.equal(current.status, 'read-only'); assert.equal(current.editable, false); assert.equal(current.runnable, false);
        assert.equal(await page.getByRole('textbox', { name: 'CEM source', exact: true }).count(), 0); await downloadSource(source);
    }
    const followOwnedPresentation = async (project, configuration, source, original) => {
        const inspected = await run(project, configuration), uri = `studio://${project}/${resource(source).path}`;
        const index = inspected.provenance.findIndex(frame => frame.uri === uri && frame.range.start > 0 && frame.range.len > 0);
        assert.ok(index >= 0, `${configuration}: original native range is missing`);
        const frame = inspected.provenance[index], artifact = inspected.artifacts[0];
        assert.ok(artifact.provenance.length > 0);
        assert.ok(artifact.sourceMaps.every(map => map.owner.kind === 'artifact' && map.owner.handleId === artifact.handleId));
        assert.ok(frame.outputRange.start + frame.outputRange.len <= artifact.byteLength);
        assert.ok(frame.range.start + frame.range.len <= Buffer.byteLength(original));
        await resultsTab('Provenance');
        await page.getByRole('listbox', { name: 'Source-map provenance', exact: true }).selectOption(String(index)); await settle();
        const followed = await state();
        assert.equal(followed.navigationError, undefined); assert.equal(followed.resourceId, source);
        assert.equal(followed.selection.kind, 'provenance');
        assert.equal(followed.selection.byteStart, frame.range.start); assert.equal(followed.selection.byteLength, frame.range.len);
        const selected = await editor.evaluate(() => getSelection().toString());
        assert.equal(selected, Buffer.from(original).subarray(frame.range.start, frame.range.start + frame.range.len).toString('utf8'));
        return inspected;
    };
    await followOwnedPresentation(project, 'inspect-xml', 'xml-source', text('xml-source'));
    await followOwnedPresentation(project, 'parse-ast', 'cem-source', text('cem-source'));
    for (const configuration of ['format-xml', 'render-html', 'render-xml']) await followOwnedPresentation(project, configuration, configuration === 'format-xml' ? 'xml-source' : 'cem-source', text(configuration === 'format-xml' ? 'xml-source' : 'cem-source'));

    for (const [configuration, resource] of [['parse-dom-input', 'dom-json'], ['inspect-ast-input', 'ast-export'],
        ['inspect-events-input', 'events-json'], ['import-dom-html', 'dom-json'], ['import-ast-xml', 'ast-export'], ['import-events-cem', 'events-json']]) {
        await followOwnedPresentation(project, configuration, resource, text(resource));
    }

    const originalEvents = await run(project, 'export-events-json'), oldRange = unicodeEvent(originalEvents, text('cem-source'), 'Café Noord');
    const editedText = 'Café Haven and Århus', edited = text('cem-source').replace('Café Noord', editedText);
    assert.notEqual(edited, text('cem-source')); await edit(edited); await click('Run configuration');
    assert.equal((await state()).dirty, true); assert.deepEqual((await state()).bytes, originalEvents.bytes);
    await click('Save and validate'); await click('Run configuration');
    const changedEvents = await state(), changedRange = unicodeEvent(changedEvents, edited, editedText);
    assert.equal(changedRange.start, oldRange.start); assert.notEqual(changedRange.len, oldRange.len);
    const changed = {};
    for (const family of ['dom', 'ast', 'events']) for (const format of ['json', 'binary']) {
        const name = `export-${family}-${format}`, result = await run(project, name);
        assert.equal(result.status, 'projected'); assert.notDeepEqual(result.bytes, baseline[name]); changed[name] = result.bytes;
        if (format === 'binary') await binaryExport(result);
        else assert.ok(result.text.includes(editedText));
    }
    const parsed = await run(project, 'parse-events'); assert.ok(parsed.text.includes(editedText));
    assert.ok(parsed.text.includes(`studio://${project}/data/dispatch.cem`));
    console.log('[cem-studio:native-projections] readonly binary sources, native diagnostic navigation and saved Unicode/range invalidation passed');

    await open({ project }); const copy = await clone(`Portable ${definition.name}`); assert.notEqual(copy, project);
    const copiedText = 'München depot', copiedSource = edited.replace(editedText, copiedText);
    await open({ project: copy, file: 'cem-source' }); await save(copiedSource);
    const copied = {};
    for (const family of ['dom', 'ast', 'events']) for (const format of ['json', 'binary']) {
        const name = `export-${family}-${format}`, result = await run(copy, name); assert.equal(result.status, 'projected');
        assert.notDeepEqual(result.bytes, changed[name]); copied[name] = result.bytes;
        if (format === 'json') assert.ok(result.text.includes(copiedText));
    }
    assert.deepEqual((await run(project, 'export-events-json')).bytes, changed['export-events-json']);
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        for (const [name, expected] of Object.entries(copied)) {
            const result = await run(copy, name); assert.equal(result.status, 'projected'); assert.deepEqual(result.bytes, expected);
            if (name.endsWith('-binary')) await binaryExport(result);
        }
        for (const name of ['parse-ast', 'parse-events', 'inspect-xml', 'inspect-text']) {
            const result = await run(copy, name); assert.equal(result.status, 'projected');
            assert.ok(result.text.startsWith('@doc cem-ml 1'));
        }
        await followOwnedPresentation(copy, 'parse-events', 'cem-source', copiedSource);
        await followOwnedPresentation(copy, 'inspect-xml', 'xml-source', text('xml-source'));
        for (const configuration of ['format-xml', 'render-html', 'render-xml']) await followOwnedPresentation(copy, configuration, configuration === 'format-xml' ? 'xml-source' : 'cem-source', configuration === 'format-xml' ? text('xml-source') : copiedSource);
        for (const [configuration, resource] of [['parse-dom-input', 'dom-json'], ['inspect-ast-input', 'ast-export'],
            ['inspect-events-input', 'events-json'], ['import-dom-html', 'dom-json'], ['import-ast-xml', 'ast-export'], ['import-events-cem', 'events-json']]) {
            await followOwnedPresentation(copy, configuration, resource, text(resource));
        }

        unicodeEvent(await run(copy, 'export-events-json'), copiedSource, copiedText);
        for (const configuration of definition.configurations.filter(item => item.repair)) assert.equal((await run(copy, configuration.id)).status, 'valid');
        for (const [name, expected] of Object.entries({ ...repaired, 'cem-source': copiedSource })) {
            await open({ project: copy, file: name }); assert.equal(await editor.evaluate(element => element.value), expected);
        }
        await open({ project: copy, file: 'events-binary' }); await downloadSource('events-binary');
        await assertOriginals();
        await open(sampleRoute('dom-binary')); assert.equal((await state()).sampleTextAvailable, false); await downloadSource('dom-binary');
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:native-projections] independent copies, all-family offline exports/transfers, retained repairs and unchanged originals passed');
    return { catalog: true, readonlyOriginals: true, nativeCemPresentation: true, retainedXmlInspection: true, plainTextInspection: true,
        explicitJsonExports: true, exactBinaryExports: true, binaryCopyRefusal: true, exactDownloads: true,
        nativeSourceRanges: true, formatterAndMarkupNavigation: true, offlineFormatterAndMarkupNavigation: true, diagnosticSourceNavigation: true, projectionShapeValidation: true, originalRepairs: true,
        binarySourceReadonly: true, binarySourceNotRunnable: true, savedUnicodeSource: true, changedProjectionRanges: true,
        unsavedSourceGuard: true, independentCopies: true, persistedRepairs: true, offlineCommands: true,
        offlineBinaryExports: true, offlineDownloads: true, unchangedCatalog: true };
}
