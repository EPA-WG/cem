/** Browser endpoint access after native AST consumers have published typed IDs. */
export interface InteractionReference { target?: Element; code?: string }
export function nativeInteractionReference(source: Element, id: string): InteractionReference {
    const root = source.getRootNode() as ParentNode;
    const candidates = [...root.querySelectorAll('[id]')].filter(node => node.id === id);
    if (candidates.length > 1) return { code: 'interaction-name-duplicate' };
    return candidates.length === 1 ? { target: candidates[0] } : { code: 'interaction-reference-missing' };
}
export function interactionReference(source: Element, value: string, name?: string): InteractionReference {
    if (name && source.hasAttribute(`data-cem-node-ref-${name}`)) return nativeInteractionReference(source, value);
    if (value.startsWith('#') && value.length > 1) return nativeInteractionReference(source, value.slice(1));
    if (!value.startsWith('@') || !/^[\w.-]+$/.test(value.slice(1))) return { code: 'interaction-reference-missing' };
    const scope = source.closest('[interaction-scope]');
    if (!scope) return { code: 'interaction-reference-missing' };
    const candidates = [...scope.querySelectorAll('[interaction-name]')].filter(node =>
        node.getAttribute('interaction-name') === value.slice(1) && node.closest('[interaction-scope]') === scope);
    if (candidates.length > 1) return { code: 'interaction-name-duplicate' };
    return candidates.length === 1 ? { target: candidates[0] } : { code: 'interaction-reference-missing' };
}
export function interactionControl(target?: Element): HTMLElement | undefined {
    if (!(target instanceof HTMLElement)) return;
    const control = target.matches('button,a[href]') ? target : target.querySelector<HTMLElement>(':scope > [part~="control"]');
    if (control?.matches('button,a[href]')) return control;
    return undefined;
}
const errors = new WeakMap<HTMLElement, Map<string, string>>();
export function reportInteractionReference(host: HTMLElement, code?: string, channel = 'endpoint'): void {
    let channels = errors.get(host);
    if (!code) { channels?.delete(channel); return; }
    if (channels?.get(channel) === code) return;
    if (!channels) { channels = new Map(); errors.set(host, channels); }
    channels.set(channel, code);
    host.dispatchEvent(new CustomEvent('cem-interaction-error', { bubbles: true, detail: { code, source: host } }));
}
const roots = new WeakMap<Node, { observer: MutationObserver; callbacks: Set<() => void> }>();
export function observeInteractionReferences(host: HTMLElement, callback: () => void): () => void {
    const root = host.getRootNode();
    let group = roots.get(root);
    if (!group) {
        const callbacks = new Set<() => void>();
        const observer = new MutationObserver(() => { for (const update of callbacks) update(); });
        observer.observe(root, { subtree: true, childList: true, attributes: true,
            attributeFilter: ['id', 'part', 'slot', 'interaction-name', 'interaction-scope', 'trigger-for', 'parent-item', 'editor-for', 'data-cem-node-ref-editor-for',
                'data-cem-node-ref-trigger-for', 'data-cem-node-ref-parent-item', 'focus-target', 'return-focus', 'anchor', 'boundary', 'anchor-lost',
                'data-cem-node-ref-focus-target', 'data-cem-node-ref-return-focus', 'data-cem-node-ref-anchor', 'data-cem-node-ref-boundary', 'hidden', 'disabled', 'inert', 'tabindex',
                'interestfor', 'kind', 'mode', 'presentation', 'placement', 'fallback', 'overflow', 'context-change', 'show-delay', 'hide-delay'] });
        group = { observer, callbacks }; roots.set(root, group);
    }
    group.callbacks.add(callback);
    return () => {
        group.callbacks.delete(callback);
        if (!group.callbacks.size) { group.observer.disconnect(); roots.delete(root); }
    };
}
