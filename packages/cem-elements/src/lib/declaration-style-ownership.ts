import {
    assertCemDeclarationScopeActive, onCemDeclarationScopeDispose, type CemDeclarationScope,
} from './declaration-scope.js';

import type { CemProcessingStylesheetResult } from './internal/runtime-support/processing-host.js';
import { declarationStylesheetAttributes } from './declaration-style-markup.js';
import { deferStylesheetNotifications, stylesheetNotificationsDeferred,
    notifyStylesheetLifecycle as runNotifications } from './internal/runtime-support/stylesheet-notifications.js';
import type { PreparedPatchFrames, RenderRevision, PatchFramesApplyDiagnostic } from './projection.js';
import { renderRevisionKey } from './projection.js';
import type { PreparedInstanceStylesheets } from './internal/runtime-support/instance-stylesheet-installation.js';

/** A complete native occurrence; CSS is never parsed by the installation layer. */
export interface CemOwnedStylesheet<TScope = { kind: 'private' } | { kind: 'shared'; name: string }> {
    index: number;
    scope: TScope;
    output: Extract<CemProcessingStylesheetResult, { status: 'ready' }>;
}

export interface CemStylesheetConsumerLease<TScope = { kind: 'private' } | { kind: 'shared'; name: string }> {
    /** Abort pending loads when this generation disconnects, changes or is disposed. */
    readonly signal: AbortSignal;
    /** Commits once, only while this connected consumer generation remains current. */
    commit(outputs: readonly CemOwnedStylesheet<TScope>[], release: () => void): boolean;
    release(): void;
}

export interface DeclarationStylesheetCommit<TScope = { kind: 'private' } | { kind: 'shared'; name: string }> {
    lease: CemStylesheetConsumerLease<TScope>;
    outputs: readonly CemOwnedStylesheet<TScope>[];
    /** Optional preparation guard; must be synchronous and side-effect free. */
    isCurrent?(): boolean;
    release(): void;
}

export interface DeclarationStylesheetPatchResult {
    status: 'applied' | 'rejected' | 'recovery-required';
    diagnostics: readonly PatchFramesApplyDiagnostic[];
    errors: readonly unknown[];
}

const leases = new WeakMap<CemStylesheetConsumerLease, { owner: DeclarationStyleOwnership; consumer: Consumer }>();

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
    previous?: Consumer;
}

const contextAttribute = 'data-cem-css-context';
// Shared source declarations can qualify the same host. One release must not
// remove a marker still held by another declaration's consumer.
const markers = new WeakMap<HTMLElement, { value: string; consumers: Set<Consumer> }>();

/** Restore reserved host state without treating a context marker as authored data. */
export function reconcileStylesheetContextMarker(element: HTMLElement): void {
    const expected = markers.get(element)?.value ?? null;
    if (element.getAttribute(contextAttribute) === expected) return;
    if (expected === null) element.removeAttribute(contextAttribute);
    else element.setAttribute(contextAttribute, expected);
}

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
    private readonly serverStyles = new Set<HTMLStyleElement>();
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
        for (const style of element.querySelectorAll<HTMLStyleElement>(':scope > style[data-cem-declaration-style][data-cem-style-key]')) {
            this.serverStyles.add(style);
        }
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
        return this.beginGeneration(element, scope, false);
    }

    /** Keep the committed generation until replacement commits; releasing a pending lease restores it. */
    stageConsumer(element: HTMLElement, scope: CemDeclarationScope): CemStylesheetConsumerLease {
        return this.beginGeneration(element, scope, true);
    }

    private beginGeneration(element: HTMLElement, scope: CemDeclarationScope, staged: boolean): CemStylesheetConsumerLease {
        assertCemDeclarationScopeActive(scope);
        assertCemDeclarationScopeActive(this.processingScope);
        const observed = documents.get(this.document);
        if (observed) reconcileDocument(observed.registrations, observed.observer.takeRecords());
        const previous = this.currentConsumers.get(element);
        const committed = previous?.committed ? previous : previous?.previous;
        const consumer: Consumer = { element: new WeakRef(element), scope, sets: [], marker: null,
            committed: false, abort: new AbortController(), previous: staged ? committed : undefined };
        this.currentConsumers.set(element, consumer);
        this.consumers.add(consumer);
        // Publish the new generation before notifying old cancellation handlers;
        // a handler may synchronously replace this generation again.
        if (previous && (!staged || previous !== committed)) this.dropConsumer(previous);
        if (!staged && committed && committed !== previous) this.dropConsumer(committed);
        this.observeScope(scope);
        this.reconcile();
        const lease: CemStylesheetConsumerLease = {
            signal: consumer.abort.signal,
            commit: (outputs, release) => this.commitConsumer(consumer, outputs, release),
            release: () => { this.dropConsumer(consumer); this.reconcile(); },
        };
        leases.set(lease, { owner: this, consumer });
        return lease;
    }

    private commitConsumer(consumer: Consumer, outputs: readonly CemOwnedStylesheet[], release: () => void): boolean {
        const observed = documents.get(this.document);
        if (observed) reconcileDocument(observed.registrations, observed.observer.takeRecords());
        this.reconcile();
        if (!this.validConsumer(consumer, outputs, new Set(consumer.previous ? [consumer.previous] : []))) {
            if (!consumer.committed) this.dropConsumer(consumer);
            release();
            return false;
        }
        const previous = this.publishConsumer(consumer, outputs, release);
        const element = consumer.element.deref();
        if (element) reconcileStylesheetContextMarker(element);
        if (previous) this.dropConsumer(previous);
        this.reconcile();
        return this.consumers.has(consumer);
    }

    private validConsumer(consumer: Consumer, outputs: readonly CemOwnedStylesheet[], replacing: Set<Consumer>): boolean {
        const element = consumer.element.deref();
        if (!element || !element.isConnected || element.ownerDocument !== this.document || !active(consumer.scope)
            || !active(this.processingScope) || !this.consumers.has(consumer)
            || this.currentConsumers.get(element) !== consumer || consumer.committed) return false;
        const contexts = new Set(outputs.map(s => s.output.identity.contextMarker).filter((s): s is string => s !== null));
        const marker = contexts.values().next().value ?? null;
        const held = markers.get(element);
        const indices = new Set(outputs.map(s => s.index));
        // Validate the entire set before publishing either styles or a marker.
        const invalid = contexts.size > 1 || indices.size !== outputs.length ||
            outputs.some(s => !Number.isSafeInteger(s.index) || s.index < 0) ||
            (marker !== null && held !== undefined && held.value !== marker
                && Array.from(held.consumers).some(member => !replacing.has(member))) || outputs.some(source => {
                const existing = this.derived.get(source.output.identity.cacheKey)?.source;
                return existing && (existing.index !== source.index || existing.output.css !== source.output.css ||
                    existing.output.identity.contextMarker !== source.output.identity.contextMarker ||
                    existing.scope.kind !== source.scope.kind || (existing.scope.kind === 'shared' &&
                        source.scope.kind === 'shared' && existing.scope.name !== source.scope.name));
            });
        return !invalid;
    }

    private publishConsumer(consumer: Consumer, outputs: readonly CemOwnedStylesheet[], release: () => void): Consumer | undefined {
        const element = consumer.element.deref();
        if (!element) return undefined; // Validated immediately before publication.
        const marker = outputs.find(source => source.output.identity.contextMarker !== null)?.output.identity.contextMarker ?? null;
        const held = markers.get(element);
        consumer.committed = true;
        consumer.release = release;
        consumer.marker = marker;
        for (const source of outputs) {
            const key = source.output.identity.cacheKey;
            let set = this.derived.get(key);
            if (!set) {
                const style = this.takeServerStyle(source) ?? this.document.createElement('style');
                for (const [name, value] of Object.entries(declarationStylesheetAttributes(source))) style.setAttribute(name, value);
                if (style.textContent !== source.output.css) style.textContent = source.output.css;
                set = { source, style, consumers: new Set() };
                this.derived.set(key, set);
            }
            set.consumers.add(consumer);
            consumer.sets.push(set);
        }
        if (marker !== null) {
            const membership = held?.value === marker ? held : { value: marker, consumers: new Set<Consumer>() };
            membership.consumers.add(consumer);
            markers.set(element, membership);
        }
        const previous = consumer.previous;
        consumer.previous = undefined;
        return previous;
    }

    /** Publish a complete group for one host after validating every owner and marker holder. */
    static commitGroup(entries: readonly DeclarationStylesheetCommit[]): boolean {
        return this.publishGroup(entries);
    }

    /** Confirm that a transferred candidate is the live installed generation. */
    static isCurrentCommit(entry: DeclarationStylesheetCommit): boolean {
        const state = leases.get(entry.lease);
        const element = state?.consumer.element.deref();
        return !!state && !!element?.isConnected && element.ownerDocument === state.owner.document
            && state.consumer.committed && state.consumer.release === entry.release
            && state.owner.currentConsumers.get(element) === state.consumer
            && state.owner.consumers.has(state.consumer) && active(state.consumer.scope)
            && active(state.owner.processingScope) && !entry.lease.signal.aborted && entry.isCurrent?.() !== false;
    }

    /**
     * Admit declaration CSS, optional instance CSS and a prepared patch together.
     * Publish both CSS candidates before the DOM patch.
     * The revision reader must be synchronous and side-effect free. The caller owns
     * recovery and scheduling subsequent updates; this primitive does not roll back.
     */
    static commitGroupWithPatch(entries: readonly DeclarationStylesheetCommit[], patch: PreparedPatchFrames,
        currentRevision: () => RenderRevision, signal?: AbortSignal,
        instanceStyles?: PreparedInstanceStylesheets): DeclarationStylesheetPatchResult {
        let started = false;
        let applied = false;
        let revision: string | undefined;
        let diagnostics: readonly PatchFramesApplyDiagnostic[] = [];
        const errors: unknown[] = [];
        const nested = stylesheetNotificationsDeferred();
        try {
            deferStylesheetNotifications(() => {
                const live = this.publishGroup(entries, {
                    check: element => {
                        if (nested || signal?.aborted || !element || patch.container !== element) return false;
                        if (instanceStyles && (instanceStyles.element !== element || !instanceStyles.check())) return false;
                        try {
                            const current = currentRevision();
                            const result = patch.check(current);
                            diagnostics = result.diagnostics;
                            revision = renderRevisionKey(current);
                            return result.status === 'ready';
                        } catch (error) { errors.push(error); return false; }
                    },
                    start: () => { started = true; },
                    commit: () => {
                        if (signal?.aborted) return;
                        if (instanceStyles && !instanceStyles.commit()) return;
                        const result = patch.commit(currentRevision());
                        diagnostics = result.diagnostics;
                        applied = result.status === 'applied';
                    },
                });
                applied &&= live;
            });
            if (started && revision !== renderRevisionKey(currentRevision())) applied = false;
        } catch (error) { errors.push(error); }
        finally {
            patch.cancel();
            if (!started) instanceStyles?.cancelPreparation();
        }
        const live = entries.every(entry => {
            const state = leases.get(entry.lease);
            const element = state?.consumer.element.deref();
            return state && element?.isConnected && element.ownerDocument === state.owner.document
                && active(state.consumer.scope) && active(state.owner.processingScope)
                && state.owner.currentConsumers.get(element) === state.consumer
                && state.owner.consumers.has(state.consumer) && !entry.lease.signal.aborted && entry.isCurrent?.() !== false;
        });
        return { status: started ? applied && live && !signal?.aborted && errors.length === 0
            && (!instanceStyles || instanceStyles.isPublished())
            ? 'applied' : 'recovery-required' : 'rejected', diagnostics, errors };
    }

    private static publishGroup(entries: readonly DeclarationStylesheetCommit[],
        publication?: { check(element: HTMLElement | undefined): boolean; start(): void; commit(): void }): boolean {
        if (!entries.length) return false;
        const members = entries.map(entry => ({ entry, state: leases.get(entry.lease) }));
        const owners = new Set(members.flatMap(member => member.state ? [member.state.owner] : []));
        // Drain disconnections before pure validation; cancellation callbacks may supersede candidates.
        for (const owner of owners) {
            const observed = documents.get(owner.document);
            if (observed) reconcileDocument(observed.registrations, observed.observer.takeRecords());
            owner.reconcile();
        }
        const element = members[0].state?.consumer.element.deref();
        const admitted = publication?.check(element) ?? true;
        const replacing = new Set(members.flatMap(member => member.state?.consumer.previous ? [member.state.consumer.previous] : []));
        const contexts = new Set(entries.flatMap(entry => entry.outputs.map(source => source.output.identity.contextMarker))
            .filter((value): value is string => value !== null));
        const valid = admitted && element && owners.size === entries.length && contexts.size <= 1 && members.every(({ state, entry }) =>
            state && entry.isCurrent?.() !== false && state.consumer.element.deref() === element
                && state.owner.validConsumer(state.consumer, entry.outputs, replacing));
        if (!valid) {
            const notifications: Array<() => void> = [];
            for (const { state } of members) if (state && !state.consumer.committed) state.owner.dropConsumer(state.consumer, notifications);
            for (const owner of owners) owner.reconcile();
            if (element) reconcileStylesheetContextMarker(element);
            for (const release of new Set(members.filter(({ state, entry }) =>
                !state?.consumer.committed || state.consumer.release !== entry.release).map(({ entry }) => entry.release))) notifications.push(release);
            runNotifications(notifications);
            return false;
        }
        const notifications: Array<() => void> = [];
        const previous: Array<{ owner: DeclarationStyleOwnership; consumer: Consumer }> = [];
        try {
            publication?.start();
            for (const { state, entry } of members) {
                if (!state) continue;
                const old = state.owner.publishConsumer(state.consumer, entry.outputs, entry.release);
                if (old) previous.push({ owner: state.owner, consumer: old });
            }
            // Retire internal ownership before notifying any old load or host attribute observer.
            for (const { owner, consumer } of previous) owner.dropConsumer(consumer, notifications);
            for (const owner of owners) owner.reconcile();
            reconcileStylesheetContextMarker(element);
            publication?.commit();
        } finally {
            runNotifications(notifications);
        }
        return members.every(({ state }) => state && state.owner.consumers.has(state.consumer));
    }

    private takeServerStyle(source: CemOwnedStylesheet): HTMLStyleElement | undefined {
        const attributes = declarationStylesheetAttributes(source);
        let retained: HTMLStyleElement | undefined;
        for (const style of this.serverStyles) {
            if (style.getAttribute('data-cem-style-key') !== source.output.identity.cacheKey) continue;
            this.serverStyles.delete(style);
            if (!retained && Object.entries(attributes).every(([name, value]) => style.getAttribute(name) === value)
                && (source.scope.kind === 'shared' || !style.hasAttribute('data-cem-style-scope'))
                && !style.hasAttribute('media') && !style.hasAttribute('type') && !style.hasAttribute('title') && !style.disabled
                && style.textContent === source.output.css) retained = style;
            else style.remove();
        }
        return retained;
    }

    private dropConsumer(consumer: Consumer, notifications?: Array<() => void>): void {
        if (!this.consumers.delete(consumer)) return;
        const element = consumer.element.deref();
        if (element && this.currentConsumers.get(element) === consumer) {
            if (consumer.previous && this.consumers.has(consumer.previous)) this.currentConsumers.set(element, consumer.previous);
            else this.currentConsumers.delete(element);
        }
        consumer.previous = undefined;
        if (element && consumer.marker !== null) {
            const membership = markers.get(element);
            membership?.consumers.delete(consumer);
            if (membership && membership.consumers.size === 0) {
                if (!notifications && element.getAttribute(contextAttribute) === membership.value) element.removeAttribute(contextAttribute);
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
        const notify = () => { try { consumer.release?.(); } finally { consumer.abort.abort(); } };
        if (notifications) notifications.push(notify);
        else runNotifications([notify]);
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
        for (const style of this.serverStyles) {
            const owner = Array.from(this.owners).find(candidate => candidate.element.deref() === style.parentElement);
            if (!owner || !processingActive || !active(owner.scope) || (owner.connectedOnce && !liveOwners.includes(owner))) {
                style.remove();
                this.serverStyles.delete(style);
            }
        }
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
