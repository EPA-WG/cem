import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { CEM_STUDIO_LIMITS, createCemStudioPreview, mountCemStudioPreview, redactCemStudioSecrets } from './preview.js';
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
                application: commandApplicationAfterEdit(state.command.application),
            }),
            projection:
                state.command.application?.execution && state.projection
                    ? freezeProjection({ ...state.projection, stale: true })
                    : state.projection,
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
    const host = document.createElement('section');
    host.setAttribute('data-cem-studio-workbench', '');
    host.setAttribute('aria-label', 'Feature Tour workbench');
    host.innerHTML = workbenchMarkup(workbench.snapshot());
    const shell = root.querySelector('[data-cem-studio-shell]');
    if (shell) shell.append(host);
    else root.append(host);
    await settleWorkbench(components.runtime, host);

    const editorHost = host.querySelector('cem-textarea[data-cem-studio-editor]');
    const saveAction = host.querySelector('cem-action[data-cem-studio-save]');
    const reloadAction = host.querySelector('cem-action[data-cem-studio-reload]');
    const parseAction = host.querySelector('cem-action[data-cem-studio-parse]');
    const inspectAction = host.querySelector('cem-action[data-cem-studio-inspect]');
    const runAction = host.querySelector('cem-action[data-cem-studio-operation-run]');
    const workbenchSelect = host.querySelector('cem-select[data-cem-studio-workbench-select]');
    const commandEditorHost = host.querySelector('cem-textarea[data-cem-studio-command-editor]');
    const commandCopyAction = host.querySelector('cem-action[data-cem-studio-command-copy]');
    const commandResetAction = host.querySelector('cem-action[data-cem-studio-command-reset]');
    const projectionCopyAction = host.querySelector('cem-action[data-cem-studio-projection-copy]');
    const projectionDownloadAction = host.querySelector('cem-action[data-cem-studio-projection-download]');
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
            const commandEditor = host.querySelector('cem-textarea[data-cem-studio-command-editor] textarea');
            if (commandEditor && commandEditor.value !== (snapshot.command?.draftText ?? '')) {
                commandEditor.value = snapshot.command?.draftText ?? '';
            }
            const commandTargetName = host.querySelector('cem-text-field[data-cem-studio-command-target-name] input');
            if (commandTargetName && commandTargetName.value !== (snapshot.command?.application?.newPageName ?? '')) {
                commandTargetName.value = snapshot.command?.application?.newPageName ?? '';
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
    const runOperation = () => {
        actionPromise = workbench.runPersistedOperation().catch(() => undefined);
    };
    const selectWorkbench = () => {
        actionPromise = workbench.selectWorkbench(workbenchSelect.value).catch(() => undefined);
    };
    const commandEdited = (event) => {
        if (event.target instanceof HTMLTextAreaElement) {
            actionPromise = workbench.updateCommandDraft(event.target.value).catch(() => undefined);
        }
    };
    const copyCommand = () => {
        const writeText = typeof clipboard?.writeText === 'function' ? (text) => clipboard.writeText(text) : undefined;
        actionPromise = workbench.copyCommand(writeText).catch(() => undefined);
    };
    const resetCommand = () => {
        actionPromise = workbench.resetCommandDraft().catch(() => undefined);
    };
    const copyProjection = () => {
        const writeText = typeof clipboard?.writeText === 'function' ? (text) => clipboard.writeText(text) : undefined;
        actionPromise = workbench.copyProjection(writeText).catch(() => undefined);
    };
    const downloadProjection = () => {
        actionPromise = workbench.downloadProjection(download).catch(() => undefined);
    };
    const commandApplicationInput = (event) => {
        const field =
            event.target instanceof Element
                ? event.target.closest('cem-text-field[data-cem-studio-command-target-name]')
                : null;
        if (!field || !(event.target instanceof HTMLInputElement)) return;
        const application = workbench.snapshot().command?.application;
        workbench.setCommandTarget({
            mode: 'new',
            name: event.target.value,
            ...(application?.newPageParentId ? { parentId: application.newPageParentId } : {}),
        });
    };
    const commandApplicationChange = (event) => {
        if (!(event.target instanceof Element)) return;
        const select = event.target.closest('cem-select');
        if (!select) return;
        const application = workbench.snapshot().command?.application;
        if (select.hasAttribute('data-cem-studio-command-target-mode')) {
            if (select.value === 'current') workbench.setCommandTarget({ mode: 'current' });
            else if (select.value === 'existing') {
                const entry =
                    application?.targets.existing.find(({ compatible }) => compatible) ??
                    application?.targets.existing[0];
                if (entry) workbench.setCommandTarget({ mode: 'existing', entryId: entry.id });
            } else {
                workbench.setCommandTarget({
                    mode: 'new',
                    name: application?.newPageName ?? '',
                    ...(application?.newPageParentId ? { parentId: application.newPageParentId } : {}),
                });
            }
        } else if (select.hasAttribute('data-cem-studio-command-target-existing')) {
            workbench.setCommandTarget({ mode: 'existing', entryId: select.value });
        } else if (select.hasAttribute('data-cem-studio-command-target-parent')) {
            workbench.setCommandTarget({
                mode: 'new',
                name: application?.newPageName ?? '',
                ...(select.value ? { parentId: select.value } : {}),
            });
        }
    };
    const commandApplicationAction = (event) => {
        if (!(event.target instanceof Element)) return;
        const action = event.target.closest('cem-action');
        if (!action) return;
        if (action.hasAttribute('data-cem-studio-command-apply')) {
            actionPromise = workbench.applyCommand().catch(() => undefined);
        } else if (action.hasAttribute('data-cem-studio-command-apply-run')) {
            actionPromise = workbench.applyAndRun().catch(() => undefined);
        } else if (action.hasAttribute('data-cem-studio-command-replace-confirm')) {
            actionPromise = workbench.confirmCommandReplacement().catch(() => undefined);
        } else if (action.hasAttribute('data-cem-studio-command-use-new')) {
            workbench.useNewCommandTarget();
        } else if (action.hasAttribute('data-cem-studio-command-replace-cancel')) {
            workbench.cancelCommandReplacement();
        }
    };
    const commandDialogDismissed = (event) => {
        if (event.target instanceof Element && event.target.matches('[data-cem-studio-command-replace-dialog]')) {
            workbench.cancelCommandReplacement();
        }
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
    runAction.addEventListener('click', runOperation);
    workbenchSelect.addEventListener('change', selectWorkbench);
    commandEditorHost.addEventListener('input', commandEdited);
    commandCopyAction.addEventListener('click', copyCommand);
    commandResetAction.addEventListener('click', resetCommand);
    projectionCopyAction.addEventListener('click', copyProjection);
    projectionDownloadAction.addEventListener('click', downloadProjection);
    host.addEventListener('input', commandApplicationInput);
    host.addEventListener('change', commandApplicationChange);
    host.addEventListener('click', commandApplicationAction);
    host.addEventListener('cem-dismiss', commandDialogDismissed);
    host.addEventListener('change', navigate);
    await renderPromise;

    return Object.freeze({
        root: host,
        workbench,
        async whenSettled() {
            while (true) {
                const actions = actionPromise;
                await actions;
                const renders = renderPromise;
                await renders;
                if (actions === actionPromise && renders === renderPromise) return;
            }
        },
        dispose() {
            unsubscribe();
            editorHost.removeEventListener('input', edited);
            saveAction.removeEventListener('click', save);
            reloadAction.removeEventListener('click', reload);
            parseAction.removeEventListener('click', parse);
            inspectAction.removeEventListener('click', inspect);
            runAction.removeEventListener('click', runOperation);
            workbenchSelect.removeEventListener('change', selectWorkbench);
            commandEditorHost.removeEventListener('input', commandEdited);
            commandCopyAction.removeEventListener('click', copyCommand);
            commandResetAction.removeEventListener('click', resetCommand);
            projectionCopyAction.removeEventListener('click', copyProjection);
            projectionDownloadAction.removeEventListener('click', downloadProjection);
            host.removeEventListener('input', commandApplicationInput);
            host.removeEventListener('change', commandApplicationChange);
            host.removeEventListener('click', commandApplicationAction);
            host.removeEventListener('cem-dismiss', commandDialogDismissed);
            host.removeEventListener('change', navigate);
            host.remove();
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
            ? operation.value.body.events
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

function workbenchMarkup(state) {
    return `
        <cem-select data-cem-studio-workbench-select name="operation-workbench" value="${escapeHtml(state.workbenchId)}">
            <span slot="label">Workbench</span>
            ${state.workbenches.map((workbench) => `<option value="${escapeHtml(workbench.id)}">${escapeHtml(`${workbench.name} — ${workbench.operation}`)}</option>`).join('')}
        </cem-select>
        <cem-card label="Feature Tour editor">
            <span slot="title">Feature Tour operation workbenches</span>
            <div data-cem-studio-workbench-content>
                <div role="status" aria-live="polite" aria-atomic="true">
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
            <cem-action data-cem-studio-operation-run variant="primary">Run configured workbench</cem-action>
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
            <cem-alert data-cem-studio-projection-alert label="Run the configured operation against the persisted revision" tone="info"></cem-alert>
            <div data-cem-studio-operation-details></div>
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
                    <section data-cem-studio-command-application aria-label="Apply Studio command"></section>
                </cem-tab>
                <cem-tab value="changes" label="Semantic changes">
                    <div data-cem-studio-command-changes></div>
                </cem-tab>
            </cem-tabs>
        </section>
        <cem-tabs label="Projection results" value="output">
            <cem-tab value="output" label="Operation output">
                <section data-cem-studio-preview aria-label="Result preview">
                    <cem-alert label="Run an operation to preview its bounded result" tone="info"></cem-alert>
                </section>
                <div>
                    <cem-action data-cem-studio-projection-copy variant="secondary">Copy exact output</cem-action>
                    <cem-action data-cem-studio-projection-download variant="quiet">Download exact output</cem-action>
                </div>
                <cem-alert data-cem-studio-projection-transfer label="No output transfer requested" tone="info"></cem-alert>
            </cem-tab>
            <cem-tab value="metadata" label="Execution"><div data-cem-studio-projection-metadata></div></cem-tab>
            <cem-tab value="expected" label="Expected"><div data-cem-studio-projection-expected></div></cem-tab>
            <cem-tab value="trace" label="Trace"><div data-cem-studio-projection-trace></div></cem-tab>
            <cem-tab value="graph" label="Graph"><div data-cem-studio-projection-graph></div></cem-tab>
        </cem-tabs>
        <cem-tabs label="Validation results" value="diagnostics">
            <cem-tab value="diagnostics" label="Diagnostics"><div data-cem-studio-diagnostics></div></cem-tab>
            <cem-tab value="report" label="Report"><div data-cem-studio-report></div></cem-tab>
            <cem-tab value="provenance" label="Provenance"><div data-cem-studio-provenance></div></cem-tab>
        </cem-tabs>`;
}

function renderWorkbench(host, state) {
    const commandBusy =
        state.command?.application?.status === 'applying' || state.command?.application?.status === 'running';
    const busy =
        state.status === 'saving' || state.status === 'validating' || state.status === 'projecting' || commandBusy;
    const status = host.querySelector('[data-cem-studio-workbench-status]');
    const revision = host.querySelector('[data-cem-studio-workbench-revision]');
    const alert = host.querySelector('[data-cem-studio-workbench-alert]');
    const save = host.querySelector('[data-cem-studio-save]');
    const reload = host.querySelector('[data-cem-studio-reload]');
    const parse = host.querySelector('[data-cem-studio-parse]');
    const inspect = host.querySelector('[data-cem-studio-inspect]');
    const runOperation = host.querySelector('[data-cem-studio-operation-run]');
    const workbenchSelect = host.querySelector('[data-cem-studio-workbench-select]');
    const commandAlert = host.querySelector('[data-cem-studio-command-alert]');
    const commandCopy = host.querySelector('[data-cem-studio-command-copy]');
    const commandReset = host.querySelector('[data-cem-studio-command-reset]');
    const projectionAlert = host.querySelector('[data-cem-studio-projection-alert]');
    const projectionCopy = host.querySelector('[data-cem-studio-projection-copy]');
    const projectionDownload = host.querySelector('[data-cem-studio-projection-download]');
    const projectionTransfer = host.querySelector('[data-cem-studio-projection-transfer]');
    workbenchSelect.value = state.workbenchId;
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
    runOperation.toggleAttribute('disabled', busy);
    runOperation.toggleAttribute('loading', state.status === 'projecting');
    alert.setAttribute('label', alertLabel(state));
    alert.setAttribute(
        'tone',
        state.error || state.status === 'invalid' || state.status === 'conflict' ? 'danger' : 'info',
    );
    host.querySelector('[data-cem-studio-diagnostics]').innerHTML = diagnosticsMarkup(
        state.projection?.diagnostics ?? state.validation?.diagnostics ?? [],
    );
    host.querySelector('[data-cem-studio-report]').innerHTML = reportMarkup(state.validation?.reportSummary);
    host.querySelector('[data-cem-studio-provenance]').innerHTML = provenanceMarkup(
        state.projection?.provenance ?? state.validation?.provenance ?? [],
    );
    projectionAlert.setAttribute('label', projectionAlertLabel(state));
    projectionAlert.setAttribute(
        'tone',
        state.status === 'projection-invalid' || state.error || state.projection?.expectedMatches === false
            ? 'danger'
            : state.projection?.stale
              ? 'warning'
              : 'info',
    );
    host.querySelector('[data-cem-studio-projection-metadata]').innerHTML = projectionMetadataMarkup(state.projection);
    host.querySelector('[data-cem-studio-operation-details]').innerHTML = operationDetailsMarkup(state);
    host.querySelector('[data-cem-studio-projection-expected]').innerHTML = projectionExpectedMarkup(
        state.projection,
        state.expected,
    );
    host.querySelector('[data-cem-studio-projection-trace]').innerHTML = operationRecordsMarkup(
        'Operation trace',
        state.projection?.trace ?? [],
    );
    host.querySelector('[data-cem-studio-projection-graph]').innerHTML = operationRecordsMarkup(
        'Transformation graph execution overlay',
        state.projection?.graph ?? [],
    );
    const previewRoot = host.querySelector('[data-cem-studio-preview]');
    if (state.projection?.preview) {
        mountCemStudioPreview(previewRoot, state.projection.preview);
    } else {
        previewRoot.innerHTML =
            '<cem-alert label="Run an operation to preview its bounded result" tone="info"></cem-alert>';
    }
    const hasOutput = Boolean(state.projection?.output);
    const copyableOutput = hasOutput && state.projection?.preview?.kind !== 'download';
    projectionCopy.toggleAttribute('disabled', !copyableOutput || busy);
    projectionDownload.toggleAttribute('disabled', !hasOutput || busy);
    projectionTransfer.setAttribute(
        'label',
        state.projection?.transfer?.message ?? 'Copy or download the exact verified operation output.',
    );
    projectionTransfer.setAttribute('tone', state.projection?.transfer?.status === 'failed' ? 'danger' : 'info');
    commandAlert.setAttribute('label', commandAlertLabel(state.command));
    commandAlert.setAttribute('tone', commandAlertTone(state.command));
    commandCopy.toggleAttribute('disabled', !state.command || state.command.status === 'checking');
    commandCopy.toggleAttribute('loading', state.command?.copy?.status === 'copying');
    commandReset.toggleAttribute(
        'disabled',
        !state.command || state.command.status === 'current' || state.command.status === 'checking',
    );
    host.querySelector('[data-cem-studio-command-changes]').innerHTML = commandChangesMarkup(state.command);
    renderCommandApplication(host.querySelector('[data-cem-studio-command-application]'), state.command);
}

function statusLabel(state) {
    if (state.status === 'stale') return 'Validation result stale';
    return (
        {
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
        }[state.status] ?? state.status
    );
}

function statusTone(status) {
    if (status === 'valid' || status === 'saved' || status === 'loaded' || status === 'projected') return 'success';
    if (status === 'invalid' || status === 'failed' || status === 'conflict' || status === 'projection-invalid')
        return 'danger';
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
    if (!state.projection) return `Run ${state.operation} against the persisted revision.`;
    const freshness = state.projection.stale ? 'stale' : 'current';
    const expected =
        state.projection.expectedMatches === undefined
            ? ''
            : state.projection.expectedMatches
              ? '; pinned expectation matched'
              : '; pinned expectation differed';
    return `${state.projection.kind} ${state.projection.mode}; ${state.projection.output.byteLength} output bytes; ${freshness} revision${expected}.`;
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
        [
            'Expected result',
            projection.expectedMatches === undefined
                ? 'not pinned'
                : projection.expectedMatches
                  ? 'matched'
                  : 'different',
        ],
        ['Freshness', projection.stale ? 'stale' : 'current'],
    ];
    return `<cem-table label="Projection execution"><div role="row"><strong role="columnheader">Measure</strong><strong role="columnheader">Value</strong></div>${rows.map(([label, value]) => `<div role="row"><span role="cell">${escapeHtml(label)}</span><span role="cell">${escapeHtml(value)}</span></div>`).join('')}</cem-table>`;
}

function operationDetailsMarkup(state) {
    const rows = [
        ['Operation', state.operation],
        ['Input path', state.path],
        ['Input content type', state.contentType],
        ['Input schema', state.schema],
        ['Literal argv', state.command?.current?.argv?.join(' ') ?? 'loading'],
        ['Pinned expectation', state.expected?.kind ?? 'none'],
    ];
    return `<cem-table label="Operation identities and configuration"><div role="row"><strong role="columnheader">Field</strong><strong role="columnheader">Value</strong></div>${rows.map(([label, value]) => `<div role="row"><span role="cell">${escapeHtml(label)}</span><span role="cell">${escapeHtml(value)}</span></div>`).join('')}</cem-table>`;
}

function projectionExpectedMarkup(projection, expected) {
    const pinned = projection?.expected ?? expected;
    if (!pinned) return '<cem-alert label="No pinned expected result for this workbench" tone="info"></cem-alert>';
    const actual = projection?.summary;
    const status =
        projection?.expectedMatches === undefined
            ? 'Run the operation to compare its native summary.'
            : projection.expectedMatches
              ? 'Native result matches the pinned expectation.'
              : 'Native result differs from the pinned expectation.';
    const rows = [...new Set([...Object.keys(pinned), ...Object.keys(actual ?? {})])]
        .sort()
        .map((key) => [
            key,
            pinned[key] === undefined ? '—' : JSON.stringify(pinned[key]),
            actual?.[key] === undefined ? '—' : JSON.stringify(actual[key]),
        ]);
    return `<cem-alert label="${escapeHtml(status)}" tone="${projection?.expectedMatches === false ? 'danger' : projection?.expectedMatches ? 'success' : 'info'}"></cem-alert><cem-table label="Pinned expected result"><div role="row"><strong role="columnheader">Measure</strong><strong role="columnheader">Expected</strong><strong role="columnheader">Actual</strong></div>${rows.map(([key, expectedValue, actualValue]) => `<div role="row"><span role="cell">${escapeHtml(key)}</span><span role="cell">${escapeHtml(expectedValue)}</span><span role="cell">${escapeHtml(actualValue)}</span></div>`).join('')}</cem-table>`;
}

function operationRecordsMarkup(label, records) {
    if (records.length === 0)
        return `<cem-alert label="No ${escapeHtml(label.toLowerCase())} records" tone="info"></cem-alert>`;
    return `<cem-table label="${escapeHtml(label)}"><div role="row"><strong role="columnheader">Path</strong><strong role="columnheader">Record</strong></div>${records
        .slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map(
            ({ path, value }) =>
                `<div role="row"><span role="cell">${escapeHtml(path)}</span><span role="cell">${escapeHtml(commandChangeValue(value))}</span></div>`,
        )
        .join('')}</cem-table>`;
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

function renderCommandApplication(container, command) {
    const application = command?.application;
    if (!application) {
        container.innerHTML = '<cem-alert label="Command targeting is loading" tone="info"></cem-alert>';
        return;
    }
    const signature = JSON.stringify({
        mode: application.target.mode,
        current: application.targets.current?.id,
        existing: application.targets.existing.map(({ id, kind }) => [id, kind]),
        parents: application.targets.parents.map(({ id }) => id),
        confirmation:
            application.status === 'confirmation-required'
                ? (application.confirmation?.target?.entryId ?? application.confirmation?.target?.mode)
                : undefined,
    });
    if (container.dataset.signature !== signature) {
        container.dataset.signature = signature;
        container.innerHTML = commandApplicationMarkup(application);
    }
    const targetMode = container.querySelector('[data-cem-studio-command-target-mode]');
    if (targetMode) targetMode.value = application.target.mode;
    const existing = container.querySelector('[data-cem-studio-command-target-existing]');
    if (existing && application.target.mode === 'existing') existing.value = application.target.entryId;
    const parent = container.querySelector('[data-cem-studio-command-target-parent]');
    if (parent) parent.value = application.newPageParentId ?? '';
    const alert = container.querySelector('[data-cem-studio-command-apply-alert]');
    alert?.setAttribute('label', commandApplicationAlertLabel(application));
    alert?.setAttribute('tone', commandApplicationAlertTone(application));
    const busy = application.status === 'applying' || application.status === 'running';
    const targetInvalid = application.target.mode === 'new' && application.newPageName.trim().length === 0;
    const disabled = busy || targetInvalid || command.status === 'invalid' || command.status === 'checking';
    const apply = container.querySelector('[data-cem-studio-command-apply]');
    const applyRun = container.querySelector('[data-cem-studio-command-apply-run]');
    apply?.toggleAttribute('disabled', disabled);
    apply?.toggleAttribute('loading', application.status === 'applying');
    applyRun?.toggleAttribute('disabled', disabled);
    applyRun?.toggleAttribute('loading', application.status === 'running');
}

function commandApplicationMarkup(application) {
    const current = application.targets.current;
    const existingOptions =
        application.targets.existing.length === 0
            ? '<option value="" disabled>No existing pages</option>'
            : application.targets.existing
                  .map(
                      (entry) => `
            <option value="${escapeHtml(entry.id)}">${escapeHtml(`${entry.name} — ${entry.kind}${entry.compatible ? '' : ' (replacement)'}`)}</option>`,
                  )
                  .join('');
    const targetDetail =
        application.target.mode === 'current'
            ? `<cem-alert label="${escapeHtml(
                  current
                      ? `${current.name}; ${current.kind}${current.compatible ? '; compatible' : '; replacement confirmation required'}`
                      : 'No current page is available',
              )}" tone="${current?.compatible ? 'success' : 'warning'}"></cem-alert>`
            : application.target.mode === 'existing'
              ? `<cem-select data-cem-studio-command-target-existing name="command-target-existing" value="${escapeHtml(application.target.entryId ?? '')}">
                    <span slot="label">Existing page</span>${existingOptions}
                </cem-select>`
              : `<cem-text-field data-cem-studio-command-target-name name="command-target-name" label="New page name" value="${escapeHtml(application.newPageName)}" required>
                    <span slot="help">A collision-safe stable id is assigned by the repository.</span>
                </cem-text-field>
                <cem-select data-cem-studio-command-target-parent name="command-target-parent" value="${escapeHtml(application.newPageParentId ?? '')}">
                    <span slot="label">Parent</span>
                    <option value="">Project root</option>
                    ${application.targets.parents.map(({ id, name }) => `<option value="${escapeHtml(id)}">${escapeHtml(name)}</option>`).join('')}
                </cem-select>`;
    const confirmation =
        application.status === 'confirmation-required'
            ? `<cem-dialog data-cem-studio-command-replace-dialog transient expanded label="Confirm incompatible page replacement">
                <p>The selected page has an incompatible kind. Creating a new inspection page is recommended.</p>
                <div>
                    <cem-action data-cem-studio-command-use-new variant="primary">Use new page</cem-action>
                    <cem-action data-cem-studio-command-replace-confirm variant="secondary">Replace selected page</cem-action>
                    <cem-action data-cem-studio-command-replace-cancel variant="quiet">Cancel</cem-action>
                </div>
            </cem-dialog>`
            : '';
    return `
        <cem-select data-cem-studio-command-target-mode name="command-target-mode" value="${application.target.mode}">
            <span slot="label">Apply to</span>
            <option value="current"${current ? '' : ' disabled'}>Current page</option>
            <option value="existing"${application.targets.existing.length ? '' : ' disabled'}>Existing page</option>
            <option value="new">New page (recommended when incompatible)</option>
        </cem-select>
        <div data-cem-studio-command-target-detail>${targetDetail}</div>
        <cem-alert data-cem-studio-command-apply-alert label="Ready to apply" tone="info"></cem-alert>
        <div>
            <cem-action data-cem-studio-command-apply variant="secondary">Apply</cem-action>
            <cem-action data-cem-studio-command-apply-run variant="primary">Apply &amp; Run</cem-action>
        </div>
        ${confirmation}`;
}

function commandApplicationAlertLabel(application) {
    if (application.error) return `${application.error.code}: ${application.error.message}`;
    if (application.status === 'applying')
        return 'Applying exact authored command bytes in one repository transaction.';
    if (application.status === 'running') return 'Running the exact committed command revision.';
    if (application.status === 'confirmation-required') {
        return 'Replacement is incompatible. Creating a new page is recommended; replacement requires confirmation.';
    }
    if (application.status === 'conflict') return 'The project changed before Apply; reload before trying again.';
    if (application.status === 'stale') return 'The command or project advanced after the last Apply.';
    if (application.status === 'run-stale')
        return 'Execution completed, but a newer command, draft, or project revision exists.';
    if (application.status === 'run-invalid') return 'The exact committed command ran with diagnostics.';
    if (application.status === 'ran') {
        return `Ran committed project ${application.result.projectRevision}, command resource ${application.result.resourceRevision}.`;
    }
    if (application.status === 'applied') {
        return `Applied project ${application.result.projectRevision}, command resource ${application.result.resourceRevision}; not executed.`;
    }
    if (application.status === 'failed') return 'The command could not be applied or executed.';
    return application.target.mode === 'new'
        ? 'Apply will create a new inspection page without executing it.'
        : 'Apply will update the selected page without executing it.';
}

function commandApplicationAlertTone(application) {
    if (['failed', 'conflict', 'run-invalid'].includes(application.status)) return 'danger';
    if (['confirmation-required', 'stale', 'run-stale'].includes(application.status)) return 'warning';
    if (['applied', 'ran'].includes(application.status)) return 'success';
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
    return `<cem-list data-diagnostic-list selectable label="Diagnostics" size="${Math.min(8, diagnostics.length)}">${diagnostics
        .map(
            (diagnostic, index) => `
        <cem-list-option value="${index}">${escapeHtml(`${diagnostic.severity}: ${diagnostic.code}: ${diagnostic.message}`)}</cem-list-option>`,
        )
        .join('')}</cem-list>`;
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
    return `<cem-list data-provenance-list selectable label="Source-map provenance" size="${Math.min(8, provenance.length)}">${provenance
        .map(
            (frame, index) => `
        <cem-list-option value="${index}">${escapeHtml(`Source ${frame.sourceId ?? 'unknown'}: ${frame.transform}; bytes ${frame.range.start}–${frame.range.start + frame.range.len}`)}</cem-list-option>`,
        )
        .join('')}</cem-list>`;
}

function escapeHtml(value) {
    return String(value)
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&#39;');
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

async function settleWorkbench(runtime, root) {
    for (let depth = 0; depth < 3; depth += 1) {
        await Promise.resolve();
        const instances = [
            ...root.querySelectorAll(
                'cem-card, cem-badge, cem-text-field, cem-textarea, cem-select, cem-action, cem-alert, cem-tabs, cem-list, cem-table, cem-dialog',
            ),
        ];
        await Promise.all(instances.map((instance) => runtime.whenRenderSettled(instance)));
    }
    await Promise.resolve();
}
