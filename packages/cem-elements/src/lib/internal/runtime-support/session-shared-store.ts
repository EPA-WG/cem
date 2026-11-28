import type { SharedStoreEntry, SharedStoreTransaction } from './shared-store.js';

type Snapshot = [epoch: string, entries: [string, SharedStoreEntry][]];

/** Each admitted issuer retains one transaction queue per named session partition.
 * Read from the page's session area on every operation so reloads recover the
 * last committed values and versions. One native setItem publishes both. */
export function sessionStoreTransactions(page: Window, namespace: string): SharedStoreTransaction {
    const key = 'cem.application.shared.session/1:' + namespace;
    const storage = page.sessionStorage;
    if (!storage) throw new DOMException('Page session storage unavailable', 'InvalidStateError');
    function checkProvider() {
        if (page.sessionStorage !== storage) throw new DOMException('Page session provider changed', 'InvalidStateError');
    }
    let pending = Promise.resolve();
    return (_mode, check, work) => {
        const operation = pending.then(async () => {
            check();
            checkProvider();
            const saved = storage.getItem(key);
            const snapshot: Snapshot = saved === null ? [crypto.randomUUID(), []] : JSON.parse(saved);
            if (!Array.isArray(snapshot) || typeof snapshot[0] !== 'string' || !Array.isArray(snapshot[1]))
                throw new DOMException('Invalid named session store snapshot', 'DataError');
            let [epoch] = snapshot;
            const entries = new Map(snapshot[1]);
            let dirty = saved === null;
            const result = await work({
                epoch: () => epoch,
                get: name => entries.get(name),
                keys: () => Array.from(entries.keys()).filter(name => entries.get(name)?.present).sort(),
                put(name, record) {
                    entries.set(name, record);
                    dirty = true;
                },
                clearIfNonempty() {
                    if (!Array.from(entries.values()).some(entry => entry.present)) return;
                    entries.clear();
                    epoch = crypto.randomUUID();
                    dirty = true;
                },
            });
            check();
            checkProvider();
            if (dirty) storage.setItem(key, JSON.stringify([epoch, Array.from(entries)]));
            return result;
        });
        // A denied or failed operation must release the queue for the next grant.
        pending = operation.then(() => undefined, () => undefined);
        return operation;
    };
}
