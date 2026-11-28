import type { ResourceStorage } from './browser-storage.js';

type Isolation = 'private' | 'shared';
type Family = 'location' | 'local-storage' | 'session-storage';
const families: Family[] = ['location', 'local-storage', 'session-storage'];

/** Both profiles include live reads. Writing a shared route means navigating the virtual group URL. */
export type CemBrowserServiceAccess = 'read' | 'read-write';
export interface CemBrowserServiceGrant {
    readonly groupId: string;
    readonly scope?: string;
    readonly location?: CemBrowserServiceAccess;
    readonly 'local-storage'?: CemBrowserServiceAccess;
    readonly 'session-storage'?: CemBrowserServiceAccess;
}

export interface ApplicationLocation {
    readonly href: string;
    write(method: string, src: string): void;
    subscribe(listener: () => void): () => void;
}

export class LogicalLocation implements ApplicationLocation {
    private readonly changes = new EventTarget();
    constructor(private address: string) {}
    get href(): string { return this.address; }
    write(method: string, src: string): void {
        const next = new URL(method === 'location.hash' ? this.href : src, this.href);
        if (method === 'location.hash') next.hash = src;
        if (method.startsWith('history.') && next.origin !== new URL(this.href).origin)
            throw new DOMException('History URL must have the same origin', 'SecurityError');
        if (next.href === this.href) return;
        this.address = next.href;
        queueMicrotask(() => this.changes.dispatchEvent(new Event('change')));
    }
    subscribe(listener: () => void) {
        this.changes.addEventListener('change', listener);
        return () => this.changes.removeEventListener('change', listener);
    }
}

export interface BrowserGroup {
    readonly location: ApplicationLocation;
    storage(kind: 'local' | 'session'): ResourceStorage;
}

function denied(): never {
    throw new DOMException('Application browser service unavailable', 'NotAllowedError');
}

function isolation(value: string | null): Isolation {
    if (value !== 'private' && value !== 'shared') throw new TypeError('Invalid application isolation');
    return value;
}

function name(value: unknown): value is string {
    return typeof value === 'string' && !!value && value === value.trim();
}

/** One declaration snapshot per admission. Group handles belong to one recipient and one grant generation. */
export class ApplicationBrowserPolicy {
    readonly profile: Isolation;
    readonly scope: string | null;
    readonly pageProjection: boolean;
    private readonly overrides = new Map<Family, Isolation>();
    private readonly previouslyGranted = new Set<Family>();
    private binding?: { grant: CemBrowserServiceGrant; group: BrowserGroup; lifetime: AbortController };

    constructor(root: Element) {
        const presets = ['private', 'shared', 'isolation'].filter(key => root.hasAttribute(key));
        if (presets.length > 1) throw new TypeError('Declare one application isolation preset');
        const preset = presets[0];
        if ((preset === 'private' || preset === 'shared') && !['', preset, 'true'].includes(root.getAttribute(preset)!))
            throw new TypeError('Use a bare application isolation attribute');
        this.profile = isolation(preset === 'isolation' ? root.getAttribute(preset) : preset ?? 'private');
        const policies = Array.from(root.children).filter(node => node.localName === 'browser-policy' && node.namespaceURI === root.namespaceURI);
        if (policies.length > 1) throw new TypeError('Declare one direct browser-policy');
        const policy = policies[0];
        this.scope = policy?.getAttribute('scope') ?? null;
        if (this.scope !== null && !name(this.scope)) throw new TypeError('Invalid browser-policy scope');
        if (policy) {
            if (policy.childElementCount) throw new TypeError('Unsupported browser-policy child');
            for (const attr of policy.attributes) {
                if (attr.name === 'scope' || attr.namespaceURI === 'http://www.w3.org/2000/xmlns/') continue;
                const family = families.find(key => key === attr.name);
                if (!family) throw new TypeError('Unknown browser-policy service: ' + attr.name);
                this.overrides.set(family, isolation(attr.value));
            }
        }
        this.pageProjection = !preset && !this.overrides.has('location');
    }

    export(grant: CemBrowserServiceGrant, group: (id: string) => BrowserGroup) {
        if (!name(grant.groupId) || (grant.scope !== undefined && !name(grant.scope))
            || (grant.scope ?? null) !== this.scope || !families.some(key => grant[key] !== undefined)
            || Object.keys(grant).some(key => key !== 'scope' && key !== 'groupId' && !families.includes(key as Family)))
            throw new TypeError('Invalid browser service grant or recipient scope');
        for (const key of families) {
            const access = grant[key];
            if (access !== undefined && access !== 'read' && access !== 'read-write')
                throw new TypeError('Invalid browser service access');
        }
        const binding = { grant: { ...grant }, group: group(grant.groupId), lifetime: new AbortController() };
        const previous = this.binding;
        this.binding = binding;
        for (const key of families) if (grant[key]) this.previouslyGranted.add(key);
        previous?.lifetime.abort();
        return { revoke: () => {
            if (this.binding === binding) this.binding = undefined;
            binding.lifetime.abort();
        } };
    }

    /** A withdrawn family stays unavailable. Ordinary shared defaults use private services for unexported families. */
    resolve(key: Family) {
        if (this.isPrivate(key)) return undefined;
        if (this.binding?.grant[key]) return this.binding;
        return denied();
    }

    isPrivate(key: Family) {
        if ((this.overrides.get(key) ?? this.profile) === 'private') return true;
        return !this.overrides.has(key) && !this.previouslyGranted.has(key);
    }
}

function watch(subscribe: (listener: () => void) => () => void, listener: () => void,
    live: () => boolean, signal?: AbortSignal) {
    if (!live()) denied();
    const stop = subscribe(() => { if (live()) listener(); });
    const dispose = () => {
        stop();
        signal?.removeEventListener('abort', dispose);
    };
    signal?.addEventListener('abort', dispose, { once: true });
    return dispose;
}

export function bindLocation(source: ApplicationLocation, writable: boolean, live: () => boolean, signal?: AbortSignal): ApplicationLocation {
    return Object.freeze({
        get href() { return live() ? source.href : denied(); },
        write(method: string, src: string) {
            if (!live() || !writable) denied();
            source.write(method, src);
        },
        subscribe(listener: () => void) { return watch(fn => source.subscribe(fn), listener, live, signal); },
    });
}

export function bindGroupStorage(source: ResourceStorage, writable: boolean, live: () => boolean, signal: AbortSignal): ResourceStorage {
    return Object.freeze({
        readOnly: !writable,
        getItem(key: string) { return live() ? source.getItem(key) : denied(); },
        setItem(key: string, value: string) {
            if (!live() || !writable) denied();
            source.setItem(key, value);
        },
        removeItem(key: string) {
            if (!live() || !writable) denied();
            source.removeItem(key);
        },
        subscribe(key: string, listener: () => void) {
            return watch(fn => source.subscribe(key, fn), listener, live, signal);
        },
        pinMethods() {
            if (!live()) denied();
            const current = source.pinMethods();
            return () => live() && current();
        },
    });
}
