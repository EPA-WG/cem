import {
    assertCemDeclarationScopeActive, onCemDeclarationScopeDispose, type CemDeclarationScope,
} from './declaration-scope.js';

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

    setStyles(styles: readonly HTMLStyleElement[]): void {
        this.styles = styles;
        this.reconcile();
    }

    reconcile(records: readonly MutationRecord[] = []): void {
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
