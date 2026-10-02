import type { CemProducedElementBehavior } from './cem-elements.js';

const aborts = new WeakMap<HTMLElement, AbortController>();
/** Native action controls with disabled links and optional projected disclosure. */
export const CEM_ACTION_CONTROL_CAPABILITY: CemProducedElementBehavior = {
    beforeRender(_host, context) {
        context.setSlices({ hasSubmenu: (context.snapshot().payload.slots.submenu?.length ?? 0) > 0 }, { render: false });
    },
    connected(host) {
        if (!host.ownerDocument.defaultView) return;
        if (aborts.has(host)) return;
        const abort = new AbortController();
        aborts.set(host, abort);
        host.addEventListener('click', event => {
            if (host.hasAttribute('disabled') && (event.target as Element).closest('[part~="control"]')?.parentElement === host) {
                event.preventDefault();
                event.stopImmediatePropagation();
            }
        }, { capture: true, signal: abort.signal });
    },
    preserveRenderedAttribute(_host, _current, _desired, attribute) {
        if (_current.parentElement === _host && _current.matches('[slot="submenu"]')) return ['hidden', 'style', 'id'].includes(attribute.name);
        return ['id', 'tabindex', 'aria-controls', 'aria-haspopup', 'aria-expanded'].includes(attribute.name) && !!_current.parentElement?.querySelector(':scope > [slot="submenu"]');
    },
    disconnected(host) { aborts.get(host)?.abort(); aborts.delete(host); },
};
