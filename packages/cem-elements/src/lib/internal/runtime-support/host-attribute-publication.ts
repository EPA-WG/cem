/** Native write-control boundary. This contains no document/DOM representation. */
export interface NativeHostAttributeCommit {
    version: 1;
    writes: readonly { name: string; value: string; datasetKey: string | null }[];
}
export interface HostAttributePublicationState {
    current(applied: boolean): boolean;
    apply(): void;
    rollback(): void;
    published(): void;
}
export interface PreparedHostAttributePublication {
    publish<T>(commit: () => T, succeeded: (result: T) => boolean): T;
    cancel(): void;
}
export function hostAttributeCommitControl(value: unknown): NativeHostAttributeCommit {
    if (!value || typeof value !== 'object' || !('version' in value) || value.version !== 1 || !('writes' in value) || !Array.isArray(value.writes) || value.writes.length > 64) throw Error('cem.host_commit.control');
    const names = new Set<string>();
    const writes = value.writes.map(write => {
        if (!write || typeof write !== 'object' || typeof write.name !== 'string' || !write.name || /[A-Z]/.test(write.name) || typeof write.value !== 'string' || !(write.datasetKey === null || typeof write.datasetKey === 'string') || names.has(write.name)) throw Error('cem.host_commit.control');
        names.add(write.name);
        return Object.freeze({ name: write.name, value: write.value, datasetKey: write.datasetKey });
    });
    return Object.freeze({ version: 1, writes: Object.freeze(writes) });
}
export function prepareHostAttributePublication(state: HostAttributePublicationState): PreparedHostAttributePublication {
    let pending = true;
    return {
        cancel: () => { pending = false; },
        publish: (commit, succeeded) => {
            if (!pending) throw Error('cem.host_commit.consumed');
            pending = false;
            if (!state.current(false)) throw Error('cem.host_commit.stale');
            let started = false;
            try {
                started = true;
                state.apply();
                if (!state.current(true)) throw Error('cem.host_commit.stale');
                const result = commit();
                if (!succeeded(result)) { started = false; state.rollback(); return result; }
                if (!state.current(true)) throw Error('cem.host_commit.authority_changed');
                state.published();
                return result;
            } catch (error) {
                if (started) {
                    try { state.rollback(); }
                    catch (rollback) { throw new AggregateError([error, rollback], 'cem.host_commit.recovery_required', { cause: rollback }); }
                }
                throw error;
            }
        },
    };
}
