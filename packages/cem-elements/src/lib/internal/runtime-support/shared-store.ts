/** Versioned value from a named application store. */
export interface CemSharedStoreValue {
    readonly value: string | null;
    readonly version: string;
}

export interface CemSharedStoreChange {
    readonly type: 'set' | 'remove' | 'clear' | 'invalidate';
    readonly scope: string;
    readonly key: string | null;
    readonly oldValue: string | null;
    readonly newValue: string | null;
    /** Current readable entries when incremental delivery could not be recovered. */
    readonly snapshot?: readonly (readonly [string, string])[];
}

export interface CemApplicationSharedStore {
    subscribe(listener: (change: CemSharedStoreChange) => void): Promise<() => void>;
    getItem(key: string): Promise<string | null>;
    read(key: string): Promise<CemSharedStoreValue>;
    keys(): Promise<readonly string[]>;
    setItem(key: string, value: string, expectedVersion?: string): Promise<void>;
    removeItem(key: string, expectedVersion?: string): Promise<void>;
    clear(): Promise<void>;
}

export interface SharedStoreEntry {
    value: string | null;
    stamp: string;
    /** Only live values participate in enumeration. */
    present?: 1;
}

interface StoreEntries {
    epoch(): string | Promise<string>;
    get(key: string): SharedStoreEntry | undefined | Promise<SharedStoreEntry | undefined>;
    keys(): string[] | Promise<string[]>;
    put(key: string, record: SharedStoreEntry): void;
    clear(): void;
    changed(change: StoreMutation): void;
}

/** The provider checks the grant again immediately before publishing its transaction. */
export type SharedStoreTransaction = <T>(mode: 'readonly' | 'readwrite', check: () => void,
    work: (entries: StoreEntries) => Promise<T>) => Promise<T>;

export interface StoreMutation {
    writer: string;
    key: string | null;
    oldValue: string | null;
    newValue: string | null;
    clearedKeys?: string[];
}

export type StorePublication = StoreMutation | { snapshot: [string, string][] };

export interface StoreObserver {
    signal: AbortSignal;
    active(): boolean;
    deliver(publication: StorePublication): void;
}

export interface SharedStoreProvider {
    transact: SharedStoreTransaction;
    observe(observer: StoreObserver): Promise<void>;
}

function version(epoch: string, key: string, entry: SharedStoreEntry | undefined): string {
    return JSON.stringify([epoch, key, entry?.stamp ?? null]);
}

function scopedChange(change: StorePublication, scope: string, writer: string,
    keys?: ReadonlySet<string>): CemSharedStoreChange | undefined {
    if ('snapshot' in change) {
        const snapshot = change.snapshot.filter(([key]) => !keys || keys.has(key))
            .map(([key, value]) => Object.freeze([key, value] as const));
        return { type: 'invalidate', scope, key: null, oldValue: null, newValue: null, snapshot: Object.freeze(snapshot) };
    }
    if (change.writer === writer) return;
    const { key, oldValue, newValue } = change;
    if (key === null) {
        if (keys && !change.clearedKeys?.some(name => keys.has(name))) return;
        return { type: 'clear', scope, key, oldValue, newValue };
    }
    if (keys && !keys.has(key)) return;
    return { type: newValue === null ? 'remove' : 'set', scope, key, oldValue, newValue };
}

type StoreOperation = 'read' | 'write' | 'conditional-write' | 'clear';
const writeFailureNames = new Set(['QuotaExceededError', 'SecurityError', 'InvalidStateError', 'AbortError']);

export function bindSharedStore(provider: SharedStoreProvider,
    { read, write, subscribe, scope, writer, signal }: {
        read: boolean; write: boolean; subscribe: boolean; scope: string; writer: string; signal: AbortSignal;
    }, keys: ReadonlySet<string> | undefined, live: () => boolean): CemApplicationSharedStore {
    function authorize(operation: StoreOperation, key?: string) {
        if (!live()) throw new DOMException('Shared store binding has expired', 'InvalidStateError');
        if ((operation !== 'write' && !read) || (operation !== 'read' && !write)
            || (key !== undefined && keys && !keys.has(key)))
            throw new DOMException('Shared store grant denies this operation', 'NotAllowedError');
        if (operation === 'clear' && keys) throw new DOMException('Clear requires a whole-store grant', 'NotAllowedError');
    }

    async function run<T>(operation: StoreOperation, key: string | undefined,
        work: (entries: StoreEntries) => Promise<T>): Promise<T> {
        const check = () => authorize(operation, key);
        check();
        try {
            const result = await provider.transact(operation === 'read' ? 'readonly' : 'readwrite', check, work);
            if (operation === 'read') check();
            return result;
        } catch (error) {
            check();
            if (!read) {
                // Provider diagnostics may contain persisted data. Only publish the failure category.
                const name = (error as DOMException | null)?.name;
                throw new DOMException('Shared store write failed', name && writeFailureNames.has(name) ? name : 'OperationError');
            }
            throw error;
        }
    }

    async function change(key: string, value: string | null, expected?: string) {
        await run(expected === undefined ? 'write' : 'conditional-write', key, async entries => {
            const old = await entries.get(key);
            if (expected !== undefined) {
                const epoch = await entries.epoch();
                if (expected !== version(epoch, key, old))
                    throw new DOMException('Shared store key version has changed', 'ConstraintError');
            }
            if ((old?.value ?? null) === value) return;
            const record: SharedStoreEntry = { value, stamp: crypto.randomUUID() };
            if (value !== null) record.present = 1;
            // A deletion keeps a fresh stamp, fencing stale reads of an absent key.
            entries.put(key, record);
            entries.changed({ writer, key, oldValue: old?.value ?? null, newValue: value });
        });
    }

    return Object.freeze({
        async subscribe(listener: (change: CemSharedStoreChange) => void) {
            authorize('read');
            if (!subscribe) throw new DOMException('Shared store subscription is not granted', 'NotAllowedError');
            if (typeof listener !== 'function') throw new TypeError('Expected a shared store change listener');
            const subscription = new AbortController();
            const stop = () => {
                subscription.abort();
                signal.removeEventListener('abort', stop);
            };
            signal.addEventListener('abort', stop);
            const active = () => {
                if (subscription.signal.aborted) return false;
                try { if (live()) return true; } catch { /* A previously admitted document can become unreachable. */ }
                stop();
                return false;
            };
            try {
                await provider.observe({
                    signal: subscription.signal, active,
                    deliver(change) {
                        if (!active()) return;
                        const event = scopedChange(change, scope, writer, keys);
                        if (!event) return;
                        try { listener(Object.freeze(event)); } catch (error) { reportError(error); }
                    },
                });
                authorize('read');
                return stop;
            } catch (error) { stop(); throw error; }
        },
        getItem(key: string) {
            key = String(key);
            return run('read', key, async entries => {
                const entry = await entries.get(key);
                return entry?.value ?? null;
            });
        },
        read(key: string) {
            key = String(key);
            return run('read', key, async entries => {
                const [epoch, entry] = await Promise.all([entries.epoch(), entries.get(key)]);
                return Object.freeze({ value: entry?.value ?? null, version: version(epoch, key, entry) });
            });
        },
        keys() {
            return run('read', undefined, async entries => {
                const names = await entries.keys();
                return names.filter(key => !keys || keys.has(key));
            });
        },
        setItem(key: string, value: string, expected?: string) { return change(String(key), String(value), expected); },
        removeItem(key: string, expected?: string) { return change(String(key), null, expected); },
        async clear() {
            await run('clear', undefined, async entries => {
                const clearedKeys = await entries.keys();
                if (clearedKeys.length === 0) return;
                entries.clear();
                entries.changed({ writer, key: null, oldValue: null, newValue: null, clearedKeys });
            });
        },
    });
}
