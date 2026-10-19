import type { PatchFrame, RenderPlanNode, SerializedNode } from './projection.js';

// Checked before publication, including incremental attribute changes. CSP and
// sandbox are independent backstops; document navigation has no general CSP gate.
function attribute(name: string, value: string | null): void {
    if (value === null) return;
    const key = name.toLowerCase();
    if (key.startsWith('on') || ['srcdoc', 'href', 'xlink:href', 'action', 'formaction', 'target', 'formtarget', 'ping', 'autofocus', 'is'].includes(key)) {
        throw new Error(`cem.preview.attribute_denied: ${name}`);
    }
}
function element(tag: string, namespace: string | null | undefined): void {
    if (namespace && namespace !== 'http://www.w3.org/1999/xhtml'
        || ['cem-element', 'script', 'base', 'meta', 'link', 'iframe', 'frame', 'frameset', 'object', 'embed', 'portal', 'html', 'head', 'body'].includes(tag.toLowerCase())) {
        throw new Error(`cem.preview.element_denied: ${tag}`);
    }
}
function serialized(node: SerializedNode): void {
    if (node.kind !== 'element') return;
    element(node.tagName, node.namespace);
    for (const [name, value] of Object.entries(node.attributes)) attribute(name, value);
    node.children.forEach(serialized);
}
export function validateLivePreviewFrames(frames: readonly PatchFrame[]): void {
    for (const frame of frames) if (frame.type === 'ops') for (const op of frame.ops) {
        if (op.op === 'setAttribute') attribute(op.name, op.value);
        else if ((op.op === 'reconcileChildren' || op.op === 'reconcileRoots')) op.children.forEach(child => serialized(child.node));
        else if ('node' in op) serialized(op.node.node);
    }
}
export function validateLivePreviewNodes(nodes: readonly RenderPlanNode[]): void {
    for (const node of nodes) if (node.kind === 'element') {
        element(node.tag, node.namespace);
        node.attributes.forEach(({ name, value }) => attribute(name, value));
        validateLivePreviewNodes(node.children);
    }
}
