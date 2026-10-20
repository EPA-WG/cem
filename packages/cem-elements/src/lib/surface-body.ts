import type { CheckpointDeferredBody, CheckpointEndpoint, CheckpointRenderProjection } from './checkpoint-interactions.js';
import { materializeRenderedTemplate, checkpointRenderedTemplateProjection } from './projection.js';
import { reportInteractionReference } from './interaction-reference.js';

interface BodyRecord { template: HTMLTemplateElement; nodes: ChildNode[]; projection?: CheckpointRenderProjection;
    topology: { node: Node; parent: Node | null; text: string | null; attributes: string | null; id?: string; children: readonly ChildNode[]; produced: boolean }[] }
// Browser projection ownership only. Never recaptured as the instance's native data.
const BODY_OWNERS = Symbol.for('cem.surface-bodies.v1');
const environment = globalThis as typeof globalThis & { [BODY_OWNERS]?: WeakMap<HTMLElement, BodyRecord> };
const bodies = environment[BODY_OWNERS] ??= new WeakMap<HTMLElement, BodyRecord>();
export function ownsCemSurfaceBodyNode(owner: HTMLElement, node: Node): boolean {
    return bodies.get(owner)?.nodes.includes(node as ChildNode) ?? false;
}
function bodyAttributes(node: Node): string | null {
    if (node.nodeType !== 1) return null;
    const element = node as Element;
    return JSON.stringify([...element.attributes].filter(attribute => !element.matches('input,textarea,select,button') || !['disabled', 'readonly'].includes(attribute.name))
        .map(attribute => [attribute.name, attribute.value]).sort(([a], [b]) => a.localeCompare(b)));
}
export class CemSurfaceBody {
    /** Preserved materialized bodies are a separate owned projection domain. */
    checkpoint(resolve: (node: HTMLElement) => CheckpointEndpoint | undefined = () => undefined): CheckpointDeferredBody | null | undefined {
        const contract = this.contract(); if (!contract) return undefined;
        const record = bodies.get(this.owner);
        // Comments are harmless to ordinary materialization, but unaccounted
        // markers must not become part of a portable body ownership claim.
        if (contract.template && [...this.owner.childNodes].some(node => node.nodeType === Node.COMMENT_NODE && !record?.nodes.includes(node) && !this.chrome(node))) return undefined;
        if (!contract.template) return record ? undefined : null;
        if (contract.mode === 'eager') return record ? undefined : null;
        const templateNode = (contract.template as HTMLTemplateElement & { cemRenderNodeId?: string }).cemRenderNodeId;
        if (!templateNode || contract.template.getAttribute('data-cem-render-node-id') !== templateNode
            || !checkpointRenderedTemplateProjection(contract.template) || record && (!record.projection || record.topology.length > 4096)) return undefined;
        const owners: NonNullable<CheckpointDeferredBody['owners']> = [];
        for (const entry of record?.topology ?? []) {
            const { node, parent, text, attributes, id, produced, children } = entry;
            if (node.parentNode !== parent || !id || (node as Node & { cemRenderNodeId?: string }).cemRenderNodeId !== id
                || node.nodeType === 1 && (node as Element).getAttribute('data-cem-render-node-id') !== id) return undefined;
            if (produced) {
                const owner = resolve(node as HTMLElement);
                if (!owner || owner.nodeId !== 'host' || owners.length >= 256) return undefined;
                owners.push({ hostNode: id, owner }); continue;
            }
            const container = node instanceof HTMLTemplateElement ? node.content : node;
            if (text !== null && node.textContent !== text || bodyAttributes(node) !== attributes
                || children.length !== container.childNodes.length || children.some((child,index) => child !== container.childNodes[index])
                || node.nodeType === 1 && ((node as Element).matches('[contenteditable]') || (node as Element).getAttribute('data-cem-render-node-id') !== id)) return undefined;
        }
        const roots = (record?.nodes ?? []).map(node => (node as Node & { cemRenderNodeId?: string }).cemRenderNodeId);
        if (roots.length > 256 || roots.some(id => !id) || new Set(roots).size !== roots.length
            || record?.nodes.some(node => node.parentNode !== this.owner)) return undefined;
        return { templateNode, mode: contract.mode as 'retain' | 'dispose', materialized: !!record,
            roots: roots as string[], disposition: 'rematerialize', ...(record?.projection ? { projection: structuredClone(record.projection), reuseOnOpen: record.template === contract.template } : {}),
            ...(owners.length ? { owners } : {}) };
    }
    constructor(private readonly owner: HTMLElement, private readonly host: HTMLElement, private readonly beforeDispose: (nodes: readonly ChildNode[]) => void,
        private readonly chrome: (node: Node) => boolean = () => false) {}
    private contract(): { template?: HTMLTemplateElement; mode: string } | undefined {
        const templates = [...this.owner.children].filter((node): node is HTMLTemplateElement => node.localName === 'template' && node.getAttribute('slot') === 'body');
        const retained = bodies.get(this.owner);
        const mode = this.host.getAttribute('materialize') ?? (templates.length ? 'retain' : 'eager');
        const conflicting = templates.length > 1 || templates.length === 1 && [...this.owner.childNodes].some(node =>
            node !== templates[0] && !retained?.nodes.includes(node as ChildNode) && !this.chrome(node) &&
            (node.nodeType === Node.ELEMENT_NODE || node.nodeType === Node.TEXT_NODE && !!node.textContent?.trim()));
        const code = conflicting ? 'interaction-body-conflict' : !['eager', 'retain', 'dispose'].includes(mode)
            || mode !== 'eager' && templates.length !== 1 ? 'interaction-materialization-invalid' : undefined;
        reportInteractionReference(this.host, code, 'body');
        return code ? undefined : { template: templates[0], mode };
    }
    valid(): boolean { return !!this.contract(); }
    eager(): boolean { return this.contract()?.mode === 'eager'; }
    mount(): boolean {
        const contract = this.contract(); if (!contract) return false;
        const retained = bodies.get(this.owner);
        if (retained && retained.template === contract.template) return true;
        this.dispose();
        if (contract.template) {
            const fragment = materializeRenderedTemplate(contract.template);
            const nodes = [...fragment.childNodes];
            const topology: BodyRecord['topology'] = [];
            const visit = (node: Node) => {
                if (topology.length > 4096) return;
                const produced = node.nodeType === 1 && !!node.ownerDocument?.defaultView?.customElements.get((node as Element).localName);
                const container = node instanceof HTMLTemplateElement ? node.content : node;
                topology.push({ node, parent: node.parentNode, text: node.nodeType === 3 || node.nodeType === 8 ? node.textContent : null,
                    attributes: produced ? null : bodyAttributes(node), id: (node as Node & { cemRenderNodeId?: string }).cemRenderNodeId,
                    children: [...container.childNodes], produced });
                if (!produced) for (const child of container.childNodes) visit(child);
            };
            for (const node of nodes) visit(node);
            bodies.set(this.owner, { template: contract.template, nodes, topology, projection: checkpointRenderedTemplateProjection(contract.template) });
            contract.template.after(fragment);
            for (const entry of topology) entry.parent = entry.node.parentNode;
        }
        return true;
    }
    close(): void { if (this.contract()?.mode === 'dispose') this.dispose(); }
    dispose(): void {
        const record = bodies.get(this.owner); bodies.delete(this.owner);
        if (record) this.beforeDispose(record.nodes);
        for (const node of record?.nodes ?? []) node.remove();
    }
}

/** Wait for finite running exit effects, including canceled transitions. */
export function surfaceExitAnimations(owner: HTMLElement): Promise<unknown>[] {
    return owner.getAnimations({ subtree: true }).filter(animation => animation.playState === 'running' && Number.isFinite(animation.effect?.getComputedTiming().endTime))
        .map(animation => animation.finished.catch(() => undefined));
}
