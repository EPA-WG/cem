import type { SharedStoreEntry, SharedStoreProvider, StoreMutation, StoreObserver, StorePublication } from './shared-store.js';

type Snapshot = [epoch: string, entries: [string, SharedStoreEntry][]];

/** Each admitted issuer retains one transaction queue per named session partition.
 * Read from the page's session area on every operation so reloads recover the
 * last committed values and versions. One native setItem publishes both. */
export function sessionSharedStore(page: Window, namespace: string): SharedStoreProvider {
    const key = 'cem.application.shared.session/1:' + namespace;
    const storage = page.sessionStorage;
    if (!storage) throw new DOMException('Page session storage unavailable', 'InvalidStateError');
    function checkProvider() {
        if (page.sessionStorage !== storage) throw new DOMException('Page session provider changed', 'InvalidStateError');
    }
    const observers = new Set<StoreObserver>();
    function publish(change: StorePublication) {
        for (const observer of [...observers]) observer.deliver(change);
    }
    function decode(saved: string | null): Snapshot {
        const snapshot: Snapshot = saved === null ? [crypto.randomUUID(), []] : JSON.parse(saved);
        if (!Array.isArray(snapshot) || typeof snapshot[0] !== 'string' || !Array.isArray(snapshot[1]))
            throw new DOMException('Invalid named session store snapshot', 'DataError');
        return snapshot;
    }
    let pending = Promise.resolve();
    const transact: SharedStoreProvider['transact'] = (_mode, check, work) => {
        const operation = pending.then(async () => {
            check();
            checkProvider();
            const saved = storage.getItem(key);
            const snapshot = decode(saved);
            let mutation: StoreMutation | undefined;
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
                changed(change) { mutation = change; },
                clear() {
                    entries.clear();
                    epoch = crypto.randomUUID();
                    dirty = true;
                },
            });
            check();
            checkProvider();
            if (dirty) storage.setItem(key, JSON.stringify([epoch, Array.from(entries)]));
            if (mutation) publish(mutation);
            return result;
        });
        // A denied or failed operation must release the queue for the next grant.
        pending = operation.then(() => undefined, () => undefined);
        return operation;
    };

    // CEM commits use the top page's Storage object. Native storage events there
    // come from other documents and carry no trustworthy logical mutation details.
    function reconcile() {
        pending = pending.then(() => {
            if (!observers.size) return;
            checkProvider();
            const [, entries] = decode(storage.getItem(key));
            const snapshot: [string, string][] = [];
            for (const [name, entry] of entries) if (entry.value !== null) snapshot.push([name, entry.value]);
            publish({ snapshot });
        }).catch(reportError);
    }
    function external(event: StorageEvent) {
        if (event.storageArea === storage && (event.key === null || event.key === key)) reconcile();
    }
    function onPageShow(event: PageTransitionEvent) { if (event.persisted) reconcile(); }

    async function observe(observer: StoreObserver) {
        checkProvider();
        observers.add(observer);
        page.addEventListener('storage', external);
        page.addEventListener('pageshow', onPageShow);
        observer.signal.addEventListener('abort', () => {
            observers.delete(observer);
            if (!observers.size) {
                page.removeEventListener('storage', external);
                page.removeEventListener('pageshow', onPageShow);
            }
        }, { once: true });
    }
    return { transact, observe };
}
