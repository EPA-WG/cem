/** Keyed access supplied to application services and declarative resource consumers. */
export interface CemApplicationStorage {
    getItem(key: string): string | null;
    setItem(key: string, value: string): void;
    removeItem(key: string): void;
    /** Invalidate readers of this key, including readers in the writing app. */
    subscribe(key: string, notify: () => void): () => void;
}

export interface ResourceStorage extends CemApplicationStorage {
    readonly readOnly?: boolean;
    /** Fence checkpoint recovery if the backing storage methods change. */
    pinMethods(): () => boolean;
}

const pageChange = 'cem-local-storage';
const privatePrefix = 'cem.application.storage/1:';
const tracked = new WeakSet<Window>();
const revisions = new WeakMap<Window, number>();
const pageStores = new WeakMap<Storage, ResourceStorage>();

function publishPageChange(window: Window, key: string | null, value: string | null) {
    if (key?.startsWith(privatePrefix)) return;
    revisions.set(window, (revisions.get(window) ?? 0) + 1);
    window.dispatchEvent(new CustomEvent(pageChange, { detail: { key, value } }));
}

/** Preserve live legacy page bindings to writes made by host code. Apps have
 * their own notification target and need no interception of browser methods. */
function trackPageStorage(window: Window, storage: Storage): void {
    if (tracked.has(window)) return;
    const { setItem, removeItem, clear } = storage;
    try {
        storage.setItem = function(key, value) {
            setItem.call(storage, key, value);
            publishPageChange(window, String(key), String(value));
        };
        storage.removeItem = function(key) {
            removeItem.call(storage, key);
            publishPageChange(window, String(key), null);
        };
        storage.clear = function() {
            clear.call(storage);
            publishPageChange(window, null, null);
        };
        tracked.add(window);
    } catch { /* Immutable host methods still allow native cross-document events. */ }
}

function bindStorage(window: Window, backing: Storage, current: () => boolean, prefix: string): ResourceStorage {
    const changes = new EventTarget();
    function requireBinding() {
        if (!current()) throw Error('cem.application.storage-unavailable');
    }
    function write(key: string, value: string | null) {
        requireBinding();
        const physicalKey = prefix + key;
        if (prefix && backing.getItem(physicalKey) === value) return;
        const revision = revisions.get(window);
        if (value === null) backing.removeItem(physicalKey);
        else backing.setItem(physicalKey, value);
        if (prefix) changes.dispatchEvent(new CustomEvent('change', { detail: key }));
        else if (revisions.get(window) === revision) publishPageChange(window, key, value);
    }
    return Object.freeze({
        getItem(key: string) { requireBinding(); return backing.getItem(prefix + key); },
        setItem(key: string, value: string) { write(String(key), String(value)); },
        removeItem(key: string) { write(String(key), null); },
        subscribe(key: string, notify: () => void) {
            requireBinding();
            const physicalKey = prefix + key;
            const external = (event: StorageEvent) => {
                if ((!prefix || event.storageArea === backing) && (event.key === null || event.key === physicalKey) && current()) notify();
            };
            const target = prefix ? changes : window;
            const type = prefix ? 'change' : pageChange;
            const local = (event: Event) => {
                const changed = prefix ? (event as CustomEvent<string>).detail : (event as CustomEvent).detail?.key;
                if ((changed == null || changed === key) && current()) notify();
            };
            if (!prefix) trackPageStorage(window, backing);
            target.addEventListener(type, local);
            window.addEventListener('storage', external);
            return () => {
                target.removeEventListener(type, local);
                window.removeEventListener('storage', external);
            };
        },
        pinMethods() {
            const { getItem, setItem, removeItem } = backing;
            return () => current() && backing.getItem === getItem && backing.setItem === setItem && backing.removeItem === removeItem;
        },
    });
}

export function privateStorage(window: Window, kind: 'local' | 'session', namespace: string, live: () => boolean): ResourceStorage {
    const property = kind === 'local' ? 'localStorage' : 'sessionStorage';
    const area = window[property];
    if (!area) throw Error('cem.application.storage-unavailable');
    return bindStorage(window, area, () => live() && window[property] === area, privatePrefix + namespace + ':');
}

/** Only used for consumers outside an application. */
export function pageLocalStorage(window: Window | null): ResourceStorage | null {
    if (!window) return null;
    const area = window.localStorage;
    let binding = pageStores.get(area);
    if (!binding) {
        binding = bindStorage(window, area, () => window.localStorage === area, '');
        pageStores.set(area, binding);
    }
    return binding;
}
