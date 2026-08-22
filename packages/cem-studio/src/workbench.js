import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { installCemStudioShellComponents } from './shell.js';

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

export const CEM_STUDIO_PARSE_PROJECTIONS = Object.freeze(['ast', 'events']);
export const CEM_STUDIO_INSPECT_VIEWS = Object.freeze([
    'summary',
    'ast',
    'events',
    'diagnostics',
    'source-offsets',
    'tree',
]);

/**
 * Own one editable Feature Tour resource and its durable validation revision.
 * @param {{repository: object, validator: object, seed: object, projectId: string, example?: object}} options
 */
export async function createCemStudioFeatureTourWorkbench(options) {
    const { repository, validator, seed, projectId } = options;
    if (!repository?.query || !repository?.execute) throw new TypeError('Feature Tour workbench requires a repository');
    if (
        !validator?.validateResource
        || !validator?.parseResource
        || !validator?.inspectResource
        || !validator?.previewResourceCommand
    ) {
        throw new TypeError('Feature Tour workbench requires a browser command validator');
    }
    if (typeof projectId !== 'string' || projectId.length === 0) {
        throw new TypeError('Feature Tour workbench requires a project id');
    }
    const example = options.example ?? featureTourCemExample(seed);
    const subscribers = new Set();
    let editVersion = 0;
    let commandEditVersion = 0;
    let state = freezeState({
        status: 'loading',
        projectId,
        resourceId: example.resourceId,
        path: example.path,
        contentType: example.contentType,
        schema: example.schema,
        projectRevision: 0,
        resourceRevision: 0,
        repositoryRevision: 0,
        persistedText: '',
        draft: '',
        dirty: false,
        validation: undefined,
        projection: undefined,
        command: undefined,
        selection: undefined,
        error: undefined,
    });

    function snapshot() {
        return state;
    }

    function publish(next) {
        state = freezeState(next);
        for (const notify of subscribers) notify(state);
        return state;
    }

    function subscribe(notify) {
        if (typeof notify !== 'function') throw new TypeError('workbench subscriber must be a function');
        subscribers.add(notify);
        notify(state);
        return () => subscribers.delete(notify);
    }

    async function readPersisted() {
        const response = await repository.query(repositoryRequest('export-project', { projectId }));
        const bundle = response.value;
        if (!bundle?.project || !bundle?.contents) throw new Error(`Feature Tour project ${projectId} is unavailable`);
        const resource = bundle.project.resources?.find(({ id }) => id === example.resourceId);
        if (!resource) throw new Error(`Feature Tour resource ${example.resourceId} is unavailable`);
        const bytes = toBytes(bundle.contents[example.resourceId]);
        const dependencies = example.dependencies.map((dependency) => {
            const dependencyResource = bundle.project.resources.find(({ id }) => id === dependency.resourceId);
            if (!dependencyResource) throw new Error(`Feature Tour dependency ${dependency.resourceId} is unavailable`);
            return {
                bytes: toBytes(bundle.contents[dependency.resourceId]),
                contentType: dependencyResource.contentType,
                schema: dependencyResource.schema,
                path: dependency.path,
            };
        });
        return {
            bytes,
            text: textDecoder.decode(bytes),
            dependencies,
            projectRevision: bundle.project.revision,
            resourceRevision: resource.revision,
            repositoryRevision: response.repositoryRevision,
            sha256: resource.sha256,
        };
    }

    async function reload() {
        commandEditVersion += 1;
        const persisted = await readPersisted();
        const command = await commandForPersisted(persisted);
        editVersion += 1;
        return publish({
            ...state,
            status: 'loaded',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            draft: persisted.text,
            dirty: false,
            validation: undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            command,
            selection: undefined,
            error: undefined,
        });
    }

    function updateDraft(draft) {
        if (typeof draft !== 'string') throw new TypeError('Feature Tour draft must be text');
        editVersion += 1;
        return publish({
            ...state,
            status: draft === state.persistedText ? 'loaded' : 'dirty',
            draft,
            dirty: draft !== state.persistedText,
            validation: state.validation ? freezeValidation({ ...state.validation, stale: true }) : undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            selection: undefined,
            error: undefined,
        });
    }

    async function saveAndValidate(options = {}) {
        const captured = {
            draft: state.draft,
            editVersion,
            projectRevision: state.projectRevision,
            resourceRevision: state.resourceRevision,
        };
        publish({ ...state, status: 'saving', error: undefined });
        let saved;
        try {
            saved = await repository.execute(repositoryRequest('save-resource', {
                projectId,
                resourceId: example.resourceId,
                expectedProjectRevision: captured.projectRevision,
                expectedResourceRevision: captured.resourceRevision,
                content: captured.draft,
            }), options.signal);
        } catch (error) {
            publish({
                ...state,
                status: error?.code === 'cem.studio.repository.revision_conflict' ? 'conflict' : 'failed',
                error: normalizedError(error),
            });
            throw error;
        }

        const persisted = await readPersisted();
        const expectedBytes = textEncoder.encode(captured.draft);
        if (
            !equalBytes(expectedBytes, persisted.bytes)
            || persisted.sha256 !== saved.value.sha256
            || persisted.projectRevision !== saved.value.projectRevision
            || persisted.resourceRevision !== saved.value.resourceRevision
        ) {
            const error = new Error('Feature Tour repository did not reload the exact committed revision');
            error.code = 'cem.studio.workbench.persistence_mismatch';
            publish({ ...state, status: 'failed', error: normalizedError(error) });
            throw error;
        }
        const changedDuringSave = editVersion !== captured.editVersion;
        const command = await commandForPersisted(persisted);
        publish({
            ...state,
            status: changedDuringSave ? 'dirty' : 'saved',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            draft: changedDuringSave ? state.draft : persisted.text,
            dirty: changedDuringSave,
            validation: undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            command,
            selection: undefined,
            error: undefined,
        });
        return validateRevision({ ...persisted, editVersion: captured.editVersion, draftMatches: true }, options.signal);
    }

    async function validatePersisted(options = {}) {
        const persisted = await readPersisted();
        const command = await commandForPersisted(persisted);
        const capturedVersion = editVersion;
        publish({
            ...state,
            status: 'validating',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            command,
            error: undefined,
        });
        return validateRevision({
            ...persisted,
            editVersion: capturedVersion,
            draftMatches: state.draft === persisted.text,
        }, options.signal);
    }

    async function validateRevision(persisted, signal) {
        publish({ ...state, status: 'validating', error: undefined });
        let outcome;
        try {
            outcome = await validator.validateResource({
                bytes: persisted.bytes,
                contentType: example.contentType,
                schema: example.schema,
                uri: example.path,
                dependencies: persisted.dependencies,
                projectId,
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                signal,
            });
        } catch (error) {
            if (!error?.result) {
                publish({ ...state, status: 'failed', error: normalizedError(error) });
                throw error;
            }
            outcome = { result: error.result, presentation: error.presentation };
        }
        const validation = projectValidation(outcome.result, outcome.presentation, persisted.text, {
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            sha256: persisted.sha256,
        });
        const stale = editVersion !== persisted.editVersion
            || persisted.draftMatches === false
            || state.projectRevision !== persisted.projectRevision
            || state.resourceRevision !== persisted.resourceRevision
            || Boolean(outcome.result.stale);
        const nextValidation = freezeValidation({ ...validation, stale });
        publish({
            ...state,
            status: stale ? 'stale' : validation.hardViolationCount > 0 ? 'invalid' : 'valid',
            validation: nextValidation,
            selection: undefined,
            error: undefined,
        });
        return state;
    }

    async function parsePersisted(projection = 'ast', options = {}) {
        if (!CEM_STUDIO_PARSE_PROJECTIONS.includes(projection)) {
            throw new TypeError(`unsupported CEM-ML parse projection: ${projection}`);
        }
        return projectPersisted('parse', projection, options);
    }

    async function inspectPersisted(view = 'summary', options = {}) {
        if (!CEM_STUDIO_INSPECT_VIEWS.includes(view)) {
            throw new TypeError(`unsupported CEM-ML inspect view: ${view}`);
        }
        return projectPersisted('inspect', view, options);
    }

    async function projectPersisted(kind, mode, options) {
        const persisted = await readPersisted();
        const capturedVersion = editVersion;
        const draftMatches = state.draft === persisted.text;
        publish({
            ...state,
            status: 'projecting',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            error: undefined,
        });
        let outcome;
        try {
            outcome = await validator[`${kind}Resource`]({
                bytes: persisted.bytes,
                contentType: example.contentType,
                schema: example.schema,
                uri: example.path,
                dependencies: persisted.dependencies,
                projectId,
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                [kind === 'parse' ? 'projection' : 'view']: mode,
                signal: options.signal,
            });
        } catch (error) {
            if (!error?.result || !error?.output) {
                publish({ ...state, status: 'failed', error: normalizedError(error) });
                throw error;
            }
            outcome = { result: error.result, presentation: error.presentation, output: error.output };
        }
        const projected = projectCommandProjection(kind, mode, outcome, persisted.text, {
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            sha256: persisted.sha256,
        });
        const stale = editVersion !== capturedVersion
            || !draftMatches
            || state.projectRevision !== persisted.projectRevision
            || state.resourceRevision !== persisted.resourceRevision
            || Boolean(outcome.result.stale);
        const projection = freezeProjection({ ...projected, stale });
        commandEditVersion += 1;
        const command = await commandForProjection(persisted, kind, mode);
        publish({
            ...state,
            status: stale ? 'projection-stale' : projected.exitCode === 0 ? 'projected' : 'projection-invalid',
            projection,
            command,
            selection: undefined,
            error: undefined,
        });
        return state;
    }

    async function updateCommandDraft(text) {
        if (typeof text !== 'string') throw new TypeError('CEM Studio command draft must be text');
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        const capturedVersion = ++commandEditVersion;
        const current = state.command.current;
        const persisted = await readPersisted();
        if (capturedVersion !== commandEditVersion) return state;
        publish({
            ...state,
            command: freezeCommand({
                ...state.command,
                status: 'checking',
                draftText: text,
                diagnostic: undefined,
                copy: undefined,
            }),
        });
        try {
            const preview = await validator.previewResourceCommand({
                ...commandResourceOptions(persisted),
                text,
            });
            if (capturedVersion !== commandEditVersion) return state;
            const changes = semanticChanges(current.semantic, preview.semantic);
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    status: changes.length === 0 ? 'current' : 'changed',
                    draftText: text,
                    parsed: preview.parsed,
                    preview,
                    changes,
                    diagnostic: undefined,
                    copy: undefined,
                }),
            });
        } catch (error) {
            if (capturedVersion !== commandEditVersion) return state;
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    status: 'invalid',
                    draftText: text,
                    parsed: undefined,
                    preview: undefined,
                    changes: Object.freeze([]),
                    diagnostic: normalizedError(error),
                    copy: undefined,
                }),
            });
        }
        return state;
    }

    async function resetCommandDraft() {
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        return updateCommandDraft(state.command.current.text);
    }

    async function copyCommand(writeText) {
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        publish({
            ...state,
            command: freezeCommand({ ...state.command, copy: Object.freeze({ status: 'copying' }) }),
        });
        try {
            if (typeof writeText !== 'function') {
                const error = new Error('Clipboard writing is unavailable; select and copy the command text instead.');
                error.code = 'cem.studio.command.clipboard_unavailable';
                throw error;
            }
            await writeText(state.command.draftText);
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    copy: Object.freeze({
                        status: 'success',
                        message: 'Displayed Studio command copied.',
                    }),
                }),
            });
        } catch (error) {
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    copy: Object.freeze({
                        status: 'failed',
                        message: normalizedError(error).message,
                    }),
                }),
            });
        }
        return state;
    }

    async function commandForPersisted(persisted) {
        if (state.command?.current?.text) {
            const current = await validator.previewResourceCommand({
                ...commandResourceOptions(persisted),
                text: state.command.current.text,
            });
            if (state.command.draftText === state.command.current.text) {
                return commandState(current, persisted);
            }
            try {
                const preview = await validator.previewResourceCommand({
                    ...commandResourceOptions(persisted),
                    text: state.command.draftText,
                });
                const changes = semanticChanges(current.semantic, preview.semantic);
                return freezeCommand({
                    ...state.command,
                    status: changes.length === 0 ? 'current' : 'changed',
                    current,
                    parsed: preview.parsed,
                    preview,
                    changes,
                    diagnostic: undefined,
                    revision: commandRevision(persisted),
                    copy: undefined,
                });
            } catch (error) {
                return freezeCommand({
                    ...state.command,
                    status: 'invalid',
                    current,
                    parsed: undefined,
                    preview: undefined,
                    changes: Object.freeze([]),
                    diagnostic: normalizedError(error),
                    revision: commandRevision(persisted),
                    copy: undefined,
                });
            }
        }
        return commandForProjection(persisted, 'parse', 'ast');
    }

    async function commandForProjection(persisted, kind, mode) {
        const preview = await validator.previewResourceCommand({
            ...commandResourceOptions(persisted),
            operation: kind,
            [kind === 'parse' ? 'projection' : 'view']: mode,
        });
        return commandState(preview, persisted);
    }

    function commandResourceOptions(persisted) {
        return {
            bytes: persisted.bytes,
            contentType: example.contentType,
            schema: example.schema,
            uri: example.path,
            dependencies: persisted.dependencies,
            projectId,
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
        };
    }

    function navigateDiagnostic(index) {
        const diagnostic = state.validation?.diagnostics[index];
        if (!diagnostic) throw new RangeError(`diagnostic ${index} is unavailable`);
        return selectRange('diagnostic', index, diagnostic.range);
    }

    function navigateProvenance(index) {
        const frame = state.validation?.provenance[index];
        if (!frame) throw new RangeError(`provenance frame ${index} is unavailable`);
        return selectRange('provenance', index, frame.range);
    }

    function selectRange(kind, index, range) {
        const selection = Object.freeze({
            kind,
            index,
            byteStart: range.start,
            byteLength: range.len,
            start: utf8ByteOffsetToCodeUnit(state.draft, range.start),
            end: utf8ByteOffsetToCodeUnit(state.draft, range.start + range.len),
        });
        publish({ ...state, selection });
        return selection;
    }

    await reload();
    return Object.freeze({
        snapshot,
        subscribe,
        updateDraft,
        reload,
        saveAndValidate,
        validatePersisted,
        parsePersisted,
        inspectPersisted,
        updateCommandDraft,
        resetCommandDraft,
        copyCommand,
        navigateDiagnostic,
        navigateProvenance,
        dispose() {
            subscribers.clear();
        },
    });
}

/** Mount the workbench with production CEM controls only. */
export async function mountCemStudioFeatureTourWorkbench({ root, workbench, clipboard = navigator.clipboard }) {
    if (!(root instanceof Element)) throw new TypeError('Feature Tour workbench root must be an Element');
    const components = await installCemStudioShellComponents();
    const host = document.createElement('section');
    host.setAttribute('data-cem-studio-workbench', '');
    host.setAttribute('aria-label', 'Feature Tour workbench');
    host.innerHTML = workbenchMarkup();
    const shell = root.querySelector('[data-cem-studio-shell]');
    if (shell) shell.append(host);
    else root.append(host);
    await settleWorkbench(components.runtime, host);

    const editorHost = host.querySelector('cem-textarea[data-cem-studio-editor]');
    const saveAction = host.querySelector('cem-action[data-cem-studio-save]');
    const reloadAction = host.querySelector('cem-action[data-cem-studio-reload]');
    const parseAction = host.querySelector('cem-action[data-cem-studio-parse]');
    const inspectAction = host.querySelector('cem-action[data-cem-studio-inspect]');
    const commandEditorHost = host.querySelector('cem-textarea[data-cem-studio-command-editor]');
    const commandCopyAction = host.querySelector('cem-action[data-cem-studio-command-copy]');
    const commandResetAction = host.querySelector('cem-action[data-cem-studio-command-reset]');
    const parseSelect = host.querySelector('cem-select[data-cem-studio-parse-projection]');
    const inspectSelect = host.querySelector('cem-select[data-cem-studio-inspect-view]');
    let renderPromise = Promise.resolve();
    let actionPromise = Promise.resolve();

    const render = (snapshot) => {
        renderPromise = renderPromise.then(async () => {
            renderWorkbench(host, snapshot);
            await settleWorkbench(components.runtime, host);
            const editor = host.querySelector('cem-textarea[data-cem-studio-editor] textarea');
            if (editor && editor.value !== snapshot.draft) editor.value = snapshot.draft;
            const projectionOutput = host.querySelector('cem-textarea[data-cem-studio-projection-output] textarea');
            if (projectionOutput && projectionOutput.value !== (snapshot.projection?.output.text ?? '')) {
                projectionOutput.value = snapshot.projection?.output.text ?? '';
            }
            const commandEditor = host.querySelector('cem-textarea[data-cem-studio-command-editor] textarea');
            if (commandEditor && commandEditor.value !== (snapshot.command?.draftText ?? '')) {
                commandEditor.value = snapshot.command?.draftText ?? '';
            }
            if (editor && snapshot.selection) {
                editor.focus();
                editor.setSelectionRange(snapshot.selection.start, snapshot.selection.end);
            }
        });
    };
    const unsubscribe = workbench.subscribe(render);
    const edited = (event) => {
        if (event.target instanceof HTMLTextAreaElement) workbench.updateDraft(event.target.value);
    };
    const save = () => {
        actionPromise = workbench.saveAndValidate().catch(() => undefined);
    };
    const reload = () => {
        actionPromise = workbench.reload().catch(() => undefined);
    };
    const parse = () => {
        actionPromise = workbench.parsePersisted(parseSelect.value).catch(() => undefined);
    };
    const inspect = () => {
        actionPromise = workbench.inspectPersisted(inspectSelect.value).catch(() => undefined);
    };
    const commandEdited = (event) => {
        if (event.target instanceof HTMLTextAreaElement) {
            actionPromise = workbench.updateCommandDraft(event.target.value).catch(() => undefined);
        }
    };
    const copyCommand = () => {
        const writeText = typeof clipboard?.writeText === 'function'
            ? (text) => clipboard.writeText(text)
            : undefined;
        actionPromise = workbench.copyCommand(writeText).catch(() => undefined);
    };
    const resetCommand = () => {
        actionPromise = workbench.resetCommandDraft().catch(() => undefined);
    };
    const navigate = (event) => {
        if (!(event.target instanceof HTMLSelectElement)) return;
        const list = event.target.closest('cem-list');
        const index = Number(event.target.value);
        if (list?.hasAttribute('data-diagnostic-list')) workbench.navigateDiagnostic(index);
        else if (list?.hasAttribute('data-provenance-list')) workbench.navigateProvenance(index);
    };
    editorHost.addEventListener('input', edited);
    saveAction.addEventListener('click', save);
    reloadAction.addEventListener('click', reload);
    parseAction.addEventListener('click', parse);
    inspectAction.addEventListener('click', inspect);
    commandEditorHost.addEventListener('input', commandEdited);
    commandCopyAction.addEventListener('click', copyCommand);
    commandResetAction.addEventListener('click', resetCommand);
    host.addEventListener('change', navigate);
    await renderPromise;

    return Object.freeze({
        root: host,
        workbench,
        async whenSettled() {
            await actionPromise;
            await renderPromise;
        },
        dispose() {
            unsubscribe();
            editorHost.removeEventListener('input', edited);
            saveAction.removeEventListener('click', save);
            reloadAction.removeEventListener('click', reload);
            parseAction.removeEventListener('click', parse);
            inspectAction.removeEventListener('click', inspect);
            commandEditorHost.removeEventListener('input', commandEdited);
            commandCopyAction.removeEventListener('click', copyCommand);
            commandResetAction.removeEventListener('click', resetCommand);
            host.removeEventListener('change', navigate);
            host.remove();
        },
    });
}

function featureTourCemExample(seed) {
    const examples = seed?.catalog?.examples;
    if (!Array.isArray(examples)) throw new Error('Feature Tour catalog has no examples');
    const example = examples.find(({ packageId, contentType }) =>
        packageId === 'cem-ml' && typeof contentType === 'string' && contentType.includes('cem'));
    if (!example) throw new Error('Feature Tour catalog has no editable CEM-ML example');
    return example;
}

function projectValidation(result, presentation, source, revision) {
    const report = inlineValidateReport(result);
    const terminalDiagnostics = Array.isArray(result?.diagnostics?.items) ? result.diagnostics.items : [];
    const diagnostics = (Array.isArray(report?.diagnostics) ? report.diagnostics : terminalDiagnostics)
        .map((diagnostic) => normalizeDiagnostic(diagnostic));
    const provenance = [];
    diagnostics.forEach((diagnostic, diagnosticIndex) => {
        diagnostic.sourceMap?.frames?.forEach((frame, frameIndex) => {
            provenance.push(normalizeFrame(frame, { diagnosticIndex, frameIndex }));
        });
    });
    for (const reference of result?.sourceMaps?.items ?? []) {
        const stack = reference.sourceMap?.storage === 'inline' ? reference.sourceMap.value : undefined;
        stack?.frames?.forEach((frame, frameIndex) => {
            provenance.push(normalizeFrame(frame, { sourceMapId: reference.sourceMapId, frameIndex }));
        });
    }
    const summary = report?.summary ?? summaryFromDiagnostics(diagnostics);
    return {
        requestId: result?.requestId,
        exitCode: result?.exitCode,
        executionIdentity: result?.identity,
        revision,
        reportSummary: Object.freeze({ ...summary }),
        hardViolationCount: summary.hardViolationCount ?? diagnostics.filter(isHardDiagnostic).length,
        diagnostics: Object.freeze(diagnostics),
        provenance: Object.freeze(provenance),
        presentation,
        sourceByteLength: textEncoder.encode(source).byteLength,
        stale: false,
    };
}

function projectCommandProjection(kind, mode, outcome, source, revision) {
    const result = outcome.result;
    const diagnostics = (result?.diagnostics?.items ?? []).map((diagnostic) => normalizeDiagnostic(diagnostic));
    const provenance = [];
    diagnostics.forEach((diagnostic, diagnosticIndex) => {
        diagnostic.sourceMap?.frames?.forEach((frame, frameIndex) => {
            provenance.push(normalizeFrame(frame, { diagnosticIndex, frameIndex }));
        });
    });
    for (const reference of result?.sourceMaps?.items ?? []) {
        const stack = reference.sourceMap?.storage === 'inline' ? reference.sourceMap.value : undefined;
        stack?.frames?.forEach((frame, frameIndex) => {
            provenance.push(normalizeFrame(frame, { sourceMapId: reference.sourceMapId, frameIndex }));
        });
    }
    return {
        kind,
        mode,
        requestId: result?.requestId,
        exitCode: result?.exitCode,
        executionIdentity: result?.identity,
        revision,
        output: outcome.output,
        nativeResult: result,
        diagnostics: Object.freeze(diagnostics),
        provenance: Object.freeze(provenance),
        presentation: outcome.presentation,
        sourceByteLength: textEncoder.encode(source).byteLength,
        stale: false,
    };
}

function inlineValidateReport(result) {
    const operation = result?.result?.storage === 'inline' ? result.result.value : undefined;
    if (operation?.kind === 'validate') return operation.value?.report;
    return result?.report?.storage === 'inline' ? result.report.value : undefined;
}

function normalizeDiagnostic(diagnostic) {
    const sourceMap = diagnostic?.sourceMap;
    const range = firstStackRange(sourceMap) ?? {
        start: Number.isSafeInteger(diagnostic?.byteOffset) ? diagnostic.byteOffset : 0,
        len: 0,
    };
    return Object.freeze({
        code: diagnostic?.code ?? 'cem.studio.validation.unknown',
        severity: diagnostic?.severity ?? 'error',
        message: diagnostic?.message ?? 'Validation failed',
        uri: diagnostic?.uri,
        line: diagnostic?.line,
        column: diagnostic?.column,
        byteOffset: diagnostic?.byteOffset,
        range: Object.freeze(range),
        sourceMap,
    });
}

function normalizeFrame(frame, owner) {
    return Object.freeze({
        ...owner,
        sourceId: frame?.source_id,
        transform: frame?.transform?.kind ?? 'Unknown',
        range: Object.freeze(firstFrameRange(frame) ?? { start: 0, len: 0 }),
    });
}

function firstStackRange(stack) {
    return firstFrameRange(stack?.frames?.[0]);
}

function firstFrameRange(frame) {
    const span = frame?.span;
    if (!span) return undefined;
    if (span.kind === 'Single') return span.ranges;
    if (span.kind === 'Multi') return span.ranges?.[0];
    return undefined;
}

function summaryFromDiagnostics(diagnostics) {
    const summary = {
        inputCount: 1,
        infoCount: 0,
        warningCount: 0,
        errorCount: 0,
        fatalCount: 0,
        hardViolationCount: 0,
    };
    for (const diagnostic of diagnostics) {
        const key = `${diagnostic.severity}Count`;
        if (key in summary) summary[key] += 1;
        if (isHardDiagnostic(diagnostic)) summary.hardViolationCount += 1;
    }
    return summary;
}

function isHardDiagnostic({ severity }) {
    return severity === 'error' || severity === 'fatal';
}

function freezeState(value) {
    return Object.freeze({ ...value });
}

function freezeValidation(value) {
    return Object.freeze({ ...value });
}

function freezeProjection(value) {
    return Object.freeze({ ...value });
}

function freezeCommand(value) {
    return Object.freeze({
        ...value,
        changes: Object.freeze([...(value.changes ?? [])]),
    });
}

function commandState(preview, persisted) {
    return freezeCommand({
        projection: 'studio',
        status: 'current',
        current: preview,
        draftText: preview.text,
        parsed: preview.parsed,
        preview,
        changes: [],
        diagnostic: undefined,
        copy: undefined,
        revision: commandRevision(persisted),
    });
}

function commandRevision(persisted) {
    return Object.freeze({
        projectRevision: persisted.projectRevision,
        resourceRevision: persisted.resourceRevision,
        sha256: persisted.sha256,
    });
}

function semanticChanges(current, preview) {
    const before = new Map();
    const after = new Map();
    flattenSemanticValue(current, [], before);
    flattenSemanticValue(preview, [], after);
    const paths = [...new Set([...before.keys(), ...after.keys()])].sort();
    return Object.freeze(paths.flatMap((path) => {
        const left = before.get(path);
        const right = after.get(path);
        if (JSON.stringify(left) === JSON.stringify(right)) return [];
        return [Object.freeze({
            category: path.split('.')[0] ?? 'command',
            path,
            kind: left === undefined ? 'added' : right === undefined ? 'removed' : 'changed',
            before: left,
            after: right,
        })];
    }));
}

function flattenSemanticValue(value, path, output) {
    if (Array.isArray(value) || value === null || typeof value !== 'object') {
        output.set(path.join('.'), value);
        return;
    }
    const entries = Object.entries(value);
    if (entries.length === 0) {
        output.set(path.join('.'), value);
        return;
    }
    for (const [key, entry] of entries) flattenSemanticValue(entry, [...path, key], output);
}

function normalizedError(error) {
    return Object.freeze({
        code: error?.code ?? 'cem.studio.workbench.failed',
        message: error instanceof Error ? error.message : String(error),
    });
}

function repositoryRequest(operation, parameters) {
    return {
        protocolVersion: 1,
        repository: CEM_STUDIO_REPOSITORY_ID,
        operation,
        requestRevision: 1,
        parameters,
    };
}

function toBytes(value) {
    if (value instanceof Uint8Array) return value;
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    throw new TypeError('Feature Tour resource bytes are unavailable');
}

function equalBytes(left, right) {
    return left.byteLength === right.byteLength && left.every((byte, index) => byte === right[index]);
}

function utf8ByteOffsetToCodeUnit(value, target) {
    const bounded = Math.max(0, Math.min(target, textEncoder.encode(value).byteLength));
    let bytes = 0;
    let codeUnits = 0;
    for (const character of value) {
        const width = textEncoder.encode(character).byteLength;
        if (bytes + width > bounded) break;
        bytes += width;
        codeUnits += character.length;
    }
    return codeUnits;
}

function workbenchMarkup() {
    return `
        <cem-card label="Feature Tour editor">
            <span slot="title">Feature Tour CEM editor</span>
            <div data-cem-studio-workbench-content>
                <div>
                    <cem-badge data-cem-studio-workbench-status label="Loading" tone="info"></cem-badge>
                    <cem-badge data-cem-studio-workbench-revision label="Revision loading" tone="info"></cem-badge>
                </div>
                <cem-textarea data-cem-studio-editor name="feature-tour-source" label="CEM source">
                    <span slot="help">Changes remain local and are checked against the loaded revision.</span>
                </cem-textarea>
                <div>
                    <cem-action data-cem-studio-save variant="primary">Save and validate</cem-action>
                    <cem-action data-cem-studio-reload variant="quiet">Reload persisted revision</cem-action>
                </div>
                <cem-alert data-cem-studio-workbench-alert label="Load a resource to begin" tone="info"></cem-alert>
            </div>
        </cem-card>
        <section aria-label="CEM-ML projections">
            <cem-select data-cem-studio-parse-projection name="parse-projection" value="ast">
                <span slot="label">Parse projection</span>
                <option value="ast">AST</option>
                <option value="events">Events</option>
            </cem-select>
            <cem-action data-cem-studio-parse variant="secondary">Parse persisted revision</cem-action>
            <cem-select data-cem-studio-inspect-view name="inspect-view" value="summary">
                <span slot="label">Inspect view</span>
                <option value="summary">Summary</option>
                <option value="ast">AST</option>
                <option value="events">Events</option>
                <option value="diagnostics">Diagnostics</option>
                <option value="source-offsets">Source offsets</option>
                <option value="tree">Tree</option>
            </cem-select>
            <cem-action data-cem-studio-inspect variant="secondary">Inspect persisted revision</cem-action>
            <cem-alert data-cem-studio-projection-alert label="Run parse or inspect against the persisted revision" tone="info"></cem-alert>
        </section>
        <section aria-label="CLI Command">
            <cem-tabs label="CLI Command" value="command">
                <cem-tab value="command" label="Studio command">
                    <cem-alert data-cem-studio-command-alert label="Loading Studio command" tone="info"></cem-alert>
                    <cem-textarea data-cem-studio-command-editor name="studio-command" label="Studio command (literal argv)">
                        <span slot="help">Parsed by the shared CEM-ML grammar without shell expansion. Editing does not mutate the project.</span>
                    </cem-textarea>
                    <div>
                        <cem-action data-cem-studio-command-copy variant="secondary">Copy command</cem-action>
                        <cem-action data-cem-studio-command-reset variant="quiet">Reset generated command</cem-action>
                    </div>
                </cem-tab>
                <cem-tab value="changes" label="Semantic changes">
                    <div data-cem-studio-command-changes></div>
                </cem-tab>
            </cem-tabs>
        </section>
        <cem-tabs label="Projection results" value="output">
            <cem-tab value="output" label="CEM-ML output">
                <cem-textarea data-cem-studio-projection-output name="projection-output" label="Target-native CEM-ML output" readonly></cem-textarea>
            </cem-tab>
            <cem-tab value="metadata" label="Execution"><div data-cem-studio-projection-metadata></div></cem-tab>
        </cem-tabs>
        <cem-tabs label="Validation results" value="diagnostics">
            <cem-tab value="diagnostics" label="Diagnostics"><div data-cem-studio-diagnostics></div></cem-tab>
            <cem-tab value="report" label="Report"><div data-cem-studio-report></div></cem-tab>
            <cem-tab value="provenance" label="Provenance"><div data-cem-studio-provenance></div></cem-tab>
        </cem-tabs>`;
}

function renderWorkbench(host, state) {
    const busy = state.status === 'saving' || state.status === 'validating' || state.status === 'projecting';
    const status = host.querySelector('[data-cem-studio-workbench-status]');
    const revision = host.querySelector('[data-cem-studio-workbench-revision]');
    const alert = host.querySelector('[data-cem-studio-workbench-alert]');
    const save = host.querySelector('[data-cem-studio-save]');
    const reload = host.querySelector('[data-cem-studio-reload]');
    const parse = host.querySelector('[data-cem-studio-parse]');
    const inspect = host.querySelector('[data-cem-studio-inspect]');
    const commandAlert = host.querySelector('[data-cem-studio-command-alert]');
    const commandCopy = host.querySelector('[data-cem-studio-command-copy]');
    const commandReset = host.querySelector('[data-cem-studio-command-reset]');
    const projectionAlert = host.querySelector('[data-cem-studio-projection-alert]');
    status.setAttribute('label', statusLabel(state));
    status.setAttribute('tone', statusTone(state.status));
    revision.setAttribute('label', `Project ${state.projectRevision}; resource ${state.resourceRevision}`);
    save.toggleAttribute('disabled', busy || !state.dirty);
    save.toggleAttribute('loading', busy);
    reload.toggleAttribute('disabled', busy);
    parse.toggleAttribute('disabled', busy);
    parse.toggleAttribute('loading', state.status === 'projecting');
    inspect.toggleAttribute('disabled', busy);
    inspect.toggleAttribute('loading', state.status === 'projecting');
    alert.setAttribute('label', alertLabel(state));
    alert.setAttribute('tone', state.error || state.status === 'invalid' || state.status === 'conflict' ? 'danger' : 'info');
    host.querySelector('[data-cem-studio-diagnostics]').innerHTML = diagnosticsMarkup(state.validation?.diagnostics ?? []);
    host.querySelector('[data-cem-studio-report]').innerHTML = reportMarkup(state.validation?.reportSummary);
    host.querySelector('[data-cem-studio-provenance]').innerHTML = provenanceMarkup(state.validation?.provenance ?? []);
    projectionAlert.setAttribute('label', projectionAlertLabel(state));
    projectionAlert.setAttribute(
        'tone',
        state.status === 'projection-invalid' || state.error ? 'danger' : state.projection?.stale ? 'warning' : 'info',
    );
    host.querySelector('[data-cem-studio-projection-metadata]').innerHTML = projectionMetadataMarkup(state.projection);
    commandAlert.setAttribute('label', commandAlertLabel(state.command));
    commandAlert.setAttribute('tone', commandAlertTone(state.command));
    commandCopy.toggleAttribute('disabled', !state.command || state.command.status === 'checking');
    commandCopy.toggleAttribute('loading', state.command?.copy?.status === 'copying');
    commandReset.toggleAttribute(
        'disabled',
        !state.command || state.command.status === 'current' || state.command.status === 'checking',
    );
    host.querySelector('[data-cem-studio-command-changes]').innerHTML = commandChangesMarkup(state.command);
}

function statusLabel(state) {
    if (state.status === 'stale') return 'Validation result stale';
    return {
        loading: 'Loading resource',
        loaded: 'Persisted revision loaded',
        dirty: 'Unsaved changes',
        saving: 'Saving revision',
        saved: 'Revision saved',
        validating: 'Validating saved revision',
        projecting: 'Projecting saved revision',
        projected: 'Saved revision projected',
        'projection-invalid': 'Projection has diagnostics',
        'projection-stale': 'Projection result stale',
        valid: 'Saved revision valid',
        invalid: 'Saved revision has diagnostics',
        stale: 'Validation result stale',
        conflict: 'Revision conflict',
        failed: 'Workbench failed',
    }[state.status] ?? state.status;
}

function statusTone(status) {
    if (status === 'valid' || status === 'saved' || status === 'loaded' || status === 'projected') return 'success';
    if (status === 'invalid' || status === 'failed' || status === 'conflict' || status === 'projection-invalid') return 'danger';
    if (status === 'dirty' || status === 'stale' || status === 'projection-stale') return 'warning';
    return 'info';
}

function alertLabel(state) {
    if (state.error) return `${state.error.code}: ${state.error.message}`;
    if (state.validation?.stale) return 'This result belongs to an older draft or durable revision.';
    if (state.validation) {
        return `${state.validation.diagnostics.length} diagnostics; ${state.validation.hardViolationCount} hard violations.`;
    }
    return `${state.path}; ${state.contentType}`;
}

function projectionAlertLabel(state) {
    if (!state.projection) return 'Run parse or inspect against the persisted revision.';
    const freshness = state.projection.stale ? 'stale' : 'current';
    return `${state.projection.kind} ${state.projection.mode}; ${state.projection.output.byteLength} CEM-ML bytes; ${freshness} revision.`;
}

function projectionMetadataMarkup(projection) {
    if (!projection) return '<cem-alert label="No parse or inspect result yet" tone="info"></cem-alert>';
    const rows = [
        ['Operation', projection.kind],
        ['Projection', projection.mode],
        ['Runtime', projection.executionIdentity?.runtime ?? 'unknown'],
        ['Project revision', projection.revision.projectRevision],
        ['Resource revision', projection.revision.resourceRevision],
        ['Output content type', projection.output.contentType],
        ['Output bytes', projection.output.byteLength],
        ['Diagnostics', projection.diagnostics.length],
        ['Source-map frames', projection.provenance.length],
        ['Freshness', projection.stale ? 'stale' : 'current'],
    ];
    return `<cem-table label="Projection execution"><div role="row"><strong role="columnheader">Measure</strong><strong role="columnheader">Value</strong></div>${rows.map(([label, value]) => `<div role="row"><span role="cell">${escapeHtml(label)}</span><span role="cell">${escapeHtml(value)}</span></div>`).join('')}</cem-table>`;
}

function commandAlertLabel(command) {
    if (!command) return 'Loading Studio command.';
    if (command.copy?.message) return command.copy.message;
    if (command.status === 'checking') return 'Checking command with the shared CEM-ML grammar.';
    if (command.status === 'invalid') {
        return `${command.diagnostic?.code ?? 'cem.command.invalid'}: ${command.diagnostic?.message ?? 'Command is invalid.'}`;
    }
    if (command.status === 'changed') {
        return `${command.changes.length} semantic changes; project records remain unchanged.`;
    }
    return `Generated Studio command for project ${command.revision.projectRevision}, resource ${command.revision.resourceRevision}; CEM-ML ${command.current.commonVersion}.`;
}

function commandAlertTone(command) {
    if (command?.copy?.status === 'failed' || command?.status === 'invalid') return 'danger';
    if (command?.status === 'changed') return 'warning';
    if (command?.copy?.status === 'success' || command?.status === 'current') return 'success';
    return 'info';
}

function commandChangesMarkup(command) {
    if (!command) return '<cem-alert label="Command preview is loading" tone="info"></cem-alert>';
    if (command.status === 'invalid') {
        return `<cem-alert label="${escapeHtml(commandAlertLabel(command))}" tone="danger"></cem-alert>`;
    }
    if (command.changes.length === 0) {
        return '<cem-alert label="The command matches the active normalized run plan" tone="success"></cem-alert>';
    }
    return `<cem-table label="CLI Command semantic changes"><div role="row"><strong role="columnheader">Area</strong><strong role="columnheader">Path</strong><strong role="columnheader">Change</strong><strong role="columnheader">Current</strong><strong role="columnheader">Draft</strong></div>${command.changes.map((change) => `<div role="row"><span role="cell">${escapeHtml(change.category)}</span><span role="cell">${escapeHtml(change.path)}</span><span role="cell">${escapeHtml(change.kind)}</span><span role="cell">${escapeHtml(commandChangeValue(change.before))}</span><span role="cell">${escapeHtml(commandChangeValue(change.after))}</span></div>`).join('')}</cem-table>`;
}

function commandChangeValue(value) {
    if (value === undefined) return '—';
    const text = typeof value === 'string' ? value : JSON.stringify(value);
    return text.length <= 240 ? text : `${text.slice(0, 239)}…`;
}

function diagnosticsMarkup(diagnostics) {
    if (diagnostics.length === 0) return '<cem-alert label="No diagnostics" tone="success"></cem-alert>';
    return `<cem-list data-diagnostic-list selectable label="Diagnostics" size="${Math.min(8, diagnostics.length)}">${diagnostics.map((diagnostic, index) => `
        <cem-list-option value="${index}">${escapeHtml(`${diagnostic.severity}: ${diagnostic.code}: ${diagnostic.message}`)}</cem-list-option>`).join('')}</cem-list>`;
}

function reportMarkup(summary) {
    if (!summary) return '<cem-alert label="No validation report yet" tone="info"></cem-alert>';
    const rows = [
        ['Inputs', summary.inputCount],
        ['Information', summary.infoCount],
        ['Warnings', summary.warningCount],
        ['Errors', summary.errorCount],
        ['Fatal', summary.fatalCount],
        ['Hard violations', summary.hardViolationCount],
    ];
    return `<cem-table label="Validation report"><div role="row"><strong role="columnheader">Measure</strong><strong role="columnheader">Count</strong></div>${rows.map(([label, value]) => `<div role="row"><span role="cell">${label}</span><span role="cell">${value ?? 0}</span></div>`).join('')}</cem-table>`;
}

function provenanceMarkup(provenance) {
    if (provenance.length === 0) return '<cem-alert label="No source-map provenance" tone="info"></cem-alert>';
    return `<cem-list data-provenance-list selectable label="Source-map provenance" size="${Math.min(8, provenance.length)}">${provenance.map((frame, index) => `
        <cem-list-option value="${index}">${escapeHtml(`Source ${frame.sourceId ?? 'unknown'}: ${frame.transform}; bytes ${frame.range.start}–${frame.range.start + frame.range.len}`)}</cem-list-option>`).join('')}</cem-list>`;
}

function escapeHtml(value) {
    return String(value)
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&#39;');
}

async function settleWorkbench(runtime, root) {
    await Promise.resolve();
    const instances = [...root.querySelectorAll(
        'cem-card, cem-badge, cem-textarea, cem-select, cem-action, cem-alert, cem-tabs, cem-list, cem-table',
    )];
    await Promise.all(instances.map((instance) => runtime.whenRenderSettled(instance)));
    await Promise.resolve();
}
