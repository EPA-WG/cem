import type { SharedStoreEntry, SharedStoreProvider, StoreMutation, StoreObserver } from './shared-store.js';

const journal = 'changes';
const retention = 64;
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
        const input = factory.open(name, 2);
        let blocked = false;
        input.onblocked = () => {
            blocked = true;
            reject(new DOMException('Shared store upgrade is blocked', 'InvalidStateError'));
        };
        input.onupgradeneeded = event => {
            if (blocked) { input.transaction?.abort(); return; }
            if (event.oldVersion === 0) {
                const entries = input.result.createObjectStore(table);
                entries.createIndex('present', 'present');
                entries.add(crypto.randomUUID(), epochKey);
            }
            input.result.createObjectStore(journal, { autoIncrement: true }).add(crypto.randomUUID(), 0);
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

export function durableSharedStore(host: Window & typeof globalThis, namespace: string): SharedStoreProvider {
    const factory = host.indexedDB;
    if (!factory) throw new DOMException('Shared store provider unavailable', 'InvalidStateError');
    const name = 'cem.application.shared/1:' + namespace;
    function channel() {
        try { return new host.BroadcastChannel(name); } catch { return undefined; }
    }

    const transact: SharedStoreProvider['transact'] = async (mode, check, work) => {
        const db = await open(factory, name);
        try {
            check();
            const tx = db.transaction(mode === 'readwrite' ? [table, journal] : table, mode);
            const completed = new Promise<DOMException | null>(resolve => {
                tx.oncomplete = () => resolve(null);
                tx.onabort = () => resolve(tx.error ?? new DOMException('Shared store operation aborted', 'AbortError'));
            });
            const entries = tx.objectStore(table);
            const present = entries.index('present');
            let result;
            let mutation: StoreMutation | undefined;
            try {
                result = await work({
                    epoch: () => request<string>(entries.get(epochKey)),
                    get: key => request<SharedStoreEntry | undefined>(entries.get(key)),
                    keys: () => request(present.getAllKeys(1)) as Promise<string[]>,
                    put(key, record) { entries.put(record, key); },
                    changed(change) { mutation = change; },
                    clear() {
                        entries.clear();
                        entries.add(crypto.randomUUID(), epochKey);
                    },
                });
                if (mutation) {
                    const log = tx.objectStore(journal);
                    const id = Number(await request(log.add(mutation)));
                    if (id > retention) log.delete(id - retention);
                }
                check();
                tx.commit();
            } catch (error) {
                try { tx.abort(); } catch { /* A request may already have aborted the transaction. */ }
                await completed;
                throw error;
            }
            const failure = await completed;
            if (failure) throw failure;
            if (mutation) {
                // A wake-up contains no store data. Periodic reconciliation also covers
                // a writer leaving the page between commit and notification.
                const wake = channel();
                try { wake?.postMessage(null); } catch { /* The stored commit remains authoritative. */ }
                finally { wake?.close(); }
            }
            return result;
        } finally { db.close(); }
    };

    interface Cursor { generation: string; position: number }
    async function batch(after?: Cursor, resync = false) {
        const db = await open(factory, name);
        try {
            const tx = db.transaction([journal, table]);
            const log = tx.objectStore(journal);
            const [generation, last] = await Promise.all([request<string>(log.get(0)), request(log.openKeyCursor(null, 'prev'))]);
            const position = Number(last?.key ?? 0);
            const cursor = { generation, position };
            if (!after) return { cursor };
            if (resync || after.generation !== generation || position < after.position || position - after.position > retention) {
                const index = tx.objectStore(table).index('present');
                const [keys, entries] = await Promise.all([request(index.getAllKeys(1)), request<SharedStoreEntry[]>(index.getAll(1))]);
                const snapshot: [string, string][] = [];
                for (const [i, entry] of entries.entries()) {
                    if (entry.value !== null) snapshot.push([String(keys[i]), entry.value]);
                }
                return { cursor, snapshot };
            }
            if (after.position === position) return { cursor };
            const changes = await request<StoreMutation[]>(log.getAll(host.IDBKeyRange.lowerBound(after.position, true)));
            return { cursor, changes };
        } finally { db.close(); }
    }

    async function observe(observer: StoreObserver) {
        const wake = channel();
        let cursor: Cursor | undefined;
        let running = false, dirty = false, resync = false;
        async function reconcile() {
            dirty = true;
            if (!cursor || running || !observer.active()) return;
            running = true;
            try {
                while (dirty && observer.active()) {
                    dirty = false;
                    const snapshotRequired = resync;
                    resync = false;
                    const update = await batch(cursor, snapshotRequired);
                    if (!observer.active()) return;
                    cursor = update.cursor;
                    if (update.snapshot) observer.deliver({ snapshot: update.snapshot });
                    else for (const change of update.changes ?? []) observer.deliver(change);
                }
            } catch (error) {
                resync = true;
                if (observer.active()) reportError(error);
            }
            finally { running = false; }
        }
        const refresh = () => { void reconcile(); };
        const onPageShow = (event: PageTransitionEvent) => {
            if (event.persisted) resync = true;
            refresh();
        };
        if (wake) wake.onmessage = refresh;
        const timer = host.setInterval(refresh, 5000);
        host.addEventListener('pageshow', onPageShow);
        observer.signal.addEventListener('abort', () => {
            host.clearInterval(timer);
            host.removeEventListener('pageshow', onPageShow);
            wake?.close();
        }, { once: true });
        // Listen before taking the baseline. Drain any wake-up that raced enrollment.
        cursor = (await batch()).cursor;
        if (dirty) refresh();
    }

    return { transact, observe };
}
