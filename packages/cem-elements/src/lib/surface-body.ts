import { materializeRenderedTemplate } from './projection.js';
import { reportInteractionReference } from './interaction-reference.js';

interface BodyRecord { template: HTMLTemplateElement; nodes: ChildNode[] }
// Browser projection ownership only. Never recaptured as the instance's native data.
const BODY_OWNERS = Symbol.for('cem.surface-bodies.v1');
const environment = globalThis as typeof globalThis & { [BODY_OWNERS]?: WeakMap<HTMLElement, BodyRecord> };
const bodies = environment[BODY_OWNERS] ??= new WeakMap<HTMLElement, BodyRecord>();
export function ownsCemSurfaceBodyNode(owner: HTMLElement, node: Node): boolean {
    return bodies.get(owner)?.nodes.includes(node as ChildNode) ?? false;
}
export class CemSurfaceBody {
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
            bodies.set(this.owner, { template: contract.template, nodes });
            contract.template.after(fragment);
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
