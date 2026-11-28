import { pageLocalStorage, privateStorage, type CemApplicationStorage, type ResourceStorage } from './internal/runtime-support/browser-storage.js';
export type { CemApplicationStorage, ResourceStorage } from './internal/runtime-support/browser-storage.js';

/** Private providers for a mounted application. Symbols share the same owner across
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

export class ApplicationLocation extends EventTarget {
    constructor(public href: string) { super(); }
    write(method: string, src: string): void {
        const next = new URL(method === 'location.hash' ? this.href : src, this.href);
        if (method === 'location.hash') next.hash = src;
        if (method.startsWith('history.') && next.origin !== new URL(this.href).origin)
            throw new DOMException('History URL must have the same origin', 'SecurityError');
        if (next.href === this.href) return;
        this.href = next.href;
        queueMicrotask(() => this.dispatchEvent(new Event('change')));
    }
}

class ApplicationResources {
    private readonly stores = new Map<'local' | 'session', ResourceStorage>();
    constructor(readonly document: Document,
        readonly path: readonly Segment[], readonly location: ApplicationLocation | undefined,
        readonly live: () => boolean) {}
    storage(kind: 'local' | 'session'): ResourceStorage {
        if (!this.live()) throw Error('cem.application.owner-changed');
        let store = this.stores.get(kind);
        if (!store) {
            const window = this.document.defaultView;
            if (!window) throw Error('cem.application.storage-unavailable');
            store = privateStorage(window, kind, JSON.stringify(this.path), this.live);
            this.stores.set(kind, store);
        }
        return store;
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
    const address = initialAddress(doc);
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
    const resources = new ApplicationResources(doc, path, pageApp ? undefined : new ApplicationLocation(address),
        () => slots.get(key)?.deref() === root && root.isConnected && root.ownerDocument === doc
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

/** Resolve a store for a host-admitted service at the application boundary.
 * No cem-element, declaration registration or renderer is required. */
export function getCemApplicationStorage(application: Element, kind: 'local' | 'session' = 'local'): CemApplicationStorage {
    if (!application.matches(applicationBoundary) || !application.isConnected) throw Error('cem.application.boundary-required');
    if (kind !== 'local' && kind !== 'session') throw Error('cem.application.storage-kind');
    return admit(application).storage(kind);
}

/** A declarative reader uses the owning app's service. A denied app never resolves to page storage. */
export function resolveLocalStorage(consumer: Element): ResourceStorage | null {
    try {
        const app = applicationResources(consumer);
        return app ? app.storage('local') : pageLocalStorage(consumer.ownerDocument.defaultView);
    } catch { return null; }
}
