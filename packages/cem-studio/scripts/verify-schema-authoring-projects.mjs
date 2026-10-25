import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Native schema source contracts use ordinary projects and shared controls. */
export async function verifySchemaAuthoringProjects({ page, context, url }) {
    const result = {};
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot();
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty,
            text: current.projection?.output?.text, diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics };
    });
    const save = async value => { await editor.fill(value); await settle(); await click('Save and validate'); };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    for (const id of ['cem-schema', 'json-schema', 'relax-ng']) {
        const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
        const resource = name => definition.sources.find(item => item.id === name);
        const operation = name => definition.configurations.find(item => item.id === name);
        const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
        const originalRoute = file => ({ collection: 'content-types-schema-authoring', sample: id, ...(file ? { file } : {}) });
        const run = async (project, name) => {
            const configuration = operation(name);
            await open({ project, configuration: name }); assert.equal((await state()).editable, true);
            const buttonName = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
            if (!await page.getByRole('button', { name: buttonName, exact: true }).isVisible()) {
                const label = configuration.kind === 'validation' ? 'Editor details' : 'Commands and results';
                await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
            }
            await click(buttonName); return state();
        };
        const valid = async (project, name) => {
            const current = await run(project, name);
            assert.equal(current.status, operation(name).kind === 'validation' ? 'valid' : 'projected', JSON.stringify(current)); return current;
        };
        const edit = async (project, name, text) => { await open({ project, file: name }); await save(text); };
        const originals = {};
        const authoredIds = definition.sources.filter(item => !item.path.startsWith('expected/') && item.id !== 'readme').map(item => item.id);
        await page.goto(`${url}/?collection=content-types-schema-authoring`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, id);
        for (const name of authoredIds) {
            await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
            originals[name] = await readonly.evaluate(element => element.value);
        }
        await open(originalRoute());
        const project = await clone(`Navigator ${definition.name}`);
        const repaired = {};
        for (const configuration of definition.configurations) {
            const current = await run(project, configuration.id);
            if (!configuration.repair) {
                assert.equal(current.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(current));
                for (const text of configuration.expectedText ?? []) assert.ok(current.text.includes(text), `${id}/${configuration.id}: ${text}`);
                if (configuration.expectedOutput) assert.equal(current.text, await fixture(configuration.expectedOutput));
            } else {
                const expected = configuration.expectedValidation;
                const diagnostic = current.diagnostics?.find(item => item.code === expected.errorCode);
                assert.ok(diagnostic, JSON.stringify(current)); assert.equal(diagnostic.severity, 'error');
                assert.equal(current.diagnostics.filter(item => item.severity === 'error').length, expected.errorCount);
                assert.equal(diagnostic.uri, `studio://${project}/${resource(configuration.source).path}`);
                for (const [key, value] of Object.entries(expected.location ?? {})) assert.equal(diagnostic[key] ?? null, value);
                const source = originals[configuration.source];
                repaired[configuration.source] = source.replace(configuration.repair.find, configuration.repair.replace);
                assert.notEqual(repaired[configuration.source], source);
                await edit(project, configuration.source, repaired[configuration.source]); await valid(project, configuration.id);
            }
        }
        console.log(`[cem-studio:${id}] catalog, native source contracts and exact original diagnostics/repairs passed`);

        let copy;
        const saved = { ...repaired };
        if (id === 'cem-schema') {
            assert.equal(repaired['invalid-schema'], originals['note-schema']); assert.equal(repaired['invalid-instance'], originals['note-instance']);
            saved['note-schema'] = originals['note-schema'].replace('A note must declare its id', 'Saved schema requires a note id');
            await edit(project, 'note-schema', saved['note-schema']); await valid(project, 'validate-schema');
            await edit(project, 'invalid-instance', originals['invalid-instance']);
            const failed = await run(project, 'repair-instance');
            assert.ok(failed.diagnostics.some(item => item.code === 'example.note.id_required' && item.message.includes('Saved schema requires a note id')), JSON.stringify(failed));
            await edit(project, 'invalid-instance', repaired['invalid-instance']); await valid(project, 'repair-instance');
            saved['note-instance'] = originals['note-instance'].replace('Café notes', 'Saved notes');
            await edit(project, 'note-instance', saved['note-instance']); await valid(project, 'validate-instance');
            await open({ project }); copy = await clone(`Portable ${definition.name}`);
            const copiedSchema = saved['note-schema'].replace('Saved schema requires a note id', 'Copied schema requires a note id');
            await edit(copy, 'note-schema', copiedSchema); await valid(copy, 'validate-schema');
            await edit(copy, 'invalid-instance', originals['invalid-instance']);
            const copiedFailure = await run(copy, 'repair-instance');
            assert.ok(copiedFailure.diagnostics.some(item => item.message.includes('Copied schema requires a note id')), JSON.stringify(copiedFailure));
            await edit(copy, 'invalid-instance', repaired['invalid-instance']); await valid(copy, 'repair-instance');
            assert.ok((await valid(project, 'format-schema')).text.includes('Saved schema requires a note id'));
            saved['note-schema'] = copiedSchema;
        } else if (id === 'json-schema') {
            assert.equal(repaired['invalid-dialect'], originals['note-schema']);
            saved['note-schema'] = originals['note-schema'].replace('"title": "Note"', '"title": "Saved note"');
            assert.notEqual(saved['note-schema'], originals['note-schema']);
            await edit(project, 'note-schema', saved['note-schema']); await valid(project, 'validate-schema');
            assert.ok((await valid(project, 'format-schema')).text.includes('Saved note'));
            saved['note-data'] = originals['note-data'].replace('Café notes', 'Saved notes');
            await edit(project, 'note-data', saved['note-data']); await valid(project, 'validate-data-syntax');
            await open({ project }); copy = await clone(`Portable ${definition.name}`);
            const copiedSchema = saved['note-schema'].replace('Saved note', 'Copied note');
            await edit(copy, 'note-schema', copiedSchema); await valid(copy, 'validate-schema');
            assert.ok((await valid(copy, 'format-schema')).text.includes('Copied note'));
            assert.ok((await valid(project, 'format-schema')).text.includes('Saved note'));
            saved['note-schema'] = copiedSchema;
        } else {
            assert.equal(repaired['invalid-xml'], originals['xml-schema']); assert.equal(repaired['invalid-compact'], originals['compact-schema']);
            saved['xml-schema'] = originals['xml-schema'].replaceAll('name="note"', 'name="memo"');
            saved['compact-schema'] = originals['compact-schema'].replaceAll('note', 'memo');
            assert.notEqual(saved['xml-schema'], originals['xml-schema']); assert.notEqual(saved['compact-schema'], originals['compact-schema']);
            await edit(project, 'xml-schema', saved['xml-schema']); await valid(project, 'validate-xml');
            assert.equal((await valid(project, 'format-xml')).text, saved['xml-schema']);
            await edit(project, 'compact-schema', saved['compact-schema']); await valid(project, 'validate-compact');
            assert.equal((await valid(project, 'format-compact')).text, saved['compact-schema']);
            saved['note-data'] = originals['note-data'].replace('Café notes', 'Saved notes');
            await edit(project, 'note-data', saved['note-data']); await valid(project, 'validate-data-syntax');
            await open({ project }); copy = await clone(`Portable ${definition.name}`);
            const copiedSchema = saved['compact-schema'].replaceAll('memo', 'entry');
            await edit(copy, 'compact-schema', copiedSchema); await valid(copy, 'validate-compact');
            assert.equal((await valid(copy, 'format-compact')).text, copiedSchema);
            assert.equal((await valid(project, 'format-compact')).text, saved['compact-schema']);
            saved['compact-schema'] = copiedSchema;
        }
        console.log(`[cem-studio:${id}] saved schema/data edits and independent copies passed`);
        await context.setOffline(true);
        try {
            await page.reload(); await ready();
            for (const [name, expected] of Object.entries(saved)) {
                await open({ project: copy, file: name }); assert.equal(await editor.evaluate(element => element.value), expected);
            }
            for (const configuration of definition.configurations) await valid(copy, configuration.id);
            if (id === 'cem-schema') {
                await edit(copy, 'invalid-instance', originals['invalid-instance']);
                const failed = await run(copy, 'repair-instance');
                assert.ok(failed.diagnostics.some(item => item.code === 'example.note.id_required' && item.message.includes('Copied schema requires a note id')), JSON.stringify(failed));
                await edit(copy, 'invalid-instance', repaired['invalid-instance']); await valid(copy, 'repair-instance');
            }
            for (const name of authoredIds) {
                await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
                assert.equal(await readonly.evaluate(element => element.value), originals[name]);
            }
        } finally { await context.setOffline(false); }
        console.log(`[cem-studio:${id}] offline native commands, saved repairs and unchanged originals passed`);
        result[id] = { catalog: true, readonlyOriginals: true, nativeCommands: true, sourceFormatting: true, originalDiagnostics: true,
            exactRepair: true, savedSchema: true, savedData: true, independentCopies: true, offline: true,
            ...(id === 'cem-schema' ? { ownedPackage: true, namespaceClaim: true, instanceValidation: true, savedContractMessage: true }
                : id === 'json-schema' ? { supportedDialect: true, syntaxOnlyData: true, noInstanceValidationClaim: true }
                    : { typedDualSyntaxParse: true, lexicalLosslessFormatting: true, referenceRefusal: true, syntaxOnlyData: true, noInstanceValidationClaim: true }) };
    }
    return result;
}
