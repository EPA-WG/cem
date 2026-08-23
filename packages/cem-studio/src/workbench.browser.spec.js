import { afterEach, describe, expect, it, vi } from 'vitest';

import { createCemStudioProjectRepository } from './repository.js';
import { mountCemStudioApplicationShell } from './shell.js';
import {
    CEM_STUDIO_INSPECT_VIEWS,
    CEM_STUDIO_PARSE_PROJECTIONS,
    createCemStudioFeatureTourWorkbench,
    mountCemStudioFeatureTourWorkbench,
} from './workbench.js';

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
        expect(editor.value).toBe('original');
        editor.value = invalid;
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(workbench.snapshot()).toMatchObject({ status: 'dirty', dirty: true });
        await view.whenSettled();

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
        expect(
            root.querySelectorAll('button:not(cem-action button):not(cem-select button):not(cem-tabs button)'),
        ).toHaveLength(0);

        const diagnostics = root.querySelector('cem-list[data-diagnostic-list] select');
        diagnostics.value = '0';
        diagnostics.dispatchEvent(new Event('change', { bubbles: true }));
        await view.whenSettled();
        expect(editor.selectionStart).toBe(workbench.snapshot().selection.start);
        expect(workbench.snapshot().selection).toMatchObject({ kind: 'diagnostic', byteStart: rangeStart });
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
        expect(
            root.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ),
        ).toHaveLength(0);
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
        const commandEditor = root.querySelector('cem-textarea[data-cem-studio-command-editor] textarea');
        expect(commandEditor.value).toBe(original);
        expect(workbench.snapshot().command).toMatchObject({
            projection: 'studio',
            status: 'current',
            changes: [],
            revision: { projectRevision: 1, resourceRevision: 1 },
        });

        const changed = original.replace('--format ast', '--format events');
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
        expect(root.querySelector('cem-table[label="CLI Command semantic changes"] [role="table"]')).not.toBeNull();

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
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ),
        ).toHaveLength(0);
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
        name.value = 'Applied inspection';
        name.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await view.whenSettled();
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
                '[data-cem-studio-command-application] button:not(cem-action button):not(cem-select button)',
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
        expect(dialog.open).toBe(true);
        expect(dialog.getAttribute('aria-label')).toBe('Confirm incompatible page replacement');
        let exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(exported.value.project.revision).toBe(1);

        const dismissed = new Promise((resolve) => {
            root.addEventListener('cem-dismiss', resolve, { once: true });
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

        root.querySelector('cem-action[data-cem-studio-command-apply-run] button').click();
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
