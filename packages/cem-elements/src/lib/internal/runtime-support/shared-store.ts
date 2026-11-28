/** Durable named stores. All participants use one IndexedDB database per
 * declaring application path and store ID, independent of the recipient's alias. */
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

interface Entry {
    value: string | null;
    stamp: string;
    /** Only live values participate in enumeration. */
    present?: 1;
}

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

function version(epoch: string, key: string, entry: Entry | undefined): string {
    return JSON.stringify([epoch, key, entry?.stamp ?? null]);
}

export function bindSharedStore(factory: IDBFactory, namespace: string, writable: boolean,
    keys: ReadonlySet<string> | undefined, live: () => boolean): CemApplicationSharedStore {
    const name = 'cem.application.shared/1:' + namespace;
    function authorize(mode: IDBTransactionMode, key?: string) {
        if (!live()) throw new DOMException('Shared store binding has expired', 'InvalidStateError');
        if ((mode === 'readwrite' && !writable) || (key !== undefined && keys && !keys.has(key)))
            throw new DOMException('Shared store grant denies this operation', 'NotAllowedError');
    }

    // Only IndexedDB requests may be awaited inside work, keeping the transaction active.
    async function run<T>(mode: IDBTransactionMode, key: string | undefined,
        work: (entries: IDBObjectStore) => Promise<T>): Promise<T> {
        authorize(mode, key);
        const db = await open(factory, name);
        try {
            authorize(mode, key);
            const tx = db.transaction(table, mode);
            const completed = new Promise<DOMException | null>(resolve => {
                tx.oncomplete = () => resolve(null);
                tx.onabort = () => resolve(tx.error ?? new DOMException('Shared store operation aborted', 'AbortError'));
            });
            let result: T;
            try {
                result = await work(tx.objectStore(table));
                authorize(mode, key);
                tx.commit();
            } catch (error) {
                try { tx.abort(); } catch { /* A request may already have aborted the transaction. */ }
                await completed;
                authorize(mode, key);
                throw error;
            }
            const failure = await completed;
            if (failure) throw failure;
            // A retired caller must not receive a read that was still in flight.
            if (mode === 'readonly') authorize(mode, key);
            return result;
        } finally { db.close(); }
    }

    async function change(key: string, value: string | null, expected?: string) {
        await run('readwrite', key, async entries => {
            const old: Entry | undefined = await request(entries.get(key));
            if (expected !== undefined) {
                const epoch: string = await request(entries.get(epochKey));
                if (expected !== version(epoch, key, old))
                    throw new DOMException('Shared store key version has changed', 'ConstraintError');
            }
            if ((old?.value ?? null) === value) return;
            const record: Entry = { value, stamp: crypto.randomUUID() };
            if (value !== null) record.present = 1;
            // A deletion keeps a fresh stamp, fencing stale reads of an absent key.
            entries.put(record, key);
        });
    }

    return Object.freeze({
        getItem(key: string) {
            key = String(key);
            return run('readonly', key, async entries => {
                const entry: Entry | undefined = await request(entries.get(key));
                return entry?.value ?? null;
            });
        },
        read(key: string) {
            key = String(key);
            return run('readonly', key, async entries => {
                const [epoch, entry] = await Promise.all([request<string>(entries.get(epochKey)), request<Entry | undefined>(entries.get(key))]);
                return Object.freeze({ value: entry?.value ?? null, version: version(epoch, key, entry) });
            });
        },
        keys() {
            return run('readonly', undefined, async entries => {
                const names = await request(entries.index('present').getAllKeys(1)) as string[];
                return names.filter(key => !keys || keys.has(key));
            });
        },
        setItem(key: string, value: string, expected?: string) { return change(String(key), String(value), expected); },
        removeItem(key: string, expected?: string) { return change(String(key), null, expected); },
        async clear() {
            authorize('readwrite');
            if (keys) throw new DOMException('Clear requires a whole-store grant', 'NotAllowedError');
            await run('readwrite', undefined, async entries => {
                if (await request(entries.index('present').getKey(1)) === undefined) return;
                entries.clear();
                entries.add(crypto.randomUUID(), epochKey);
            });
        },
    });
}
