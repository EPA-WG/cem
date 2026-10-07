import { CemPlacementAuthority, type CemPlacementLease, type CemPlacementTransaction } from './element-placement-authority.js';
import type { CemElementPlacementUse, CemElementReferenceInputs } from './element-reference-inputs.js';
import { placementPlanElement, placementRoutes, withPlacementId, withoutPlacementRelationships } from './element-placement-plans.js';
import type { RenderPlan } from './projection.js';
export interface CemSsrPlacementStage { readonly plan: RenderPlan; current(): boolean }
export interface CemSsrPlacementPublication { stage: CemSsrPlacementStage; inputs?: CemElementReferenceInputs; uses?: readonly CemElementPlacementUse[] }
export interface CemPlacementResumeHints {
    kind: 'cem-placement-resume-hints-v1';
    profile: 'wai-aria-1.2-rec-20230606';
    producers: { producer: string; revision: string; scopePolicyStamp: string; placements: { path: readonly number[]; id: string }[] }[];
}
/** Retained native export plans remain private until the complete SSR group is ready. */
export class CemSsrPlacementCoordinator {
    private readonly authority = new CemPlacementAuthority<CemSsrPlacementCoordinator>();
    private readonly revisions = new Map<string, Readonly<Record<string, string>>>();
    private readonly committed = new Map<string, RenderPlan>();
    private readonly stages = new WeakMap<CemSsrPlacementStage, { plan: RenderPlan }>();
    private readonly placements = new Map<string, { producer: string; revision: string; path: readonly number[]; id: string }>();
    transaction(participants: readonly string[], revisions: Readonly<Record<string, string>>): CemPlacementTransaction { const transaction = this.authority.transaction(participants, revisions);
        this.revisions.set(transaction.token, { ...revisions });
        return Object.freeze({ token: transaction.token, cancel: () => { transaction.cancel(); this.revisions.delete(transaction.token); } }); }
    stage(plan: RenderPlan, current: () => boolean = () => true): CemSsrPlacementStage {
        const stage = Object.freeze({ plan, current }); this.stages.set(stage, { plan }); return stage;
    }
    registerPrepared(transaction: CemPlacementTransaction, stage: CemSsrPlacementStage, path: readonly number[], source: CemElementReferenceInputs['sources'][number], select: string): CemPlacementLease {
        const prepared = this.requireStage(stage), node = placementPlanElement(prepared.plan, path);
        const producer = stage.plan.instanceId, revision = stage.plan.dataRevision;
        const id = node.attributes.find(a => a.name === 'id')?.value ?? `${producer}-ref-${path.join('-')}`;
        const lease = this.authority.register({ producer, revision, path, id, source, select }, {
            ready: (_root, staging) => {
                const plan = staging ? prepared.plan : this.committed.get(producer);
                if (!plan || plan.dataRevision !== revision || !stage.current()) return false;
                try { const placed = placementPlanElement(plan, path);
                    return placed.renderNodeId === node.renderNodeId && (staging || placed.attributes.some(a => a.name === 'id' && a.value === id));
                } catch { return false; }
            },
            reserve: () => { const old = prepared.plan; prepared.plan = withPlacementId(old, path, id); return () => { prepared.plan = old; }; },
        }, transaction);
        this.placements.set(lease.token, { producer, revision, path: [...path], id });
        return Object.freeze({ token: lease.token, dispose: () => { lease.dispose(); this.placements.delete(lease.token); } });
    }
    grant(requester: string, lease: CemPlacementLease, properties: readonly string[]): () => void { return this.authority.grant(requester, lease, properties); }
    prepare(requester: string, inputs: CemElementReferenceInputs, transaction?: CemPlacementTransaction): CemElementReferenceInputs { return this.authority.prepare(requester, inputs, this, transaction); }
    publishGroup(transaction: CemPlacementTransaction, publications: readonly CemSsrPlacementPublication[]): ReadonlyMap<string, RenderPlan> {
        const revisions = this.revisions.get(transaction.token);
        if (!revisions || publications.some(p => revisions[p.stage.plan.instanceId] !== p.stage.plan.dataRevision)) throw new Error('SSR stage differs from its transaction revision');
        const prior = new Map<string, RenderPlan | undefined>();
        this.authority.publishGroup(transaction, publications.map(({ stage, inputs, uses = [] }) => ({
            requester: stage.plan.instanceId, inputs, uses, current: stage.current,
            commit: () => { prior.set(stage.plan.instanceId, this.committed.get(stage.plan.instanceId)); this.committed.set(stage.plan.instanceId, this.requireStage(stage).plan); },
            rollback: () => { const previous = prior.get(stage.plan.instanceId); if (previous) this.committed.set(stage.plan.instanceId, previous); else this.committed.delete(stage.plan.instanceId); },
            activate: () => {
                for (const route of placementRoutes(stage.plan)) if (!uses.some(use => use.renderNodeId === route.renderNodeId && use.attribute === route.name)) throw new Error('Unadmitted SSR placement route');
                this.checkIds();
            },
            invalidate: lost => { const current = this.committed.get(stage.plan.instanceId); if (current) this.committed.set(stage.plan.instanceId, withoutPlacementRelationships(current, lost)); },
        })));
        this.revisions.delete(transaction.token); this.authority.sweep(); return this.plans();
    }
    plans(): ReadonlyMap<string, RenderPlan> { return new Map(this.committed); }
    resumeHints(): CemPlacementResumeHints {
        return { kind: 'cem-placement-resume-hints-v1', profile: 'wai-aria-1.2-rec-20230606', producers: [...this.committed.values()].map(plan => ({
            producer: plan.instanceId, revision: plan.dataRevision, scopePolicyStamp: plan.scopePolicyStamp,
            placements: [...this.placements.values()].filter(p => p.producer === plan.instanceId && p.revision === plan.dataRevision).map(p => ({ path: [...p.path], id: p.id })),
        })) };
    }
    dispose(): void { this.authority.dispose(); this.committed.clear(); this.placements.clear(); this.revisions.clear(); }
    private requireStage(stage: CemSsrPlacementStage): { plan: RenderPlan } { const prepared = this.stages.get(stage); if (!prepared) throw new Error('Unknown SSR placement stage'); return prepared; }
    private checkIds(): void {
        const ids = new Set<string>();
        const visit = (nodes: RenderPlan['nodes']) => { for (const node of nodes) if (node.kind === 'element') {
            const id = node.attributes.find(a => a.name === 'id')?.value;
            if (id !== undefined) { if (!id || /\s/.test(id) || ids.has(id)) throw new Error('SSR producer ID reservation conflict'); ids.add(id); }
            visit(node.children);
        } };
        for (const plan of this.committed.values()) visit(plan.nodes);
    }
}
