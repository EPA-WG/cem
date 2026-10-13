import { redactCemStudioSecrets } from './preview.js';

/** @typedef {import('./project-session.js').CemStudioProjectSelection} Selection */
/** @typedef {import('./project-session.js').CemStudioSessionWorkbench} Workbench */
/** @typedef {import('./project-session.js').CemStudioProjectSessionState} SessionState */
/** @typedef {import('./project-session.js').CemStudioProjectTransition} TransitionState */
/** @typedef {import('./workbench.js').CemStudioWorkbenchState} WorkbenchState */
/** @typedef {import('./workbench.js').CemStudioWorkbenchCommand} Command */
/**
 * @typedef {object} Transition
 * @property {number} id
 * @property {Selection} target
 * @property {TransitionState['stage']} stage
 * @property {AbortController} controller
 * @property {string} [sourceConsent]
 * @property {string} [commandConsent]
 * @property {TransitionState['error']} [error]
 * @property {{promise?: Promise<SessionState>}} [operation]
 */

/**
 * Admit durable project/page/resource identities after resolving both drafts.
 * The resolver owns repository/native command loading and returns a fresh,
 * isolated workbench. This service owns no UI, DOM, or portable project data.
 * @param {import('./project-session.js').CemStudioProjectSessionOptions} options
 * @returns {import('./project-session.js').CemStudioProjectSession}
 */
export function createCemStudioProjectSession({ openWorkbench, storage, storageKey = 'cem-studio-selection-v1' }) {
    if (typeof openWorkbench !== 'function') throw new TypeError('A project session requires a workbench resolver');
    /** @type {Set<(state: SessionState) => void>} */
    const subscribers = new Set();
    /** @type {Workbench | undefined} */
    let active;
    /** @type {Selection | undefined} */
    let selection;
    /** @type {(() => void) | undefined} */
    let unsubscribe;
    /** @type {Transition | undefined} */
    let transition;
    let recoveryError;
    /** @type {ReturnType<typeof createDraftJournal> | undefined} */
    let journal;
    let nextId = 1;
    let pendingWrites = 0;
    let disposed = false;
    /** @type {SessionState} */
    let state;

    function publish() {
        state = Object.freeze({
            status: disposed ? 'disposed' : transition?.error ? 'failed'
                : transition ? (transition.stage === 'source' || transition.stage === 'command' || transition.stage === 'confirmation') ? 'guarding' : transition.stage
                    : pendingWrites ? 'settling' : active ? 'ready' : 'empty',
            selection,
            recoveryError,
            draftRecovery: journal?.snapshot(),
            workbench: active?.snapshot(),
            pendingWrite: pendingWrites > 0,
            transition: transition && Object.freeze({
                id: transition.id,
                target: transition.target,
                stage: transition.stage,
                error: transition.error,
            }),
        });
        for (const notify of subscribers) notify(state);
        return state;
    }

    /** @param {Transition} request */
    function current(request) {
        return !disposed && transition === request;
    }

    /** @param {Transition} request */
    function requiredStage(request) {
        const snapshot = active?.snapshot();
        if (snapshot?.dirty && request.sourceConsent !== sourceFingerprint(snapshot)) return 'source';
        if (snapshot?.command && commandDirty(snapshot.command) && request.commandConsent !== commandFingerprint(snapshot.command)) return 'command';
        return undefined;
    }

    /** @param {Transition} request */
    async function advance(request) {
        if (!current(request)) return state;
        request.error = undefined;
        const stage = requiredStage(request);
        if (stage) {
            request.stage = stage;
            return publish();
        }
        return openCandidate(request);
    }

    /** @param {Transition} request */
    function openCandidate(request) {
        request.stage = 'opening';
        // Register the operation before invoking user-supplied services or subscribers.
        /** @type {{promise?: Promise<SessionState>}} */
        const operation = {};
        request.operation = operation;
        operation.promise = Promise.resolve().then(async () => {
            let candidate;
            try {
                if (!current(request)) return state;
                candidate = await openWorkbench(request.target, { signal: request.controller.signal });
                if (candidate === active) throw new TypeError('A candidate workbench must be isolated from the active workbench');
                if (!candidate?.snapshot || !candidate?.subscribe || !candidate?.dispose) {
                    throw new TypeError('A candidate requires snapshot, subscribe and dispose');
                }
                const loaded = candidate.snapshot();
                if (loaded.projectId !== request.target.projectId ||
                    (request.target.entryId !== undefined && (loaded.entryId ?? loaded.command?.application?.currentEntryId) !== request.target.entryId) ||
                    (request.target.resourceId !== undefined && loaded.resourceId !== request.target.resourceId)) {
                    throw new Error('The candidate workbench does not match the requested identity');
                }
                if (!current(request)) {
                    candidate.dispose();
                    return state;
                }
                request.operation = undefined;
                // Consent belongs to the exact draft, including its persisted baseline.
                if (requiredStage(request)) {
                    candidate.dispose();
                    return advance(request);
                }
                const candidateJournal = createDraftJournal(candidate, storage, storageKey);
                await candidateJournal.restore();
                if (!current(request)) { candidate.dispose(); return state; }
                if (requiredStage(request)) { candidate.dispose(); return advance(request); }
                // Discard consent takes effect only after successful admission.
                if (request.sourceConsent || request.commandConsent) journal?.clear();
                const previous = active;
                unsubscribe?.();
                active = candidate;
                journal = candidateJournal;
                selection = request.target;
                try { storage?.setItem(storageKey, JSON.stringify(selection)); recoveryError = undefined; }
                catch (error) { recoveryError = sessionError(error); }
                transition = undefined;
                unsubscribe = active.subscribe(() => { journal?.capture(); publish(); });
                previous?.dispose();
                return publish();
            } catch (error) {
                if (candidate && candidate !== active) candidate.dispose?.();
                if (!current(request)) return state;
                request.operation = undefined;
                request.error = sessionError(error);
                return publish();
            }
        });
        publish();
        return operation.promise;
    }

    /** @param {Selection} target @param {boolean} [refresh] */
    function select(target, refresh = false) {
        if (disposed) throw new Error('The project session is disposed');
        const identity = selectionIdentity(target);
        // A cancelled write may still commit. Do not start another admission or
        // write until it settles; callers can retry from the next ready snapshot.
        if (pendingWrites) return Promise.resolve(state);
        if (transition) {
            if (sameIdentity(identity, transition.target)) return transition.operation?.promise ?? Promise.resolve(state);
            // Replacing a pending target requires an explicit cancellation.
            return Promise.resolve(state);
        }
        if (!refresh && sameIdentity(identity, selection)) return Promise.resolve(state);
        transition = { id: nextId++, target: identity, stage: 'opening', controller: new AbortController() };
        return advance(transition);
    }

    /** @param {number} id @param {import('./project-session.js').CemStudioProjectTransitionDecision} decision */
    function resolveTransition(id, decision) {
        const request = transition;
        if (disposed || !request || request.id !== id) return Promise.resolve(state);
        if (decision === 'cancel') return Promise.resolve(cancelTransition(id));
        if (request.operation) return request.operation.promise;
        if (decision === 'retry' && request.error) return advance(request);
        const stage = requiredStage(request);
        const workbench = active;
        if (!workbench) return Promise.resolve(state);
        if (!stage && ['source', 'command', 'confirmation'].includes(request.stage)) return advance(request);
        if (request.stage === 'confirmation' && stage !== 'source' &&
            workbench.snapshot().command?.application?.status === 'confirmation-required') {
            if (decision === 'confirm-replacement' && workbench.confirmCommandReplacement) return persist(request, 'command', () => workbench.confirmCommandReplacement?.({ signal: request.controller.signal }) ?? Promise.resolve(workbench.snapshot()));
            if (decision === 'discard') {
                request.commandConsent = commandFingerprint(workbench.snapshot().command);
                return advance(request);
            }
        } else if (request.stage !== stage && stage) {
            request.stage = stage;
            request.error = undefined;
            return Promise.resolve(publish());
        }
        if (decision === 'discard' && stage) {
            if (stage === 'source') request.sourceConsent = sourceFingerprint(workbench.snapshot());
            else request.commandConsent = commandFingerprint(workbench.snapshot().command);
            return advance(request);
        }
        if (decision === 'save' && stage === 'source' && workbench.saveSource) {
            return persist(request, 'source', () => workbench.saveSource?.({ signal: request.controller.signal }) ?? Promise.resolve(workbench.snapshot()));
        }
        if (decision === 'apply' && stage === 'command' && workbench.applyCommand) {
            return persist(request, 'command', () => workbench.applyCommand?.({ signal: request.controller.signal }) ?? Promise.resolve(workbench.snapshot()));
        }
        return Promise.resolve(state);
    }

    /** @param {Transition} request @param {'source' | 'command'} draft @param {() => Promise<WorkbenchState>} action */
    function persist(request, draft, action) {
        pendingWrites += 1;
        request.stage = draft === 'source' ? 'saving' : 'applying';
        request.error = undefined;
        /** @type {{promise?: Promise<SessionState>}} */
        const operation = {};
        request.operation = operation;
        operation.promise = Promise.resolve().then(async () => {
            try {
                if (current(request)) await action();
            } catch (error) {
                if (current(request)) {
                    request.stage = draft;
                    request.error = sessionError(error);
                }
            } finally {
                pendingWrites -= 1;
                request.operation = undefined;
            }
            if (!current(request)) return publish();
            if (request.error) return publish();
            if (draft === 'command' && active?.snapshot().command?.application?.status === 'confirmation-required') {
                request.stage = 'confirmation';
                return publish();
            }
            return advance(request);
        });
        publish();
        return operation.promise;
    }

    /** @param {number} id */
    function cancelTransition(id) {
        if (!transition || transition.id !== id) return state;
        const cancelled = transition;
        transition = undefined;
        cancelled.controller.abort();
        if (active?.snapshot().command?.application?.status === 'confirmation-required') active.cancelCommandReplacement?.();
        return publish();
    }

    publish();
    return Object.freeze({
        snapshot: () => state,
        activeWorkbench: () => active,
        draftBackup: () => journal?.backup() ?? '',
        clearDraftRecovery() {
            try { journal?.clear(); } finally { publish(); }
            journal?.capture();
            return publish();
        },
        select,
        refresh: () => selection ? select(selection, true) : Promise.resolve(state),
        async recover(fallback) {
            if (active || transition) return state;
            let saved;
            try {
                const text = storage?.getItem(storageKey);
                if (text) saved = selectionIdentity(JSON.parse(text));
            } catch (error) { recoveryError = sessionError(error); }
            if (saved) {
                await select(saved);
                if (active) return state;
                recoveryError = transition?.error;
                if (transition) cancelTransition(transition.id);
            }
            const error = recoveryError;
            if (fallback) await select(fallback);
            recoveryError = error ?? recoveryError;
            return publish();
        },
        resolveTransition,
        cancelTransition,
        subscribe(notify) {
            if (disposed) throw new Error('The project session is disposed');
            if (typeof notify !== 'function') throw new TypeError('A session subscriber must be a function');
            subscribers.add(notify);
            notify(state);
            return () => subscribers.delete(notify);
        },
        dispose() {
            if (disposed) return;
            disposed = true;
            transition?.controller.abort();
            transition = undefined;
            unsubscribe?.();
            active?.dispose();
            publish();
            subscribers.clear();
        },
    });
}

/** @param {Selection} value @returns {Selection} */
function selectionIdentity(value) {
    for (const key of /** @type {const} */ (['projectId', 'entryId', 'resourceId'])) {
        if (key !== 'projectId' && value?.[key] === undefined) continue;
        if (typeof value?.[key] !== 'string' || value[key].trim().length === 0) throw new TypeError(`Selection requires a nonempty ${key}`);
    }
    return Object.freeze({ projectId: value.projectId,
        ...(value.entryId === undefined ? {} : { entryId: value.entryId }),
        ...(value.resourceId === undefined ? {} : { resourceId: value.resourceId }) });
}

/** @param {Selection} left @param {Selection | undefined} right */
function sameIdentity(left, right) {
    return right && left.projectId === right.projectId && left.entryId === right.entryId && left.resourceId === right.resourceId;
}

/** @param {WorkbenchState} state */
function sourceFingerprint(state) {
    return JSON.stringify([state.draft, state.persistedText]);
}

/** @param {Command | undefined} command */
function commandDirty(command) {
    return command && command.draftText !== command.current.text;
}

/** @param {Command | undefined} command */
function commandFingerprint(command) {
    return JSON.stringify([command?.draftText, command?.current.text, command?.application.target]);
}

/** @param {unknown} error */
function sessionError(error) {
    return Object.freeze({
        code: error && typeof error === 'object' && 'code' in error && typeof error.code === 'string' ? error.code : 'cem.studio.session.failed',
        message: redactCemStudioSecrets(error instanceof Error ? error.message : String(error)),
    });
}

/** Local recovery is a named JSON storage/backup boundary, never project data.
 * @param {Workbench} workbench
 * @param {import('./project-session.js').CemStudioProjectSessionOptions['storage']} storage
 * @param {string} prefix
 */
function createDraftJournal(workbench, storage, prefix) {
    const initial = workbench.snapshot();
    const identity = [initial.projectId, initial.entryId ?? '', initial.resourceId];
    const key = `${prefix}:draft-v1:${JSON.stringify(identity)}`;
    let raw = '';
    let backup = '';
    let blocked = false;
    /** @type {import('./project-session.js').CemStudioDraftRecovery} */
    let state = { status: 'ready', message: 'Local draft recovery is ready. Save and Apply remain explicit.', available: false, blocked: false };
    /** @param {unknown} error @param {boolean} [conflict] */
    function fail(error, conflict = false) {
        blocked = true;
        state = { status: conflict ? 'conflict' : 'error', message: `${conflict ? 'The project or recovery record changed. Stored drafts were not restored.' : 'Local draft recovery could not be saved or read.'} Download a recovery backup before clearing it or leaving this page. ${sessionError(error).message}`, available: true, blocked: true };
    }
    function capture() {
        if (!storage || blocked) return;
        const current = workbench.snapshot();
        const command = current.command;
        const record = current.dirty || commandDirty(command) ? {
            version: 1, identity, projectRevision: current.projectRevision,
            resourceRevision: current.resourceRevision, persistedText: current.persistedText,
            draft: current.draft,
            command: command && { baseline: command.current.text, draft: command.draftText, target: command.application.target },
        } : undefined;
        const next = record ? JSON.stringify(record) : '';
        if (next === raw) return;
        backup = next;
        try {
            if ((storage.getItem(key) ?? '') !== raw) {
                backup = JSON.stringify({ version: 1, local: next, stored: storage.getItem(key) });
                throw new Error('Another session changed this recovery record.');
            }
            // Bound synchronous storage work. Quota errors retain the in-memory backup.
            if (next.length > 1_000_000) throw new Error('Draft recovery exceeds the 1,000,000 character limit.');
            storage.setItem(key, next);
            raw = next;
            state = { status: next ? 'saved' : 'ready', message: next ? 'Drafts saved locally for recovery. Save and Apply remain explicit.' : 'No unsaved drafts. Save and Apply remain explicit.', available: Boolean(next), blocked: false };
        } catch (error) { fail(error); }
    }
    return {
        snapshot: () => Object.freeze(state),
        backup: () => blocked ? JSON.stringify({ version: 1, stored: backup, current: {
            identity, projectRevision: workbench.snapshot().projectRevision,
            resourceRevision: workbench.snapshot().resourceRevision,
            persistedText: workbench.snapshot().persistedText, draft: workbench.snapshot().draft,
            command: workbench.snapshot().command && {
                baseline: workbench.snapshot().command?.current.text,
                draft: workbench.snapshot().command?.draftText,
                target: workbench.snapshot().command?.application.target,
            },
        } }) : backup,
        capture,
        clear() {
            if (!storage) return;
            // Clearing is explicit and only removes the version the user inspected.
            const latest = storage.getItem(key) ?? '';
            if (latest !== raw) { raw = latest; backup = latest; fail(new Error('Recovery changed again. Download the latest backup and retry.'), true); throw new Error('Recovery changed again'); }
            try { storage.setItem(key, ''); }
            catch (error) { fail(error); throw error; }
            raw = ''; backup = ''; blocked = false;
            state = { status: 'ready', message: 'Stored recovery cleared. Current editor drafts are retained.', available: false, blocked: false };
        },
        async restore() {
            if (!storage) return;
            try {
                raw = storage.getItem(key) ?? '';
                backup = raw;
                if (!raw) return;
                if (raw.length > 1_000_000) throw new Error('Recovery record exceeds the supported size.');
                const record = JSON.parse(raw);
                if (record?.version !== 1 || JSON.stringify(record.identity) !== JSON.stringify(identity) ||
                    typeof record.draft !== 'string' || typeof record.persistedText !== 'string' ||
                    !Number.isSafeInteger(record.projectRevision) || !Number.isSafeInteger(record.resourceRevision) ||
                    (record.command && (typeof record.command.baseline !== 'string' || typeof record.command.draft !== 'string'))) {
                    throw new Error('Invalid recovery record.');
                }
                if (record.projectRevision !== initial.projectRevision || record.resourceRevision !== initial.resourceRevision ||
                    record.persistedText !== initial.persistedText ||
                    (record.command && record.command.baseline !== initial.command?.current.text)) {
                    fail(new Error('The original revision no longer matches.'), true);
                    return;
                }
                if (!workbench.updateDraft || (record.command && !workbench.updateCommandDraft)) throw new Error('This workbench does not support draft recovery.');
                workbench.updateDraft(record.draft);
                if (record.command) {
                    await workbench.updateCommandDraft(record.command.draft);
                    if (record.command.target) workbench.setCommandTarget?.(record.command.target);
                }
                state = { status: 'recovered', message: 'Recovered local drafts. Review them before Save or Apply.', available: true, blocked: false };
            } catch (error) { fail(error); }
        },
    };
}
