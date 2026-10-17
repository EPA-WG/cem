import { parseCemMlCommandText } from '@epa-wg/cem-ml-cli/browser-commands';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

import { describeCemStudioOperationFields, describeCemStudioOperationGroups, editCemStudioOperationField, editCemStudioOperationFields, createCemStudioBrowserValidator } from './feature-tour.js';
import { createCemStudioFileSystemProvider, serializeCemStudioProjectBundle } from './file-system-provider.js';
import { createCemStudioProjectRepository } from './repository.js';
import { createCemStudioProjectSession } from './project-session.js';
import { mountCemStudioApplicationShell } from './shell.js';
import {
    CEM_STUDIO_INSPECT_VIEWS,
    CEM_STUDIO_PARSE_PROJECTIONS,
    createCemStudioFeatureTourWorkbench,
    createCemStudioProjectWorkbench,
    createCemStudioSessionWorkbench,
    mountCemStudioFeatureTourWorkbench,
} from './workbench.js';

// Supply the same resource identity as the production page's import map.
beforeAll(() => {
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: {
        '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href,
    } });
    document.head.append(map);
});

const repositories = [];
const workbenches = [];
const views = [];
const shells = [];

afterEach(async () => {
    for (const view of views.splice(0)) view.dispose();
    for (const shell of shells.splice(0)) shell.dispose();
    for (const workbench of workbenches.splice(0)) workbench.dispose();
    const names = [...new Set(repositories.map(({ databaseName }) => databaseName))];
    for (const repository of repositories.splice(0)) repository.close();
    for (const name of names) await deleteDatabase(name);
    document.body.replaceChildren();
});

describe('CEM Studio Feature Tour workbench', () => {
    it('stages native graph attributes with stale guards and bounded reversible drafts', async () => {
        const { workbench, repository, validator } = await graphWorkbench();
        const original = workbench.snapshot().draft;
        const fields = (attribute, value) => ({ node: ['result'], attribute: [attribute], value: [value], mode: ['set'], version: [workbench.snapshot().graphConfiguration.version] });
        const first = fields('out', 'café output.svg');
        await workbench.updateGraphAttribute(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        const changed = workbench.snapshot().draft;
        expect(changed).not.toBe(original);
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1, graphConfiguration: { canUndo: true, canRedo: false } });
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.updateGraphAttribute(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        expect(workbench.snapshot().draft).toBe(changed);
        await workbench.updateGraphAttribute(fields('style-policy', 'invented'));
        expect(workbench.snapshot().draft).toBe(changed);
        await workbench.updateGraphAttribute({ ...fields('out', 'ignored'), extra: ['no'] });
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('Unexpected');
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(original);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(changed);
        await workbench.saveSource();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, projectRevision: 2, resourceRevision: 2 });
        workbench.undoGraphDraft(); expect(workbench.snapshot()).toMatchObject({ dirty: true, draft: original });
        workbench.redoGraphDraft(); expect(workbench.snapshot().dirty).toBe(false);
        for (let index = 0; index < 24; index++) await workbench.updateGraphAttribute(fields('out', `output-${index}.svg`));
        let undoCount = 0;
        while (workbench.snapshot().graphConfiguration.canUndo) { workbench.undoGraphDraft(); undoCount++; }
        expect(undoCount).toBe(20);
        await workbench.reload();
        expect(workbench.snapshot().draft).toBe(changed);
        expect(workbench.snapshot().graphConfiguration).toMatchObject({ canUndo: false, canRedo: false });
    });

    it('creates native named graph nodes with revision guards and reversible unsaved drafts', async () => {
        const { workbench, repository, validator } = await graphWorkbench();
        const original = workbench.snapshot().draft;
        const fields = () => ({ parent: ['data'], kind: ['export'], id: ['copy'], 'attribute.out': ['café copy.svg'], version: [workbench.snapshot().graphConfiguration.version] });
        const first = fields();
        await workbench.createGraphNode(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'copy')).toMatchObject({ kind: 'export' });
        const draft = workbench.snapshot().draft;
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1, resourceRevision: 1 });
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.createGraphNode(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        for (const invalid of [
            { ...fields(), id: ['duplicate', 'again'] }, { ...fields(), 'attribute.id': ['renamed'] },
            { ...fields(), parent: ['result'] }, { ...fields(), id: ['result'] }, { ...fields(), 'attribute.out': [''] },
        ]) {
            await workbench.createGraphNode(invalid);
            expect(workbench.snapshot().graphConfiguration.error).toBeDefined();
            expect(workbench.snapshot().draft).toBe(draft);
        }
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(original);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(draft);
        await workbench.saveSource(); await workbench.reload();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, draft, projectRevision: 2 });
    });

    it('composes schema-backed graph creation forms with native reset and pending disabling', async () => {
        const { workbench, validator } = await graphWorkbench();
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await view.whenSettled();
        const choose = async (name, value) => {
            const select = root.querySelector(`cem-select[name="${name}"]`);
            expect(select).not.toBeNull(); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); await view.whenSettled();
        };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-create-parent', 'data'); await choose('graph-create-kind', 'export');
        let form = root.querySelector('form[host-command="create-graph-node"]');
        expect(root.querySelectorAll('form[host-command="create-graph-node"]')).toHaveLength(1);
        const set = (name, value) => {
            const input = form.querySelector(`cem-text-field[name="${name}"] input`);
            input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); return input;
        };
        set('id', 'copy'); const output = set('attribute.out', 'copy.svg');
        expect(output.required).toBe(true);
        action('Reset new graph node').click(); await view.whenSettled();
        expect(output.value).toBe(''); expect(workbench.snapshot().dirty).toBe(false);
        set('id', 'copy'); set('attribute.out', 'copy.svg');
        action('Create graph node').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(workbench.snapshot().graphConfiguration.nodes.some(node => node.id === 'copy')).toBe(true);
        expect(workbench.snapshot().projectRevision).toBe(1);
        action('Undo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(false);
        action('Redo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(true);
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate();
        await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        form = root.querySelector('form[host-command="create-graph-node"]');
        expect(form.querySelector('fieldset').disabled).toBe(true);
        const saved = workbench.snapshot().draft;
        await workbench.createGraphNode({ parent: ['data'], kind: ['export'], id: ['blocked'], 'attribute.out': ['blocked.svg'], version: [workbench.snapshot().graphConfiguration.version] });
        expect(workbench.snapshot().draft).toBe(saved);
        pending.resolve(validResult()); await saving; await view.whenSettled();
        expect(root.querySelector('form[host-command="create-graph-node"] fieldset').disabled).toBe(false);
        await choose('graph-create-parent', '');
        expect(root.querySelector('form[host-command="create-graph-node"]')).toBeNull();
        await choose('graph-create-kind', 'import');
        expect(root.querySelector('form[host-command="create-graph-node"] cem-text-field[name="attribute.src"]')).not.toBeNull();
        await choose('graph-create-parent', 'data'); await choose('graph-create-kind', 'join');
        expect(root.querySelector('form[host-command="create-graph-node"] cem-select[name="attribute.mode"]')).not.toBeNull();
    });

    it('stages native graph edges with cycle, foreign, repeated and stale guards', async () => {
        const { workbench, repository, validator } = await graphWorkbench(edgeGraphSource);
        const original = workbench.snapshot().draft;
        const fields = (role, source, name) => ({ node: ['view'], role: [role], source: [source], ...(name === undefined ? {} : { name: [name] }), version: [workbench.snapshot().graphConfiguration.version] });
        const first = fields('input', 'other');
        await workbench.updateGraphEdge(first);
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        const input = workbench.snapshot().draft;
        expect(workbench.snapshot()).toMatchObject({ dirty: true, projectRevision: 1 });
        await workbench.updateGraphEdge(first);
        expect(workbench.snapshot().graphConfiguration.error.message).toContain('graph changed');
        for (const invalid of [fields('input', 'result'), { ...fields('input', 'other'), version: ['foreign'] }, { ...fields('input', 'other'), source: ['other', 'data'] }, { ...fields('input', 'other'), extra: ['no'] }]) {
            await workbench.updateGraphEdge(invalid); expect(workbench.snapshot().draft).toBe(input);
            expect(workbench.snapshot().graphConfiguration.error).toBeDefined();
        }
        await workbench.updateGraphEdge(fields('input', ''));
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'input').source).toBeNull();
        await workbench.updateGraphEdge(fields('with', 'other', 'palette'));
        expect(workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'with').bindings).toEqual([{ name: 'palette', source: 'other' }]);
        await workbench.updateGraphEdge(fields('parent', 'other'));
        const changed = workbench.snapshot().draft;
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).not.toBe(changed);
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(changed);
        expect(validator.runResourceCommand).not.toHaveBeenCalled();
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value.project.revision).toBe(1);
        await workbench.saveSource(); await workbench.reload();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, draft: changed, projectRevision: 2 });
        expect(changed).not.toBe(original);
    });

    it('composes graph edge forms declaratively with native reset, bindings and pending disabling', async () => {
        const { workbench, validator } = await graphWorkbench(edgeGraphSource);
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view); await view.whenSettled();
        const choose = async (name, value) => { const host = root.querySelector(`cem-select[name="${name}"]`); expect(host).not.toBeNull(); host.value = value; host.dispatchEvent(new Event('change', { bubbles: true })); await view.whenSettled(); };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-edge-node', 'view'); await choose('graph-edge-role', 'input');
        expect(root.querySelectorAll('form[host-command="edit-graph-edge"]')).toHaveLength(1);
        await choose('source', 'other'); action('Reset staged edge change').click(); await view.whenSettled();
        expect(root.querySelector('form[host-command="edit-graph-edge"] cem-select[name=source]').value).toBe('');
        expect(workbench.snapshot().dirty).toBe(false);
        await choose('source', 'other'); action('Apply graph edge').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        action('Undo graph edit').click(); await view.whenSettled(); expect(workbench.snapshot().dirty).toBe(false);
        action('Redo graph edit').click(); await view.whenSettled(); expect(workbench.snapshot().dirty).toBe(true);
        await choose('graph-edge-role', 'with');
        const binding = root.querySelector('form[host-command="edit-graph-edge"] cem-text-field[name=name] input');
        binding.value = 'palette'; binding.dispatchEvent(new Event('input', { bubbles: true }));
        await choose('source', 'other'); action('Apply graph edge').click(); await view.whenSettled();
        expect(workbench.snapshot().graphConfiguration.error).toBeUndefined();
        expect(root.querySelector('cem-fieldset[label="Graph edge"]').textContent).toContain('palette');
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate(); await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        expect(root.querySelector('cem-fieldset[label="Graph edge"] fieldset').disabled).toBe(true);
        const saved = workbench.snapshot().draft;
        await workbench.updateGraphEdge({ node: ['view'], role: ['input'], source: ['data'], version: [workbench.snapshot().graphConfiguration.version] });
        expect(workbench.snapshot().draft).toBe(saved);
        pending.resolve(validResult()); await saving; await view.whenSettled();
        expect(root.querySelector('cem-fieldset[label="Graph edge"] fieldset').disabled).toBe(false);
        await choose('graph-edge-role', 'parent');
        expect(root.querySelector('form[host-command="edit-graph-edge"] cem-select[name=source]').value).toBe('data');
    });

    it('bounds graph draft history by UTF-8 bytes during undo and redo', async () => {
        const { workbench } = await graphWorkbench();
        const small = `${workbench.snapshot().draft}// Retained small draft\n`;
        workbench.updateDraft(small);
        const large = '🙂'.repeat(2 * 1024 * 1024);
        workbench.updateDraft(large);
        workbench.undoGraphDraft();
        expect(workbench.snapshot().draft).toBe(small);
        expect(workbench.snapshot().graphConfiguration).toMatchObject({ canUndo: false, canRedo: true });
        workbench.redoGraphDraft(); expect(workbench.snapshot().draft).toBe(large);
        workbench.undoGraphDraft(); expect(workbench.snapshot().draft).toBe(small);
        workbench.updateDraft(`${large}🙂`);
        workbench.undoGraphDraft();
        expect(workbench.snapshot().draft).toBe(small);
        expect(workbench.snapshot().graphConfiguration).toMatchObject({ canUndo: false, canRedo: false });
    });

    it('composes one graph attribute form declaratively with native reset and pending disabling', async () => {
        const { workbench, validator } = await graphWorkbench();
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await view.whenSettled();
        const choose = async (name, value) => {
            const select = root.querySelector(`cem-select[name="${name}"]`);
            expect(select).not.toBeNull(); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true }));
            await view.whenSettled();
        };
        const action = name => [...root.querySelectorAll('button')].find(button => button.textContent.trim() === name);
        await choose('graph-node', 'result'); await choose('graph-attribute', 'out');
        const group = root.querySelector('cem-fieldset[label="Graph configuration"] fieldset');
        expect(group.querySelector('legend').textContent.trim()).toBe('Graph configuration');
        expect(root.querySelectorAll('form[host-command="edit-graph"]')).toHaveLength(1);
        const input = group.querySelector('cem-text-field[name=value] input');
        expect(input.value).toBe('result.svg');
        input.value = 'staged.svg'; input.dispatchEvent(new Event('input', { bubbles: true }));
        action('Reset staged graph change').click(); await view.whenSettled();
        expect(input.value).toBe('result.svg');
        input.value = 'changed.svg'; input.dispatchEvent(new Event('input', { bubbles: true }));
        action('Set graph attribute').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toContain('changed.svg');
        expect(workbench.snapshot().projectRevision).toBe(1);
        action('Undo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(false);
        action('Redo graph edit').click(); await view.whenSettled();
        expect(workbench.snapshot().dirty).toBe(true);
        const pending = deferred(); validator.validateResource.mockImplementationOnce(() => pending.promise);
        const saving = workbench.saveAndValidate();
        await vi.waitFor(() => expect(workbench.snapshot().status).toBe('validating')); await view.whenSettled();
        expect(group.disabled).toBe(true);
        const saved = workbench.snapshot().draft;
        await workbench.updateGraphAttribute({ node: ['result'], attribute: ['out'], value: ['pending.svg'], mode: ['set'], version: [workbench.snapshot().graphConfiguration.version] });
        expect(workbench.snapshot().draft).toBe(saved);
        pending.resolve(validResult()); await saving; await view.whenSettled();
        expect(root.querySelector('cem-fieldset[label="Graph configuration"] fieldset').disabled).toBe(false);
        await choose('graph-attribute', 'style-policy');
        expect(root.querySelectorAll('form[host-command="edit-graph"]')).toHaveLength(1);
        expect(root.querySelector('cem-fieldset[label="Graph configuration"] cem-select[name=value] [role=combobox]')).not.toBeNull();
    });

    it('uses native metadata to edit enum, boolean and literal fields without dropping other options', () => {
        const text = 'cem-ml parse studio://alpha/source.cem --namespace a=urn:a --namespace b=urn:b';
        const fields = describeCemStudioOperationFields(text);
        expect(fields.find(field => field.id === 'format')).toMatchObject({ type: 'enum', value: 'ast', defaultValue: 'ast' });
        expect(fields.find(field => field.id === 'preserve_source_offsets').type).toBe('boolean');
        let edited = editCemStudioOperationField(text, 'format', 'events');
        edited = editCemStudioOperationField(edited, 'preserve_source_offsets', 'true');
        const parsed = parseCemMlCommandText(edited);
        expect(parsed.options).toMatchObject({ format: 'events', preserve_source_offsets: true, namespaces: ['a=urn:a', 'b=urn:b'] });
        expect(parseCemMlCommandText(editCemStudioOperationField(edited, 'format', '', true)).options.format).toBe('ast');
        expect(() => editCemStudioOperationField(text, 'format', 'invented')).toThrow('Choose a value');
        expect(() => editCemStudioOperationField(text, 'input', 'file:///elsewhere')).toThrow('not available');
        expect(() => editCemStudioOperationField(text, 'preserve_source_offsets', 'yes')).toThrow('true or false');
        const expression = 'concat("café", "\\", "$(literal)")';
        const query = editCemStudioOperationField('cem-ml query studio://alpha/source.cem --query . --query-content-type application/vnd.cem.query-expression+xpath', 'query', expression);
        expect(parseCemMlCommandText(query).options.query).toBe(expression);
        for (const operation of ['parse', 'inspect', 'convert', 'query', 'transform', 'trace']) {
            const argv = operation === 'query' ? '--query . --query-content-type application/vnd.cem.query-expression+xpath' : operation === 'transform' ? '--template-expression .' : '';
            expect(describeCemStudioOperationFields(`cem-ml ${operation} studio://alpha/source.cem ${argv}`).length).toBeGreaterThan(0);
        }
    });

    it('validates grouped option batches together with the native grammar', () => {
        const source = 'cem-ml parse studio://alpha/source.cem --namespace a=urn:a --namespace b=urn:b --from-format cem';
        const group = describeCemStudioOperationGroups(source).find(group => group.id === 'operation-options');
        expect(group.fields.map(field => field.id)).toEqual(['format', 'fail_level', 'preserve_source_offsets']);
        expect(describeCemStudioOperationGroups(source).every(group => group.fields.length >= 2 && group.fields.length <= 3)).toBe(true);
        const changed = editCemStudioOperationFields(source, [
            { id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' },
        ]);
        expect(parseCemMlCommandText(changed).options).toMatchObject({
            format: 'events', preserve_source_offsets: true, from_format: 'cem', namespaces: ['a=urn:a', 'b=urn:b'],
        });
        expect(editCemStudioOperationFields(source, [])).toBe(source);
        for (const changes of [
            [{ id: 'format', value: 'events' }, { id: 'fail_level', value: 'invented' }],
            [{ id: 'format', value: 'ast' }, { id: 'format', value: 'events' }],
            [{ id: 'input', value: 'file:///elsewhere' }],
            [{ id: 'preserve_source_offsets', value: 'yes' }],
        ]) expect(() => editCemStudioOperationFields(source, changes)).toThrow();
        const query = 'cem-ml query studio://alpha/source.cem --query . --query-content-type application/vnd.cem.query-expression+xpath';
        expect(describeCemStudioOperationGroups(query).find(group => group.id === 'query_source').fields.map(field => field.id)).toEqual(['query', 'query_file']);
        expect(() => editCemStudioOperationField(query, 'query_file', 'studio://alpha/query.xpath')).toThrow();
        const replaced = editCemStudioOperationFields(query, [
            { id: 'query', value: '', reset: true }, { id: 'query_file', value: 'studio://alpha/query.xpath' },
        ]);
        expect(parseCemMlCommandText(replaced).options).toMatchObject({ query_file: 'studio://alpha/query.xpath', query_content_type: 'application/vnd.cem.query-expression+xpath' });
        expect(parseCemMlCommandText(replaced).options.query).toBeUndefined();
        expect(() => editCemStudioOperationFields(query, [{ id: 'query', value: '', reset: true }])).toThrow();
        const literal = 'concat("café", "\\", "$(literal)")';
        const updated = editCemStudioOperationFields(query, [{ id: 'query', value: literal }, { id: 'output', value: 'cem' }]);
        expect(parseCemMlCommandText(updated).options).toMatchObject({ query: literal, output: 'cem' });
    });

    it('commits grouped drafts once and preserves them after invalid, stale or failed batches', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const other = await createWorkbench(repository, validator);
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const initial = workbench.snapshot().command;
        const changes = [{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }];
        await workbench.updateCommandFields(changes, other.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
        expect(workbench.snapshot().command.formError.message).toContain('command changed');
        await workbench.updateCommandFields([...changes, { id: 'format', value: 'ast' }], workbench.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
        validator.previewResourceCommand.mockRejectedValueOnce(new Error('Grouped preview unavailable'));
        await workbench.updateCommandFields(changes, workbench.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
        expect(workbench.snapshot().command.status).toBe(initial.status);
        expect(workbench.snapshot().command.formError.message).toBe('Grouped preview unavailable');
        const observed = []; const unsubscribe = workbench.subscribe(state => observed.push(state.command.draftText));
        await workbench.updateCommandFields(changes, workbench.snapshot().command.formVersion);
        unsubscribe();
        const committed = workbench.snapshot().command;
        expect(parseCemMlCommandText(committed.draftText).options).toMatchObject({ format: 'events', preserve_source_offsets: true });
        expect(new Set(observed)).toEqual(new Set([initial.draftText, committed.draftText]));
        expect(committed.formError).toBeUndefined();
        await workbench.updateCommandFields([{ id: 'format', value: '', reset: true }], initial.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(committed.draftText);
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(validator.validateResource).not.toHaveBeenCalled();
    });

    it('rejects repeated pending batches and ignores a superseded grouped preview', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const initial = workbench.snapshot().command;
        const pending = deferred();
        validator.previewResourceCommand.mockImplementationOnce(async options => { await pending.promise; return commandPreviewOutcome(options); });
        const version = initial.formVersion;
        const change = [{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }];
        const previews = validator.previewResourceCommand.mock.calls.length;
        const update = workbench.updateCommandFields(change, version);
        try {
            await expect.poll(() => workbench.snapshot().command.status).toBe('checking');
            await expect.poll(() => validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
            expect(workbench.snapshot().command.draftText).toBe(initial.draftText);
            await workbench.updateCommandFields(change, version);
            expect(workbench.snapshot().command.formError.message).toContain('command changed');
            await workbench.updateCommandDraft(initial.draftText.replace('--format ast', '--format events'));
        } finally { pending.resolve(); }
        await update;
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.preserve_source_offsets).not.toBe(true);
        expect(workbench.snapshot().command.formError).toBeUndefined();
    });

    it('persists grouped options only through one explicit Apply and reopens stable resources', async () => {
        const native = await createCemStudioBrowserValidator();
        try {
            const validator = { ...validatorFor(validResult()), previewResourceCommand: native.previewResourceCommand,
                serializeResourceCommand: native.serializeResourceCommand, decodeResourceCommand: native.decodeResourceCommand };
            const repository = await repositoryWithSource('{article | Native grouped options}');
            const workbench = await createWorkbench(repository, validator);
            await workbench.parsePersisted('ast');
            const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
            await workbench.updateCommandFields([{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }], workbench.snapshot().command.formVersion);
            expect(workbench.snapshot().command.parsed.options).toMatchObject({ format: 'events', preserve_source_offsets: true });
            expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
            workbench.setCommandTarget({ mode: 'new', name: 'Grouped inspection', parentId: 'schema-package-examples' });
            await workbench.applyCommand();
            const applied = workbench.snapshot().command.application;
            expect(applied.status).toBe('applied');
            const after = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
            expect(after.project.revision).toBe(before.project.revision + 1);
            for (const resource of before.project.resources) {
                expect(after.project.resources.find(item => item.id === resource.id)).toEqual(resource);
                expect(after.contents[resource.id]).toEqual(before.contents[resource.id]);
            }
            const committed = native.decodeResourceCommand(after.contents[applied.result.commandResource.id]);
            expect(committed.options).toMatchObject({ format: 'events', preserve_source_offsets: true });
            const reopened = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'feature-tour', entryId: applied.result.entry.id } });
            workbenches.push(reopened);
            expect(reopened.snapshot().command.parsed.options).toMatchObject({ format: 'events', preserve_source_offsets: true });
            expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        } finally { await native.close(); }
    }, 30_000);

    it('rejects malformed grouped form capture without changing the command', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.parsePersisted('ast');
        const original = workbench.snapshot().command;
        const fields = { group: ['operation-options'], version: [original.formVersion],
            'mode.format': ['keep'], 'mode.fail_level': ['keep'], 'mode.preserve_source_offsets': ['keep'] };
        await workbench.updateCommandGroup('operation-options', fields, original.formVersion);
        expect(workbench.snapshot().command.formVersion).toBe(original.formVersion);
        for (const invalid of [
            { ...fields, 'mode.format': ['set', 'reset'] },
            { ...fields, 'mode.format': ['invented'] },
            { ...fields, 'mode.format': ['set'] },
            { ...fields, 'value.query': ['unexpected'] },
        ]) {
            await workbench.updateCommandGroup('operation-options', invalid, original.formVersion);
            expect(workbench.snapshot().command.draftText).toBe(original.draftText);
            expect(workbench.snapshot().command.formError).toBeDefined();
        }
        await workbench.updateCommandGroup('operation-options', fields, original.formVersion);
        expect(workbench.snapshot().command.formError).toBeUndefined();
        expect(workbench.snapshot().command.formVersion).toBe(original.formVersion);
    });

    it('guards typed field versions and edits drafts without Save, Apply or Run', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const other = await createWorkbench(repository, validator);
        await workbench.updateCommandField('format', 'events', other.snapshot().command.formVersion);
        expect(workbench.snapshot().command.formError.message).toContain('command changed');
        const version = workbench.snapshot().command.formVersion;
        await workbench.updateCommandField('format', 'events', version);
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('events');
        const edited = workbench.snapshot().command.draftText;
        await workbench.updateCommandField('format', 'ast', version);
        expect(workbench.snapshot().command.formError.message).toContain('command changed');
        expect(workbench.snapshot().command.draftText).toBe(edited);
        await workbench.updateCommandField('format', 'invalid', workbench.snapshot().command.formVersion);
        expect(workbench.snapshot().command.draftText).toBe(edited);
        await workbench.updateCommandField('format', '', workbench.snapshot().command.formVersion, true);
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('ast');
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(validator.validateResource).not.toHaveBeenCalled();
    });

    it('saves and reloads exact revisions, retains native validation data, and navigates with CEM controls', async () => {
        const invalid = '@doc cem-ml 1\n\n{article |\n    {h1 | Missing article close}\n';
        const rangeStart = new TextEncoder().encode(invalid.slice(0, invalid.indexOf('{article'))).byteLength;
        const validator = validatorFor(invalidResult(rangeStart, 10));
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root });
        shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        const editor = root.querySelector('cem-textarea[data-cem-studio-editor] textarea');
        expect(root.querySelector('cem-studio-workbench-controls')).not.toBeNull();
        expect(editor.value).toBe('original');
        editor.focus();
        editor.value = invalid;
        editor.setSelectionRange(2, 5);
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot()).toMatchObject({ status: 'dirty', dirty: true });
        await view.whenSettled();
        expect(root.querySelector('cem-textarea[data-cem-studio-editor] textarea')).toBe(editor);
        expect(document.activeElement).toBe(editor);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 5]);

        root.querySelector('cem-action[data-cem-studio-save] button').click();
        await view.whenSettled();
        const snapshot = workbench.snapshot();
        expect(snapshot).toMatchObject({
            status: 'invalid',
            projectRevision: 2,
            resourceRevision: 2,
            dirty: false,
        });
        expect(snapshot.validation).toMatchObject({
            hardViolationCount: 1,
            stale: false,
            reportSummary: { inputCount: 1, errorCount: 1, hardViolationCount: 1 },
            diagnostics: [{ code: 'cem.tokenizer.unterminated_node' }],
            provenance: [{ transform: 'CemTokenizer', range: { start: rangeStart, len: 10 } }],
        });
        expect(validator.validateResource).toHaveBeenCalledWith(
            expect.objectContaining({
                projectId: 'feature-tour',
                projectRevision: 2,
                resourceRevision: 2,
            }),
        );
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe(invalid);
        expect(exported.value.project.revision).toBe(2);
        expect(exported.value.project.resources.find(({ id }) => id === 'source').revision).toBe(2);

        expect(root.querySelector('cem-tabs [role="tablist"]')).not.toBeNull();
        expect(root.querySelector('cem-table [role="table"]')).not.toBeNull();
        expect(root.querySelector('cem-list')).not.toBeNull();
        const reportRows = [...root.querySelectorAll('[data-cem-studio-report] [role="row"]')]
            .map(row => [...row.querySelectorAll('[role="cell"]')].map(cell => cell.textContent.trim()));
        expect(reportRows).toEqual(expect.arrayContaining([['Inputs', '1'], ['Warnings', '0'], ['Hard violations', '1']]));
        expect(
            root.querySelectorAll('button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button)'),
        ).toHaveLength(0);

        const paneTabs = [...root.querySelector('cem-tabs[label="Workbench panes"]').querySelectorAll('[role="tab"]')]
            .filter(tab => tab.closest('cem-tabs').getAttribute('label') === 'Workbench panes');
        const diagnostics = root.querySelector('cem-list[data-diagnostic-list] select');
        for (let attempt = 0; attempt < 2; attempt++) {
            paneTabs.find(tab => tab.textContent === 'Results').click(); await view.whenSettled();
            expect(editor.closest('[role="tabpanel"]').hidden).toBe(true);
            diagnostics.value = '0'; diagnostics.dispatchEvent(new Event('change', { bubbles: true }));
            await view.whenSettled();
            expect(editor.closest('[role="tabpanel"]').hidden).toBe(false);
            expect(document.activeElement).toBe(editor);
            expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        }
        diagnostics.value = '0';
        diagnostics.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        expect(workbench.snapshot().selection).toMatchObject({ kind: 'diagnostic', byteStart: rangeStart });
        expect(document.activeElement).toBe(editor);
        editor.setSelectionRange(0, 0);
        const reload = root.querySelector('[data-cem-studio-reload] button');
        reload.focus();
        await workbench.copyCommand(async () => undefined);
        await view.whenSettled();
        expect(document.activeElement).toBe(reload);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([0, 0]);
        workbench.navigateDiagnostic(0);
        await view.whenSettled();
        expect(document.activeElement).toBe(editor);
        expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        const provenance = root.querySelector('cem-list[data-provenance-list] select');
        provenance.value = '0';
        provenance.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        expect(workbench.snapshot().selection).toMatchObject({ kind: 'provenance', byteStart: rangeStart });

        const parseSelect = root.querySelector('cem-select[data-cem-studio-parse-projection]');
        parseSelect.value = 'events';
        root.querySelector('cem-action[data-cem-studio-parse] button').click();
        await view.whenSettled();
        expect(workbench.snapshot()).toMatchObject({
            status: 'projected',
            projection: {
                kind: 'parse',
                mode: 'events',
                revision: { projectRevision: 2, resourceRevision: 2 },
                output: { contentType: 'application/cem', text: 'parse events projection\n' },
                nativeResult: { operation: 'parse' },
                stale: false,
            },
        });
        expect(root.querySelector('[data-cem-studio-preview] pre').textContent).toBe('parse events projection\n');

        const inspectSelect = root.querySelector('cem-select[data-cem-studio-inspect-view]');
        inspectSelect.value = 'tree';
        root.querySelector('cem-action[data-cem-studio-inspect] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().projection).toMatchObject({
            kind: 'inspect',
            mode: 'tree',
            output: { text: 'inspect tree projection\n' },
        });
        expect(root.querySelector('cem-table[label="Projection execution"] [role="table"]')).not.toBeNull();
    }, 30_000);

    it('groups the active option declaratively and disables it during an atomic draft update', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.parsePersisted('ast');
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        selectValue(root, '[name="operation-option"]', 'format'); await view.whenSettled();
        await expect.poll(() => root.querySelector('form[host-command="edit-field"] cem-fieldset fieldset')).toBeTruthy();
        const form = root.querySelector('form[host-command="edit-field"]');
        const host = form.querySelector('cem-fieldset'), group = host.querySelector('fieldset');
        expect(group.querySelector('legend').textContent).toBe('Edit format');
        expect(group.getAttribute('role')).toBeNull();
        expect(group.getAttribute('aria-describedby')).toBeTruthy();
        const choice = form.querySelector('cem-select[name="value"]');
        choice.value = 'events'; choice.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        const pending = deferred();
        validator.previewResourceCommand.mockImplementationOnce(async options => { await pending.promise; return commandPreviewOutcome(options); });
        const submit = form.querySelector('cem-action[value="set"] button');
        const previews = validator.previewResourceCommand.mock.calls.length;
        submit.click();
        try {
            await expect.poll(() => group.disabled).toBe(true);
            expect(choice.querySelector('button').matches(':disabled')).toBe(true);
            await expect.poll(() => validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
            submit.click();
            expect(validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
        } finally { pending.resolve(); }
        await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('events');
        expect(form.querySelector('fieldset')).toBe(group);
        expect(group.disabled).toBe(false);
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        form.querySelector('cem-action[value="reset"] button').click(); await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options.format).toBe('ast');
        expect(root.querySelectorAll('form[host-command="edit-field"]')).toHaveLength(1);
    }, 30_000);

    it('stages a bounded option group declaratively and submits one native form', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.parsePersisted('ast');
        const before = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        selectValue(root, '[name="operation-option"]', 'group:operation-options'); await view.whenSettled();
        const form = root.querySelector('form[host-command="edit-group"]');
        expect(form).not.toBeNull();
        expect(root.querySelectorAll('form[host-command="edit-field"]')).toHaveLength(0);
        expect(form.querySelector('legend').textContent).toBe('Edit Operation options together');
        expect(form.querySelectorAll('cem-select[name^="mode."]')).toHaveLength(3);
        const button = form.querySelector('cem-action[type="submit"] button');
        const original = workbench.snapshot().command;
        selectValue(form, '[name="value.format"]', 'events'); await view.whenSettled();
        button.click(); await view.whenSettled();
        expect(workbench.snapshot().command.draftText).toBe(original.draftText);
        expect(workbench.snapshot().command.formVersion).toBe(original.formVersion);
        form.querySelector('cem-action[type="reset"] button').click(); await view.whenSettled();
        expect(form.querySelector('cem-select[name="value.format"]').value).toBe('ast');
        selectValue(form, '[name="mode.format"]', 'set');
        selectValue(form, '[name="value.format"]', 'events');
        selectValue(form, '[name="mode.preserve_source_offsets"]', 'set');
        selectValue(form, '[name="value.preserve_source_offsets"]', 'true');
        await view.whenSettled();
        const pending = deferred(); const previews = validator.previewResourceCommand.mock.calls.length;
        validator.previewResourceCommand.mockImplementationOnce(async options => { await pending.promise; return commandPreviewOutcome(options); });
        button.click();
        try {
            await expect.poll(() => form.querySelector('fieldset').disabled).toBe(true);
            expect(workbench.snapshot().command.draftText).toBe(original.draftText);
            expect(form.querySelector('cem-select[name="value.format"]').value).toBe('events');
            button.click();
            await expect.poll(() => validator.previewResourceCommand.mock.calls.length).toBe(previews + 1);
        } finally { pending.resolve(); }
        await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options).toMatchObject({ format: 'events', preserve_source_offsets: true });
        expect(root.querySelector('form[host-command="edit-group"]')).toBe(form);
        expect(form.querySelector('fieldset').disabled).toBe(false);
        expect((await repository.query(request('export-project', { projectId: 'feature-tour' }))).value).toEqual(before);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        selectValue(form, '[name="mode.format"]', 'reset');
        selectValue(form, '[name="mode.preserve_source_offsets"]', 'keep');
        button.click(); await view.whenSettled();
        expect(parseCemMlCommandText(workbench.snapshot().command.draftText).options).toMatchObject({ format: 'ast', preserve_source_offsets: true });
    }, 30_000);

    it('replaces a coupled query source through the grouped declarative form', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        await workbench.updateCommandDraft('cem-ml query studio://feature-tour/data/cem-ml/basic.cem --query . --query-content-type application/vnd.cem.query-expression+xpath');
        const root = document.createElement('main'); document.body.append(root);
        const shell = await mountCemStudioApplicationShell({ root }); shells.push(shell);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        selectValue(root, '[name="operation-option"]', 'group:query_source'); await view.whenSettled();
        const form = root.querySelector('form[host-command="edit-group"]');
        expect(form.querySelector('legend').textContent).toBe('Edit Query source together');
        expect(form.querySelectorAll('cem-select[name^="mode."]')).toHaveLength(2);
        selectValue(form, '[name="mode.query"]', 'reset');
        selectValue(form, '[name="mode.query_file"]', 'set');
        const file = form.querySelector('cem-text-field[name="value.query_file"] input');
        file.value = 'studio://feature-tour/queries/source.xpath';
        file.dispatchEvent(new InputEvent('input', { bubbles: true }));
        form.querySelector('cem-action[type="submit"] button').click(); await view.whenSettled();
        const parsed = parseCemMlCommandText(workbench.snapshot().command.draftText);
        expect(parsed.options.query).toBeUndefined();
        expect(parsed.options.query_file).toBe('studio://feature-tour/queries/source.xpath');
        expect(parsed.options.query_content_type).toBe('application/vnd.cem.query-expression+xpath');
        expect(workbench.snapshot().command.formError).toBeUndefined();
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
    }, 30_000);

    it('retains declarative editor state through reload and rejects repeated pending commands', async () => {
        let finishValidation;
        const result = new Promise(resolve => { finishValidation = resolve; });
        const validator = validatorFor(validResult());
        validator.validateResource.mockImplementationOnce(() => result);
        const repository = await repositoryWithSource('durable');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const editor = root.querySelector('[data-cem-studio-editor] textarea');
        editor.value = 'saved draft'; editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        const save = root.querySelector('[data-cem-studio-save] button');
        save.click(); save.click();
        await vi.waitFor(() => expect(validator.validateResource).toHaveBeenCalledTimes(1));
        finishValidation(validResult());
        await view.whenSettled();
        expect(workbench.snapshot()).toMatchObject({ resourceRevision: 2, dirty: false, draft: 'saved draft' });
        expect(root.querySelector('[data-cem-studio-workbench-alert]').textContent).toContain('0 diagnostics; 0 hard violations.');
        expect(root.querySelector('[data-cem-studio-save]').hasAttribute('disabled')).toBe(true);
        editor.value = 'discard me'; editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        root.querySelector('[data-cem-studio-reload] button').click();
        await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-editor] textarea')).toBe(editor);
        expect(editor.value).toBe('saved draft');
        expect(root.querySelector('[data-cem-studio-workbench-status]').getAttribute('label')).toBe('Persisted revision loaded');
        expect(root.querySelector('[data-cem-studio-reload]').getAttribute('variant')).toBe('contextual');
        expect(root.querySelector('[data-cem-studio-parse]').getAttribute('variant')).toBe('alternate');
        view.dispose();
        editor.value = 'disposed'; editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot().draft).toBe('saved draft');
    });

    it('executes every CEM-ML parse projection and inspect view against the durable revision', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Durable projection}\n');
        const workbench = await createWorkbench(repository, validator);

        for (const projection of CEM_STUDIO_PARSE_PROJECTIONS) {
            const snapshot = await workbench.parsePersisted(projection);
            expect(snapshot.projection).toMatchObject({
                kind: 'parse',
                mode: projection,
                revision: { projectRevision: 1, resourceRevision: 1 },
                output: { text: `parse ${projection} projection\n` },
                stale: false,
            });
        }
        for (const view of CEM_STUDIO_INSPECT_VIEWS) {
            const snapshot = await workbench.inspectPersisted(view);
            expect(snapshot.projection).toMatchObject({
                kind: 'inspect',
                mode: view,
                revision: { projectRevision: 1, resourceRevision: 1 },
                output: { text: `inspect ${view} projection\n` },
                stale: false,
            });
        }

        expect(validator.parseResource).toHaveBeenCalledTimes(CEM_STUDIO_PARSE_PROJECTIONS.length);
        expect(validator.inspectResource).toHaveBeenCalledTimes(CEM_STUDIO_INSPECT_VIEWS.length);
        expect(validator.inspectResource).toHaveBeenLastCalledWith(
            expect.objectContaining({
                projectId: 'feature-tour',
                projectRevision: 1,
                resourceRevision: 1,
                view: 'tree',
            }),
        );
    });

    it('runs a portable operation and exposes expected, copy, and download evidence with CEM controls', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Portable conversion}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        let copied;
        let downloaded;
        const view = await mountCemStudioFeatureTourWorkbench({
            root,
            workbench,
            clipboard: {
                writeText: async (text) => {
                    copied = text;
                },
            },
            download: async (file) => {
                downloaded = file;
            },
        });
        views.push(view);

        selectValue(root, '[data-cem-studio-workbench-select]', 'conversion');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-operation-run] button').click();
        await view.whenSettled();

        const projection = workbench.snapshot().projection;
        expect(projection).toMatchObject({
            kind: 'convert',
            mode: 'conversion',
            summary: { kind: 'convert', outputCount: 1 },
            expected: { kind: 'convert', outputCount: 1 },
            expectedMatches: true,
            stale: false,
        });
        expect(validator.runResourceCommand).toHaveBeenCalledTimes(1);
        expect(validator.runResourceCommand.mock.calls[0][0].argv).toEqual(
            expect.arrayContaining([
                'convert',
                'studio://feature-tour/data/cem-ml/basic.cem',
                '--to-format',
                'dom-json',
            ]),
        );
        root.querySelector('cem-action[data-cem-studio-projection-copy] button').click();
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-projection-download] button').click();
        await view.whenSettled();
        expect(copied).toBe(projection.output.text);
        expect([...downloaded.bytes]).toEqual([...projection.output.bytes]);
        expect(root.querySelector('[data-cem-studio-projection-expected] cem-table [role="table"]')).not.toBeNull();
        expect(root.querySelector('[data-cem-studio-projection-expected]').textContent).toContain('Native result matches the pinned expectation.');
        expect(root.querySelector('[data-cem-studio-projection-metadata]').textContent).toContain('wasm-browser-worker');
        const metadataRows = [...root.querySelectorAll('[data-cem-studio-projection-metadata] [role="row"]')]
            .map(row => [...row.querySelectorAll('[role="cell"]')].map(cell => cell.textContent.trim()));
        expect(metadataRows).toEqual(expect.arrayContaining([['Diagnostics', '0'], ['Source-map frames', '0']]));
        expect(root.querySelector('[data-cem-studio-projection-trace]').textContent).toContain('No operation trace records');
        expect(root.querySelector('[data-cem-studio-projection-graph]').textContent).toContain('No transformation graph execution overlay records');
        expect(root.querySelector('[data-cem-studio-diagnostics]').textContent).toContain('No diagnostics');
        expect(root.querySelector('[data-cem-studio-provenance]').textContent).toContain('No source-map provenance');
        expect(
            root.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button)',
            ),
        ).toHaveLength(0);
    });

    it('retains declarative result owners and excludes duplicate exact output transfers', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Result ownership}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main'); document.body.append(root);
        const copied = Promise.withResolvers(), downloaded = Promise.withResolvers();
        const clipboard = { writeText: vi.fn(() => copied.promise) };
        const download = vi.fn(() => downloaded.promise);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard, download }); views.push(view);
        expect(root.querySelector('cem-studio-workbench-layout')).not.toBeNull();
        expect(root.querySelector('cem-studio-workbench-results')).not.toBeNull();
        expect(root.querySelector('[data-cem-studio-preview] cem-alert').getAttribute('tone')).toBe('info');
        await workbench.parsePersisted('events'); await view.whenSettled();
        root.querySelector('cem-tabs[label="Workbench panes"]').selectedIndex = 1; await view.whenSettled();
        const output = root.querySelector('[data-cem-studio-preview] cem-media-preview pre');
        expect(output.textContent).toBe(workbench.snapshot().projection.output.text);
        output.focus(); workbench.updateDraft('a newer draft'); await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-preview] pre')).toBe(output);
        expect(document.activeElement).toBe(output);
        const copy = root.querySelector('[data-cem-studio-projection-copy] button');
        const save = root.querySelector('[data-cem-studio-projection-download] button');
        copy.click(); copy.click(); save.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1);
        expect(clipboard.writeText).toHaveBeenCalledWith(workbench.snapshot().projection.output.text);
        expect(download).not.toHaveBeenCalled();
        copied.resolve(); await view.whenSettled();
        save.click(); save.click(); expect(download).toHaveBeenCalledTimes(1);
        expect([...download.mock.calls[0][0].bytes]).toEqual([...workbench.snapshot().projection.output.bytes]);
        downloaded.resolve(); await view.whenSettled();
        const outcome = projectionOutcome('parse', 'ast');
        const source = '<p>Isolated operation result</p>';
        const bytes = [...new TextEncoder().encode(source)];
        validator.parseResource.mockResolvedValueOnce({ ...outcome, output: { ...outcome.output,
            contentType: 'text/html', text: source, bytes, byteLength: bytes.length } });
        await workbench.parsePersisted('ast'); await view.whenSettled();
        const frame = root.querySelector('[data-cem-studio-preview] iframe');
        expect(frame.getAttribute('sandbox')).toBe(''); expect(frame.srcdoc).toContain(source);
        const changes = [];
        const observer = new MutationObserver(records => changes.push(...records));
        observer.observe(frame, { attributes: true, attributeFilter: ['srcdoc'] });
        workbench.updateDraft('another draft'); await view.whenSettled();
        changes.push(...observer.takeRecords()); observer.disconnect();
        expect(root.querySelector('[data-cem-studio-preview] iframe')).toBe(frame);
        expect(changes).toHaveLength(0);
        view.dispose(); copy.click(); save.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1); expect(download).toHaveBeenCalledTimes(1);
    });

    it('renders bounded native trace and graph values without parsing their text as markup', async () => {
        const validator = validatorFor(validResult());
        const outcome = projectionOutcome('transform', 'graph');
        outcome.result.result.value = { kind: 'transform', value: { value: {
            artifacts: [], events: [false, null, '🚀'.repeat(241)], nodes: ['<node>'],
        } } };
        validator.runResourceCommand.mockResolvedValueOnce(outcome);
        const repository = await repositoryWithSource('{main | Graph values}\n');
        const seed = featureTourSeed();
        seed.catalog.workbenches.push({ ...seed.catalog.workbenches[0], id: 'graph',
            operation: 'transform', kind: 'transformation-graph', name: 'Graph',
            commandArguments: ['transform', '$input', '--format', 'json'],
            expectedSummary: { kind: 'transform', mode: 'graph', artifactCount: 0, itemCount: 0 } });
        const workbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        await workbench.selectWorkbench('graph'); await workbench.runPersistedOperation(); await view.whenSettled();
        const trace = root.querySelector('[data-cem-studio-projection-trace]');
        const graph = root.querySelector('[data-cem-studio-projection-graph]');
        const values = [...trace.querySelectorAll('[role="cell"]')].map(cell => cell.textContent.trim());
        expect(values).toEqual(expect.arrayContaining(['false', 'null', `${'🚀'.repeat(239)}…`]));
        expect(graph.textContent).toContain('<node>'); expect(graph.querySelector('node')).toBeNull();
        expect(root.querySelector('[data-cem-studio-projection-expected]').textContent)
            .toContain('Native result matches the pinned expectation.');
        const traceOutcome = projectionOutcome('trace', 'trace');
        const events = Array.from({ length: 105 }, (_, index) => ({ kind: 'value', value: `event ${index}` }));
        traceOutcome.result.result.value = { kind: 'trace', value: { body: { events } } };
        validator.runResourceCommand.mockResolvedValueOnce(traceOutcome);
        seed.catalog.workbenches.push({ ...seed.catalog.workbenches[0], id: 'native-trace', operation: 'trace',
            kind: 'trace', name: 'Native trace', commandArguments: ['trace', '$input'],
            expectedSummary: { kind: 'trace', hasEvents: true } });
        const traceWorkbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' });
        workbenches.push(traceWorkbench); view.dispose();
        const traceView = await mountCemStudioFeatureTourWorkbench({ root, workbench: traceWorkbench }); views.push(traceView);
        await traceWorkbench.selectWorkbench('native-trace'); await traceWorkbench.runPersistedOperation(); await traceView.whenSettled();
        expect(traceWorkbench.snapshot().projection.trace).toHaveLength(100);
        const traceRows = root.querySelectorAll('[data-cem-studio-projection-trace] [role="row"]');
        expect(traceRows).toHaveLength(101);
        expect(traceRows[1].textContent).toContain('value.body.events.0');
        expect(traceRows[1].textContent).toContain(JSON.stringify(events[0]));
        expect(traceRows[100].textContent).toContain(JSON.stringify(events[99]));
    });

    it('round trips the Studio command, copies displayed text, and previews semantic changes without mutation', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Command source}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const clipboard = { writeText: vi.fn(async () => undefined) };
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard });
        views.push(view);

        const original = workbench.snapshot().command.current.text;
        expect(root.querySelector('[data-cem-studio-command-changes]').textContent)
            .toContain('The command matches the active normalized run plan');
        const commandEditor = root.querySelector('cem-textarea[data-cem-studio-command-editor] textarea');
        expect(commandEditor.value).toBe(original);
        expect(workbench.snapshot().command).toMatchObject({
            projection: 'studio',
            status: 'current',
            changes: [],
            revision: { projectRevision: 1, resourceRevision: 1 },
        });

        const changed = original.replace('--format ast', '--format events');
        validator.previewResourceCommand.mockImplementationOnce(async options => {
            const result = commandPreviewOutcome(options);
            return { ...result, semantic: { ...result.semantic, configuration: {
                label: '🚀'.repeat(241), enabled: false, nothing: null,
            } } };
        });
        commandEditor.value = changed;
        commandEditor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        expect(workbench.snapshot().command).toMatchObject({
            status: 'changed',
            draftText: changed,
            preview: { parsed: { commandPath: ['parse'], options: { format: 'events' } } },
        });
        expect(workbench.snapshot().command.changes).toEqual(
            expect.arrayContaining([expect.objectContaining({ category: 'operation', kind: 'changed' })]),
        );
        expect(root.querySelector('[data-cem-studio-command-alert]').textContent)
            .toContain(`${workbench.snapshot().command.changes.length} semantic changes`);
        expect(root.querySelector('cem-table[label="CLI Command semantic changes"] [role="table"]')).not.toBeNull();
        const cells = [...root.querySelectorAll('cem-table[label="CLI Command semantic changes"] [role="cell"]')]
            .map(cell => cell.textContent.trim());
        expect(cells).toEqual(expect.arrayContaining(['ast', 'events', '—', 'false', 'null', `${'🚀'.repeat(239)}…`]));

        root.querySelector('cem-action[data-cem-studio-command-copy] button').click();
        await view.whenSettled();
        expect(clipboard.writeText).toHaveBeenCalledWith(changed);
        expect(workbench.snapshot().command.copy).toMatchObject({ status: 'success' });

        commandEditor.value = `${changed} --unknown-studio-option`;
        commandEditor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        expect(workbench.snapshot().command).toMatchObject({
            status: 'invalid',
            diagnostic: { code: 'cem.command.unknown_option' },
        });
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe('{main | Command source}\n');
        expect(exported.value.project.revision).toBe(1);

        root.querySelector('cem-action[data-cem-studio-command-reset] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command).toMatchObject({ status: 'current', draftText: original, changes: [] });
        expect(
            root.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button)',
            ),
        ).toHaveLength(0);
    });

    it('retains command edits across pending validation and excludes duplicate clipboard commands', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Command editing}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main'); document.body.append(root);
        const copied = Promise.withResolvers();
        const clipboard = { writeText: vi.fn(() => copied.promise) };
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard }); views.push(view);
        expect(root.querySelector('cem-studio-workbench-command')).not.toBeNull();
        const editor = root.querySelector('[data-cem-studio-command-editor] textarea');
        const original = editor.value;
        const first = Promise.withResolvers();
        validator.previewResourceCommand.mockImplementationOnce(async options => {
            await first.promise; return commandPreviewOutcome(options);
        });
        editor.focus(); editor.value = original.replace('--format ast', '--format events');
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot().command).toMatchObject({ status: 'checking', draftText: editor.value });
        await expect.poll(() => validator.previewResourceCommand.mock.calls.some(([options]) => options.text === editor.value)).toBe(true);
        editor.value = original; editor.setSelectionRange(2, 7);
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        workbench.updateDraft('unrelated source edit');
        await expect.poll(() => workbench.snapshot().command.status).toBe('current');
        first.resolve(); await view.whenSettled();
        expect(workbench.snapshot().command.draftText).toBe(original);
        expect(root.querySelector('[data-cem-studio-command-editor] textarea')).toBe(editor);
        expect(editor.value).toBe(original);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 7]);
        expect(document.activeElement).toBe(editor);
        const copy = root.querySelector('[data-cem-studio-command-copy] button');
        copy.click(); copy.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1); // Same synchronous activation stack.
        expect(clipboard.writeText).toHaveBeenCalledWith(original);
        copied.resolve(); await view.whenSettled();
        view.dispose(); copy.click();
        expect(clipboard.writeText).toHaveBeenCalledTimes(1);
    });

    it('applies named new and compatible existing pages through CEM controls without executing', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Apply only}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'ready',
            target: { mode: 'new', parentId: 'schema-package-examples' },
            targets: { current: { id: 'validate-cem-ml', kind: 'validation', compatible: false } },
        });
        const name = root.querySelector('cem-text-field[data-cem-studio-command-target-name] input');
        name.focus();
        name.value = 'Applied inspection';
        name.setSelectionRange(2, 6);
        name.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-command-target-name] input')).toBe(name);
        expect(document.activeElement).toBe(name);
        expect([name.selectionStart, name.selectionEnd]).toEqual([2, 6]);
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();

        const application = workbench.snapshot().command.application;
        expect(application).toMatchObject({
            status: 'applied',
            target: { mode: 'current', entryId: 'applied-inspection' },
            result: {
                disposition: 'created',
                projectRevision: 2,
                resourceRevision: 1,
                entry: { id: 'applied-inspection', kind: 'inspection', name: 'Applied inspection' },
            },
        });
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        const commandResourceId = application.result.commandResource.id;
        expect(new TextDecoder().decode(exported.value.contents[commandResourceId])).toBe(
            new TextDecoder().decode(application.result.commandBytes),
        );
        expect(root.querySelector('cem-alert[data-cem-studio-command-apply-alert]').getAttribute('tone')).toBe(
            'success',
        );
        selectValue(root, '[data-cem-studio-command-target-mode]', 'existing');
        await view.whenSettled();
        expect(workbench.snapshot().command.application.target).toEqual({
            mode: 'existing',
            entryId: 'applied-inspection',
        });
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'applied',
            result: {
                disposition: 'updated',
                projectRevision: 3,
                resourceRevision: 2,
                entry: { id: 'applied-inspection' },
            },
        });
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(
            root.querySelectorAll(
                '[data-cem-studio-command-application] button:not(cem-action button):not(cem-icon-button button):not(cem-select button)',
            ),
        ).toHaveLength(0);
    });

    it('recommends a new page and confirmation-gates incompatible replacement in a CEM dialog', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Confirm replacement}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        selectValue(root, '[data-cem-studio-command-target-mode]', 'current');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'confirmation-required',
            confirmation: { target: { mode: 'current', entryId: 'validate-cem-ml' }, runAfterApply: false },
        });
        const dialog = root.querySelector('cem-dialog[data-cem-studio-command-replace-dialog] dialog');
        await expect.poll(() => dialog.open).toBe(true);
        expect(dialog.matches(':modal')).toBe(true);
        const entry = document.activeElement;
        workbench.updateDraft('unrelated edit while confirming');
        await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-command-replace-dialog] dialog')).toBe(dialog);
        expect(document.activeElement).toBe(entry);
        dialog.querySelector('cem-text').dispatchEvent(new CustomEvent('cem-close', { bubbles: true }));
        expect(workbench.snapshot().command.application.status).toBe('confirmation-required');
        await expect.element(dialog).toHaveAccessibleName('Confirm incompatible page replacement');
        expect(dialog.querySelector('[data-cem-studio-command-replace-confirm]').getAttribute('variant')).toBe('destructive');
        let exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.revision).toBe(1);

        const dismissed = new Promise((resolve) => {
            root.addEventListener('cem-close', resolve, { once: true });
        });
        dialog.close();
        await dismissed;
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'ready',
            target: { mode: 'current' },
            confirmation: undefined,
        });
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        const { userEvent } = await import('vitest/browser');
        await userEvent.keyboard('{Escape}');
        await expect.poll(() => workbench.snapshot().command.application.status).toBe('ready');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        dialogAction(root, 'Use new page').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({ status: 'ready', target: { mode: 'new' } });

        selectValue(root, '[data-cem-studio-command-target-mode]', 'current');
        await view.whenSettled();
        root.querySelector('cem-action[data-cem-studio-command-apply] button').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application.status).toBe('confirmation-required');
        dialogAction(root, 'Replace selected page').click();
        await view.whenSettled();
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'applied',
            result: { disposition: 'updated', entry: { id: 'validate-cem-ml', kind: 'inspection' } },
        });
        exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.revision).toBe(2);
        expect(exported.value.project.entries.find(({ id }) => id === 'validate-cem-ml')).toMatchObject({
            kind: 'inspection',
            runConfigResourceId: 'run-cem-ml',
        });
    });

    it('runs exactly the command bytes and revisions returned by Apply', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Apply and run}\n');
        const workbench = await createWorkbench(repository, validator);
        const root = document.createElement('main');
        document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench });
        views.push(view);

        const applyRun = root.querySelector('cem-action[data-cem-studio-command-apply-run] button');
        applyRun.click(); applyRun.click();
        await view.whenSettled();
        const snapshot = workbench.snapshot();
        expect(snapshot.command.application).toMatchObject({
            status: 'ran',
            result: { projectRevision: 2, resourceRevision: 1 },
            execution: { projectRevision: 2, resourceRevision: 1, stale: false },
        });
        expect(snapshot.projection).toMatchObject({
            kind: 'parse',
            mode: 'ast',
            revision: { projectRevision: 2, resourceRevision: 1 },
            stale: false,
        });
        expect(validator.executeAuthoredResourceCommand).toHaveBeenCalledTimes(1);
        const execution = validator.executeAuthoredResourceCommand.mock.calls[0][0];
        expect(execution.projectRevision).toBe(snapshot.command.application.result.projectRevision);
        expect([...execution.commandResource]).toEqual([
            ...new Uint8Array(snapshot.command.application.result.commandBytes),
        ]);
    });

    it('marks Apply & Run stale when the repository advances during exact-revision execution', async () => {
        const validator = validatorFor(validResult());
        let releaseExecution;
        let startedExecution;
        const started = new Promise((resolve) => {
            startedExecution = resolve;
        });
        validator.executeAuthoredResourceCommand.mockImplementation(
            (options) =>
                new Promise((resolve) => {
                    releaseExecution = () => resolve(authoredCommandOutcome(options));
                    startedExecution();
                }),
        );
        const repository = await repositoryWithSource('{main | Before run}\n');
        const workbench = await createWorkbench(repository, validator);
        const running = workbench.applyAndRun();
        await started;
        await repository.execute(
            request('save-resource', {
                projectId: 'feature-tour',
                resourceId: 'source',
                expectedProjectRevision: 2,
                expectedResourceRevision: 1,
                content: '{main | Concurrent revision}\n',
            }),
        );
        releaseExecution();
        await running;

        expect(workbench.snapshot()).toMatchObject({
            status: 'projection-stale',
            projection: { stale: true, revision: { projectRevision: 2, resourceRevision: 1 } },
            command: {
                application: {
                    status: 'run-stale',
                    result: { projectRevision: 2, resourceRevision: 1 },
                    execution: { stale: true },
                },
            },
        });
    });

    it('surfaces a stale project conflict without applying the command draft', async () => {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('{main | Loaded revision}\n');
        const workbench = await createWorkbench(repository, validator);
        await repository.execute(
            request('save-resource', {
                projectId: 'feature-tour',
                resourceId: 'source',
                expectedProjectRevision: 1,
                expectedResourceRevision: 1,
                content: '{main | Newer repository revision}\n',
            }),
        );

        await expect(workbench.applyCommand()).rejects.toMatchObject({
            code: 'cem.studio.repository.revision_conflict',
        });
        expect(workbench.snapshot().command.application).toMatchObject({
            status: 'conflict',
            error: { code: 'cem.studio.repository.revision_conflict' },
            result: undefined,
        });
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.entries).toHaveLength(2);
        expect(exported.value.project.revision).toBe(2);
    });

    it('marks an in-flight saved-revision result stale when the draft advances', async () => {
        let releaseValidation;
        let startedValidation;
        const started = new Promise((resolve) => {
            startedValidation = resolve;
        });
        const validator = validatorFor(
            () =>
                new Promise((resolve) => {
                    releaseValidation = () => resolve(validResult());
                    startedValidation();
                }),
        );
        const repository = await repositoryWithSource('original');
        const workbench = await createWorkbench(repository, validator);
        workbench.updateDraft('saved revision');
        const saving = workbench.saveAndValidate();
        await started;
        workbench.updateDraft('new unsaved draft');
        releaseValidation();
        await saving;

        expect(workbench.snapshot()).toMatchObject({
            status: 'stale',
            dirty: true,
            persistedText: 'saved revision',
            draft: 'new unsaved draft',
            validation: { stale: true, revision: { projectRevision: 2, resourceRevision: 2 } },
        });
    });

    it('marks an in-flight persisted projection stale when the draft advances', async () => {
        let releaseProjection;
        let startedProjection;
        const started = new Promise((resolve) => {
            startedProjection = resolve;
        });
        const validator = validatorFor(validResult());
        validator.parseResource.mockImplementation(
            () =>
                new Promise((resolve) => {
                    releaseProjection = () => resolve(projectionOutcome('parse', 'ast'));
                    startedProjection();
                }),
        );
        const repository = await repositoryWithSource('{main | Persisted}\n');
        const workbench = await createWorkbench(repository, validator);
        const projecting = workbench.parsePersisted('ast');
        await started;
        workbench.updateDraft('{main | New draft}\n');
        releaseProjection();
        await projecting;

        expect(workbench.snapshot()).toMatchObject({
            status: 'projection-stale',
            dirty: true,
            projection: {
                kind: 'parse',
                mode: 'ast',
                revision: { projectRevision: 1, resourceRevision: 1 },
                stale: true,
            },
        });
    });
});

describe('Project-owned workbench loading', () => {
    async function fixture(id = 'alpha', empty = false, extra = false) {
        const repository = await repositoryWithSource('project source');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        bundle.project.id = id;
        bundle.project.name = id;
        bundle.project.rootUri = `studio://${id}/`;
        if (empty) { bundle.project.entries = []; bundle.project.resources = []; bundle.contents = {}; }
        if (extra) {
            const bytes = new TextEncoder().encode('additional text');
            bundle.project.resources.push({ ...bundle.project.resources.find(item => item.id === 'source'), id: 'extra', role: 'data', path: 'extra.cem', referencePath: 'extra.cem', sha256: await digest(bytes) });
            bundle.contents.extra = bytes;
        }
        await repository.execute(request('import-project', { bundle }));
        return repository;
    }

    it('refreshes a renamed resource from the dataset while retaining selection and its resolver address', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const before = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        await workbench.editStructure({ kind: 'rename-resource', id: 'source', path: 'moved/renamed.cem' });
        expect(workbench.snapshot().structure.error).toBeUndefined();
        expect(workbench.snapshot()).toMatchObject({ resourceId: 'source', path: 'moved/renamed.cem' });
        expect(session.snapshot().selection).toEqual({ projectId: 'alpha', resourceId: 'source' });
        const after = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        expect(after.contents).toEqual(before.contents);
        expect(after.project.resources.find(item => item.id === 'source').referencePath)
            .toBe(before.project.resources.find(item => item.id === 'source').referencePath);
        expect(workbench.snapshot().explorer.nodes.flatMap(project => project.children ?? []).length).toBeGreaterThan(0);
        await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().path).toBe(before.project.resources.find(item => item.id === 'source').path);
    });

    it('edits structure with bounded undo/redo and guards drafts and divergent revisions', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        workbench.updateDraft('retained draft');
        await workbench.editStructure({ kind: 'rename-project', name: 'Must not commit' });
        expect(workbench.snapshot().structure.error.message).toContain('Save/Apply');
        expect((await repository.query(request('get-project', { projectId: 'alpha' }))).value.name).toBe('alpha');
        for (const kind of ['rename-resource', 'delete-resource']) {
            await workbench.editStructure({ kind, id: 'source', path: 'moved.cem' });
            expect(workbench.snapshot().structure.error.message).toContain('Save/Apply');
            expect(workbench.snapshot().draft).toBe('retained draft');
        }
        expect((await repository.query(request('get-project', { projectId: 'alpha' }))).value.revision).toBe(1);
        await workbench.reload();
        await workbench.editStructure({ kind: 'rename-project', name: 'Edited Alpha' });
        expect(workbench.snapshot().structure).toMatchObject({ name: 'Edited Alpha', canUndo: true });
        expect(workbench.snapshot().projectRevision).toBe(2);
        await workbench.editStructure({ kind: 'create-resource', path: 'data/new.cem', content: '{p | New source}' });
        expect(workbench.snapshot().structure.resources.some(resource => resource.path === 'data/new.cem')).toBe(true);
        await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().structure.resources.some(resource => resource.path === 'data/new.cem')).toBe(false);
        await workbench.editStructure({ kind: 'redo' });
        expect(workbench.snapshot().structure.resources.some(resource => resource.path === 'data/new.cem')).toBe(true);
        const selection = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'feature-tour' && item.selection.resourceId === 'source');
        await workbench.selectWorkbench(selection.id);
        expect(workbench.snapshot().structure.canUndo).toBe(false);
        await workbench.selectWorkbench(JSON.stringify({ projectId: 'alpha', resourceId: 'source' }));
        expect(workbench.snapshot().structure.canUndo).toBe(true);
        workbench.updateDraft('new durable content'); await workbench.saveSource();
        expect(workbench.snapshot().structure.canUndo).toBe(false);
        await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().structure.error.message).toContain('History is unavailable');
        expect(workbench.snapshot().persistedText).toBe('new durable content');
    });

    it('retains edits made during a structural commit and bounds undo history to twenty revisions', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const execute = repository.execute.bind(repository);
        const spy = vi.spyOn(repository, 'execute').mockImplementationOnce(async request => {
            const result = await execute(request);
            workbench.updateDraft('typed during commit');
            return result;
        });
        await workbench.editStructure({ kind: 'rename-project', name: 'Committed name' });
        expect(session.snapshot().transition.stage).toBe('source');
        expect(workbench.snapshot().draft).toBe('typed during commit');
        await workbench.resolveNavigation('cancel');
        expect(workbench.snapshot().draft).toBe('typed during commit');
        spy.mockRestore();
        await workbench.reload();
        for (let index = 0; index < 21; index++) await workbench.editStructure({ kind: 'rename-project', name: `Name ${index}` });
        for (let index = 0; index < 20; index++) await workbench.editStructure({ kind: 'undo' });
        expect(workbench.snapshot().structure).toMatchObject({ name: 'Name 0', canUndo: false, canRedo: true });
    });

    it('recovers admitted identities and exact bytes, retaining drafts through external trash and restore', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const records = new Map();
        const storage = { getItem: key => records.get(key) ?? null, setItem: (key, value) => { records.set(key, value); } };
        const openWorkbench = selection => createCemStudioProjectWorkbench({ repository, validator, selection });
        const session = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(session);
        await session.recover({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const exact = 'saved bytes\r\n{div | café}\n';
        workbench.updateDraft(exact); await workbench.saveSource();
        workbench.updateDraft('uncommitted');
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        session.cancelTransition(session.snapshot().transition.id);
        expect(JSON.parse(records.get('cem-studio-selection-v1'))).toEqual({ projectId: 'alpha', resourceId: 'source' });
        const restored = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(restored);
        await restored.recover();
        expect(restored.activeWorkbench().snapshot().persistedText).toBe(exact);
        expect(restored.activeWorkbench().snapshot()).toMatchObject({ dirty: true, draft: 'uncommitted' });
        await workbench.searchProjects('alpha');
        expect(workbench.snapshot().explorer.search.results.length).toBeGreaterThan(0);
        const remote = createCemStudioProjectRepository({ databaseName: repository.databaseName, validateProject: async bundle => bundle });
        repositories.push(remote); await remote.open();
        const revision = workbench.snapshot().projectRevision;
        await remote.execute(request('trash-project', { projectId: 'alpha', expectedRevision: revision }));
        await vi.waitFor(() => expect(workbench.snapshot().navigationError?.code).toBe('cem.studio.selection.unavailable'));
        await vi.waitFor(() => expect(workbench.snapshot().explorer.search.status).toBe('ready'));
        expect(workbench.snapshot().draft).toBe('uncommitted');
        expect(workbench.snapshot().navigationError.code).toBe('cem.studio.selection.unavailable');
        expect(workbench.snapshot().explorer.search.results).toHaveLength(0);
        await expect(workbench.saveSource()).rejects.toMatchObject({ code: 'cem.studio.repository.project_trashed' });
        const fallback = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(fallback);
        await fallback.recover({ projectId: 'feature-tour', resourceId: 'source' });
        expect(fallback.snapshot().selection.projectId).toBe('feature-tour');
        expect(fallback.snapshot().recoveryError.message).toContain('Trash');
        await workbench.setProjectTrashed('alpha', false);
        expect(workbench.snapshot().draft).toBe('uncommitted');
        expect(workbench.snapshot().explorer.search.results.length).toBeGreaterThan(0);
        await expect(workbench.saveSource()).rejects.toMatchObject({ code: 'cem.studio.repository.revision_conflict' });
        expect(workbench.snapshot().draft).toBe('uncommitted');
        const exported = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        expect(new TextDecoder().decode(exported.contents.source)).toBe(exact);
        records.set('cem-studio-selection-v1', JSON.stringify({ projectId: 'missing', resourceId: 'source' }));
        const missing = createCemStudioProjectSession({ openWorkbench, storage }); workbenches.push(missing);
        await missing.recover({ projectId: 'alpha', resourceId: 'source' });
        expect(missing.snapshot().selection.projectId).toBe('alpha');
    });

    it('composes Home creation and archive import with guarded project admission', async () => {
        const repository = await fixture();
        const starter = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        starter.project.entries = [];
        const archive = structuredClone(starter); archive.project.id = 'imported'; archive.project.rootUri = 'studio://imported/';
        const selectImport = vi.fn(async () => serializeCemStudioProjectBundle(archive));
        const provider = createCemStudioFileSystemProvider({ repository });
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'alpha', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session, home: { starter, provider, selectImport } });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const name = root.querySelector('cem-text-field[name="name"] input');
        name.value = 'My project'; name.dispatchEvent(new InputEvent('input', { bubbles: true }));
        root.querySelector('form[host-command="home-create"]').requestSubmit(); await view.whenSettled();
        expect(workbench.snapshot().home.error).toBeUndefined();
        const created = workbench.snapshot().home.projects.find(item => item.name === 'My project');
        expect(created).toBeDefined();
        expect(JSON.parse(created.id).projectId).not.toBe('alpha');
        selectValue(root.querySelector('form[host-command="home-trash"]'), '[name="project"]', created.id);
        root.querySelector('form[host-command="home-trash"]').requestSubmit(); await view.whenSettled();
        expect(workbench.snapshot().home.trash.some(item => item.id === JSON.parse(created.id).projectId)).toBe(true);
        root.querySelector('form[host-command="home-restore"]').requestSubmit(); await view.whenSettled();
        expect(workbench.snapshot().home.trash).toHaveLength(0);
        expect(workbench.snapshot().home.projects.some(item => item.id === created.id)).toBe(true);
        workbench.updateDraft('keep this draft');
        selectValue(root, '[name="project"]', created.id);
        root.querySelector('form[host-command="home-open"]').requestSubmit(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toBe('keep this draft');
        await workbench.importProject(); await view.whenSettled();
        expect(workbench.snapshot().home.projects.some(item => JSON.parse(item.id).projectId === 'imported')).toBe(true);
        const before = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        await workbench.importProject();
        expect(workbench.snapshot().home.error).toBeDefined();
        await workbench.createProject('  ');
        expect(workbench.snapshot().home.error.message).toContain('project name');
        const execute = vi.spyOn(repository, 'execute').mockRejectedValueOnce(new DOMException('Storage quota exceeded', 'QuotaExceededError'));
        await workbench.createProject('Quota failure'); execute.mockRestore();
        expect(workbench.snapshot().home.error.message).toContain('quota');
        expect((await repository.query(request('export-project', { projectId: 'alpha' }))).value).toEqual(before);
        selectImport.mockResolvedValueOnce(new TextEncoder().encode('invalid archive'));
        await workbench.importProject(); expect(workbench.snapshot().home.error).toBeDefined();
        selectImport.mockRejectedValueOnce(new DOMException('Cancelled', 'AbortError'));
        await workbench.importProject(); expect(workbench.snapshot().home.error).toBeUndefined();
        expect(workbench.snapshot().draft).toBe('keep this draft');
        const imported = workbench.snapshot().home.projects.find(item => JSON.parse(item.id).projectId === 'imported');
        await workbench.selectWorkbench(imported.id);
        await workbench.resolveNavigation('discard');
        expect(session.snapshot().selection.projectId).toBe('imported');
    });

    it('presents an empty Home and unavailable storage without requiring an active project', async () => {
        const repository = createCemStudioProjectRepository({ databaseName: `cem-home-${crypto.randomUUID()}`, validateProject: async bundle => bundle });
        repositories.push(repository);
        const session = createCemStudioProjectSession({ openWorkbench: async () => { throw new Error('unused'); } });
        workbenches.push(session);
        const workbench = await createCemStudioSessionWorkbench({ repository, session, home: {} });
        workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        expect(root.textContent).toContain('No projects yet');
        const query = vi.spyOn(repository, 'query').mockRejectedValueOnce(new Error('Storage unavailable'));
        await expect(workbench.refreshNavigation()).rejects.toThrow('Storage unavailable'); query.mockRestore();
        await view.whenSettled(); expect(root.textContent).toContain('Storage unavailable');
    });

    it('retains editor panes and admits recursive explorer activation independently of focus and expansion', async () => {
        const repository = await fixture('project with spaces', false, true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'project with spaces', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const tree = root.querySelector('cem-tree');
        const node = key => [...tree.querySelectorAll('[role="treeitem"]')].find(item => item.dataset.treeValue === key);
        const key = selection => encodeURIComponent(JSON.stringify(selection));
        const current = key({ projectId: 'project with spaces', resourceId: 'source' });
        const target = key({ projectId: 'project with spaces', resourceId: 'extra' });
        const project = node(`project:${key({ projectId: 'project with spaces' })}`);
        project.focus(); project.click(); await view.whenSettled();
        expect(project.getAttribute('aria-expanded')).toBe('true');
        expect(session.snapshot().transition).toBeUndefined();
        const { userEvent } = await import('vitest/browser');
        await userEvent.keyboard('{ArrowRight}');
        expect(document.activeElement).not.toBe(project);
        expect(tree.getAttribute('selected-values')).toBe(current);
        const folder = node(`branch:${key({ projectId: 'project with spaces', entryId: 'schema-package-examples' })}`);
        folder.click(); await view.whenSettled();
        const page = node(key({ projectId: 'project with spaces', entryId: 'validate-cem-ml' }));
        expect(page.getAttribute('aria-level')).toBe('3');
        node(`resources:${key({ projectId: 'project with spaces' })}`).click(); await view.whenSettled();
        workbench.updateDraft('draft with retained selection'); await view.whenSettled();
        const editor = root.querySelector('[data-cem-studio-editor] textarea');
        editor.focus(); editor.setSelectionRange(2, 7);
        const tabs = root.querySelector('cem-tabs[label="Workbench panes"]');
        const buttons = [...tabs.querySelectorAll(':scope > [role="tablist"] [role="tab"], :scope > div > [role="tablist"] [role="tab"]')];
        const paneButtons = buttons.length ? buttons : [...tabs.querySelectorAll('[role="tab"]')].filter(item => item.closest('cem-tabs') === tabs);
        paneButtons.find(item => item.textContent === 'Results').click(); await view.whenSettled();
        expect(editor.closest('[role="tabpanel"]').hidden).toBe(true);
        paneButtons.find(item => item.textContent === 'Source and command').click(); await view.whenSettled();
        expect(root.querySelector('[data-cem-studio-editor] textarea')).toBe(editor);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 7]);
        const launcher = node(target); launcher.focus(); launcher.click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        expect(tree.getAttribute('selected-values')).toBe(current);
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(workbench.snapshot().draft).toBe('draft with retained selection');
        expect(node(target)).toBe(launcher);
        expect(document.activeElement).toBe(launcher);
        launcher.click(); await view.whenSettled();
        dialogAction(root, 'Discard draft').click(); await view.whenSettled();
        expect(session.snapshot().selection.resourceId).toBe('extra');
        expect(tree.getAttribute('selected-values')).toBe(target);
        expect([...tree.querySelectorAll('[role="treeitem"]')].every(item => !/\s/.test(item.dataset.treeValue))).toBe(true);
    }, 30000);

    it('searches repository records deterministically, ignores stale completions and guards result activation', async () => {
        const repository = await fixture('beta', false, true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
        await session.select({ projectId: 'beta', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const query = root.querySelector('cem-text-field[name="query"] input');
        query.value = 'additional'; query.dispatchEvent(new InputEvent('input', { bubbles: true }));
        root.querySelector('form[host-command="search-projects"]').requestSubmit(); await view.whenSettled();
        const found = workbench.snapshot().explorer.search.results;
        expect(found).toHaveLength(1); expect(found[0].entityId).toBe('extra');
        await workbench.searchProjects('additional'); expect(workbench.snapshot().explorer.search.results).toEqual(found);
        workbench.updateDraft('search keeps draft'); await view.whenSettled();
        selectValue(root, '[name="result"]', found[0].key);
        root.querySelector('form[host-command="open-search"]').requestSubmit(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('source');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        let resolveOld;
        const original = repository.query.bind(repository);
        const spy = vi.spyOn(repository, 'query').mockImplementationOnce(() => new Promise(resolve => { resolveOld = resolve; }));
        const old = workbench.searchProjects('old');
        await workbench.searchProjects('additional');
        resolveOld({ value: [] }); await old;
        expect(workbench.snapshot().explorer.search.results).toEqual(found);
        spy.mockRestore();
        await workbench.searchProjects('zzzznotfound'); await view.whenSettled();
        expect(root.textContent).toContain('No matching projects');
        const failure = vi.spyOn(repository, 'query').mockRejectedValueOnce(new Error('Search unavailable'));
        await workbench.searchProjects('additional'); failure.mockRestore(); await view.whenSettled();
        expect(root.textContent).toContain('Search unavailable');
        expect(workbench.snapshot().draft).toBe('search keeps draft');
        expect((await original(request('get-project', { projectId: 'beta' }))).value.id).toBe('beta');
    }, 30000);

    it('refreshes explorer and search after a native command creates a durable page', async () => {
        const repository = await fixture();
        const validator = await createCemStudioBrowserValidator();
        try {
            const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) }); workbenches.push(session);
            await session.select({ projectId: 'alpha', resourceId: 'source' });
            const workbench = await createCemStudioSessionWorkbench({ repository, session }); workbenches.push(workbench);
            await workbench.searchProjects('New inspection');
            await workbench.parsePersisted('events');
            workbench.setCommandTarget({ mode: 'new', name: 'New inspection', parentId: 'schema-package-examples' });
            await workbench.applyCommand();
            const entryId = workbench.snapshot().command.application.result.entry.id;
            const key = encodeURIComponent(JSON.stringify({ projectId: 'alpha', entryId }));
            const flatten = nodes => nodes.flatMap(node => [node, ...flatten(node.children)]);
            expect(flatten(workbench.snapshot().explorer.nodes).some(node => node.key === key)).toBe(true);
            expect(workbench.snapshot().explorer.search.results.some(result => result.key === key)).toBe(true);
            await workbench.openEntity(key);
            expect(workbench.snapshot()).toMatchObject({ entryId, resourceId: 'source', runnable: true });
        } finally { await validator.close(); }
    }, 30000);

    it('loads nested durable identities without a catalog and never infers a missing command', async () => {
        const repository = await fixture();
        const validator = validatorFor(validResult());
        const workbench = await createCemStudioProjectWorkbench({ repository, validator,
            selection: { projectId: 'alpha', entryId: 'validate-cem-ml', resourceId: 'source' } });
        workbenches.push(workbench);
        expect(workbench.snapshot()).toMatchObject({ projectId: 'alpha', entryId: 'validate-cem-ml',
            resourceId: 'source', draft: 'project source', editable: true });
        expect(workbench.snapshot().command).toBeUndefined();
        expect(validator.previewResourceCommand).not.toHaveBeenCalled();
        await expect(workbench.runPersistedOperation()).rejects.toThrow(/command/i);
        workbench.updateDraft('saved independently');
        await workbench.saveSource();
        expect(workbench.snapshot()).toMatchObject({ dirty: false, persistedText: 'saved independently' });
        await expect(createCemStudioProjectWorkbench({ repository, validator,
            selection: { projectId: 'alpha', entryId: 'missing' } })).rejects.toThrow(/missing/);
    });

    it('reopens a native command-created page from repository bytes and runs its exact command', async () => {
        const repository = await fixture('alpha', false, true);
        const validator = await createCemStudioBrowserValidator();
        try {
            const source = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'alpha', resourceId: 'source' } });
            workbenches.push(source);
            await source.parsePersisted('events');
            source.setCommandTarget({ mode: 'new', name: 'Native events', parentId: 'schema-package-examples' });
            await source.applyCommand();
            const entryId = source.snapshot().command.application.result.entry.id;
            const reopened = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'alpha', entryId } });
            workbenches.push(reopened);
            expect(reopened.snapshot()).toMatchObject({ entryId, resourceId: 'source', runnable: true });
            expect(reopened.snapshot().command.parsed.options.format).toBe('events');
            await reopened.runPersistedOperation();
            expect(reopened.snapshot().projection).toMatchObject({ kind: 'parse', mode: 'events', stale: false });
            const reordered = await repository.execute(request('apply-command-page', {
                projectId: 'alpha', expectedProjectRevision: reopened.snapshot().projectRevision,
                target: { mode: 'new', name: 'Reordered resources' },
                commandResource: validator.serializeResourceCommand(reopened.snapshot().command.parsed),
                referencedResourceIds: ['extra', 'source'],
            }));
            const ordered = await createCemStudioProjectWorkbench({ repository, validator,
                selection: { projectId: 'alpha', entryId: reordered.value.entry.id } });
            workbenches.push(ordered);
            expect(ordered.snapshot().resourceId).toBe('source');
        } finally { await validator.close(); }
    }, 30000);

    it('uses page-owned dependency identities and preserves binary downloads exactly', async () => {
        const repository = await fixture();
        const bundle = (await repository.query(request('export-project', { projectId: 'alpha' }))).value;
        bundle.project.id = 'gamma'; bundle.project.rootUri = 'studio://gamma/';
        const bytes = new Uint8Array([0, 255, 1, 2]);
        bundle.project.resources.push({ id: 'binary', role: 'data', sourceKind: 'project-file', path: 'assets/picture.bin',
            contentType: 'application/octet-stream', schema: 'https://cem.dev/ns/data/xml/1', revision: 1, sha256: await digest(bytes) });
        bundle.contents.binary = bytes;
        bundle.project.entries.find(item => item.id === 'validate-cem-ml').resourceIds.push('binary');
        await repository.execute(request('import-project', { bundle }));
        const validator = validatorFor(validResult());
        const source = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'gamma', entryId: 'validate-cem-ml' } });
        workbenches.push(source);
        await source.parsePersisted();
        expect(validator.parseResource.mock.calls[0][0].dependencies).toEqual([expect.objectContaining({ resourceId: 'binary', path: 'studio://gamma/assets/picture.bin', bytes })]);
        const binary = await createCemStudioProjectWorkbench({ repository, validator, selection: { projectId: 'gamma', resourceId: 'binary' } });
        workbenches.push(binary);
        expect(binary.snapshot()).toMatchObject({ status: 'read-only', editable: false, resourceId: 'binary', byteLength: 4 });
        let downloaded; await binary.downloadSource(async value => { downloaded = value; });
        expect(downloaded.bytes).toEqual(bytes);
    });

    it('keeps both drafts on failed Save and stale Apply through the mounted guard', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        await workbench.parsePersisted();
        workbench.updateDraft('saved source');
        await workbench.updateCommandDraft(workbench.snapshot().command.current.text.replace('--format ast', '--format events'));
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        selectValue(root, '[data-cem-studio-workbench-select]', target.id); await view.whenSettled();
        const execute = vi.spyOn(repository, 'execute'); execute.mockRejectedValueOnce(new Error('Save unavailable'));
        dialogAction(root, 'Save source').click(); await view.whenSettled();
        expect(session.snapshot()).toMatchObject({ status: 'failed', transition: { stage: 'source' }, workbench: { dirty: true } });
        expect(root.textContent).toContain('Save unavailable');
        dialogAction(root, 'Save source').click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('command');
        const loaded = workbench.snapshot();
        await repository.execute(request('save-resource', { projectId: loaded.projectId, resourceId: loaded.resourceId,
            expectedProjectRevision: loaded.projectRevision, expectedResourceRevision: loaded.resourceRevision, content: 'external change' }));
        dialogAction(root, 'Apply command').click(); await view.whenSettled();
        expect(session.snapshot().selection.projectId).toBe('feature-tour');
        expect(session.snapshot().transition.error).toBeDefined();
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(session.snapshot().transition).toBeUndefined();
    });

    it('persists both drafts through the dialog, then admits the requested project', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        await workbench.parsePersisted();
        workbench.updateDraft('durable source');
        await workbench.updateCommandDraft(workbench.snapshot().command.current.text.replace('--format ast', '--format events'));
        workbench.setCommandTarget({ mode: 'existing', entryId: 'validate-cem-ml' });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        selectValue(root, '[data-cem-studio-workbench-select]', target.id); await view.whenSettled();
        dialogAction(root, 'Save source').click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('command');
        dialogAction(root, 'Apply command').click(); await view.whenSettled();
        expect(session.snapshot().transition.stage).toBe('confirmation');
        expect(root.querySelectorAll('dialog[open]')).toHaveLength(1);
        dialogAction(root, 'Replace page').click(); await view.whenSettled();
        expect(session.snapshot().selection.projectId).toBe('beta');
        const bundle = (await repository.query(request('export-project', { projectId: 'feature-tour' }))).value;
        expect(new TextDecoder().decode(bundle.contents.source)).toBe('durable source');
        expect(bundle.project.entries.some(item => item.kind === 'inspection')).toBe(true);
        await workbench.refreshNavigation();
        expect(workbench.snapshot().workbenches.some(item => item.operation === 'inspection')).toBe(true);
    });

    it('cancels a late candidate from the native dialog without discarding the active draft', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const gate = deferred();
        const session = createCemStudioProjectSession({ openWorkbench: async selection => {
            const candidate = await createCemStudioProjectWorkbench({ repository, validator, selection });
            if (selection.projectId === 'beta') await gate.promise;
            return candidate;
        } });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        workbench.updateDraft('retain late draft');
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        selectValue(root, '[data-cem-studio-workbench-select]', target.id); await view.whenSettled();
        dialogAction(root, 'Discard draft').click();
        await vi.waitFor(() => expect(session.snapshot().status).toBe('opening'));
        dialogAction(root, 'Cancel navigation').click();
        gate.resolve(); await view.whenSettled();
        expect(session.snapshot()).toMatchObject({ selection: { projectId: 'feature-tour' }, workbench: { draft: 'retain late draft', dirty: true } });
        expect(root.querySelector('dialog[open]')).toBeNull();
    });

    it('admits an empty second project and guards mounted source navigation with native dialogs', async () => {
        const repository = await fixture('beta', true);
        const validator = validatorFor(validResult());
        const session = createCemStudioProjectSession({ openWorkbench: selection =>
            createCemStudioProjectWorkbench({ repository, validator, selection }) });
        workbenches.push(session);
        await session.select({ projectId: 'feature-tour', resourceId: 'source' });
        const workbench = await createCemStudioSessionWorkbench({ repository, session });
        const root = document.createElement('main'); document.body.append(root);
        const view = await mountCemStudioFeatureTourWorkbench({ root, workbench }); views.push(view);
        workbench.updateDraft('retain me');
        const target = workbench.snapshot().workbenches.find(item => item.selection.projectId === 'beta');
        selectValue(root, '[data-cem-studio-workbench-select]', target.id);
        await view.whenSettled();
        expect(root.querySelector('dialog[open]')).not.toBeNull();
        dialogAction(root, 'Cancel navigation').click(); await view.whenSettled();
        expect(session.snapshot().workbench.draft).toBe('retain me');
        expect(root.querySelector('cem-select[data-cem-studio-workbench-select]').value).toBe(workbench.snapshot().workbenchId);
        const selector = root.querySelector('cem-select[data-cem-studio-workbench-select] button');
        selector.focus();
        await vi.waitFor(() => expect(document.activeElement).toBe(selector));
        selectValue(root, '[data-cem-studio-workbench-select]', target.id); await view.whenSettled();
        const dialog = root.querySelector('dialog[open]');
        dialog.requestClose(); await view.whenSettled();
        expect(session.snapshot().transition).toBeUndefined();
        await vi.waitFor(() => expect(document.activeElement).toBe(selector));
        selectValue(root, '[data-cem-studio-workbench-select]', target.id); await view.whenSettled();
        dialogAction(root, 'Discard draft').click(); await view.whenSettled();
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: { projectId: 'beta' },
            workbench: { status: 'empty', editable: false } });
        expect(root.querySelector('dialog[open]')).toBeNull();
        expect(root.textContent).toContain('This project has no resources');
    });
});

describe('Studio project session admission', () => {
    const first = { projectId: 'feature-tour', entryId: 'validate-cem-ml', resourceId: 'source' };
    const second = { ...first, projectId: 'user-project' };

    async function fixture(storage) {
        const validator = validatorFor(validResult());
        const repository = await repositoryWithSource('original');
        const exported = await repository.query(request('export-project', { projectId: first.projectId }));
        await repository.execute(request('import-project', { bundle: {
            ...exported.value,
            project: { ...exported.value.project, id: second.projectId, rootUri: 'studio://user-project/' },
        } }));
        const openWorkbench = vi.fn(async selection => {
            const workbench = await createCemStudioFeatureTourWorkbench({
                repository, validator, seed: featureTourSeed(), projectId: selection.projectId,
            });
            workbenches.push(workbench);
            return workbench;
        });
        const session = createCemStudioProjectSession({ openWorkbench, storage });
        workbenches.push(session);
        await session.select(first);
        return { repository, validator, session, openWorkbench, workbench: session.activeWorkbench() };
    }

    async function commandDraft(workbench, mode = 'events') {
        await workbench.updateCommandDraft(workbench.snapshot().command.current.text.replace('--format ast', `--format ${mode}`));
    }

    async function resolve(session, decision) {
        return session.resolveTransition(session.snapshot().transition.id, decision);
    }

    function draftStorage() {
        const records = new Map();
        return { records, getItem: key => records.get(key) ?? null,
            setItem: vi.fn((key, value) => records.set(key, value)) };
    }

    async function reopen(openWorkbench, storage, selection = first) {
        const session = createCemStudioProjectSession({ openWorkbench, storage });
        workbenches.push(session);
        await session.select(selection);
        return session;
    }

    it.each(['', 'draft 🙂\nsecond line'])('recovers exact source and command drafts without Save, Apply or Run: %s', async draft => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench, repository, validator } = await fixture(storage);
        const before = await repository.query(request('export-project', { projectId: first.projectId }));
        workbench.updateDraft(draft);
        await commandDraft(workbench);
        const expected = workbench.snapshot().command.draftText;
        expect(session.snapshot().draftRecovery.status).toBe('saved');
        session.dispose();
        const restored = await reopen(openWorkbench, storage);
        expect(restored.snapshot().workbench).toMatchObject({ draft, dirty: true, command: { draftText: expected } });
        expect(restored.snapshot().draftRecovery.status).toBe('recovered');
        const after = await repository.query(request('export-project', { projectId: first.projectId }));
        expect(after.value).toEqual(before.value);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        await restored.activeWorkbench().saveSource();
        restored.dispose();
        const commandOnly = await reopen(openWorkbench, storage);
        expect(commandOnly.activeWorkbench().snapshot()).toMatchObject({ draft, dirty: false, command: { draftText: expected } });
        await commandOnly.activeWorkbench().resetCommandDraft();
        expect(commandOnly.draftBackup()).toBe('');
    });

    it('recovers an accepted grouped draft with every option and no persisted command mutation', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench, repository } = await fixture(storage);
        const before = (await repository.query(request('export-project', { projectId: first.projectId }))).value;
        await workbench.updateCommandFields([{ id: 'format', value: 'events' }, { id: 'preserve_source_offsets', value: 'true' }], workbench.snapshot().command.formVersion);
        const expected = workbench.snapshot().command.draftText;
        session.dispose();
        const restored = await reopen(openWorkbench, storage);
        expect(restored.snapshot().draftRecovery.status).toBe('recovered');
        expect(restored.activeWorkbench().snapshot().command.draftText).toBe(expected);
        expect(parseCemMlCommandText(expected).options).toMatchObject({ format: 'events', preserve_source_offsets: true });
        expect((await repository.query(request('export-project', { projectId: first.projectId }))).value).toEqual(before);
    });

    it('retains conflicting and corrupt records for download until explicit clearing', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench } = await fixture(storage);
        workbench.updateDraft('recover me');
        session.dispose();
        const other = await openWorkbench(first);
        other.updateDraft('new durable source');
        await other.saveSource();
        const restored = await reopen(openWorkbench, storage);
        expect(restored.snapshot().draftRecovery).toMatchObject({ status: 'conflict', blocked: true });
        expect(restored.activeWorkbench().snapshot().draft).toBe('new durable source');
        expect(restored.draftBackup()).toContain('recover me');
        restored.clearDraftRecovery();
        expect(restored.draftBackup()).toBe('');
        restored.dispose();
        const key = [...storage.records.keys()].find(key => key.includes(':draft-v1:'));
        storage.records.set(key, '{broken');
        const corrupt = await reopen(openWorkbench, storage);
        expect(corrupt.snapshot().draftRecovery.status).toBe('error');
        expect(corrupt.draftBackup()).toContain('{broken');
        expect(storage.records.get(key)).toBe('{broken');
    });

    it('retains latest in-memory text after quota failure and protects changed journal records', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench } = await fixture(storage);
        storage.setItem.mockImplementation(() => { throw new DOMException('Quota exceeded', 'QuotaExceededError'); });
        workbench.updateDraft('first draft');
        workbench.updateDraft('latest draft');
        expect(session.snapshot().draftRecovery.status).toBe('error');
        expect(JSON.parse(session.draftBackup()).current.draft).toBe('latest draft');
        storage.setItem.mockImplementation((key, value) => storage.records.set(key, value));
        session.clearDraftRecovery();
        const competitor = await reopen(openWorkbench, storage);
        competitor.activeWorkbench().updateDraft('other tab draft');
        workbench.updateDraft('local draft');
        expect(session.snapshot().draftRecovery.blocked).toBe(true);
        expect(session.draftBackup()).toContain('other tab draft');
        expect(JSON.parse(session.draftBackup()).current.draft).toBe('local draft');
    });

    it('clears discarded drafts only on admission and isolates project journals', async () => {
        const storage = draftStorage();
        const { session, workbench, openWorkbench } = await fixture(storage);
        workbench.updateDraft('alpha draft');
        await session.select(second);
        session.cancelTransition(session.snapshot().transition.id);
        expect(session.draftBackup()).toContain('alpha draft');
        openWorkbench.mockRejectedValueOnce(new Error('Unavailable destination'));
        await session.select(second);
        await resolve(session, 'discard');
        expect(session.snapshot().status).toBe('failed');
        expect(session.draftBackup()).toContain('alpha draft');
        await resolve(session, 'retry');
        session.activeWorkbench().updateDraft('beta draft');
        session.dispose();
        const alpha = await reopen(openWorkbench, storage);
        expect(alpha.activeWorkbench().snapshot().draft).toBe('original');
        const beta = await reopen(openWorkbench, storage, second);
        expect(beta.activeWorkbench().snapshot().draft).toBe('beta draft');
    });

    it('retries an initial open failure without committing a selection', async () => {
        const { openWorkbench } = await fixture();
        const opener = vi.fn(openWorkbench).mockRejectedValueOnce(new Error('Project temporarily unavailable'));
        const session = createCemStudioProjectSession({ openWorkbench: opener });
        workbenches.push(session);
        await session.select(second);
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: undefined, workbench: undefined });
        await resolve(session, 'retry');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second });
    });

    it.each(['projectId', 'entryId', 'resourceId'])('rejects a candidate with the wrong %s', async field => {
        const { session, openWorkbench } = await fixture();
        const realOpen = openWorkbench.getMockImplementation();
        let candidate;
        openWorkbench.mockImplementationOnce(async selection => {
            const actual = await realOpen(selection);
            const snapshot = actual.snapshot();
            const wrong = field === 'entryId'
                ? { ...snapshot, command: { ...snapshot.command,
                    application: { ...snapshot.command.application, currentEntryId: 'wrong-entry' } } }
                : { ...snapshot, [field]: 'wrong-identity' };
            candidate = { ...actual, snapshot: () => wrong, dispose: vi.fn(actual.dispose) };
            return candidate;
        });
        await session.select(second);
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first });
        expect(candidate.dispose).toHaveBeenCalledTimes(1);
    });

    it('requires both draft decisions and saves before Apply without running validation', async () => {
        const { repository, validator, session, workbench } = await fixture();
        workbench.updateDraft('saved source');
        await commandDraft(workbench);
        await session.select(second);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'source' } });
        const id = session.snapshot().transition.id;
        await session.select(second);
        expect(session.snapshot().transition.id).toBe(id);
        await resolve(session, 'discard');
        expect(session.snapshot().transition.stage).toBe('command');
        session.cancelTransition(id);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: undefined });
        expect(workbench.snapshot()).toMatchObject({ draft: 'saved source', dirty: true });
        expect(workbench.snapshot().command.draftText).toContain('--format events');

        await session.select(second);
        await resolve(session, 'save');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
        expect(workbench.snapshot().dirty).toBe(false);
        expect(validator.validateResource).not.toHaveBeenCalled();
        await resolve(session, 'apply');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second, transition: undefined });
        const saved = await repository.query(request('export-project', { projectId: first.projectId }));
        expect(new TextDecoder().decode(saved.value.contents.source)).toBe('saved source');
        expect(saved.value.project.revision).toBe(3);
        expect(saved.value.project.entries).toHaveLength(3);
        expect(workbench.snapshot().command).toMatchObject({ status: 'current', application: { status: 'applied' } });
        expect(workbench.snapshot().command.draftText).toBe(workbench.snapshot().command.current.text);
        expect(validator.executeAuthoredResourceCommand).not.toHaveBeenCalled();
        expect(saved.value.project).not.toHaveProperty('selection');
    });

    it('defers both discards until a candidate opens, retaining drafts after failure or cancellation', async () => {
        const { session, workbench, openWorkbench } = await fixture();
        workbench.updateDraft('unsaved');
        await commandDraft(workbench);
        await session.select(second);
        await resolve(session, 'discard');
        openWorkbench.mockRejectedValueOnce(new Error('Project unavailable'));
        await resolve(session, 'discard');
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first });
        expect(workbench.snapshot()).toMatchObject({ draft: 'unsaved', dirty: true });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        session.cancelTransition(session.snapshot().transition.id);
        await session.select(second);
        await resolve(session, 'discard');
        await resolve(session, 'discard');
        expect(session.snapshot().selection).toEqual(second);
    });

    it('persists source independently of native command preview availability', async () => {
        const { session, workbench, validator } = await fixture();
        workbench.updateDraft('durable without a native result');
        await commandDraft(workbench);
        await session.select(second);
        validator.previewResourceCommand.mockRejectedValueOnce(new Error('Worker unavailable'));
        await resolve(session, 'save');
        expect(workbench.snapshot()).toMatchObject({ dirty: false, persistedText: 'durable without a native result' });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
        expect(validator.validateResource).not.toHaveBeenCalled();
        await workbench.updateCommandDraft(workbench.snapshot().command.draftText);
        await resolve(session, 'apply');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second });
    });

    it('retains the applied baseline when an older projection finishes', async () => {
        const { workbench, validator } = await fixture();
        await commandDraft(workbench);
        const projection = deferred();
        validator.parseResource.mockImplementation(() => projection.promise);
        const projecting = workbench.parsePersisted('ast');
        await vi.waitFor(() => expect(validator.parseResource).toHaveBeenCalledTimes(1));
        await workbench.applyCommand();
        projection.resolve(projectionOutcome('parse', 'ast'));
        await projecting;
        expect(workbench.snapshot().command.current.text).toContain('--format events');
        expect(workbench.snapshot().command.draftText).toBe(workbench.snapshot().command.current.text);
        expect(workbench.snapshot().command.application.status).toBe('applied');
    });

    it('retains command edits when a background native projection completes', async () => {
        const { session, workbench, validator } = await fixture();
        const projection = deferred();
        validator.parseResource.mockImplementation(() => projection.promise);
        const projecting = workbench.parsePersisted('ast');
        await vi.waitFor(() => expect(validator.parseResource).toHaveBeenCalledTimes(1));
        await commandDraft(workbench);
        await session.select(second);
        projection.resolve(projectionOutcome('parse', 'ast'));
        await projecting;
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
    });

    it('retains the newest command edit through a delayed validation preview refresh', async () => {
        const { workbench, validator } = await fixture();
        const preview = deferred();
        validator.previewResourceCommand.mockImplementationOnce(() => preview.promise);
        const validating = workbench.validatePersisted();
        await vi.waitFor(() => expect(validator.previewResourceCommand).toHaveBeenCalledTimes(2));
        await commandDraft(workbench);
        preview.resolve(commandPreviewOutcome({ operation: 'parse', projection: 'ast', uri: 'data/cem-ml/basic.cem',
            projectId: first.projectId, contentType: 'application/cem', schema: 'https://cem.dev/ns/cem-ml/1',
            projectRevision: 1, resourceRevision: 1 }));
        await validating;
        expect(workbench.snapshot().command.draftText).toContain('--format events');
    });

    it('rechecks admission after both drafts are reverted', async () => {
        const { session, workbench, repository } = await fixture();
        workbench.updateDraft('unsaved');
        await commandDraft(workbench);
        await session.select(second);
        workbench.updateDraft('original');
        await workbench.resetCommandDraft();
        await resolve(session, 'discard');
        expect(session.snapshot()).toMatchObject({ status: 'ready', selection: second });
        const saved = await repository.query(request('export-project', { projectId: first.projectId }));
        expect(saved.value.project.revision).toBe(1);
    });

    it('keeps drafts and the original selection on a failed source save', async () => {
        const { session, workbench, repository } = await fixture();
        workbench.updateDraft('not durable');
        await commandDraft(workbench);
        vi.spyOn(repository, 'execute').mockRejectedValueOnce(new Error('Quota exceeded'));
        await session.select(second);
        await resolve(session, 'save');
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first,
            transition: { stage: 'source', error: { message: 'Quota exceeded' } } });
        expect(workbench.snapshot()).toMatchObject({ draft: 'not durable', persistedText: 'original', dirty: true });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
    });

    it('deduplicates Save, ignores cancelled admission, and preserves edits made during persistence', async () => {
        const { session, workbench, repository } = await fixture();
        const gate = deferred();
        const execute = repository.execute.bind(repository);
        const spy = vi.spyOn(repository, 'execute').mockImplementation(async (...args) => {
            await gate.promise;
            // Persistence can complete despite cancellation; admission must still be revoked.
            return execute(args[0]);
        });
        workbench.updateDraft('committing');
        await session.select(second);
        const id = session.snapshot().transition.id;
        const saving = session.resolveTransition(id, 'save');
        await vi.waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
        const duplicate = session.resolveTransition(id, 'save');
        workbench.updateDraft('newer draft');
        session.cancelTransition(id);
        await session.select(second);
        expect(session.snapshot().selection).toEqual(first);
        gate.resolve();
        await Promise.all([saving, duplicate]);
        expect(spy).toHaveBeenCalledTimes(1);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: undefined });
        expect(workbench.snapshot()).toMatchObject({ persistedText: 'committing', draft: 'newer draft', dirty: true });
        await session.select(second);
        expect(session.snapshot().transition.stage).toBe('source');
        await session.resolveTransition(id, 'discard');
        expect(session.snapshot().selection).toEqual(first);
    });

    it('retains a newer command edit when Apply completes and asks again', async () => {
        const { session, workbench, repository } = await fixture();
        await commandDraft(workbench);
        const gate = deferred();
        const execute = repository.execute.bind(repository);
        const spy = vi.spyOn(repository, 'execute').mockImplementation(async (...args) => {
            await gate.promise;
            return execute(...args);
        });
        await session.select(second);
        const applying = resolve(session, 'apply');
        await vi.waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
        const duplicate = resolve(session, 'apply');
        await workbench.updateCommandDraft(workbench.snapshot().command.draftText.replace('--format events', '--format ast'));
        gate.resolve();
        await Promise.all([applying, duplicate]);
        expect(spy).toHaveBeenCalledTimes(1);
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'command' } });
        expect(workbench.snapshot().command).toMatchObject({ application: { status: 'stale' } });
        expect(workbench.snapshot().command.current.text).toContain('--format events');
        expect(workbench.snapshot().command.draftText).toContain('--format ast');
    });

    it('keeps stale Apply conflicts and explicit replacement confirmation inside the guard', async () => {
        const { session, workbench, repository } = await fixture();
        await commandDraft(workbench);
        workbench.setCommandTarget({ mode: 'existing', entryId: first.entryId });
        await session.select(second);
        await resolve(session, 'apply');
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'confirmation' } });
        session.cancelTransition(session.snapshot().transition.id);
        expect(workbench.snapshot().command.draftText).toContain('--format events');
        await session.select(second);
        await resolve(session, 'apply');
        await repository.execute(request('save-resource', { projectId: first.projectId, resourceId: first.resourceId,
            expectedProjectRevision: 1, expectedResourceRevision: 1, content: 'concurrent save' }));
        await resolve(session, 'confirm-replacement');
        expect(session.snapshot()).toMatchObject({ status: 'failed', selection: first,
            transition: { stage: 'command', error: { code: 'cem.studio.repository.revision_conflict' } } });
        expect(workbench.snapshot().command.draftText).toContain('--format events');
    });

    it('admits an explicitly confirmed compatible replacement after Apply', async () => {
        const { session, workbench } = await fixture();
        await commandDraft(workbench);
        workbench.setCommandTarget({ mode: 'existing', entryId: first.entryId });
        await session.select(second);
        await resolve(session, 'apply');
        expect(session.snapshot().transition.stage).toBe('confirmation');
        await resolve(session, 'confirm-replacement');
        expect(session.snapshot().selection).toEqual(second);
        expect(workbench.snapshot().command.application.result.entry.id).toBe(first.entryId);
    });

    it('disposes a cancelled candidate and ignores late results from the former active workbench', async () => {
        const { session, workbench, openWorkbench, validator } = await fixture();
        const projection = deferred();
        validator.parseResource.mockImplementation(() => projection.promise);
        const projecting = workbench.parsePersisted('ast');
        await vi.waitFor(() => expect(validator.parseResource).toHaveBeenCalledTimes(1));
        const opening = deferred();
        const realOpen = openWorkbench.getMockImplementation();
        let candidate;
        openWorkbench.mockImplementationOnce(async selection => {
            candidate = await realOpen(selection);
            candidate = { ...candidate, dispose: vi.fn(candidate.dispose) };
            await opening.promise;
            return candidate;
        });
        const selecting = session.select(second);
        await vi.waitFor(() => expect(candidate).toBeDefined());
        const id = session.snapshot().transition.id;
        session.cancelTransition(id);
        await session.select(second);
        expect(session.snapshot().selection).toEqual(second);
        const admitted = session.snapshot().workbench;
        opening.resolve();
        await selecting;
        projection.resolve(projectionOutcome('parse', 'ast'));
        await projecting;
        expect(session.snapshot().selection).toEqual(second);
        expect(session.snapshot().workbench).toBe(admitted);
        expect(candidate.dispose).toHaveBeenCalledTimes(1);
    });

    it('rechecks discard consent if a draft changes while the candidate is opening', async () => {
        const { session, workbench, openWorkbench } = await fixture();
        workbench.updateDraft('discarded version');
        const opening = deferred();
        const realOpen = openWorkbench.getMockImplementation();
        openWorkbench.mockImplementationOnce(async selection => {
            const candidate = await realOpen(selection);
            await opening.promise;
            return candidate;
        });
        await session.select(second);
        const selecting = resolve(session, 'discard');
        await vi.waitFor(() => expect(openWorkbench).toHaveBeenCalledTimes(2));
        workbench.updateDraft('keep this version');
        opening.resolve();
        await selecting;
        expect(session.snapshot()).toMatchObject({ selection: first, transition: { stage: 'source' } });
        expect(workbench.snapshot().draft).toBe('keep this version');
    });
});

let nativeGraphAuthor;
const edgeGraphSource = '{run | {import @id=other @src=other.xml} {import @id=data @src=data.xml | {transform @id=view @src=view.xpath @template-content-type=application/vnd.cem.xpath | {export @id=result @out=result.xml}}}}';
async function graphWorkbench(source = '{run | {import @id=data @src=asset.svg @opaque=true @content-type=image/svg+xml | {export @id=result @out=result.svg}}}\n') {
    if (!nativeGraphAuthor) {
        const native = await createCemStudioBrowserValidator();
        nativeGraphAuthor = native.authorGraph; await native.close();
    }
    const repository = await repositoryWithSource(source);
    const validator = validatorFor(validResult()); validator.authorGraph = nativeGraphAuthor;
    const seed = featureTourSeed();
    seed.catalog.workbenches = [{ ...seed.catalog.workbenches[0], id: 'graph', operation: 'transform', kind: 'transformation-graph', commandArguments: ['transform', '--config', '$input'] }];
    const workbench = await createCemStudioFeatureTourWorkbench({ repository, validator, seed, projectId: 'feature-tour' });
    workbenches.push(workbench); await workbench.selectWorkbench('graph');
    return { workbench, repository, validator };
}

function deferred() {
    let resolve;
    const promise = new Promise(complete => { resolve = complete; });
    return { promise, resolve };
}

function selectValue(root, selector, value) {
    const select = root.querySelector(`cem-select${selector}`);
    select.value = value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
}

function dialogAction(root, label) {
    const buttons = [...root.ownerDocument.querySelectorAll('button')];
    const action = buttons.find((button) => button.textContent.trim() === label);
    if (!action) throw new Error(`Dialog action ${label} is unavailable`);
    return action;
}

async function createWorkbench(repository, validator) {
    const workbench = await createCemStudioFeatureTourWorkbench({
        repository,
        validator,
        seed: featureTourSeed(),
        projectId: 'feature-tour',
    });
    workbenches.push(workbench);
    return workbench;
}

function validatorFor(outcome) {
    return {
        validateProject: async (bundle) => bundle,
        previewResourceCommand: vi.fn(async (options) => commandPreviewOutcome(options)),
        serializeResourceCommand: vi.fn((parsed) => serializeCommandResource(parsed)),
        executeAuthoredResourceCommand: vi.fn(async (options) => authoredCommandOutcome(options)),
        parseResource: vi.fn(async ({ projection = 'ast' }) => projectionOutcome('parse', projection)),
        inspectResource: vi.fn(async ({ view = 'summary' }) => projectionOutcome('inspect', view)),
        runResourceCommand: vi.fn(async ({ argv }) => portableOperationOutcome(argv)),
        validateResource: vi.fn(async (options) => {
            const value = typeof outcome === 'function' ? await outcome(options) : outcome;
            if (value.result.exitCode === 0) return value;
            const error = new Error('invalid saved revision');
            error.result = value.result;
            error.presentation = value.presentation;
            throw error;
        }),
    };
}

function serializeCommandResource(parsed) {
    const operation = parsed.commandPath[0];
    const uri = parsed.positionals.inputs;
    const argv =
        operation === 'parse'
            ? ['parse', uri, '--format', parsed.options.format ?? 'ast']
            : ['inspect', uri, '--show', parsed.options.show ?? 'summary', '--format', parsed.options.format ?? 'cem'];
    return `${JSON.stringify(
        {
            $schema: 'https://cem.dev/ns/cli/command/1',
            schemaVersion: 1,
            commandSchemaVersion: 1,
            commonVersion: '0.1.0',
            binaryName: 'cem-ml',
            argv,
        },
        null,
        2,
    )}\n`;
}

function authoredCommandOutcome(options) {
    const resource = JSON.parse(new TextDecoder().decode(new Uint8Array(options.commandResource)));
    const operation = resource.argv[0];
    const mode =
        operation === 'parse'
            ? resource.argv[resource.argv.indexOf('--format') + 1]
            : resource.argv[resource.argv.indexOf('--show') + 1];
    const outcome = projectionOutcome(operation, mode);
    const authored = {
        resource,
        command: commandPreviewOutcome({
            operation,
            projection: operation === 'parse' ? mode : undefined,
            view: operation === 'inspect' ? mode : undefined,
            uri: 'data/cem-ml/basic.cem',
            projectId: options.projectId,
            contentType: options.contentType,
            schema: options.schema,
            projectRevision: options.projectRevision,
            resourceRevision: options.resourceRevision,
        }).parsed,
    };
    return {
        ...outcome,
        parsed: authored.command,
    };
}

function commandPreviewOutcome(options) {
    if (options.text?.includes('--unknown-studio-option')) {
        const error = new Error('unknown CEM-ML option `--unknown-studio-option`');
        error.code = 'cem.command.unknown_option';
        throw error;
    }
    const operation = options.text?.match(/\b(parse|inspect|validate)\b/u)?.[1] ?? options.operation ?? 'parse';
    const format =
        options.text?.match(/--format\s+([^\s]+)/u)?.[1] ??
        (operation === 'parse' ? (options.projection ?? 'ast') : operation === 'inspect' ? 'cem' : 'json');
    const view = options.text?.match(/--show\s+([^\s]+)/u)?.[1] ?? options.view ?? 'summary';
    const uri = /^[a-z][a-z0-9+.-]*:/iu.test(options.uri)
        ? options.uri
        : `studio://${options.projectId}/${options.uri}`;
    const argv =
        operation === 'parse'
            ? ['parse', uri, '--format', format]
            : operation === 'inspect'
              ? ['inspect', uri, '--show', view, '--format', format]
              : ['validate', uri, '--format', format];
    const text =
        options.text ?? `cem-ml ${argv.join(' ')} --content-type ${options.contentType} --schema ${options.schema}`;
    const parsed = Object.freeze({
        schemaVersion: 1,
        commonVersion: '0.1.0',
        commandPath: Object.freeze([operation]),
        globalOptions: Object.freeze({}),
        options: Object.freeze({ format, ...(operation === 'inspect' ? { show: view } : {}) }),
        positionals: Object.freeze({ inputs: uri }),
    });
    return Object.freeze({
        projection: 'studio',
        binaryName: 'cem-ml',
        commonVersion: '0.1.0',
        argv: Object.freeze(argv),
        text,
        parsed,
        semantic: Object.freeze({
            operation: Object.freeze({ kind: operation, ...(operation === 'parse' ? { projection: format } : {}) }),
            inputs: Object.freeze({ normalized: Object.freeze([{ uri }]) }),
            identity: Object.freeze({ contentType: options.contentType, schema: options.schema }),
            configuration: Object.freeze({ view }),
            outputs: Object.freeze({ format }),
            scope: Object.freeze({
                projectRevision: options.projectRevision,
                resourceRevision: options.resourceRevision,
            }),
        }),
    });
}

function projectionOutcome(kind, mode) {
    const text = `${kind} ${mode} projection\n`;
    const bytes = [...new TextEncoder().encode(text)];
    return {
        result: {
            protocolVersion: 1,
            requestId: `workbench-${kind}-${mode}`,
            operation: kind,
            exitCode: 0,
            result: { storage: 'inline', value: { kind, value: { projection: mode } } },
            diagnostics: { items: [], originalCount: 0 },
            sourceMaps: { items: [], originalCount: 0 },
            identity: { runtime: 'wasm-browser-worker' },
        },
        presentation: { writes: [] },
        output: {
            uri: 'cem-stdio://stdout',
            contentType: 'application/cem',
            byteLength: bytes.length,
            sha256: 'fixture-output-sha256',
            bytes,
            text,
        },
    };
}

function portableOperationOutcome(argv) {
    const outcome = projectionOutcome('convert', 'conversion');
    return {
        ...outcome,
        result: {
            ...outcome.result,
            operation: argv[0],
            result: {
                storage: 'inline',
                value: {
                    kind: 'convert',
                    value: { outputs: { items: [{ response: { primary: { kind: 'dom' } } }] } },
                },
            },
        },
    };
}

function invalidResult(start, len) {
    const diagnostic = {
        uri: 'cem-studio://feature-tour/data/cem-ml/basic.cem',
        line: 3,
        column: 1,
        byteOffset: start,
        code: 'cem.tokenizer.unterminated_node',
        severity: 'error',
        message: 'node scope must close before end of input',
        node: 'article',
        details: null,
        sourceMap: {
            frames: [
                {
                    source_id: 7,
                    span: { kind: 'Single', ranges: { start, len } },
                    transform: { kind: 'CemTokenizer' },
                },
            ],
        },
    };
    const report = {
        generatedAt: '2026-08-21T00:00:00Z',
        inputs: ['data/cem-ml/basic.cem'],
        summary: {
            inputCount: 1,
            infoCount: 0,
            warningCount: 0,
            errorCount: 1,
            fatalCount: 0,
            hardViolationCount: 1,
        },
        options: {},
        diagnostics: [diagnostic],
        reportAst: {},
    };
    return commandOutcome(1, report, [diagnostic]);
}

function validResult() {
    return commandOutcome(
        0,
        {
            generatedAt: '2026-08-21T00:00:00Z',
            inputs: ['data/cem-ml/basic.cem'],
            summary: {
                inputCount: 1,
                infoCount: 0,
                warningCount: 0,
                errorCount: 0,
                fatalCount: 0,
                hardViolationCount: 0,
            },
            options: {},
            diagnostics: [],
            reportAst: {},
        },
        [],
    );
}

function commandOutcome(exitCode, report, diagnostics) {
    return {
        result: {
            protocolVersion: 1,
            requestId: 'workbench-test',
            exitCode,
            result: { storage: 'inline', value: { kind: 'validate', value: { report } } },
            diagnostics: { items: diagnostics, originalCount: diagnostics.length },
            sourceMaps: { items: [], originalCount: 0 },
            identity: { runtime: 'wasm-browser-worker' },
        },
        presentation: { writes: [] },
    };
}

async function repositoryWithSource(content) {
    const repository = createCemStudioProjectRepository({
        databaseName: `cem-studio-workbench-${crypto.randomUUID()}`,
        validateProject: async (bundle) => bundle,
        now: () => '2026-08-21T00:00:00Z',
    });
    repositories.push(repository);
    const bytes = new TextEncoder().encode(content);
    const sha256 = await digest(bytes);
    const runConfigBytes = new TextEncoder().encode('{}\n');
    const runConfigSha256 = await digest(runConfigBytes);
    await repository.execute(
        request('import-project', {
            bundle: {
                project: {
                    $schema: 'https://cem.dev/ns/studio/project/1',
                    schemaVersion: 1,
                    id: 'feature-tour',
                    name: 'Feature Tour',
                    rootUri: 'studio://feature-tour/',
                    revision: 1,
                    createdAt: '2026-08-21T00:00:00Z',
                    updatedAt: '2026-08-21T00:00:00Z',
                    entries: [
                        {
                            id: 'schema-package-examples',
                            kind: 'subproject',
                            name: 'Schema package examples',
                        },
                        {
                            id: 'validate-cem-ml',
                            parentId: 'schema-package-examples',
                            kind: 'validation',
                            name: 'CEM ML: Basic',
                            runConfigResourceId: 'run-cem-ml',
                            resourceIds: ['source', 'run-cem-ml'],
                        },
                    ],
                    resources: [
                        {
                            id: 'source',
                            role: 'data',
                            sourceKind: 'project-file',
                            path: 'data/cem-ml/basic.cem',
                            contentType: 'application/cem',
                            schema: 'https://cem.dev/ns/cem-ml/1',
                            revision: 1,
                            sha256,
                        },
                        {
                            id: 'run-cem-ml',
                            role: 'run-config',
                            sourceKind: 'project-file',
                            path: 'config/cem-ml.validate.json',
                            contentType: 'application/json',
                            schema: 'https://cem.dev/ns/cli/run-config/1',
                            revision: 1,
                            sha256: runConfigSha256,
                        },
                    ],
                },
                contents: { source: bytes, 'run-cem-ml': runConfigBytes },
            },
        }),
    );
    return repository;
}

function featureTourSeed() {
    return {
        catalog: {
            examples: [
                {
                    packageId: 'cem-ml',
                    resourceId: 'source',
                    runConfigResourceId: 'run-cem-ml',
                    path: 'data/cem-ml/basic.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    dependencies: [],
                },
            ],
            workbenches: [
                {
                    id: 'conversion',
                    operation: 'convert',
                    kind: 'conversion',
                    name: 'CEM-ML conversion',
                    resourceId: 'source',
                    runConfigResourceId: 'run-cem-ml',
                    path: 'data/cem-ml/basic.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    dependencies: [],
                    expectedSummary: { kind: 'convert', outputCount: 1 },
                    commandArguments: [
                        'convert',
                        '$input',
                        '--content-type',
                        'application/cem',
                        '--schema',
                        'https://cem.dev/ns/cem-ml/1',
                        '--to-format',
                        'dom-json',
                    ],
                },
            ],
        },
    };
}

function request(operation, parameters) {
    return {
        protocolVersion: 1,
        repository: 'studio-projects',
        operation,
        requestRevision: 1,
        parameters,
    };
}

async function digest(bytes) {
    const result = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(result)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function deleteDatabase(name) {
    return new Promise((resolve, reject) => {
        const request = indexedDB.deleteDatabase(name);
        request.onsuccess = () => resolve(undefined);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error(`database ${name} remained open`));
    });
}
