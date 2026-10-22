import type { CheckpointEndpoint } from './checkpoint-interactions.js';
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

function checkpointActionControl(host: HTMLElement) {
    const abort = aborts.get(host), controls = host.querySelectorAll<HTMLElement>(':scope > [part~="control"]');
    const submenus = host.querySelectorAll<HTMLElement>(':scope > [slot="submenu"]');
    if (!abort || abort.signal.aborted || !host.isConnected || controls.length !== 1 || submenus.length > 1) return;
    const control = controls[0];
    if (!control.matches('button,a') || control.matches('[commandfor],[popovertarget],[interestfor]')
        || (control as HTMLButtonElement).popoverTargetElement
        || 'commandForElement' in control && (control as HTMLButtonElement & { commandForElement: Element | null }).commandForElement) return;
    return { abort, control, submenu: submenus[0] };
}
export function cemActionControlCheckpoint(host: HTMLElement, endpoint: (node: HTMLElement) => CheckpointEndpoint | undefined) {
    const inspected = checkpointActionControl(host); if (!inspected) return;
    const controlNode = inspected.control.getAttribute('data-cem-render-node-id');
    const submenu = inspected.submenu ? endpoint(inspected.submenu) : null;
    if (!controlNode || inspected.submenu && !submenu) return;
    return { kind: 'action-control' as const, controlNode, submenu: submenu ?? null };
}
export function prepareCemActionControlGuard(host: HTMLElement) {
    const saved = checkpointActionControl(host); if (!saved) throw Error('cem.checkpoint.resume.action-stale');
    const href = (saved.control as HTMLAnchorElement).href, form = (saved.control as HTMLButtonElement).form;
    const formState = () => { const button = saved.control as HTMLButtonElement; return JSON.stringify([form && [form.action, form.method, form.target, form.enctype, form.noValidate],
        button.formAction, button.formMethod, button.formTarget, button.formEnctype, button.formNoValidate]); };
    const signature = formState();
    return () => { const value = checkpointActionControl(host); return !!value && value.abort === saved.abort && value.control === saved.control
        && value.submenu === saved.submenu && (value.control as HTMLAnchorElement).href === href && (value.control as HTMLButtonElement).form === form && formState() === signature; };
}
