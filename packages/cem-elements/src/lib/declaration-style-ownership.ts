import {
    assertCemDeclarationScopeActive, onCemDeclarationScopeDispose, type CemDeclarationScope,
} from './declaration-scope.js';

import type { CemProcessingStylesheetResult } from './internal/runtime-support/processing-host.js';

/** A complete native occurrence; CSS is never parsed by the installation layer. */
export interface CemOwnedStylesheet {
    index: number;
    scope: { kind: 'private' } | { kind: 'shared'; name: string };
    output: Extract<CemProcessingStylesheetResult, { status: 'ready' }>;
}

export interface CemStylesheetConsumerLease {
    /** Abort pending loads when this generation disconnects, changes or is disposed. */
    readonly signal: AbortSignal;
    /** Commits once, only while this connected consumer generation remains current. */
    commit(outputs: readonly CemOwnedStylesheet[], release: () => void): boolean;
    release(): void;
}

interface DerivedSet {
    source: CemOwnedStylesheet;
    style: HTMLStyleElement;
    consumers: Set<Consumer>;
}

interface Consumer {
    element: WeakRef<HTMLElement>;
    scope: CemDeclarationScope;
    sets: DerivedSet[];
    marker: string | null;
    release?: () => void;
    committed: boolean;
    abort: AbortController;
}

const contextAttribute = 'data-cem-css-context';
// Shared source declarations can qualify the same host. One release must not
// remove a marker still held by another declaration's consumer.
const markers = new WeakMap<HTMLElement, { value: string; consumers: Set<Consumer> }>();

interface Owner {
    element: WeakRef<HTMLElement>;
    scope: CemDeclarationScope;
    connectedOnce: boolean;
}

interface DocumentOwnership {
    observer: MutationObserver;
    registrations: Set<WeakRef<DeclarationStyleOwnership>>;
}

const documents = new WeakMap<Document, DocumentOwnership>();

/** Browser registration survives removal; stylesheet ownership follows its live declarations. */
export class DeclarationStyleOwnership {
    private readonly owners = new Set<Owner>();
    private readonly observedScopes = new WeakSet<CemDeclarationScope>();
    private readonly mountedScopes = new WeakSet<CemDeclarationScope>();
    private currentOwner?: Owner;
    private readonly derived = new Map<string, DerivedSet>();
    private readonly consumers = new Set<Consumer>();
    private readonly currentConsumers = new WeakMap<HTMLElement, Consumer>();
    styles?: readonly HTMLStyleElement[];

    constructor(private readonly document: Document, private readonly processingScope: CemDeclarationScope) {
        let ownership = documents.get(document);
        if (!ownership && document.defaultView) {
            const registrations = new Set<WeakRef<DeclarationStyleOwnership>>();
            const observer = new document.defaultView.MutationObserver(records => reconcileDocument(registrations, records));
            observer.observe(document, { childList: true, subtree: true });
            ownership = { observer, registrations };
            documents.set(document, ownership);
        }
        ownership?.registrations.add(new WeakRef(this));
        this.observeScope(processingScope);
    }

    add(element: HTMLElement, scope: CemDeclarationScope, connectedOnce = element.isConnected): void {
        assertCemDeclarationScopeActive(scope);
        assertCemDeclarationScopeActive(this.processingScope);
        for (const owner of this.owners) {
            if (owner.element.deref() === element) {
                this.reconcile();
                return;
            }
        }
        this.owners.add({ element: new WeakRef(element), scope, connectedOnce });
        this.observeScope(scope);
        this.reconcile();
    }

    remove(element: HTMLElement): void {
        for (const owner of this.owners) {
            if (owner.element.deref() === element) this.owners.delete(owner);
        }
        this.reconcile();
    }

    canRemount(scope: CemDeclarationScope): boolean {
        const ownership = documents.get(this.document);
        if (ownership) reconcileDocument(ownership.registrations, ownership.observer.takeRecords());
        for (const owner of this.owners) {
            if (owner.scope !== scope) continue;
            const element = owner.element.deref();
            if (!element) continue;
            // A declaration registered before its first mount still reserves the binding.
            if ((element.isConnected && element.ownerDocument === this.document) || !owner.connectedOnce) return false;
        }
        return this.mountedScopes.has(scope);
    }

    beginConsumer(element: HTMLElement, scope: CemDeclarationScope): CemStylesheetConsumerLease {
        assertCemDeclarationScopeActive(scope);
        assertCemDeclarationScopeActive(this.processingScope);
        const observed = documents.get(this.document);
        if (observed) reconcileDocument(observed.registrations, observed.observer.takeRecords());
        const previous = this.currentConsumers.get(element);
        const consumer: Consumer = { element: new WeakRef(element), scope, sets: [], marker: null,
            committed: false, abort: new AbortController() };
        this.currentConsumers.set(element, consumer);
        this.consumers.add(consumer);
        // Publish the new generation before notifying old cancellation handlers;
        // a handler may synchronously replace this generation again.
        if (previous) this.dropConsumer(previous);
        this.observeScope(scope);
        this.reconcile();
        return {
            signal: consumer.abort.signal,
            commit: (outputs, release) => this.commitConsumer(consumer, outputs, release),
            release: () => { this.dropConsumer(consumer); this.reconcile(); },
        };
    }

    private commitConsumer(consumer: Consumer, outputs: readonly CemOwnedStylesheet[], release: () => void): boolean {
        const observed = documents.get(this.document);
        if (observed) reconcileDocument(observed.registrations, observed.observer.takeRecords());
        this.reconcile();
        const element = consumer.element.deref();
        if (!element || !this.consumers.has(consumer) || this.currentConsumers.get(element) !== consumer || consumer.committed) {
            if (!consumer.committed) this.dropConsumer(consumer);
            release();
            return false;
        }
        const contexts = new Set(outputs.map(s => s.output.identity.contextMarker).filter((s): s is string => s !== null));
        const marker = contexts.values().next().value ?? null;
        const held = markers.get(element);
        const indices = new Set(outputs.map(s => s.index));
        // Validate the entire set before publishing either styles or a marker.
        const invalid = contexts.size > 1 || indices.size !== outputs.length ||
            outputs.some(s => !Number.isSafeInteger(s.index) || s.index < 0) ||
            (marker !== null && held !== undefined && held.value !== marker) || outputs.some(source => {
                const existing = this.derived.get(source.output.identity.cacheKey)?.source;
                return existing && (existing.index !== source.index || existing.output.css !== source.output.css ||
                    existing.output.identity.contextMarker !== source.output.identity.contextMarker ||
                    existing.scope.kind !== source.scope.kind || (existing.scope.kind === 'shared' &&
                        source.scope.kind === 'shared' && existing.scope.name !== source.scope.name));
            });
        if (invalid) {
            this.dropConsumer(consumer);
            release();
            return false;
        }
        consumer.committed = true;
        consumer.release = release;
        consumer.marker = marker;
        for (const source of outputs) {
            const key = source.output.identity.cacheKey;
            let set = this.derived.get(key);
            if (!set) {
                const style = this.document.createElement('style');
                style.setAttribute('data-cem-declaration-style', source.scope.kind);
                if (source.scope.kind === 'shared') style.setAttribute('data-cem-style-scope', source.scope.name);
                style.textContent = source.output.css;
                set = { source, style, consumers: new Set() };
                this.derived.set(key, set);
            }
            set.consumers.add(consumer);
            consumer.sets.push(set);
        }
        if (marker !== null) {
            const membership = held ?? { value: marker, consumers: new Set<Consumer>() };
            membership.consumers.add(consumer);
            markers.set(element, membership);
            element.setAttribute(contextAttribute, marker);
        }
        this.reconcile();
        return true;
    }

    private dropConsumer(consumer: Consumer): void {
        if (!this.consumers.delete(consumer)) return;
        const element = consumer.element.deref();
        if (element && this.currentConsumers.get(element) === consumer) this.currentConsumers.delete(element);
        if (element && consumer.marker !== null) {
            const membership = markers.get(element);
            membership?.consumers.delete(consumer);
            if (membership && membership.consumers.size === 0) {
                if (element.getAttribute(contextAttribute) === membership.value) element.removeAttribute(contextAttribute);
                markers.delete(element);
            }
        }
        for (const set of consumer.sets) {
            set.consumers.delete(consumer);
            if (!set.consumers.size) {
                set.style.remove();
                this.derived.delete(set.source.output.identity.cacheKey);
            }
        }
        try { consumer.release?.(); } finally { consumer.abort.abort(); }
    }

    setStyles(styles: readonly HTMLStyleElement[]): void {
        this.styles = styles;
        this.reconcile();
    }

    reconcile(records: readonly MutationRecord[] = []): void {
        for (const consumer of this.consumers) {
            const element = consumer.element.deref();
            const removed = element && records.some(record =>
                Array.from(record.removedNodes).some(node => node === element || node.contains(element)));
            if (!element || !element.isConnected || element.ownerDocument !== this.document || removed ||
                !active(consumer.scope) || !active(this.processingScope)) this.dropConsumer(consumer);
        }
        const liveOwners: Owner[] = [];
        const processingActive = active(this.processingScope);
        for (const owner of this.owners) {
            const element = owner.element.deref();
            if (!element) {
                this.owners.delete(owner);
                continue;
            }
            const connected = element.isConnected && element.ownerDocument === this.document;
            owner.connectedOnce ||= connected || records.some(record =>
                Array.from(record.addedNodes).some(node => node === element || node.contains(element)));
            if (owner.connectedOnce) this.mountedScopes.add(owner.scope);
            if (connected && processingActive && active(owner.scope)) liveOwners.push(owner);
        }
        if (!this.currentOwner || !liveOwners.includes(this.currentOwner)) this.currentOwner = liveOwners[0];
        const element = this.currentOwner?.element.deref();
        for (const style of this.styles ?? []) {
            if (element) {
                if (style.parentElement !== element) element.append(style);
            } else {
                style.remove();
            }
        }
        const ordered = Array.from(this.derived.values()).sort((a, b) => a.source.index - b.source.index).map(s => s.style);
        if (!element) {
            for (const style of ordered) style.remove();
        } else {
            const mounted = Array.from(element.children).filter(child => ordered.includes(child as HTMLStyleElement));
            if (mounted.length !== ordered.length || mounted.some((style, index) => style !== ordered[index])) {
                for (const style of ordered) element.append(style);
            }
        }
    }

    private observeScope(scope: CemDeclarationScope): void {
        for (let current: CemDeclarationScope | null = scope; current; current = current.parent) {
            if (this.observedScopes.has(current)) continue;
            this.observedScopes.add(current);
            const registration = new WeakRef(this);
            onCemDeclarationScopeDispose(current, () => registration.deref()?.reconcile());
        }
    }
}

function active(scope: CemDeclarationScope): boolean {
    for (let current: CemDeclarationScope | null = scope; current; current = current.parent) {
        if (current.disposed) return false;
    }
    return true;
}

function reconcileDocument(registrations: Set<WeakRef<DeclarationStyleOwnership>>, records: readonly MutationRecord[]): void {
    for (const reference of registrations) {
        const ownership = reference.deref();
        if (ownership) ownership.reconcile(records);
        else registrations.delete(reference);
    }
}
