import type { CemDeclarationRegistrationIdentity } from './cem-elements.js';

/**
 * Opaque logical declaration-scope handle.
 *
 * Object identity is the scope identity. The handle deliberately carries no
 * `scopePolicyStamp`: declaration ownership/lifetime and processing/cache policy
 * are separate contracts.
 */
export interface CemDeclarationScope {
    readonly document: Document;
    readonly parent: CemDeclarationScope | null;
    readonly disposed: boolean;
    dispose(): void;
}

export interface CemDeclarationScopeOptions {
    document: Document;
    parent?: CemDeclarationScope | null;
}

export type CemDeclarationScopeErrorCode =
    | 'cem-element.scope_foreign'
    | 'cem-element.scope_document_required'
    | 'cem-element.scope_parent_document_mismatch'
    | 'cem-element.scope_parent_disposed'
    | 'cem-element.scope_disposed'
    | 'cem-element.scope_ancestor_disposed'
    | 'cem-element.scope_tag_required'
    | 'cem-element.scope_registration_identity_required'
    | 'cem-element.scope_same_scope_binding_exists';

export class CemDeclarationScopeError extends Error {
    readonly code: CemDeclarationScopeErrorCode;

    constructor(code: CemDeclarationScopeErrorCode, message: string) {
        super(message);
        this.name = 'CemDeclarationScopeError';
        this.code = code;
    }
}

/** @internal Runtime-owned value associated with a logical declaration name. */
export interface CemDeclarationScopeRegistration<TDeclaration = unknown>
    extends CemDeclarationRegistrationIdentity {
    declaration: TDeclaration;
}

/** @internal Current and nearest-parent bindings used by the pure registration decision core. */
export interface CemDeclarationScopeRegistrationLookup<TDeclaration = unknown> {
    sameScope?: CemDeclarationScopeRegistration<TDeclaration>;
    inherited?: CemDeclarationScopeRegistration<TDeclaration>;
}

interface CemDeclarationScopeState {
    document: Document;
    parent: CemDeclarationScope | null;
    disposed: boolean;
    registrations: Map<string, CemDeclarationScopeRegistration>;
    disposeListeners: Set<() => void>;
}

const scopeStates = new WeakMap<CemDeclarationScope, CemDeclarationScopeState>();
const defaultDocumentScopes = new WeakMap<Document, CemDeclarationScope>();

class LogicalCemDeclarationScope implements CemDeclarationScope {
    get document(): Document {
        return scopeState(this).document;
    }

    get parent(): CemDeclarationScope | null {
        return scopeState(this).parent;
    }

    get disposed(): boolean {
        return scopeState(this).disposed;
    }

    dispose(): void {
        const state = scopeState(this);
        if (state.disposed) {
            return;
        }
        state.registrations.clear();
        state.disposed = true;
        for (const listener of state.disposeListeners) {
            listener();
        }
        state.disposeListeners.clear();
    }
}

/** Create an explicit root or child scope. Parentage is immutable and never inferred from the DOM. */
export function createCemDeclarationScope(options: CemDeclarationScopeOptions): CemDeclarationScope {
    const { document, parent = null } = options;
    if (!document || (typeof document !== 'object' && typeof document !== 'function')) {
        throw new CemDeclarationScopeError(
            'cem-element.scope_document_required',
            'a logical CEM declaration scope requires an owning Document'
        );
    }
    if (parent) {
        const parentState = scopeState(parent);
        if (parentState.disposed) {
            throw new CemDeclarationScopeError(
                'cem-element.scope_parent_disposed',
                'a logical CEM declaration scope cannot use a disposed parent'
            );
        }
        assertScopeChainActive(parent);
        if (parentState.document !== document) {
            throw new CemDeclarationScopeError(
                'cem-element.scope_parent_document_mismatch',
                'a logical CEM declaration scope and its parent must own the same Document'
            );
        }
    }

    const scope = Object.freeze(new LogicalCemDeclarationScope());
    scopeStates.set(scope, {
        document,
        parent,
        disposed: false,
        registrations: new Map(),
        disposeListeners: new Set(),
    });
    return scope;
}

/** Return the current default root for a Document, replacing it after explicit disposal. */
export function getDefaultCemDeclarationScope(document: Document): CemDeclarationScope {
    const existing = defaultDocumentScopes.get(document);
    if (existing && !existing.disposed) {
        return existing;
    }
    const scope = createCemDeclarationScope({ document });
    defaultDocumentScopes.set(document, scope);
    return scope;
}

/** @internal Resolve the local and nearest inherited logical declarations for a produced tag. */
export function lookupCemDeclarationScopeRegistration<TDeclaration = unknown>(
    scope: CemDeclarationScope,
    tag: string
): CemDeclarationScopeRegistrationLookup<TDeclaration> {
    assertScopeChainActive(scope);
    const name = requiredTag(tag);
    const state = scopeState(scope);
    const sameScope = state.registrations.get(name) as
        | CemDeclarationScopeRegistration<TDeclaration>
        | undefined;
    let inherited: CemDeclarationScopeRegistration<TDeclaration> | undefined;
    for (let parent = state.parent; parent; parent = scopeState(parent).parent) {
        const candidate = scopeState(parent).registrations.get(name) as
            | CemDeclarationScopeRegistration<TDeclaration>
            | undefined;
        if (candidate) {
            inherited = candidate;
            break;
        }
    }
    return { sameScope, inherited };
}

/** @internal Commit a decision-core-approved local declaration or inherited alias. */
export function bindCemDeclarationScopeRegistration<TDeclaration>(
    scope: CemDeclarationScope,
    tag: string,
    registration: CemDeclarationScopeRegistration<TDeclaration>
): void {
    assertScopeChainActive(scope);
    const name = requiredTag(tag);
    const identity = registration.registrationIdentity.trim();
    if (!identity) {
        throw new CemDeclarationScopeError(
            'cem-element.scope_registration_identity_required',
            'a logical CEM declaration binding requires a stable registration identity'
        );
    }
    const state = scopeState(scope);
    if (state.registrations.has(name)) {
        throw new CemDeclarationScopeError(
            'cem-element.scope_same_scope_binding_exists',
            `logical CEM declaration \`${name}\` is already bound in this scope`
        );
    }
    state.registrations.set(
        name,
        Object.freeze({
            registrationIdentity: identity,
            declaration: registration.declaration,
        })
    );
}

/** @internal Roll back a local binding when document-global browser definition fails. */
export function unbindCemDeclarationScopeRegistration<TDeclaration>(
    scope: CemDeclarationScope,
    tag: string,
    registration?: CemDeclarationScopeRegistration<TDeclaration>
): boolean {
    assertScopeChainActive(scope);
    const state = scopeState(scope);
    const name = requiredTag(tag);
    const existing = state.registrations.get(name);
    if (!existing) {
        return false;
    }
    if (
        registration
        && (existing.registrationIdentity !== registration.registrationIdentity
            || existing.declaration !== registration.declaration)
    ) {
        return false;
    }
    return state.registrations.delete(name);
}

/** @internal Fail closed when a scope or any explicitly supplied ancestor was disposed. */
export function assertCemDeclarationScopeActive(scope: CemDeclarationScope): void {
    assertScopeChainActive(scope);
}

/** @internal Observe explicit scope lifetime without exposing processing-host state publicly. */
export function onCemDeclarationScopeDispose(scope: CemDeclarationScope, listener: () => void): () => void {
    const state = scopeState(scope);
    if (state.disposed) {
        listener();
        return () => undefined;
    }
    state.disposeListeners.add(listener);
    return () => state.disposeListeners.delete(listener);
}

function assertScopeChainActive(scope: CemDeclarationScope): void {
    let current: CemDeclarationScope | null = scope;
    let first = true;
    while (current) {
        const state = scopeState(current);
        if (state.disposed) {
            throw new CemDeclarationScopeError(
                first ? 'cem-element.scope_disposed' : 'cem-element.scope_ancestor_disposed',
                first
                    ? 'the logical CEM declaration scope is disposed'
                    : 'an ancestor of the logical CEM declaration scope is disposed'
            );
        }
        current = state.parent;
        first = false;
    }
}

function scopeState(scope: CemDeclarationScope): CemDeclarationScopeState {
    const state = scopeStates.get(scope);
    if (!state) {
        throw new CemDeclarationScopeError(
            'cem-element.scope_foreign',
            'the logical CEM declaration scope was not created by this runtime'
        );
    }
    return state;
}

function requiredTag(tag: string): string {
    const name = tag.trim();
    if (!name) {
        throw new CemDeclarationScopeError(
            'cem-element.scope_tag_required',
            'a logical CEM declaration binding requires a produced tag'
        );
    }
    return name;
}
