/** Versioned value from a named application store. */
export interface CemSharedStoreValue {
    readonly value: string | null;
    readonly version: string;
}

export interface CemApplicationSharedStore {
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
    clearIfNonempty(): void | Promise<void>;
}

/** The provider checks the grant again immediately before publishing its transaction. */
export type SharedStoreTransaction = <T>(mode: 'readonly' | 'readwrite', check: () => void,
    work: (entries: StoreEntries) => Promise<T>) => Promise<T>;

const table = 'entries';
// Logical keys are strings. The numeric key belongs exclusively to the provider.
const epochKey = 0;

function request<T>(input: IDBRequest<T>): Promise<T> {
    return new Promise((resolve, reject) => {
        input.onsuccess = () => resolve(input.result);
        input.onerror = () => reject(input.error);
    });
}

function open(factory: IDBFactory, name: string): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
        const input = factory.open(name, 1);
        let blocked = false;
        input.onblocked = () => {
            blocked = true;
            reject(new DOMException('Shared store upgrade is blocked', 'InvalidStateError'));
        };
        input.onupgradeneeded = () => {
            if (blocked) { input.transaction?.abort(); return; }
            const entries = input.result.createObjectStore(table);
            entries.createIndex('present', 'present');
            entries.add(crypto.randomUUID(), epochKey);
        };
        input.onerror = () => reject(input.error);
        input.onsuccess = () => {
            const db = input.result;
            if (blocked) { db.close(); return; }
            db.onversionchange = () => db.close();
            resolve(db);
        };
    });
}

function version(epoch: string, key: string, entry: SharedStoreEntry | undefined): string {
    return JSON.stringify([epoch, key, entry?.stamp ?? null]);
}

export function durableStoreTransactions(factory: IDBFactory, namespace: string): SharedStoreTransaction {
    const name = 'cem.application.shared/1:' + namespace;
    return async (mode, check, work) => {
        const db = await open(factory, name);
        try {
            check();
            const tx = db.transaction(table, mode);
            const completed = new Promise<DOMException | null>(resolve => {
                tx.oncomplete = () => resolve(null);
                tx.onabort = () => resolve(tx.error ?? new DOMException('Shared store operation aborted', 'AbortError'));
            });
            const entries = tx.objectStore(table);
            const present = entries.index('present');
            let result;
            try {
                result = await work({
                    epoch: () => request<string>(entries.get(epochKey)),
                    get: key => request<SharedStoreEntry | undefined>(entries.get(key)),
                    keys: () => request(present.getAllKeys(1)) as Promise<string[]>,
                    put(key, record) { entries.put(record, key); },
                    async clearIfNonempty() {
                        if (await request(present.getKey(1)) === undefined) return;
                        entries.clear();
                        entries.add(crypto.randomUUID(), epochKey);
                    },
                });
                check();
                tx.commit();
            } catch (error) {
                try { tx.abort(); } catch { /* A request may already have aborted the transaction. */ }
                await completed;
                throw error;
            }
            const failure = await completed;
            if (failure) throw failure;
            return result;
        } finally { db.close(); }
    };
}

type StoreOperation = 'read' | 'write' | 'conditional-write' | 'clear';
const writeFailureNames = new Set(['QuotaExceededError', 'SecurityError', 'InvalidStateError', 'AbortError']);

export function bindSharedStore(transaction: SharedStoreTransaction, { read, write }: { read: boolean; write: boolean },
    keys: ReadonlySet<string> | undefined, live: () => boolean): CemApplicationSharedStore {
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
            const result = await transaction(operation === 'read' ? 'readonly' : 'readwrite', check, work);
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
        });
    }

    return Object.freeze({
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
                await entries.clearIfNonempty();
            });
        },
    });
}
