import { CemPlacementAuthority, type CemPlacementLease, type CemPlacementTransaction } from './element-placement-authority.js';
import type { CemElementPlacementUse, CemElementReferenceInputs } from './element-reference-inputs.js';
import { placementPlanElement, placementRoutes, withoutPlacementRelationships } from './element-placement-plans.js';
import { materializeRenderPlan, type RenderPlan, type RenderPlanDomRange } from './projection.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, type CemValueArtifactLimits } from './native-values.js';
export type { CemPlacementLease, CemPlacementTransaction } from './element-placement-authority.js';
export interface CemBrowserPlacementRegistration {
    producer: string; revision: string; path: readonly number[]; element: Element;
    source: CemElementReferenceInputs['sources'][number]; select: string;
    currentRevision(): string | undefined;
}
export interface CemBrowserPlacementStage {
    readonly host: HTMLElement;
    readonly plan: RenderPlan;
    readonly fragment: DocumentFragment;
    readonly bounds?: RenderPlanDomRange;
    current(): boolean;
}
export interface CemBrowserPlacementPublication {
    stage: CemBrowserPlacementStage;
    inputs?: CemElementReferenceInputs;
    uses?: readonly CemElementPlacementUse[];
    invalidate?(uses: readonly CemElementPlacementUse[]): void;
}
interface Group { hosts: Map<string, HTMLElement>; revisions: Readonly<Record<string, string>>; stages: Set<CemBrowserPlacementStage> }
interface WatchedPlacement { transaction?: string; lease: CemPlacementLease; ready(): boolean; staged: boolean }

/** DOM adapter for host-issued native placement authority and publication. */
export class CemElementPlacementCoordinator {
    private readonly authority: CemPlacementAuthority<HTMLElement>;
    private readonly observer: MutationObserver;
    private readonly watched = new Set<WatchedPlacement>();
    private readonly staged = new WeakMap<CemBrowserPlacementStage, { original: Node[]; applied: boolean }>();
    private readonly groups = new Map<string, Group>();
    private readonly revisions = new Map<string, string>();
    private disposed = false;
    constructor(private readonly root: Document | ShadowRoot, private readonly limits: CemValueArtifactLimits = DEFAULT_CEM_VALUE_ARTIFACT_LIMITS) {
        this.authority = new CemPlacementAuthority(limits);
        this.observer = new MutationObserver(() => this.refresh());
        this.observer.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ['id'] });
    }
    register(registration: CemBrowserPlacementRegistration): CemPlacementLease {
        const id = this.placementId(registration.element, registration.producer, registration.path);
        const ready = () => registration.element.isConnected && registration.element.getRootNode() === this.root
            && registration.currentRevision() === registration.revision && this.uniqueId(registration.element, id);
        if (!ready()) throw new Error('Placement requires a committed connected producer');
        return this.registerEndpoint(registration, id, ready);
    }
    transaction(participants: readonly { producer: string; host: HTMLElement }[], revisions: Readonly<Record<string, string>>): CemPlacementTransaction {
        this.active();
        const hosts = new Map(participants.map(p => [p.producer, p.host]));
        if (hosts.size !== participants.length || new Set(hosts.values()).size !== hosts.size) throw new Error('Duplicate transaction producer or host');
        const transaction = this.authority.transaction([...hosts.keys()], revisions);
        this.groups.set(transaction.token, { hosts, revisions: { ...revisions }, stages: new Set() });
        return Object.freeze({ token: transaction.token, cancel: () => { transaction.cancel(); this.groups.delete(transaction.token);
            for (const watched of this.watched) if (watched.transaction === transaction.token) { watched.lease.dispose(); this.watched.delete(watched); }
            this.refresh(); } });
    }
    stage(host: HTMLElement, plan: RenderPlan, current: () => boolean, transaction?: CemPlacementTransaction, bounds?: RenderPlanDomRange): CemBrowserPlacementStage {
        this.active();
        const stage = Object.freeze({ host, plan, current, bounds, fragment: materializeRenderPlan(withoutPlacementRelationships(plan), host.ownerDocument) });
        this.staged.set(stage, { original: bounds ? this.rangeNodes(bounds) : [...host.childNodes], applied: false });
        if (transaction) this.enlist(transaction, stage);
        return stage;
    }
    /** Enlist an already prepared parent forest before declaring its nested producers. */
    enlist(transaction: CemPlacementTransaction, stage: CemBrowserPlacementStage): void {
        const group = this.group(transaction);
        if (!this.staged.has(stage) || group.hosts.get(stage.plan.instanceId) !== stage.host
            || group.revisions[stage.plan.instanceId] !== stage.plan.dataRevision
            || [...group.stages].some(other => other !== stage && other.plan.instanceId === stage.plan.instanceId)) throw new Error('Stage differs from its declared transaction producer');
        group.stages.add(stage);
    }
    registerPrepared(transaction: CemPlacementTransaction, stage: CemBrowserPlacementStage, path: readonly number[], source: CemElementReferenceInputs['sources'][number], select: string): CemPlacementLease {
        const group = this.group(transaction); this.enlist(transaction, stage);
        if (group.hosts.get(stage.plan.instanceId) !== stage.host) throw new Error('Prepared placement host is outside its transaction');
        const node = placementPlanElement(stage.plan, path);
        const element = this.find(stage.fragment, node.renderNodeId);
        const producer = stage.plan.instanceId, revision = stage.plan.dataRevision;
        const id = this.placementId(element, producer, path);
        const ready = (_requester: HTMLElement, prepared: boolean) => prepared
            ? stage.current() && stage.fragment.contains(element) && this.preparedRoot(stage.host, group, new Set()) && this.uniqueId(element, id, group)
            : element.isConnected && element.getRootNode() === this.root && this.revisions.get(producer) === revision && this.uniqueId(element, id);
        return this.registerEndpoint({ producer, revision, path, element, source, select, currentRevision: () => this.revisions.get(producer) }, id,
            () => ready(stage.host, false), transaction, ready);
    }
    grant(requester: string, lease: CemPlacementLease, properties: readonly string[]): () => void { this.active(); return this.authority.grant(requester, lease, properties); }
    prepare(requester: HTMLElement, instanceId: string, inputs: CemElementReferenceInputs, transaction?: CemPlacementTransaction): CemElementReferenceInputs {
        this.active();
        if (!(transaction ? this.preparedRoot(requester, this.group(transaction), new Set()) : requester.isConnected && requester.getRootNode() === this.root)) throw new Error('Requester is outside the permitted DOM root');
        return this.authority.prepare(instanceId, inputs, requester, transaction);
    }
    publish<T>(requester: HTMLElement, inputs: CemElementReferenceInputs, uses: readonly CemElementPlacementUse[], commit: () => T,
        invalidate: (uses: readonly CemElementPlacementUse[]) => void = () => undefined, current: () => boolean = () => requester.isConnected): T {
        this.active();
        if (!current() || requester.getRootNode() !== this.root) throw new Error('Placement publication was superseded');
        let routes: Map<CemElementPlacementUse, Element> | undefined;
        try { return this.authority.publish(inputs, uses, () => {
            if (!current()) throw new Error('Placement publication was superseded');
            const result = commit();
            routes = this.verifyRoutes(requester, uses);
            if (!current() || requester.getRootNode() !== this.root) throw new Error('Placement publication changed during commit');
            return result;
        }, lost => { this.clearRoutes(routes, lost); invalidate(lost); }); }
        catch (error) { if (routes) { this.clearRoutes(routes, uses); invalidate(uses); } throw error; }
    }
    publishGroup(transaction: CemPlacementTransaction, publications: readonly CemBrowserPlacementPublication[]): void {
        this.active(); const group = this.group(transaction);
        for (const { stage } of publications) {
            this.enlist(transaction, stage);
        }
        const prior = new Map<CemBrowserPlacementStage, { nodes: Node[]; revision?: string }>();
        const routes = new Map<CemBrowserPlacementStage, Map<CemElementPlacementUse, Element>>();
        for (const { stage, uses = [] } of publications) for (const route of placementRoutes(stage.plan)) {
            if (!uses.some(use => use.renderNodeId === route.renderNodeId && use.attribute === route.name)) throw new Error('Unadmitted placement route');
            this.find(stage.fragment, route.renderNodeId);
        }
        this.authority.publishGroup(transaction, publications.map(({ stage, inputs, uses = [], invalidate }) => ({
            requester: stage.plan.instanceId, inputs, uses,
            current: () => this.currentStage(stage) && this.preparedRoot(stage.host, group, new Set()),
            commit: () => {
                const nodes = stage.bounds ? this.rangeNodes(stage.bounds) : [...stage.host.childNodes];
                prior.set(stage, { nodes, revision: this.revisions.get(stage.plan.instanceId) });
                nodes.forEach(node => node.parentNode?.removeChild(node));
                if (stage.bounds) stage.bounds.end.before(stage.fragment); else stage.host.append(stage.fragment);
                this.revisions.set(stage.plan.instanceId, stage.plan.dataRevision);
                this.stageState(stage).applied = true;
            },
            rollback: () => {
                const old = prior.get(stage); if (!old) return;
                const nodes = stage.bounds ? this.rangeNodes(stage.bounds) : [...stage.host.childNodes];
                nodes.forEach(node => stage.fragment.append(node));
                clearCemPlacementRelationships(stage.fragment);
                if (stage.bounds) stage.bounds.end.before(...old.nodes); else stage.host.append(...old.nodes);
                this.stageState(stage).applied = false;
                if (old.revision === undefined) this.revisions.delete(stage.plan.instanceId); else this.revisions.set(stage.plan.instanceId, old.revision);
            },
            activate: () => {
                for (const route of placementRoutes(stage.plan)) {
                    const element = this.find(stage.host, route.renderNodeId);
                    element.setAttribute(route.name, route.value); element.setAttribute(`data-cem-placement-ref-${route.name}`, '');
                    if (route.interaction) element.setAttribute(`data-cem-node-ref-${route.name}`, '');
                }
                routes.set(stage, this.verifyRoutes(stage.host, uses));
            },
            invalidate: lost => { this.clearRoutes(routes.get(stage), lost); invalidate?.(lost); },
        })));
        for (const watched of this.watched) if (watched.staged && watched.ready()) watched.staged = false;
        this.groups.delete(transaction.token); this.refresh();
    }
    release(requester: string): void { this.authority.release(requester); }
    refresh(): void {
        if (this.disposed) return;
        for (const watched of this.watched) if (!watched.staged && !watched.ready()) { watched.lease.dispose(); this.watched.delete(watched); }
        this.authority.sweep();
    }
    dispose(): void { if (this.disposed) return; this.disposed = true; this.observer.disconnect(); this.authority.dispose(); this.watched.clear(); this.groups.clear(); }
    private registerEndpoint(registration: CemBrowserPlacementRegistration, id: string, live: () => boolean, transaction?: CemPlacementTransaction,
        prepared?: (requester: HTMLElement, staged: boolean) => boolean): CemPlacementLease {
        this.active(); let reserved = registration.element.hasAttribute('id');
        const lease = this.authority.register({ ...registration, id }, {
            ready: (requester, staged) => (staged && transaction ? this.preparedRoot(requester, this.group(transaction), new Set())
                : requester.isConnected && requester.getRootNode() === this.root)
                && (prepared ? prepared(requester, staged) : live()) && (!reserved || registration.element.id === id),
            reserve: () => {
                const previous = registration.element.getAttribute('id'), old = reserved;
                if (previous !== null && previous !== id) throw new Error('Placement ID changed');
                registration.element.id = id; reserved = true;
                return () => { reserved = old; if (previous === null) registration.element.removeAttribute('id'); else registration.element.setAttribute('id', previous); };
            },
        }, transaction);
        const watched: WatchedPlacement = { transaction: transaction?.token, lease, staged: !!transaction, ready: () => live() && (!reserved || registration.element.id === id) };
        this.watched.add(watched); return Object.freeze({ token: lease.token, dispose: () => { lease.dispose(); this.watched.delete(watched); } });
    }
    private placementId(element: Element, producer: string, path: readonly number[]): string { return element.hasAttribute('id') ? element.id : `${producer}-ref-${path.join('-')}`; }
    private uniqueId(element: Element, id: string, group?: Group): boolean {
        const nodes = [...this.root.querySelectorAll('[id]')]; if (nodes.length > this.limits.maxValues) return false;
        return !nodes.some(node => node !== element && node.id === id && ![...(group?.hosts.values() ?? [])].some(host => host.contains(node)));
    }
    private preparedRoot(host: HTMLElement, group: Group, active: Set<HTMLElement>): boolean {
        if (host.isConnected) return host.getRootNode() === this.root;
        if (active.has(host)) return false; active.add(host);
        const parent = [...group.stages].find(stage => stage.fragment.contains(host));
        return !!parent && this.preparedRoot(parent.host, group, active);
    }
    private group(transaction: CemPlacementTransaction): Group { const group = this.groups.get(transaction.token); if (!group) throw new Error('Unknown placement group'); return group; }
    private find(root: ParentNode, id: string): Element {
        const nodes = [...root.querySelectorAll('[data-cem-render-node-id]')].filter(node => node.getAttribute('data-cem-render-node-id') === id);
        if (nodes.length !== 1) throw new Error('Placement render identity is missing or ambiguous'); return nodes[0];
    }
    private verifyRoutes(requester: HTMLElement, uses: readonly CemElementPlacementUse[]): Map<CemElementPlacementUse, Element> {
        const routes = new Map<CemElementPlacementUse, Element>();
        for (const use of uses) { const element = this.find(requester, use.renderNodeId);
            if (!element.hasAttribute(`data-cem-placement-ref-${use.attribute}`) || !element.getAttribute(use.attribute)?.split(/\s+/).includes(use.id)) throw new Error('Published relationship differs from native admission');
            routes.set(use, element); }
        return routes;
    }
    private clearRoutes(routes: Map<CemElementPlacementUse, Element> | undefined, uses: readonly CemElementPlacementUse[]): void {
        for (const use of uses) {
            const element = routes && [...routes].find(([route]) => route.token === use.token && route.renderNodeId === use.renderNodeId && route.attribute === use.attribute)?.[1];
            if (element) { element.removeAttribute(use.attribute); element.removeAttribute(`data-cem-placement-ref-${use.attribute}`); element.removeAttribute(`data-cem-node-ref-${use.attribute}`); }
        }
    }
    private stageState(stage: CemBrowserPlacementStage): { original: Node[]; applied: boolean } {
        const state = this.staged.get(stage); if (!state) throw new Error('Unknown browser placement stage'); return state;
    }
    private currentStage(stage: CemBrowserPlacementStage): boolean {
        const state = this.staged.get(stage); if (!state || !stage.current()) return false;
        if (state.applied) return true;
        const current = stage.bounds ? this.rangeNodes(stage.bounds) : [...stage.host.childNodes];
        return current.length === state.original.length && current.every((node, index) => node === state.original[index]);
    }
    private rangeNodes(bounds: RenderPlanDomRange): Node[] {
        if (!bounds.start.parentNode || bounds.start.parentNode !== bounds.end.parentNode) throw new Error('Placement render range changed');
        const result: Node[] = []; let node = bounds.start.nextSibling;
        while (node && node !== bounds.end) { result.push(node); node = node.nextSibling; }
        if (node !== bounds.end) throw new Error('Placement render range is incomplete'); return result;
    }
    private active(): void { if (this.disposed) throw new Error('Placement coordinator was disposed'); }
}

/** Resume markers carry no authority. Re-admission or invalidation owns activation. */
export function clearCemPlacementRelationships(root: ParentNode, uses?: readonly CemElementPlacementUse[]): void {
    for (const element of root.querySelectorAll('*')) for (const attribute of [...element.attributes]) if (attribute.name.startsWith('data-cem-placement-ref-')) {
        const name = attribute.name.slice('data-cem-placement-ref-'.length);
        if (uses && !uses.some(use => use.renderNodeId === element.getAttribute('data-cem-render-node-id') && use.attribute === name)) continue;
        element.removeAttribute(name); element.removeAttribute(attribute.name); element.removeAttribute(`data-cem-node-ref-${name}`);
    }
}
export function hasCemPlacementRelationships(root: ParentNode): boolean {
    return [...root.querySelectorAll('*')].some(element => element.getAttributeNames().some(name => name.startsWith('data-cem-placement-ref-')));
}
