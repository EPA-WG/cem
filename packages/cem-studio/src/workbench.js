import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { CEM_STUDIO_LIMITS, createCemStudioPreview, redactCemStudioSecrets } from './preview.js';
import { installCemStudioShellComponents } from './shell.js';
import { mountCemView } from '@epa-wg/cem-elements';

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
        !validator?.validateResource ||
        !validator?.parseResource ||
        !validator?.inspectResource ||
        !validator?.previewResourceCommand ||
        !validator?.serializeResourceCommand ||
        !validator?.executeAuthoredResourceCommand
    ) {
        throw new TypeError('Feature Tour workbench requires a browser command validator');
    }
    if (typeof projectId !== 'string' || projectId.length === 0) {
        throw new TypeError('Feature Tour workbench requires a project id');
    }
    const initialExample = options.example ?? featureTourCemExample(seed);
    const workbenches = featureTourWorkbenches(seed, initialExample);
    if (
        workbenches.some((entry) => !['parse', 'inspect'].includes(workbenchOperation(entry))) &&
        !validator?.runResourceCommand
    ) {
        throw new TypeError('Portable operation workbenches require a browser command runner');
    }
    let [example] = workbenches;
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
        workbenchId: example.id,
        workbenches: Object.freeze(workbenches.map(workbenchDescriptor)),
        operation: workbenchOperation(example),
        expected: undefined,
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
        const currentEntries = bundle.project.entries.filter(
            ({ runConfigResourceId, resourceIds }) =>
                runConfigResourceId === example.runConfigResourceId ||
                (!example.runConfigResourceId && resourceIds?.includes(example.resourceId)),
        );
        if (currentEntries.length > 1) {
            throw new Error(`Feature Tour resource ${example.resourceId} resolves to more than one current page`);
        }
        const bytes = toBytes(bundle.contents[example.resourceId]);
        const dependencies = example.dependencies.map((dependency) => {
            const dependencyResource = bundle.project.resources.find(({ id }) => id === dependency.resourceId);
            if (!dependencyResource) throw new Error(`Feature Tour dependency ${dependency.resourceId} is unavailable`);
            return {
                bytes: toBytes(bundle.contents[dependency.resourceId]),
                contentType: dependencyResource.contentType,
                schema: dependencyResource.schema,
                path: dependency.path,
                resourceId: dependency.resourceId,
            };
        });
        const expected = freezeWorkbenchValue(
            example.expectedSummary ??
                (example.expectedResourceId
                    ? JSON.parse(textDecoder.decode(toBytes(bundle.contents[example.expectedResourceId])))
                    : undefined),
        );
        return {
            bytes,
            text: textDecoder.decode(bytes),
            dependencies,
            projectRevision: bundle.project.revision,
            resourceRevision: resource.revision,
            repositoryRevision: response.repositoryRevision,
            sha256: resource.sha256,
            project: bundle.project,
            contents: bundle.contents,
            currentEntry: currentEntries[0],
            expected,
            operation: workbenchOperation(example),
            pageKind: example.kind ?? 'inspection',
            referencedResourceIds: Object.freeze([
                example.resourceId,
                ...example.dependencies.map(({ resourceId }) => resourceId),
            ]),
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
            workbenchId: example.id,
            operation: persisted.operation,
            expected: persisted.expected,
        });
    }

    async function selectWorkbench(workbenchId) {
        const selected = workbenches.find(({ id }) => id === workbenchId);
        if (!selected) throw new RangeError(`Feature Tour workbench ${String(workbenchId)} is unavailable`);
        example = selected;
        commandEditVersion += 1;
        editVersion += 1;
        publish({
            ...state,
            status: 'loading',
            workbenchId: selected.id,
            resourceId: selected.resourceId,
            path: selected.path,
            contentType: selected.contentType,
            schema: selected.schema,
            operation: workbenchOperation(selected),
            validation: undefined,
            projection: undefined,
            command: undefined,
            selection: undefined,
            expected: undefined,
            error: undefined,
        });
        return reload();
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
            saved = await repository.execute(
                repositoryRequest('save-resource', {
                    projectId,
                    resourceId: example.resourceId,
                    expectedProjectRevision: captured.projectRevision,
                    expectedResourceRevision: captured.resourceRevision,
                    content: captured.draft,
                }),
                options.signal,
            );
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
            !equalBytes(expectedBytes, persisted.bytes) ||
            persisted.sha256 !== saved.value.sha256 ||
            persisted.projectRevision !== saved.value.projectRevision ||
            persisted.resourceRevision !== saved.value.resourceRevision
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
        return validateRevision(
            { ...persisted, editVersion: captured.editVersion, draftMatches: true },
            options.signal,
        );
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
        return validateRevision(
            {
                ...persisted,
                editVersion: capturedVersion,
                draftMatches: state.draft === persisted.text,
            },
            options.signal,
        );
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
        const stale =
            editVersion !== persisted.editVersion ||
            persisted.draftMatches === false ||
            state.projectRevision !== persisted.projectRevision ||
            state.resourceRevision !== persisted.resourceRevision ||
            Boolean(outcome.result.stale);
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

    async function runPersistedOperation(options = {}) {
        const operation = workbenchOperation(example);
        if (operation === 'parse') return parsePersisted('ast', options);
        if (operation === 'inspect') return inspectPersisted('summary', options);
        const persisted = await readPersisted();
        const capturedVersion = editVersion;
        const draftMatches = state.draft === persisted.text;
        const argv = commandArgumentsForExample(example, projectId, 'studio');
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
            outcome = await validator.runResourceCommand({
                ...commandResourceOptions(persisted),
                argv,
                signal: options.signal,
            });
        } catch (error) {
            if (!error?.result || !error?.output) {
                publish({ ...state, status: 'failed', error: normalizedError(error) });
                throw error;
            }
            outcome = { result: error.result, presentation: error.presentation, output: error.output };
        }
        const mode = commandProjectionMode(undefined, operation, example.kind);
        const projected = projectCommandProjection(
            operation,
            mode,
            outcome,
            persisted.text,
            {
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                sha256: persisted.sha256,
            },
            persisted.expected,
        );
        const stale =
            editVersion !== capturedVersion ||
            !draftMatches ||
            state.projectRevision !== persisted.projectRevision ||
            state.resourceRevision !== persisted.resourceRevision ||
            Boolean(outcome.result.stale);
        const projection = freezeProjection({ ...projected, stale });
        commandEditVersion += 1;
        const command = await commandForProjection(persisted, operation, mode);
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
        const projected = projectCommandProjection(
            kind,
            mode,
            outcome,
            persisted.text,
            {
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                sha256: persisted.sha256,
            },
            persisted.expected,
        );
        const stale =
            editVersion !== capturedVersion ||
            !draftMatches ||
            state.projectRevision !== persisted.projectRevision ||
            state.resourceRevision !== persisted.resourceRevision ||
            Boolean(outcome.result.stale);
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
        publish({
            ...state,
            command: freezeCommand({
                ...state.command,
                status: 'checking',
                draftText: text,
                diagnostic: undefined,
                copy: undefined,
                application: commandApplicationAfterEdit(state.command.application),
            }),
            projection:
                state.command.application?.execution && state.projection
                    ? freezeProjection({ ...state.projection, stale: true })
                    : state.projection,
        });
        try {
            const persisted = await readPersisted();
            if (capturedVersion !== commandEditVersion) return state;
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
                    application: commandApplicationState(persisted, state.command.application, preview.parsed),
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

    async function copyProjection(writeText) {
        if (!state.projection?.output) throw new Error('CEM Studio has no operation output to copy');
        if (state.projection.preview?.kind === 'download') {
            throw new Error('Binary, invalid-text, and oversized active results must be downloaded as exact bytes.');
        }
        try {
            if (typeof writeText !== 'function') {
                const error = new Error('Clipboard writing is unavailable; select and copy the result text instead.');
                error.code = 'cem.studio.projection.clipboard_unavailable';
                throw error;
            }
            await writeText(state.projection.output.text);
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'copied', message: 'Exact operation output copied.' }),
                }),
            });
        } catch (error) {
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'failed', message: normalizedError(error).message }),
                }),
            });
        }
        return state;
    }

    async function downloadProjection(save) {
        if (!state.projection?.output) throw new Error('CEM Studio has no operation output to download');
        const extension = state.projection.output.contentType?.includes('json') ? 'json' : 'cem';
        const filename = `${state.projection.kind}-${String(state.projection.mode).replace(/[^a-z0-9-]+/gi, '-')}.${extension}`;
        try {
            if (typeof save !== 'function') throw new Error('Result download is unavailable in this browser.');
            await save({
                filename,
                contentType: state.projection.output.contentType,
                bytes: new Uint8Array(state.projection.output.bytes),
            });
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'downloaded', message: `Downloaded ${filename}.` }),
                }),
            });
        } catch (error) {
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'failed', message: normalizedError(error).message }),
                }),
            });
        }
        return state;
    }

    function setCommandTarget(target) {
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        const application = configureCommandApplication(state.command.application, target);
        publish({
            ...state,
            command: freezeCommand({ ...state.command, application }),
            error: undefined,
        });
        return state;
    }

    async function applyCommand(options = {}) {
        return applyCommandPage(false, options);
    }

    async function applyAndRun(options = {}) {
        return applyCommandPage(true, options);
    }

    async function confirmCommandReplacement(options = {}) {
        const confirmation = state.command?.application?.confirmation;
        if (!confirmation) throw new Error('No incompatible command-page replacement is awaiting confirmation');
        return applyCommandPage(confirmation.runAfterApply, options, {
            ...confirmation.target,
            confirmIncompatibleReplacement: true,
        });
    }

    function useNewCommandTarget() {
        const application = state.command?.application;
        if (!application) throw new Error('CEM Studio command target is unavailable');
        return setCommandTarget({
            mode: 'new',
            name: application.newPageName,
            ...(application.newPageParentId ? { parentId: application.newPageParentId } : {}),
        });
    }

    function cancelCommandReplacement() {
        const command = state.command;
        if (!command?.application) return state;
        publish({
            ...state,
            command: freezeCommand({
                ...command,
                application: freezeCommandApplication({
                    ...command.application,
                    status: 'ready',
                    confirmation: undefined,
                    error: undefined,
                }),
            }),
        });
        return state;
    }

    async function applyCommandPage(runAfterApply, options, confirmedTarget) {
        const command = state.command;
        if (!command?.preview?.parsed || command.status === 'invalid' || command.status === 'checking') {
            throw new Error('A valid checked Studio command is required before Apply');
        }
        const capturedVersion = commandEditVersion;
        const application = command.application;
        const target = confirmedTarget ?? application.target;
        const commandResource = validator.serializeResourceCommand(command.preview.parsed);
        const commandBytes = textEncoder.encode(commandResource);
        publish({
            ...state,
            command: freezeCommand({
                ...command,
                application: freezeCommandApplication({
                    ...application,
                    status: 'applying',
                    confirmation: undefined,
                    error: undefined,
                }),
            }),
            error: undefined,
        });

        let applied;
        try {
            applied = await repository.execute(
                repositoryRequest('apply-command-page', {
                    projectId,
                    expectedProjectRevision: command.revision.projectRevision,
                    target,
                    commandResource,
                    referencedResourceIds: application.referencedResourceIds,
                }),
                options.signal,
            );
        } catch (error) {
            if (error?.code === 'cem.studio.repository.command_target_incompatible') {
                publish({
                    ...state,
                    command: freezeCommand({
                        ...state.command,
                        application: freezeCommandApplication({
                            ...state.command.application,
                            status: 'confirmation-required',
                            confirmation: Object.freeze({ target, runAfterApply }),
                            error: normalizedError(error),
                        }),
                    }),
                });
                return state;
            }
            publishCommandApplicationFailure(error);
            throw error;
        }

        const persisted = await readPersisted();
        try {
            assertAppliedCommandCommit(persisted, applied.value, commandBytes);
        } catch (error) {
            publishCommandApplicationFailure(error);
            throw error;
        }
        const refreshedCommand = await commandForPersisted(persisted);
        const changedDuringApply = capturedVersion !== commandEditVersion;
        const selectedApplication = commandApplicationState(persisted, {
            ...refreshedCommand.application,
            currentEntryId: applied.value.entry.id,
            target: { mode: 'current', entryId: applied.value.entry.id },
            result: freezeAppliedCommandResult(applied.value),
        });
        const appliedApplication = freezeCommandApplication({
            ...selectedApplication,
            status: changedDuringApply ? 'stale' : 'applied',
            confirmation: undefined,
            error: undefined,
        });
        publish({
            ...state,
            status: state.dirty ? 'dirty' : state.validation ? 'stale' : 'loaded',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            command: freezeCommand({ ...refreshedCommand, application: appliedApplication }),
            validation: state.validation ? freezeValidation({ ...state.validation, stale: true }) : undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            error: undefined,
        });
        if (!runAfterApply) return state;
        return runAppliedCommand(persisted, applied.value, commandBytes, capturedVersion, options.signal);
    }

    async function runAppliedCommand(persisted, applied, commandBytes, capturedVersion, signal) {
        publish({
            ...state,
            command: freezeCommand({
                ...state.command,
                application: freezeCommandApplication({
                    ...state.command.application,
                    status: 'running',
                }),
            }),
        });
        let outcome;
        try {
            outcome = await validator.executeAuthoredResourceCommand({
                ...commandResourceOptions(persisted),
                commandResource: commandBytes,
                signal,
            });
        } catch (error) {
            if (!error?.result || !error?.output) {
                publishCommandApplicationFailure(error);
                throw error;
            }
            outcome = {
                result: error.result,
                presentation: error.presentation,
                output: error.output,
                parsed: error.parsed ?? state.command.preview?.parsed,
            };
        }
        const latest = await readPersisted();
        const stale =
            capturedVersion !== commandEditVersion ||
            !isAppliedCommandCommit(latest, applied, commandBytes) ||
            Boolean(outcome.result?.stale);
        const operation = outcome.parsed?.commandPath?.[0] ?? applied.operation;
        const mode = commandProjectionMode(outcome.parsed, operation);
        const projection = freezeProjection({
            ...projectCommandProjection(
                operation,
                mode,
                outcome,
                persisted.text,
                {
                    projectRevision: applied.projectRevision,
                    resourceRevision: applied.resourceRevision,
                    sha256: applied.sha256,
                },
                persisted.expected,
            ),
            stale,
        });
        publish({
            ...state,
            status: stale ? 'projection-stale' : projection.exitCode === 0 ? 'projected' : 'projection-invalid',
            projection,
            command: freezeCommand({
                ...state.command,
                application: freezeCommandApplication({
                    ...state.command.application,
                    status: stale ? 'run-stale' : projection.exitCode === 0 ? 'ran' : 'run-invalid',
                    execution: Object.freeze({
                        requestId: projection.requestId,
                        exitCode: projection.exitCode,
                        projectRevision: applied.projectRevision,
                        resourceRevision: applied.resourceRevision,
                        sha256: applied.sha256,
                        stale,
                    }),
                }),
            }),
            error: undefined,
        });
        return state;
    }

    function publishCommandApplicationFailure(error) {
        const command = state.command;
        publish({
            ...state,
            command: command
                ? freezeCommand({
                      ...command,
                      application: freezeCommandApplication({
                          ...command.application,
                          status: error?.code === 'cem.studio.repository.revision_conflict' ? 'conflict' : 'failed',
                          confirmation: undefined,
                          error: normalizedError(error),
                      }),
                  })
                : command,
            error: normalizedError(error),
        });
    }

    async function commandForPersisted(persisted) {
        if (state.command?.current?.text) {
            const current = await validator.previewResourceCommand({
                ...commandResourceOptions(persisted),
                text: state.command.current.text,
            });
            if (state.command.draftText === state.command.current.text) {
                return commandState(current, persisted, state.command.application);
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
                    application: commandApplicationState(persisted, state.command.application, preview.parsed),
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
                    application: commandApplicationState(persisted, state.command.application),
                });
            }
        }
        const operation = persisted.operation ?? 'parse';
        return commandForProjection(
            persisted,
            operation,
            operation === 'parse' ? 'ast' : operation === 'inspect' ? 'summary' : example.kind,
        );
    }

    async function commandForProjection(persisted, kind, mode) {
        const preview = await validator.previewResourceCommand({
            ...commandResourceOptions(persisted),
            operation: kind,
            ...(kind === 'parse' || kind === 'inspect'
                ? {}
                : { argv: commandArgumentsForExample(example, projectId, 'studio') }),
            [kind === 'parse' ? 'projection' : 'view']: mode,
        });
        return commandState(preview, persisted, state.command?.application);
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
            ...(example.commandArguments ? { argv: commandArgumentsForExample(example, projectId, 'studio') } : {}),
        };
    }

    function navigateDiagnostic(index) {
        const diagnostic = (state.projection?.diagnostics ?? state.validation?.diagnostics)?.[index];
        if (!diagnostic) throw new RangeError(`diagnostic ${index} is unavailable`);
        return selectRange('diagnostic', index, diagnostic.range);
    }

    function navigateProvenance(index) {
        const frame = (state.projection?.provenance ?? state.validation?.provenance)?.[index];
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
        selectWorkbench,
        saveAndValidate,
        validatePersisted,
        parsePersisted,
        inspectPersisted,
        runPersistedOperation,
        updateCommandDraft,
        resetCommandDraft,
        copyCommand,
        copyProjection,
        downloadProjection,
        setCommandTarget,
        applyCommand,
        applyAndRun,
        confirmCommandReplacement,
        useNewCommandTarget,
        cancelCommandReplacement,
        navigateDiagnostic,
        navigateProvenance,
        dispose() {
            subscribers.clear();
        },
    });
}

/** Mount the workbench with production CEM controls only. */
export async function mountCemStudioFeatureTourWorkbench({
    root,
    workbench,
    clipboard = navigator.clipboard,
    download = browserDownload,
}) {
    if (!(root instanceof Element)) throw new TypeError('Feature Tour workbench root must be an Element');
    const components = await installCemStudioShellComponents();
    // This empty container is only a mount boundary; the declaration owns UI.
    const mount = document.createElement('div');
    (root.querySelector('[data-cem-studio-shell]') ?? root).append(mount);
    const layout = mountCemView(components.runtime, { root: mount, tag: 'cem-studio-workbench-layout' });
    await layout.whenSettled();
    const host = layout.element;

    let renderPromise = Promise.resolve();
    let controlPromise = Promise.resolve();
    let editPromise = Promise.resolve();
    let pending = false;
    let active = true;
    let selection;
    let selectionRequest = 0;
    const perform = (operation) => {
        if (pending || !active || workbenchBusy(workbench.snapshot())) return;
        pending = true;
        controls.update({ pending });
        commandView.update({ pending });
        results.update({ pending });
        // Start on the command event stack; the service owns durable state.
        controlPromise = (async () => {
            try { await operation(); }
            catch { /* Workbench services publish their structured failure. */ }
            finally {
                pending = false;
                controls.update({ pending });
                commandView.update({ pending });
                results.update({ pending });
            }
        })();
        return controlPromise;
    };
    const controls = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-controls-mount]'),
        tag: 'cem-studio-workbench-controls',
        state: { state: workbenchControlState(workbench.snapshot()), selectionRequest: '', pending },
        commands: {
            edit: ({ value }) => workbench.updateDraft(value ?? ''),
            save: () => perform(() => workbench.saveAndValidate()),
            reload: () => perform(() => workbench.reload()),
            parse: ({ fields }) => perform(() => workbench.parsePersisted(fields.mode[0])),
            inspect: ({ fields }) => perform(() => workbench.inspectPersisted(fields.mode[0])),
            run: () => perform(() => workbench.runPersistedOperation()),
            'select-workbench': ({ value }) => perform(() => workbench.selectWorkbench(value)),
        },
    });

    const newTarget = (values = {}) => {
        const application = workbench.snapshot().command?.application;
        workbench.setCommandTarget({ mode: 'new', name: application?.newPageName ?? '',
            ...(application?.newPageParentId ? { parentId: application.newPageParentId } : {}), ...values });
    };
    const dismissReplacement = () => {
        if (workbench.snapshot().command?.application?.status === 'confirmation-required') {
            workbench.cancelCommandReplacement();
        }
    };
    const commandView = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-command-mount]'),
        tag: 'cem-studio-workbench-command',
        state: { command: workbenchCommandState(workbench.snapshot().command), pending, busy: workbenchBusy(workbench.snapshot()) },
        commands: {
            edit: ({ value }) => {
                const edit = workbench.updateCommandDraft(value ?? '').catch(() => undefined);
                editPromise = Promise.all([editPromise, edit]).then(() => undefined);
                return edit;
            },
            copy: () => perform(() => workbench.copyCommand(typeof clipboard?.writeText === 'function'
                ? text => clipboard.writeText(text) : undefined)),
            reset: () => perform(() => workbench.resetCommandDraft()),
            'target-name': ({ value }) => newTarget({ name: value ?? '' }),
            'target-parent': ({ value }) => newTarget({ parentId: value || undefined }),
            'target-existing': ({ value }) => workbench.setCommandTarget({ mode: 'existing', entryId: value }),
            'target-mode': ({ value }) => {
                if (value === 'current') workbench.setCommandTarget({ mode: 'current' });
                else if (value === 'existing') {
                    const targets = workbench.snapshot().command?.application?.targets.existing ?? [];
                    const entry = targets.find(({ compatible }) => compatible) ?? targets[0];
                    if (entry) workbench.setCommandTarget({ mode: 'existing', entryId: entry.id });
                } else if (value === 'new') newTarget();
            },
            apply: () => perform(() => workbench.applyCommand()),
            'apply-run': () => perform(() => workbench.applyAndRun()),
            'confirm-replacement': () => perform(() => workbench.confirmCommandReplacement()),
            'use-new': () => workbench.useNewCommandTarget(),
            'cancel-replacement': dismissReplacement,
            'dismiss-replacement': dismissReplacement,
        },
    });
    const results = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-results-mount]'),
        tag: 'cem-studio-workbench-results',
        state: { state: workbenchResultState(workbench.snapshot()), pending },
        commands: {
            'copy-output': () => perform(() => workbench.copyProjection(typeof clipboard?.writeText === 'function'
                ? text => clipboard.writeText(text) : undefined)),
            'download-output': () => perform(() => workbench.downloadProjection(download)),
            diagnostic: ({ value }) => workbench.navigateDiagnostic(Number(value)),
            provenance: ({ value }) => workbench.navigateProvenance(Number(value)),
        },
    });
    let latestSnapshot;
    const render = (snapshot) => {
        latestSnapshot = snapshot;
        renderPromise = renderPromise.then(async () => {
            if (!active) return;
            const current = latestSnapshot;
            if (current.selection && current.selection !== selection) selectionRequest++;
            selection = current.selection;
            controls.update({ state: workbenchControlState(current), selectionRequest: selection ? String(selectionRequest) : '', pending });
            commandView.update({ command: workbenchCommandState(current.command), pending, busy: workbenchBusy(current) });
            results.update({ state: workbenchResultState(current), pending });
            await Promise.all([controls.whenSettled(), commandView.whenSettled(), results.whenSettled()]);
        });
    };
    const unsubscribe = workbench.subscribe(render);
    await renderPromise;

    return Object.freeze({
        root: host,
        workbench,
        async whenSettled() {
            while (true) {
                const controlAction = controlPromise;
                const edits = editPromise;
                await controlAction;
                await edits;
                const renders = renderPromise;
                await renders;
                await Promise.all([controls.whenSettled(), commandView.whenSettled(), results.whenSettled()]);
                if (controlAction === controlPromise && edits === editPromise && renders === renderPromise) return;
            }
        },
        dispose() {
            active = false;
            unsubscribe();
            controls.dispose();
            commandView.dispose();
            results.dispose();
            layout.dispose();
            mount.remove();
        },
    });
}

function featureTourCemExample(seed) {
    const examples = seed?.catalog?.examples;
    if (!Array.isArray(examples)) throw new Error('Feature Tour catalog has no examples');
    const example = examples.find(
        ({ packageId, contentType }) =>
            packageId === 'cem-ml' && typeof contentType === 'string' && contentType.includes('cem'),
    );
    if (!example) throw new Error('Feature Tour catalog has no editable CEM-ML example');
    return example;
}

function featureTourWorkbenches(seed, initialExample) {
    const scenarios = Array.isArray(seed?.catalog?.workbenches) ? seed.catalog.workbenches : [];
    const values = [initialExample, ...scenarios.filter(({ id }) => id !== initialExample.id)];
    return Object.freeze(
        values.map((entry, index) =>
            Object.freeze({
                ...entry,
                id: entry.id ?? (index === 0 ? 'source-editor' : `workbench-${index + 1}`),
                name:
                    entry.name ?? (index === 0 ? 'CEM-ML source editor' : humanizeKind(entry.kind ?? entry.operation)),
                dependencies: Object.freeze([...(entry.dependencies ?? [])]),
                commandArguments: entry.commandArguments ? Object.freeze([...entry.commandArguments]) : undefined,
            }),
        ),
    );
}

function workbenchDescriptor(example) {
    return Object.freeze({
        id: example.id,
        name: example.name ?? humanizeKind(example.kind ?? 'inspection'),
        operation: workbenchOperation(example),
        kind: example.kind ?? 'inspection',
        contentType: example.contentType,
        schema: example.schema,
    });
}

function workbenchOperation(example) {
    return example.kind && ['convert', 'query', 'transform', 'trace'].includes(example.operation)
        ? example.operation
        : 'parse';
}

function commandArgumentsForExample(example, projectId, scheme) {
    if (!Array.isArray(example.commandArguments)) return undefined;
    const inputUri = resourceUri(example.path, projectId, scheme);
    const dependencies = new Map(
        (example.dependencies ?? []).map((dependency) => [
            dependency.resourceId,
            resolveVirtualUri(inputUri, dependency.path),
        ]),
    );
    return Object.freeze(
        example.commandArguments.map((argument) => {
            if (argument === '$input') return inputUri;
            if (argument.startsWith('$resource:')) {
                const resourceId = argument.slice('$resource:'.length);
                const uri = dependencies.get(resourceId);
                if (!uri) throw new Error(`Feature Tour command references unavailable resource ${resourceId}`);
                return uri;
            }
            return argument;
        }),
    );
}

function resourceUri(uri, projectId, scheme = 'studio') {
    return /^[a-z][a-z0-9+.-]*:/i.test(uri) ? uri : new URL(uri, `${scheme}://${projectId}/`).href;
}

function resolveVirtualUri(baseUri, relativePath) {
    return new URL(relativePath, baseUri).href;
}

function projectValidation(result, presentation, source, revision) {
    const report = inlineValidateReport(result);
    const terminalDiagnostics = Array.isArray(result?.diagnostics?.items) ? result.diagnostics.items : [];
    const diagnostics = (Array.isArray(report?.diagnostics) ? report.diagnostics : terminalDiagnostics)
        .slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map((diagnostic) => normalizeDiagnostic(diagnostic));
    const provenance = [];
    diagnostics.forEach((diagnostic, diagnosticIndex) => {
        diagnostic.sourceMap?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { diagnosticIndex, frameIndex }));
            }
        });
    });
    for (const reference of result?.sourceMaps?.items ?? []) {
        const stack = reference.sourceMap?.storage === 'inline' ? reference.sourceMap.value : undefined;
        stack?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { sourceMapId: reference.sourceMapId, frameIndex }));
            }
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

function projectCommandProjection(kind, mode, outcome, source, revision, expected) {
    const result = outcome.result;
    const diagnostics = (result?.diagnostics?.items ?? [])
        .slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map((diagnostic) => normalizeDiagnostic(diagnostic));
    const provenance = [];
    diagnostics.forEach((diagnostic, diagnosticIndex) => {
        diagnostic.sourceMap?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { diagnosticIndex, frameIndex }));
            }
        });
    });
    for (const reference of result?.sourceMaps?.items ?? []) {
        const stack = reference.sourceMap?.storage === 'inline' ? reference.sourceMap.value : undefined;
        stack?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { sourceMapId: reference.sourceMapId, frameIndex }));
            }
        });
    }
    const summary = summarizePortableOperation(kind, mode, result);
    const expectedMatches = expected === undefined ? undefined : matchesExpectedSummary(summary, expected);
    const operation = result?.result?.storage === 'inline' ? result.result.value : undefined;
    const trace =
        kind === 'trace' && Array.isArray(operation?.value?.body?.events)
            ? operation.value.body.events.slice(0, CEM_STUDIO_LIMITS.structuredRows)
                .map((value, index) => Object.freeze({ path: `value.body.events.${index}`, value }))
            : collectOperationRecords(operation, ['trace', 'stages', 'events']);
    const graph =
        kind === 'transform' && mode === 'graph'
            ? collectOperationRecords(operation, ['artifacts', 'stages', 'nodes', 'edges'])
            : [];
    return {
        kind,
        mode,
        requestId: result?.requestId,
        exitCode: result?.exitCode,
        executionIdentity: result?.identity,
        revision,
        output: outcome.output,
        preview: createCemStudioPreview({
            bytes: outcome.output.bytes,
            contentType: outcome.output.contentType,
            label: `${humanizeKind(kind)} result preview`,
        }),
        nativeResult: result,
        diagnostics: Object.freeze(diagnostics),
        provenance: Object.freeze(provenance),
        presentation: outcome.presentation,
        sourceByteLength: textEncoder.encode(source).byteLength,
        summary: Object.freeze(summary),
        expected: expected === undefined ? undefined : Object.freeze({ ...expected }),
        expectedMatches,
        trace: Object.freeze(trace),
        graph: Object.freeze(graph),
        stale: false,
    };
}

function summarizePortableOperation(kind, mode, result) {
    const operation = result?.result?.storage === 'inline' ? result.result.value : undefined;
    const value = operation?.value;
    if (kind === 'convert') {
        return { kind, outputCount: value?.outputs?.items?.length ?? 0 };
    }
    if (kind === 'query') {
        return { kind, itemCount: sequenceItems(value?.output).length };
    }
    if (kind === 'transform') {
        const outputs = value?.value?.outputs?.items ?? [];
        const artifacts = value?.value?.artifacts ?? [];
        const primary = mode === 'graph' ? artifacts[0]?.primary : outputs[0]?.response?.primary;
        return {
            kind,
            mode,
            ...(mode === 'graph' ? { artifactCount: artifacts.length } : {}),
            itemCount: sequenceItems(primary).length,
        };
    }
    if (kind === 'trace') {
        return { kind, hasEvents: (value?.body?.events?.length ?? 0) > 0 };
    }
    return { kind, mode };
}

function sequenceItems(value) {
    return value?.result?.sequence?.items ?? value?.sequence?.items ?? [];
}

function matchesExpectedSummary(actual, expected) {
    return Object.entries(expected).every(([key, value]) => JSON.stringify(actual[key]) === JSON.stringify(value));
}

function collectOperationRecords(value, keys) {
    const records = [];
    const visit = (candidate, path = []) => {
        if (!candidate || typeof candidate !== 'object' || records.length >= CEM_STUDIO_LIMITS.structuredRows) return;
        for (const [key, child] of Object.entries(candidate)) {
            if (keys.includes(key) && Array.isArray(child)) {
                for (const [index, entry] of child.entries()) {
                    if (records.length >= CEM_STUDIO_LIMITS.structuredRows) break;
                    records.push(
                        Object.freeze({
                            path: [...path, key, index].join('.'),
                            value: entry,
                        }),
                    );
                }
            } else if (path.length < 8) {
                visit(child, [...path, key]);
            }
        }
    };
    visit(value);
    return records;
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

function freezeWorkbenchValue(value) {
    if (Array.isArray(value)) return Object.freeze(value.map((entry) => freezeWorkbenchValue(entry)));
    if (value && typeof value === 'object') {
        return Object.freeze(
            Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, freezeWorkbenchValue(entry)])),
        );
    }
    return value;
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

function freezeCommandApplication(value) {
    return Object.freeze({
        ...value,
        target: value.target ? Object.freeze({ ...value.target }) : undefined,
        referencedResourceIds: Object.freeze([...(value.referencedResourceIds ?? [])]),
        targets: value.targets
            ? Object.freeze({
                  current: value.targets.current ? Object.freeze({ ...value.targets.current }) : undefined,
                  existing: Object.freeze(value.targets.existing.map((entry) => Object.freeze({ ...entry }))),
                  parents: Object.freeze(value.targets.parents.map((entry) => Object.freeze({ ...entry }))),
              })
            : undefined,
    });
}

function commandApplicationState(persisted, previous, parsed) {
    const pageKind = commandPageKindFromParsed(parsed, persisted.pageKind);
    const entries = persisted.project.entries.filter(({ kind }) => kind !== 'subproject');
    const preferredCurrentId = previous?.currentEntryId ?? persisted.currentEntry?.id;
    const currentEntry = entries.find(({ id }) => id === preferredCurrentId) ?? persisted.currentEntry;
    const describe = (entry) =>
        Object.freeze({
            id: entry.id,
            name: entry.name,
            kind: entry.kind,
            parentId: entry.parentId,
            compatible: entry.kind === pageKind,
        });
    const current = currentEntry ? describe(currentEntry) : undefined;
    const existing = entries
        .map(describe)
        .sort(
            (left, right) =>
                Number(right.compatible) - Number(left.compatible) ||
                left.name.localeCompare(right.name) ||
                left.id.localeCompare(right.id),
        );
    const parents = persisted.project.entries
        .filter(({ kind }) => kind === 'subproject')
        .map(({ id, name, parentId }) => Object.freeze({ id, name, parentId }))
        .sort((left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id));
    const defaultName = `${current?.name ?? 'Command'} ${humanizeKind(pageKind)}`;
    const newPageName = previous?.newPageName ?? defaultName;
    const newPageParentId = previous?.newPageParentId ?? current?.parentId;
    const defaultExisting = existing.find(({ compatible }) => compatible) ?? existing[0];
    const defaultMode = current?.compatible ? 'current' : 'new';
    let target = previous?.target;
    if (
        (target?.mode === 'current' && !current) ||
        (target?.mode === 'existing' && !existing.some(({ id }) => id === target.entryId)) ||
        !['current', 'existing', 'new'].includes(target?.mode)
    ) {
        target = undefined;
    }
    target ??=
        defaultMode === 'current'
            ? { mode: 'current', entryId: current.id }
            : { mode: 'new', name: newPageName, ...(newPageParentId ? { parentId: newPageParentId } : {}) };
    if (target.mode === 'current') target = { mode: 'current', entryId: current.id };
    if (target.mode === 'existing' && !target.entryId && defaultExisting) {
        target = { mode: 'existing', entryId: defaultExisting.id };
    }
    if (target.mode === 'new') {
        target = {
            mode: 'new',
            name: target.name ?? newPageName,
            ...((target.parentId ?? newPageParentId) ? { parentId: target.parentId ?? newPageParentId } : {}),
        };
    }
    const resultCurrent =
        previous?.result &&
        previous.result.projectRevision === persisted.projectRevision &&
        persisted.project.resources.some(
            ({ id, revision, sha256 }) =>
                id === previous.result.commandResource.id &&
                revision === previous.result.resourceRevision &&
                sha256 === previous.result.sha256,
        );
    return freezeCommandApplication({
        status: resultCurrent ? previous.status : previous?.result ? 'stale' : 'ready',
        currentEntryId: current?.id,
        newPageName,
        newPageParentId,
        referencedResourceIds: persisted.referencedResourceIds,
        targets: { current, existing, parents },
        target,
        result: previous?.result,
        execution: previous?.execution,
        confirmation: resultCurrent ? previous?.confirmation : undefined,
        error: resultCurrent ? previous?.error : undefined,
    });
}

function configureCommandApplication(application, target) {
    if (!application || !target || typeof target !== 'object') {
        throw new TypeError('CEM Studio command target must be an object');
    }
    let normalized;
    let newPageName = application.newPageName;
    let newPageParentId = application.newPageParentId;
    if (target.mode === 'current') {
        if (!application.targets.current) throw new Error('The workbench has no current page target');
        normalized = { mode: 'current', entryId: application.targets.current.id };
    } else if (target.mode === 'existing') {
        const entry = application.targets.existing.find(({ id }) => id === target.entryId);
        if (!entry) throw new Error(`Existing command-page target ${String(target.entryId)} is unavailable`);
        normalized = { mode: 'existing', entryId: entry.id };
    } else if (target.mode === 'new') {
        if (typeof target.name !== 'string') throw new TypeError('A new command page requires a name');
        newPageName = target.name;
        newPageParentId = target.parentId || undefined;
        normalized = {
            mode: 'new',
            name: newPageName,
            ...(newPageParentId ? { parentId: newPageParentId } : {}),
        };
    } else {
        throw new TypeError(`Unsupported CEM Studio command target mode: ${String(target.mode)}`);
    }
    return freezeCommandApplication({
        ...application,
        status: application.result ? 'stale' : 'ready',
        target: normalized,
        newPageName,
        newPageParentId,
        confirmation: undefined,
        error: undefined,
    });
}

function commandApplicationAfterEdit(application) {
    return freezeCommandApplication({
        ...application,
        status: application.result ? 'stale' : 'ready',
        confirmation: undefined,
        error: undefined,
    });
}

function freezeAppliedCommandResult(value) {
    return Object.freeze({
        ...value,
        entry: Object.freeze({ ...value.entry }),
        commandResource: Object.freeze({ ...value.commandResource }),
        commandBytes: value.commandBytes.slice(0),
    });
}

function isAppliedCommandCommit(persisted, applied, expectedBytes) {
    const resource = persisted.project.resources.find(({ id }) => id === applied.commandResource.id);
    const entry = persisted.project.entries.find(({ id }) => id === applied.entry.id);
    const content = persisted.contents[applied.commandResource.id];
    return (
        persisted.projectRevision === applied.projectRevision &&
        resource?.revision === applied.resourceRevision &&
        resource?.sha256 === applied.sha256 &&
        entry?.runConfigResourceId === applied.commandResource.id &&
        content !== undefined &&
        equalBytes(toBytes(content), expectedBytes) &&
        equalBytes(toBytes(applied.commandBytes), expectedBytes)
    );
}

function assertAppliedCommandCommit(persisted, applied, expectedBytes) {
    if (isAppliedCommandCommit(persisted, applied, expectedBytes)) return;
    const error = new Error('CEM Studio repository did not reload the exact applied command revision');
    error.code = 'cem.studio.workbench.apply_persistence_mismatch';
    throw error;
}

function commandProjectionMode(parsed, operation, fallback) {
    if (operation === 'parse') return parsed?.options?.format ?? 'ast';
    if (operation === 'inspect') return parsed?.options?.show ?? 'summary';
    if (operation === 'convert')
        return parsed?.options?.to_format ?? parsed?.options?.to_content_type ?? fallback ?? 'convert';
    if (operation === 'query') return parsed?.options?.query_content_type ?? fallback ?? 'query';
    if (operation === 'transform')
        return parsed?.options?.config ? 'graph' : fallback === 'transformation-graph' ? 'graph' : 'direct';
    if (operation === 'trace') return parsed?.options?.format ?? fallback ?? 'trace';
    return fallback ?? 'unknown';
}

function humanizeKind(value) {
    return String(value)
        .split('-')
        .map((part) => part[0]?.toUpperCase() + part.slice(1))
        .join(' ');
}

function commandPageKindFromParsed(parsed, fallback = 'inspection') {
    const operation = parsed?.commandPath?.[0];
    if (operation === 'parse' || operation === 'inspect') return 'inspection';
    if (operation === 'convert') return 'conversion';
    if (operation === 'query') return 'query';
    if (operation === 'trace') return 'trace';
    if (operation === 'transform') return parsed?.options?.config ? 'transformation-graph' : 'transformation';
    return fallback;
}

function commandState(preview, persisted, previousApplication) {
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
        application: commandApplicationState(persisted, previousApplication, preview.parsed),
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
    return Object.freeze(
        paths.flatMap((path) => {
            const left = before.get(path);
            const right = after.get(path);
            if (JSON.stringify(left) === JSON.stringify(right)) return [];
            return [
                Object.freeze({
                    category: path.split('.')[0] ?? 'command',
                    path,
                    kind: left === undefined ? 'added' : right === undefined ? 'removed' : 'changed',
                    before: left,
                    after: right,
                }),
            ];
        }),
    );
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
        message: redactCemStudioSecrets(error instanceof Error ? error.message : String(error)),
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

function workbenchBusy(state) {
    return ['saving', 'validating', 'projecting'].includes(state.status)
        || ['applying', 'running'].includes(state.command?.application?.status);
}

// Forward service data used by this view, excluding potentially large result
// bytes, graphs and command previews. Presentation remains in the declaration.
function workbenchControlState(state) {
    const { status, workbenchId, workbenches, draft, dirty, projectRevision,
        resourceRevision, selection, error, path, contentType, validation } = state;
    return { status, workbenchId, workbenches, draft, dirty, projectRevision,
        resourceRevision, selection, error, path, contentType, validation,
        command: { application: { status: state.command?.application?.status } } };
}

function workbenchCommandState(command) {
    if (!command) return null;
    const { status, draftText, copy, diagnostic, revision, current, changes, application } = command;
    const { result, execution: _execution, ...targeting } = application ?? {};
    const text = value => typeof value === 'string' ? value : JSON.stringify(value);
    return { status, draftText, copy, diagnostic, revision, current: { commonVersion: current.commonVersion },
        changes: changes.map(({ category, path, kind, before, after }) => ({ category, path, kind,
            beforeText: text(before), afterText: text(after) })),
        application: application ? { ...targeting, result: result && {
            projectRevision: result.projectRevision, resourceRevision: result.resourceRevision } } : null };
}

// Transport only the data this declaration consumes. Preserve exact downloadable
// bytes in the service; serialize typed record values without presentation labels.
function workbenchResultState(state) {
    const { status, error, operation, path, contentType, schema, expected, validation, command, projection } = state;
    const pinned = projection?.expected ?? expected;
    const actual = projection?.summary;
    const expectedRows = pinned ? [...new Set([...Object.keys(pinned), ...Object.keys(actual ?? {})])].sort()
        .map(key => ({ key, expectedText: JSON.stringify(pinned[key]), actualText: JSON.stringify(actual?.[key]) })) : [];
    const records = values => (values ?? []).slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map(({ path, value }) => ({ path, text: typeof value === 'string' ? value : JSON.stringify(value) }));
    return { status, error, operation, path, contentType, schema, expected, expectedRows,
        validation: validation && { reportSummary: validation.reportSummary, diagnostics: validation.diagnostics,
            provenance: validation.provenance },
        command: { current: { argv: command?.current?.argv }, application: { status: command?.application?.status } },
        projection: projection && { kind: projection.kind, mode: projection.mode, stale: projection.stale,
            expectedMatches: projection.expectedMatches, executionIdentity: projection.executionIdentity,
            revision: projection.revision, diagnostics: projection.diagnostics, provenance: projection.provenance,
            preview: projection.preview, expected: projection.expected, summary: projection.summary,
            transfer: projection.transfer, output: projection.output && {
                contentType: projection.output.contentType, byteLength: projection.output.byteLength },
            trace: records(projection.trace), graph: records(projection.graph) } };
}

async function browserDownload({ filename, contentType, bytes }) {
    const url = URL.createObjectURL(new Blob([bytes], { type: contentType }));
    try {
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        link.hidden = true;
        document.body.append(link);
        link.click();
        link.remove();
    } finally {
        URL.revokeObjectURL(url);
    }
}
