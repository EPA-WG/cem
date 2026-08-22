import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { installCemStudioShellComponents } from './shell.js';

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

/**
 * Own one editable Feature Tour resource and its durable validation revision.
 * @param {{repository: object, validator: object, seed: object, projectId: string, example?: object}} options
 */
export async function createCemStudioFeatureTourWorkbench(options) {
    const { repository, validator, seed, projectId } = options;
    if (!repository?.query || !repository?.execute) throw new TypeError('Feature Tour workbench requires a repository');
    if (!validator?.validateResource) throw new TypeError('Feature Tour workbench requires a browser validator');
    if (typeof projectId !== 'string' || projectId.length === 0) {
        throw new TypeError('Feature Tour workbench requires a project id');
    }
    const example = options.example ?? featureTourCemExample(seed);
    const subscribers = new Set();
    let editVersion = 0;
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
        const persisted = await readPersisted();
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
            selection: undefined,
            error: undefined,
        });
        return validateRevision({ ...persisted, editVersion: captured.editVersion, draftMatches: true }, options.signal);
    }

    async function validatePersisted(options = {}) {
        const persisted = await readPersisted();
        const capturedVersion = editVersion;
        publish({
            ...state,
            status: 'validating',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
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
        navigateDiagnostic,
        navigateProvenance,
        dispose() {
            subscribers.clear();
        },
    });
}

/** Mount the workbench with production CEM controls only. */
export async function mountCemStudioFeatureTourWorkbench({ root, workbench }) {
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
    let renderPromise = Promise.resolve();
    let actionPromise = Promise.resolve();

    const render = (snapshot) => {
        renderPromise = renderPromise.then(async () => {
            renderWorkbench(host, snapshot);
            await settleWorkbench(components.runtime, host);
            const editor = host.querySelector('cem-textarea[data-cem-studio-editor] textarea');
            if (editor && editor.value !== snapshot.draft) editor.value = snapshot.draft;
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
        <cem-tabs label="Validation results" value="diagnostics">
            <cem-tab value="diagnostics" label="Diagnostics"><div data-cem-studio-diagnostics></div></cem-tab>
            <cem-tab value="report" label="Report"><div data-cem-studio-report></div></cem-tab>
            <cem-tab value="provenance" label="Provenance"><div data-cem-studio-provenance></div></cem-tab>
        </cem-tabs>`;
}

function renderWorkbench(host, state) {
    const busy = state.status === 'saving' || state.status === 'validating';
    const status = host.querySelector('[data-cem-studio-workbench-status]');
    const revision = host.querySelector('[data-cem-studio-workbench-revision]');
    const alert = host.querySelector('[data-cem-studio-workbench-alert]');
    const save = host.querySelector('[data-cem-studio-save]');
    const reload = host.querySelector('[data-cem-studio-reload]');
    status.setAttribute('label', statusLabel(state));
    status.setAttribute('tone', statusTone(state.status));
    revision.setAttribute('label', `Project ${state.projectRevision}; resource ${state.resourceRevision}`);
    save.toggleAttribute('disabled', busy || !state.dirty);
    save.toggleAttribute('loading', busy);
    reload.toggleAttribute('disabled', busy);
    alert.setAttribute('label', alertLabel(state));
    alert.setAttribute('tone', state.error || state.status === 'invalid' || state.status === 'conflict' ? 'danger' : 'info');
    host.querySelector('[data-cem-studio-diagnostics]').innerHTML = diagnosticsMarkup(state.validation?.diagnostics ?? []);
    host.querySelector('[data-cem-studio-report]').innerHTML = reportMarkup(state.validation?.reportSummary);
    host.querySelector('[data-cem-studio-provenance]').innerHTML = provenanceMarkup(state.validation?.provenance ?? []);
}

function statusLabel(state) {
    if (state.validation?.stale) return 'Validation result stale';
    return {
        loading: 'Loading resource',
        loaded: 'Persisted revision loaded',
        dirty: 'Unsaved changes',
        saving: 'Saving revision',
        saved: 'Revision saved',
        validating: 'Validating saved revision',
        valid: 'Saved revision valid',
        invalid: 'Saved revision has diagnostics',
        stale: 'Validation result stale',
        conflict: 'Revision conflict',
        failed: 'Workbench failed',
    }[state.status] ?? state.status;
}

function statusTone(status) {
    if (status === 'valid' || status === 'saved' || status === 'loaded') return 'success';
    if (status === 'invalid' || status === 'failed' || status === 'conflict') return 'danger';
    if (status === 'dirty' || status === 'stale') return 'warning';
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
        'cem-card, cem-badge, cem-textarea, cem-action, cem-alert, cem-tabs, cem-list, cem-table',
    )];
    await Promise.all(instances.map((instance) => runtime.whenRenderSettled(instance)));
    await Promise.resolve();
}
