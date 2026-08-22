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
        expect(validator.validateResource).toHaveBeenCalledWith(expect.objectContaining({
            projectId: 'feature-tour',
            projectRevision: 2,
            resourceRevision: 2,
        }));
        const exported = await repository.query(request('export-project', { projectId: 'feature-tour' }));
        expect(new TextDecoder().decode(exported.value.contents.source)).toBe(invalid);
        expect(exported.value.project.revision).toBe(2);
        expect(exported.value.project.resources[0].revision).toBe(2);

        expect(root.querySelector('cem-tabs [role="tablist"]')).not.toBeNull();
        expect(root.querySelector('cem-table [role="table"]')).not.toBeNull();
        expect(root.querySelector('cem-list')).not.toBeNull();
        expect(root.querySelectorAll(
            'button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
        )).toHaveLength(0);

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
        expect(root.querySelector('cem-textarea[data-cem-studio-projection-output] textarea').value)
            .toBe('parse events projection\n');

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
        expect(validator.inspectResource).toHaveBeenLastCalledWith(expect.objectContaining({
            projectId: 'feature-tour',
            projectRevision: 1,
            resourceRevision: 1,
            view: 'tree',
        }));
    });

    it('marks an in-flight saved-revision result stale when the draft advances', async () => {
        let releaseValidation;
        let startedValidation;
        const started = new Promise((resolve) => {
            startedValidation = resolve;
        });
        const validator = validatorFor(() => new Promise((resolve) => {
            releaseValidation = () => resolve(validResult());
            startedValidation();
        }));
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
        validator.parseResource.mockImplementation(() => new Promise((resolve) => {
            releaseProjection = () => resolve(projectionOutcome('parse', 'ast'));
            startedProjection();
        }));
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
        parseResource: vi.fn(async ({ projection = 'ast' }) => projectionOutcome('parse', projection)),
        inspectResource: vi.fn(async ({ view = 'summary' }) => projectionOutcome('inspect', view)),
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
            frames: [{
                source_id: 7,
                span: { kind: 'Single', ranges: { start, len } },
                transform: { kind: 'CemTokenizer' },
            }],
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
    return commandOutcome(0, {
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
    }, []);
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
    await repository.execute(request('import-project', {
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
                entries: [],
                resources: [{
                    id: 'source',
                    role: 'data',
                    sourceKind: 'project-file',
                    path: 'data/cem-ml/basic.cem',
                    contentType: 'application/cem',
                    schema: 'https://cem.dev/ns/cem-ml/1',
                    revision: 1,
                    sha256,
                }],
            },
            contents: { source: bytes },
        },
    }));
    return repository;
}

function featureTourSeed() {
    return {
        catalog: {
            examples: [{
                packageId: 'cem-ml',
                resourceId: 'source',
                path: 'data/cem-ml/basic.cem',
                contentType: 'application/cem',
                schema: 'https://cem.dev/ns/cem-ml/1',
                dependencies: [],
            }],
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
