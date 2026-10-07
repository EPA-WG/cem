import type { RenderPlan, RenderPlanNode } from './projection.js';
export type PlacementPlanElement = Extract<RenderPlanNode, { kind: 'element' }>;
export interface PlacementRoute { renderNodeId: string; name: string; value: string; interaction: boolean }
export function placementPlanElement(plan: RenderPlan, path: readonly number[]): PlacementPlanElement {
    let nodes = plan.nodes; let node: RenderPlanNode | undefined;
    for (const index of path) { node = nodes[index]; if (node?.kind !== 'element') throw new Error('Placement path does not select an element'); nodes = node.children; }
    if (node?.kind !== 'element') throw new Error('Empty placement path');
    return node;
}
export function placementRoutes(plan: RenderPlan): PlacementRoute[] {
    const result: PlacementRoute[] = [];
    const visit = (nodes: readonly RenderPlanNode[]) => { for (const node of nodes) if (node.kind === 'element') {
        for (const marker of node.attributes.filter(a => a.name.startsWith('data-cem-placement-ref-'))) {
            const name = marker.name.slice('data-cem-placement-ref-'.length);
            const value = node.attributes.find(a => a.name === name)?.value;
            if (value === undefined) throw new Error('Placement marker has no relationship');
            result.push({ renderNodeId: node.renderNodeId, name, value, interaction: node.attributes.some(a => a.name === `data-cem-node-ref-${name}`) });
        }
        visit(node.children);
    } };
    visit(plan.nodes); return result;
}
export function withoutPlacementRelationships(plan: RenderPlan, uses?: readonly { renderNodeId: string; attribute: string }[]): RenderPlan {
    const visit = (nodes: readonly RenderPlanNode[]): RenderPlanNode[] => nodes.map(node => {
        if (node.kind !== 'element') return node;
        const names = new Set(node.attributes.filter(a => a.name.startsWith('data-cem-placement-ref-') && (!uses || uses.some(use => use.renderNodeId === node.renderNodeId && a.name === `data-cem-placement-ref-${use.attribute}`))).flatMap(a => {
            const name = a.name.slice('data-cem-placement-ref-'.length); return [name, a.name, `data-cem-node-ref-${name}`];
        }));
        return { ...node, attributes: node.attributes.filter(a => !names.has(a.name)), children: visit(node.children) };
    });
    return { ...plan, nodes: visit(plan.nodes) };
}
export function withPlacementId(plan: RenderPlan, path: readonly number[], id: string): RenderPlan {
    placementPlanElement(plan, path);
    const visit = (nodes: RenderPlanNode[], depth: number): RenderPlanNode[] => nodes.map((node, index) => {
        if (index !== path[depth] || node.kind !== 'element') return node;
        if (depth + 1 < path.length) return { ...node, children: visit(node.children, depth + 1) };
        const current = node.attributes.find(a => a.name === 'id');
        if (current && current.value !== id) throw new Error('Placement ID changed');
        return current ? node : { ...node, attributes: [...node.attributes, { name: 'id', value: id, namespace: null }] };
    });
    return { ...plan, nodes: visit(plan.nodes, 0) };
}
