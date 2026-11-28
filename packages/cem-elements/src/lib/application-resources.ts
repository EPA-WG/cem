import { checkNetwork, denyNetwork } from './internal/runtime-support/application-network-policy.js';
import { ApplicationBrowserPolicy, bindGroupStorage, bindLocation, LogicalLocation,
    type ApplicationLocation, type BrowserGroup, type CemBrowserServiceGrant } from './internal/runtime-support/application-browser-policy.js';
import { sessionSharedStore } from './internal/runtime-support/session-shared-store.js';
import { durableSharedStore } from './internal/runtime-support/durable-shared-store.js';
import { bindSharedStore, type CemApplicationSharedStore, type SharedStoreProvider } from './internal/runtime-support/shared-store.js';
import { pageLocalStorage, privateStorage, type CemApplicationStorage, type ResourceStorage } from './internal/runtime-support/browser-storage.js';
export type { ApplicationLocation, CemBrowserServiceGrant } from './internal/runtime-support/application-browser-policy.js';
export type { CemApplicationSharedStore, CemSharedStoreValue, CemSharedStoreChange } from './internal/runtime-support/shared-store.js';
export type { CemApplicationStorage, ResourceStorage } from './internal/runtime-support/browser-storage.js';

/** Resource providers for a mounted application. Symbols share the same owner across
 * independent runtime modules in same-origin frames without serializing authority. */
const mounted = Symbol.for('cem.elements.applicationResources.v1');
const claims = Symbol.for('cem.elements.applicationSlots.v1');
const pageAddress = Symbol.for('cem.elements.applicationPage.v1');
const applicationBoundary = 'cem-application';

type Boundary = Element & { [mounted]?: ApplicationResources };
type HostPage = Document & { [pageAddress]?: string };
type Container = (Document | Element) & { [claims]?: Map<string, WeakRef<Element>> };
type Segment = readonly [string, string, number?];

function embeddingElement(doc: Document): Element | null {
    return doc.defaultView?.frameElement ?? null;
}

function nearest(node: Element | null, doc: Document): Boundary | null {
    for (;;) {
        const boundary = node?.closest(applicationBoundary);
        if (boundary) return boundary;
        node = embeddingElement(doc);
        if (!node) return null;
        doc = node.ownerDocument;
    }
}

/** Count application slots across layout wrappers. A same-origin frame contributes its
 * root slots at its placement. Stop at each app so equal IDs under other owners are local. */
function childBoundaries(root: Document | Element): Boundary[] {
    const children: Boundary[] = [];
    const enclosing = root.nodeType === 1 ? root : null;
    for (const element of root.querySelectorAll(applicationBoundary + ', iframe')) {
        if ((element.parentElement?.closest(applicationBoundary) ?? null) !== enclosing) continue;
        if (element.localName === 'iframe') {
            const doc = (element as HTMLIFrameElement).contentDocument;
            if (doc) children.push(...childBoundaries(doc));
        } else children.push(element);
    }
    return children;
}

function initialAddress(doc: HostPage): string {
    const cached = doc[pageAddress];
    if (cached) return cached;
    const source = doc.URL === 'about:blank' || doc.URL === 'about:srcdoc' ? doc.baseURI : doc.URL;
    const url = new URL(source);
    url.hash = '';
    url.search = '';
    url.username = '';
    url.password = '';
    Object.defineProperty(doc, pageAddress, { value: url.href });
    return url.href;
}

class ApplicationResources {
    private readonly stores = new Map<'local' | 'session', ResourceStorage>();
    private writerId?: string;
    get sharedWriterId(): string { return this.writerId ??= crypto.randomUUID(); }
    private readonly sessionProviders = new Map<string, SharedStoreProvider>();
    readonly sharedStores = new Map<string, { issuer: ApplicationResources; store: CemApplicationSharedStore; lifetime: AbortController }>();

    private readonly groups = new Map<string, BrowserGroup>();
    private readonly privateLocation?: ApplicationLocation;
    private sharedLocation?: { signal: AbortSignal; view: ApplicationLocation };
    private readonly sharedStorageViews = new Map<'local' | 'session', { signal: AbortSignal; view: ResourceStorage }>();

    constructor(readonly document: Document, readonly parent: ApplicationResources | null,
        readonly path: readonly Segment[], readonly policy: ApplicationBrowserPolicy, pageApp: boolean,
        readonly live: () => boolean) {
        if (!(pageApp && policy.pageProjection)) this.privateLocation = bindLocation(
            new LogicalLocation(initialAddress(document)), true, () => live() && policy.isPrivate('location'));
    }

    get location(): ApplicationLocation | undefined {
        const binding = this.policy.resolve('location');
        if (!binding) return this.privateLocation;
        const signal = binding.lifetime.signal;
        if (this.sharedLocation?.signal !== signal) this.sharedLocation = { signal,
            view: bindLocation(binding.group.location, binding.grant.location === 'read-write',
                () => this.live() && !signal.aborted, signal) };
        return this.sharedLocation.view;
    }

    browserGroup(id: string): BrowserGroup {
        let group = this.groups.get(id);
        if (!group) {
            // A tagged group segment cannot collide with a private app ID or the async named-store namespace.
            const namespace = JSON.stringify([...this.path, ['browser-group', id]]);
            const stores = new Map<'local' | 'session', ResourceStorage>();
            group = { location: new LogicalLocation(initialAddress(this.document)), storage: kind => {
                let store = stores.get(kind);
                if (!store) {
                    store = this.partition(kind, namespace, this.live);
                    stores.set(kind, store);
                }
                return store;
            } };
            this.groups.set(id, group);
        }
        return group;
    }

    sharedProvider(kind: 'local' | 'session', storeId: string): SharedStoreProvider {
        const window = this.document.defaultView;
        if (!window) throw new DOMException('Shared store provider unavailable', 'InvalidStateError');
        const namespace = JSON.stringify([this.path, storeId]);
        if (kind === 'local') return durableSharedStore(window, namespace);
        let provider = this.sessionProviders.get(storeId);
        if (!provider) {
            const page = window.top;
            if (!page) throw new DOMException('Page session unavailable', 'InvalidStateError');
            provider = sessionSharedStore(page, namespace);
            this.sessionProviders.set(storeId, provider);
        }
        return provider;
    }
    storage(kind: 'local' | 'session'): ResourceStorage {
        if (!this.live()) throw Error('cem.application.owner-changed');
        const family = kind === 'local' ? 'local-storage' : 'session-storage';
        const binding = this.policy.resolve(family);
        if (binding) {
            const signal = binding.lifetime.signal;
            let cached = this.sharedStorageViews.get(kind);
            if (cached?.signal !== signal) {
                cached = { signal, view: bindGroupStorage(binding.group.storage(kind),
                    binding.grant[family] === 'read-write', () => this.live() && !signal.aborted, signal) };
                this.sharedStorageViews.set(kind, cached);
            }
            return cached.view;
        }
        let store = this.stores.get(kind);
        if (!store) {
            store = this.partition(kind, JSON.stringify(this.path), () => this.live() && this.policy.isPrivate(family));
            this.stores.set(kind, store);
        }
        return store;
    }

    private partition(kind: 'local' | 'session', namespace: string, live: () => boolean) {
        const owner = this.document.defaultView;
        const provider = kind === 'session' ? owner?.top : owner;
        if (!provider) throw Error('cem.application.storage-unavailable');
        return privateStorage(provider, kind, namespace, live);
    }
}

function admit(root: Boundary): ApplicationResources {
    const existing = root[mounted];
    if (existing) {
        if (!existing.live()) throw Error('cem.application.owner-changed');
        return existing;
    }
    const doc = root.ownerDocument;
    const parentRoot = nearest(root.parentElement, doc);
    const parent = parentRoot ? admit(parentRoot) : null;
    let page = doc;
    const frames: { frame: HTMLIFrameElement; document: Document }[] = [];
    for (let frame = embeddingElement(page); frame; frame = embeddingElement(page)) {
        frames.push({ frame: frame as HTMLIFrameElement, document: page });
        page = frame.ownerDocument;
    }
    if (page.defaultView && page.defaultView !== page.defaultView.top) throw Error('cem.application.embedding-provider-required');
    const container: Container = parentRoot ?? page;
    const peers = new Set(childBoundaries(container));
    if (!peers.has(root)) throw Error('cem.application.slot-unavailable');
    const slots = container[claims] ?? new Map<string, WeakRef<Element>>();
    if (!container[claims]) Object.defineProperty(container, claims, { value: slots });
    for (const [name, owner] of slots) if (!owner.deref()) slots.delete(name);
    const occupied = (segment: Segment) => {
        const owner = slots.get(JSON.stringify(segment))?.deref();
        return owner !== undefined && peers.has(owner);
    };
    const policy = new ApplicationBrowserPolicy(root);
    let segment: Segment;
    if (root.id) {
        segment = ['id', root.id];
        if (Array.from(peers).some(peer => peer !== root && peer.id === root.id)) throw Error('cem.application.duplicate-id');
    } else {
        const origin = doc.defaultView?.origin;
        if (!origin || origin === 'null') segment = ['ephemeral', crypto.randomUUID()];
        else {
            const anonymous = Array.from(peers).filter(peer => !peer.id && peer.ownerDocument.defaultView?.origin === origin);
            let position = anonymous.indexOf(root) + 1;
            // An insertion or reorder cannot take a previously admitted live app's slot.
            while (occupied(['slot', origin, position])) position++;
            segment = ['slot', origin, position];
        }
    }
    const path: Segment[] = [...(parent?.path ?? [['page', initialAddress(page)] as Segment]), segment];
    if (occupied(segment)) throw Error('cem.application.occupied-slot');
    const key = JSON.stringify(segment);
    const pageApp = !parent && doc.defaultView === doc.defaultView?.top && peers.size === 1;
    const resources = new ApplicationResources(doc, parent, path, policy, pageApp,
        () => slots.get(key)?.deref() === root && root.isConnected && root.ownerDocument === doc
            && page.defaultView?.document === page
            && nearest(root.parentElement, doc) === parentRoot && (!parent || parent.live())
            && frames.every(edge => edge.frame.isConnected && edge.frame.contentDocument === edge.document));
    slots.set(key, new WeakRef(root));
    Object.defineProperty(root, mounted, { value: resources });
    return resources;
}

export function applicationResources(instance: Element): ApplicationResources | null {
    const root = nearest(instance, instance.ownerDocument);
    return root && admit(root);
}

function requireApplication(application: Element): ApplicationResources {
    if (!application.matches(applicationBoundary) || !application.isConnected) throw Error('cem.application.boundary-required');
    return admit(application);
}

/** Publish an issuer-owned virtual group to one direct child. Aliases and origins carry no authority. */
export function grantCemApplicationBrowserServices(owner: Element, recipient: Element, grant: CemBrowserServiceGrant): { revoke(): void } {
    const issuer = requireApplication(owner);
    const target = requireApplication(recipient);
    if (target.parent !== issuer) throw new DOMException('A browser service export requires the direct container', 'NotAllowedError');
    return target.policy.export(grant, id => issuer.browserGroup(id));
}

/** Trusted host operation. A root ceiling is supplied out of band. Child exports
 * come from the immediate issuer's admission snapshot and require upstream delegation. */
export function grantCemApplicationNetwork(owner: Element | null, recipient: Element, ceiling?: Element): { revoke(): void } {
    const target = requireApplication(recipient);
    if (owner === null) {
        if (target.parent || !ceiling || ceiling.getAttribute('to') !== recipient.id
            || ceiling.getAttribute('scope') !== target.policy.scope) return denyNetwork();
        return target.policy.network.install(ceiling);
    }
    const issuer = requireApplication(owner);
    const upstream = issuer.policy.network.binding;
    if (target.parent !== issuer || ceiling || !upstream) return denyNetwork();
    return target.policy.network.install(issuer.policy.network.exportFor(recipient.id, target.policy.scope), upstream);
}

/** Pin the initiating consumer, its mounted owner and every issuing generation before resolving a URL. */
export function applicationNetworkLease(consumer: Element) {
    const owner = applicationResources(consumer);
    if (!owner) return null;
    const grant = owner.policy.network.binding;
    if (!grant) return denyNetwork();
    const ownsConsumer = () => consumer.isConnected && applicationResources(consumer) === owner;
    const current = () => {
        grant.signal.throwIfAborted();
        if (!ownsConsumer()) denyNetwork();
    };
    current();
    return { signal: grant.signal, current, ownsConsumer,
        check(url: URL, method: string, credentials: RequestCredentials, following = false, transport: 'http' | 'websocket' = 'http') {
            current();
            checkNetwork(grant, { url, method, credentials, transport }, following);
        } };
}

export interface CemSharedStoreGrant {
    /** Storage lifetime. Omission preserves the durable local store. */
    readonly kind?: 'local' | 'session';
    /** Stable ID in the granting application's partition for this kind. */
    readonly storeId: string;
    /** Alias exposed only to this recipient. */
    readonly scope: string;
    readonly read: boolean;
    readonly write: boolean;
    /** Opt in to change delivery for readable keys. */
    readonly subscribe?: boolean;
    /** Omit for the whole store. A key-limited binding cannot clear the store. */
    readonly keys?: readonly string[];
}

function encloses(issuer: ApplicationResources, recipient: ApplicationResources): boolean {
    for (let app: ApplicationResources | null = recipient; app; app = app.parent) {
        if (app === issuer) return true;
    }
    return false;
}

/** Trusted host operation. Reissuing an alias replaces its current binding.
 * The returned revoker only applies to the generation it issued. */
export function grantCemApplicationSharedStore(owner: Element, recipient: Element, grant: CemSharedStoreGrant): { revoke(): void } {
    const issuer = requireApplication(owner);
    const target = requireApplication(recipient);
    const { storeId, scope, read, write, keys, subscribe = false, kind = 'local' } = grant;
    if (typeof storeId !== 'string' || !storeId || typeof scope !== 'string' || !scope
        || (kind !== 'local' && kind !== 'session')
        || typeof read !== 'boolean' || typeof write !== 'boolean' || (!read && !write)
        || typeof subscribe !== 'boolean' || (subscribe && !read)
        || (keys !== undefined && (!Array.isArray(keys) || !keys.every(key => typeof key === 'string'))))
        throw new TypeError('Invalid shared store grant');
    const current = target.sharedStores.get(scope);
    if (!encloses(issuer, target) || (current && !encloses(issuer, current.issuer)))
        throw new DOMException('Shared store grant requires the owning application', 'NotAllowedError');
    const provider = issuer.sharedProvider(kind, storeId);
    const bound = () => target.sharedStores.get(scope)?.store === store;
    const lifetime = new AbortController();
    const store: CemApplicationSharedStore = bindSharedStore(provider,
        { read, write, subscribe, scope, writer: target.sharedWriterId, signal: lifetime.signal },
        keys && new Set(keys), () => bound() && target.live() && issuer.live());
    target.sharedStores.set(scope, { issuer, store, lifetime });
    current?.lifetime.abort();
    return Object.freeze({ revoke() {
        if (bound()) target.sharedStores.delete(scope);
        lifetime.abort();
    } });
}

/** Look up the named grant on this exact application, including across a same-origin frame. */
export function getCemApplicationSharedStore(application: Element, scope: string): CemApplicationSharedStore {
    const binding = requireApplication(application).sharedStores.get(scope);
    if (!binding) throw new DOMException('Shared store scope has no grant', 'NotAllowedError');
    return binding.store;
}

/** Resolve a store for a host-admitted service at the application boundary.
 * No cem-element, declaration registration or renderer is required. */
export function getCemApplicationStorage(application: Element, kind: 'local' | 'session' = 'local'): CemApplicationStorage {
    const app = requireApplication(application);
    if (kind !== 'local' && kind !== 'session') throw Error('cem.application.storage-kind');
    return app.storage(kind);
}

/** A declarative reader uses the owning app's service. A denied app never resolves to page storage. */
export function resolveLocalStorage(consumer: Element): ResourceStorage | null {
    try {
        const app = applicationResources(consumer);
        return app ? app.storage('local') : pageLocalStorage(consumer.ownerDocument.defaultView);
    } catch { return null; }
}
