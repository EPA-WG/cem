import type { CemElementPlacementUse, CemElementReferenceInputs } from './element-reference-inputs.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, type CemValueArtifactLimits } from './native-values.js';

export interface CemPlacementDescription {
    producer: string;
    path: readonly number[];
    revision: string;
    id: string;
    source: CemElementReferenceInputs['sources'][number];
    select: string;
}
export interface CemPlacementEndpoint<Root> {
    ready(root: Root, prepared: boolean): boolean;
    /** The owning producer reserves the ID; rollback restores its prior state. */
    reserve(): () => void;
}
export interface CemPlacementLease { readonly token: string; dispose(): void }
export interface CemPlacementTransaction { readonly token: string; cancel(): void }
export interface CemPlacementPublication {
    requester: string;
    inputs?: CemElementReferenceInputs;
    uses?: readonly CemElementPlacementUse[];
    current(): boolean;
    commit(): void;
    rollback(): void;
    /** Runs only after every producer forest is committed and checked. */
    activate?(): void;
    invalidate?(uses: readonly CemElementPlacementUse[]): void;
}
interface Entry<Root> {
    token: string;
    description: CemPlacementDescription;
    endpoint: CemPlacementEndpoint<Root>;
    transaction?: Transaction;
    active: boolean;
}
interface Grant<Root> { requester: string; entry: Entry<Root>; properties: Set<string>; active: boolean }
interface Transaction { token: string; participants: Set<string>; revisions: Record<string, string>; open: boolean }
interface Snapshot<Root> { requester: string; root: Root; grants: Grant<Root>[]; byToken: Map<string, Grant<Root>[]>; transaction?: Transaction }
interface Dependency<Root> { snapshot: Snapshot<Root>; uses: readonly CemElementPlacementUse[]; invalidate(uses: readonly CemElementPlacementUse[]): void }
const token = (value: string) => typeof value === 'string' && !!value && !/[\s\p{Cc}]/u.test(value);
const properties = new Set(['commandfor', 'popovertarget', 'interestfor', 'form', 'list', 'for', 'aria-controls', 'aria-labelledby', 'aria-describedby',
    'aria-owns', 'aria-activedescendant', 'aria-details', 'aria-errormessage', 'headers', 'command-target', 'interaction', 'trigger-for', 'parent-item',
    'focus-target', 'return-focus', 'anchor', 'boundary']);

/** Host authority stays in this registry. Exported metadata is an invocation snapshot. */
export class CemPlacementAuthority<Root> {
    private readonly entries = new Map<string, Entry<Root>>();
    private readonly grants = new Set<Grant<Root>>();
    private readonly transactions = new Map<string, Transaction>();
    private readonly snapshots = new WeakMap<CemElementReferenceInputs, Snapshot<Root>>();
    private readonly dependencies = new Map<string, Dependency<Root>>();
    constructor(private readonly limits: CemValueArtifactLimits = DEFAULT_CEM_VALUE_ARTIFACT_LIMITS) {}

    register(description: CemPlacementDescription, endpoint: CemPlacementEndpoint<Root>, transaction?: CemPlacementTransaction): CemPlacementLease {
        if (this.entries.size >= this.limits.maxValues || ![description.producer, description.revision, description.id].every(token)
            || !description.path.length || description.path.length > this.limits.maxDepth
            || description.path.some(n => !Number.isSafeInteger(n) || n < 0) || !description.select) throw new Error('Invalid placement registration');
        const prepared = transaction ? this.requireTransaction(transaction) : undefined;
        if (prepared && prepared.revisions[description.producer] !== description.revision) throw new Error('Placement revision is outside its transaction');
        for (const entry of this.entries.values()) if (entry.active && entry.description.producer === description.producer
            && entry.description.revision === description.revision && entry.description.path.join('.') === description.path.join('.')) throw new Error('Duplicate producer placement');
        const entry: Entry<Root> = { token: crypto.randomUUID(), description: { producer: description.producer, revision: description.revision,
            id: description.id, source: description.source, select: description.select, path: Object.freeze([...description.path]) }, endpoint, active: true, transaction: prepared };
        this.entries.set(entry.token, entry);
        return Object.freeze({ token: entry.token, dispose: () => {
            if (!entry.active) return;
            entry.active = false; this.entries.delete(entry.token);
            for (const grant of this.grants) if (grant.entry === entry) { grant.active = false; this.grants.delete(grant); }
            this.sweep();
        } });
    }
    grant(requester: string, lease: CemPlacementLease, permitted: readonly string[]): () => void {
        const entry = this.entries.get(lease.token);
        if (!entry?.active || !token(requester) || requester === entry.description.producer || !permitted.length
            || permitted.length > this.limits.maxValues || permitted.some(name => !properties.has(name)) || this.grants.size >= this.limits.maxValues) throw new Error('Invalid placement grant');
        const grant: Grant<Root> = { requester, entry, properties: new Set(permitted), active: true };
        this.grants.add(grant);
        return () => { if (grant.active) { grant.active = false; this.grants.delete(grant); this.sweep(); } };
    }
    transaction(participants: readonly string[], revisions: Readonly<Record<string, string>>): CemPlacementTransaction {
        if (this.transactions.size >= this.limits.maxValues || !participants.length || participants.length > this.limits.maxValues || participants.some(p => !token(p))
            || new Set(participants).size !== participants.length || participants.some(p => !Object.hasOwn(revisions, p) || !token(revisions[p])) || Object.entries(revisions).some(([p, r]) => !participants.includes(p) || !token(r))) throw new Error('Invalid placement transaction');
        const transaction: Transaction = { token: crypto.randomUUID(), participants: new Set(participants), revisions: { ...revisions }, open: true };
        this.transactions.set(transaction.token, transaction);
        return Object.freeze({ token: transaction.token, cancel: () => {
            if (!transaction.open) return;
            transaction.open = false; this.transactions.delete(transaction.token);
            for (const entry of this.entries.values()) if (entry.transaction === transaction) {
                entry.active = false; this.entries.delete(entry.token);
                for (const grant of this.grants) if (grant.entry === entry) { grant.active = false; this.grants.delete(grant); }
            }
            this.sweep();
        } });
    }
    prepare(requester: string, input: CemElementReferenceInputs, root: Root, transaction?: CemPlacementTransaction): CemElementReferenceInputs {
        if (!token(requester)) throw new Error('Invalid requesting producer');
        const prepared = transaction ? this.requireTransaction(transaction) : undefined;
        if (prepared && !prepared.participants.has(requester)) throw new Error('Requester is outside placement transaction');
        const grants = [...this.grants].filter(g => g.active && g.requester === requester && g.entry.active
            && (!g.entry.transaction || g.entry.transaction === prepared)
            && (!prepared?.revisions[g.entry.description.producer] || prepared.revisions[g.entry.description.producer] === g.entry.description.revision)
            && g.entry.endpoint.ready(root, !!g.entry.transaction));
        const sources = [...input.sources];
        const entries = [...new Set(grants.map(g => g.entry))];
        const admissions = entries.map(entry => {
            const { source, ...description } = entry.description;
            let index = sources.indexOf(source);
            if (index < 0) { index = sources.length; sources.push(source); }
            return Object.freeze({ ...description, source: index, token: entry.token });
        });
        const committedRevisions: Record<string, string> = Object.create(null);
        for (const entry of entries) if (!entry.transaction) {
            const { producer, revision } = entry.description;
            if (committedRevisions[producer] && committedRevisions[producer] !== revision) throw new Error('Conflicting producer revisions');
            committedRevisions[producer] = revision;
        }
        const result: CemElementReferenceInputs = Object.freeze({ ...input, sources: Object.freeze(sources), placements: Object.freeze({
            admissions: Object.freeze(admissions), grants: Object.freeze(grants.map(g => Object.freeze({ requester, token: g.entry.token, properties: Object.freeze([...g.properties]) }))),
            committedRevisions: Object.freeze(committedRevisions), ...(prepared ? { preparedTransaction: Object.freeze({ token: prepared.token,
                participants: Object.freeze([...prepared.participants]), producerRevisions: Object.freeze({ ...prepared.revisions }) }) } : {}),
        }) });
        if (new TextEncoder().encode(JSON.stringify(result.placements)).length > Math.min(128 * 1024, this.limits.maxBytes)) throw new Error('Placement metadata byte limit exceeded');
        const byToken = new Map<string, Grant<Root>[]>();
        for (const grant of grants) { const selected = byToken.get(grant.entry.token) ?? []; selected.push(grant); byToken.set(grant.entry.token, selected); }
        this.snapshots.set(result, { requester, root, grants, byToken, transaction: prepared });
        return result;
    }
    check(input: CemElementReferenceInputs, uses: readonly CemElementPlacementUse[], committed = false): void {
        const snapshot = this.snapshots.get(input);
        if (!snapshot || uses.length > this.limits.maxValues || (snapshot.transaction && !snapshot.transaction.open)) throw new Error('Unknown or closed placement snapshot');
        for (const use of uses) {
            const grant = (snapshot.byToken.get(use.token) ?? []).find(g => g.active && g.entry.active && g.entry.token === use.token && g.properties.has(use.attribute));
            const entry = grant?.entry, description = entry?.description;
            if (!entry || !description || description.producer !== use.producer || description.revision !== use.revision || description.id !== use.id
                || use.transaction !== entry.transaction?.token || !entry.endpoint.ready(snapshot.root, !committed && !!entry.transaction)) throw new Error('Placement admission changed before publication');
        }
    }
    publish<T>(input: CemElementReferenceInputs, uses: readonly CemElementPlacementUse[], commit: () => T, invalidate: (uses: readonly CemElementPlacementUse[]) => void = () => undefined): T {
        const snapshot = this.snapshots.get(input);
        if (!snapshot || snapshot.transaction) throw new Error('Prepared placements require coordinated publication');
        this.check(input, uses);
        const undo = this.reserve([input], [uses]);
        try { const result = commit(); this.check(input, uses, true); this.track(input, uses, invalidate); return result; }
        catch (error) { undo.reverse().forEach(restore => restore()); throw error; }
    }
    publishGroup(transaction: CemPlacementTransaction, publications: readonly CemPlacementPublication[]): void {
        const prepared = this.requireTransaction(transaction);
        if (publications.length !== prepared.participants.size || new Set(publications.map(p => p.requester)).size !== publications.length
            || publications.some(p => !prepared.participants.has(p.requester) || !p.current())) throw new Error('Incomplete or superseded placement group');
        for (const publication of publications) if (publication.inputs) {
            if (this.snapshots.get(publication.inputs)?.transaction !== prepared) throw new Error('Publication belongs to another transaction');
            this.check(publication.inputs, publication.uses ?? []);
        } else if (publication.uses?.length) throw new Error('Missing placement snapshot');
        const inputs = publications.flatMap(p => p.inputs ? [p.inputs] : []);
        const uses = publications.filter(p => p.inputs).map(p => p.uses ?? []);
        const undo = this.reserve(inputs, uses);
        const applied: CemPlacementPublication[] = [];
        try {
            for (const publication of publications) { applied.push(publication); publication.commit(); }
            for (const publication of publications) {
                if (!publication.current()) throw new Error('Placement group was superseded during publication');
                if (publication.inputs) this.check(publication.inputs, publication.uses ?? [], true);
            }
            for (const publication of publications) publication.activate?.();
            for (const publication of publications) {
                if (!publication.current()) throw new Error('Placement group changed during activation');
                if (publication.inputs) this.check(publication.inputs, publication.uses ?? [], true);
            }
            for (const publication of publications) if (publication.inputs) this.track(publication.inputs, publication.uses ?? [], publication.invalidate ?? (() => undefined));
            for (const entry of this.entries.values()) if (entry.transaction === prepared) entry.transaction = undefined;
            prepared.open = false; this.transactions.delete(prepared.token);
        } catch (error) {
            for (const publication of applied.reverse()) { try { publication.rollback(); } catch { /* Continue restoring the remaining producers. */ } }
            for (const restore of undo.reverse()) { try { restore(); } catch { /* Preserve the publication failure. */ } }
            throw error;
        }
    }
    sweep(): void {
        for (const [requester, dependency] of this.dependencies) {
            const invalid = dependency.uses.filter(use => !(dependency.snapshot.byToken.get(use.token) ?? []).some(g => g.active && g.entry.active && g.entry.token === use.token
                && g.properties.has(use.attribute) && g.entry.endpoint.ready(dependency.snapshot.root, false)));
            if (invalid.length) {
                const lost = new Set(invalid);
                const remaining = dependency.uses.filter(use => !lost.has(use));
                if (remaining.length) dependency.uses = remaining; else this.dependencies.delete(requester);
                dependency.invalidate(invalid);
            }
        }
    }
    release(requester: string): void {
        this.dependencies.delete(requester);
        for (const grant of this.grants) if (grant.requester === requester) { grant.active = false; this.grants.delete(grant); }
    }
    dispose(): void {
        for (const entry of this.entries.values()) entry.active = false;
        for (const grant of this.grants) grant.active = false;
        this.sweep(); this.entries.clear(); this.grants.clear();
        for (const transaction of this.transactions.values()) transaction.open = false;
        this.transactions.clear();
    }
    private requireTransaction(transaction: CemPlacementTransaction): Transaction {
        const prepared = this.transactions.get(transaction.token);
        if (!prepared?.open) throw new Error('Unknown or closed placement transaction');
        return prepared;
    }
    private reserve(inputs: readonly CemElementReferenceInputs[], uses: readonly (readonly CemElementPlacementUse[])[]): Array<() => void> {
        const selected = new Set(uses.flatMap(u => u.map(v => v.token)));
        const entries = new Map<string, Entry<Root>>();
        for (const input of inputs) for (const grant of this.snapshots.get(input)?.grants ?? []) if (selected.has(grant.entry.token)) entries.set(grant.entry.token, grant.entry);
        const undo: Array<() => void> = [];
        try { for (const entry of entries.values()) undo.push(entry.endpoint.reserve()); return undo; }
        catch (error) { undo.reverse().forEach(restore => restore()); throw error; }
    }
    private track(input: CemElementReferenceInputs, uses: readonly CemElementPlacementUse[], invalidate: (uses: readonly CemElementPlacementUse[]) => void): void {
        const snapshot = this.snapshots.get(input);
        if (!snapshot) throw new Error('Missing publication snapshot');
        if (uses.length) this.dependencies.set(snapshot.requester, { snapshot, uses: uses.map(u => ({ ...u })), invalidate });
        else this.dependencies.delete(snapshot.requester);
    }
}
