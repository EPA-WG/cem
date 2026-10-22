import type { CheckpointEndpoint } from './checkpoint-interactions.js';
import { isDomInstance } from './dom-realm.js';
import { interactionReference as reference, nativeInteractionReference as nativeReference } from './interaction-reference.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { captureCemSurfaceInvocation, type CemSurfaceInvocation } from './surface-invocation.js';

/** Transient browser invocation metadata; portable CEM context stays in its typed channel. */
export interface CemActionInvocation extends CemSurfaceInvocation {
    readonly source: HTMLButtonElement;
    readonly command: string | null;
    readonly contextKey: string | null;
}
const invocations = new WeakMap<HTMLButtonElement, CemActionInvocation>();
export function getCemActionInvocation(source: HTMLButtonElement): CemActionInvocation | undefined {
    return invocations.get(source);
}
interface State { abort: AbortController; root: Document | ShadowRoot; error?: string; wired?: boolean; }
interface Resolution { descriptor?: Element; target?: Element; command?: string; contextKey?: string | null; code?: string; }
const states = new WeakMap<HTMLElement, State>();
const roots = new WeakMap<Document | ShadowRoot, { hosts: Set<HTMLElement>; observer: MutationObserver; abort: AbortController }>();
const nativeCommands = new Set(['toggle-popover', 'show-popover', 'hide-popover', 'show-modal', 'close', 'request-close']);
let sequence = 0;
function attribute(element: Element, name: string, value: string | null): void {
    if (value === null) { if (element.hasAttribute(name)) element.removeAttribute(name); }
    else if (element.getAttribute(name) !== value) element.setAttribute(name, value);
}
function control(host: HTMLElement): HTMLButtonElement | null {
    return host.querySelector<HTMLButtonElement>(':scope > button[part~="control"]');
}
function resolve(host: HTMLElement): Resolution {
    let descriptor: Element | undefined;
    if (host.hasAttribute('interaction')) {
        const result = reference(host, host.getAttribute('interaction') ?? '', 'interaction');
        if (result.code) return result;
        descriptor = result.target;
        if (descriptor?.localName !== 'cem-interaction' || descriptor.hasAttribute('interaction')) {
            return { code: 'interaction-reference-conflict' };
        }
    }
    const targetName = host.getAttribute('command-target') ?? descriptor?.getAttribute('command-target');
    const command = host.getAttribute('command') ?? descriptor?.getAttribute('command') ?? undefined;
    const contextKey = host.getAttribute('context-key') ?? descriptor?.getAttribute('context-key') ?? null;
    if (targetName !== undefined && targetName !== null) {
        if (host.hasAttribute('commandfor') || host.hasAttribute('popovertarget')) return { code: 'interaction-dual-route' };
        const result = reference(host.hasAttribute('command-target') ? host : (descriptor ?? host), targetName, 'command-target');
        if (result.code) return result;
        if (result.target?.localName === 'cem-interaction') return { code: 'interaction-reference-conflict' };
        if (descriptor && host.hasAttribute('command-target')) {
            const inherited = reference(descriptor, descriptor.getAttribute('command-target') ?? '', 'command-target');
            if (inherited.code || inherited.target !== result.target) return { code: 'interaction-reference-conflict' };
        }
        if (!command || (!nativeCommands.has(command) && !command.startsWith('--'))) {
            return { code: 'interaction-command-incompatible' };
        }
        if (nativeCommands.has(command)) {
            const target = result.target;
            if (!target) return { code: 'interaction-reference-missing' };
            const owners = target.matches('dialog,[popover]') ? [target] : [...target.querySelectorAll(':scope > [part~="surface"]')];
            if (owners.length !== 1) return { code: 'interaction-surface-ambiguous' };
            result.target = owners[0];
        }
        return { ...result, command, contextKey, descriptor };
    }
    if (descriptor) return { code: 'interaction-reference-missing' };
    if (host.hasAttribute('commandfor') && host.hasAttribute('popovertarget')) return { code: 'interaction-dual-route' };
    if (host.hasAttribute('commandfor') && (!command || (!nativeCommands.has(command) && !command.startsWith('--')))) {
        return { code: 'interaction-command-incompatible' };
    }
    const nativeTarget = host.getAttribute('commandfor') ?? host.getAttribute('popovertarget');
    if (nativeTarget !== null) {
        const result = nativeReference(host, nativeTarget);
        if (result.code) return result;
        return { ...result, command: host.hasAttribute('commandfor') ? command : undefined, contextKey };
    }
    return { contextKey };
}
function compatible(target: Element, command: string | undefined): boolean {
    if (!command) return target.hasAttribute('popover');
    if (command.startsWith('--')) return true;
    if (command.endsWith('-popover')) return target.hasAttribute('popover');
    return target.localName === 'dialog';
}
function report(host: HTMLElement, code: string, force = false): void {
    const state = states.get(host);
    if (!state || (!force && state.error === code)) return;
    state.error = code;
    host.dispatchEvent(new CustomEvent('cem-interaction-error', {
        bubbles: true, detail: { code, source: control(host), command: host.getAttribute('command'), contextKey: host.getAttribute('context-key') },
    }));
}
function restoreAria(host: HTMLElement, button: HTMLButtonElement): void {
    attribute(button, 'aria-controls', host.getAttribute('aria-controls'));
    attribute(button, 'aria-expanded', host.getAttribute('aria-expanded') ?? host.getAttribute('expanded'));
}
function synchronize(host: HTMLElement, emitError = true): string | undefined {
    const button = control(host);
    if (!button) return;
    const cemRoute = host.hasAttribute('command-target') || host.hasAttribute('interaction');
    if (!cemRoute && !host.hasAttribute('commandfor') && !host.hasAttribute('popovertarget') && !states.get(host)?.wired) return;
    const result = resolve(host);
    let code = result.code;
    // Surface-command wiring must never turn a form submit/reset into an invoker.
    if (button.type !== 'button' && (cemRoute || host.hasAttribute('commandfor') || host.hasAttribute('popovertarget'))) {
        code = 'interaction-command-incompatible';
    }
    if (result.target && !compatible(result.target, result.command)) code = 'interaction-command-incompatible';
    if (code) {
        if (cemRoute) {
            attribute(button, 'commandfor', host.getAttribute('commandfor'));
            attribute(button, 'command', host.getAttribute('command'));
        }
        if (cemRoute || states.get(host)?.wired) restoreAria(host, button);
        if (emitError) report(host, code);
        return code;
    }
    const state = states.get(host);
    if (state) state.error = undefined;
    if (cemRoute && result.target) {
        if (state) state.wired = true;
        if (!result.target.id) {
            const root = host.getRootNode() as Document | ShadowRoot;
            do { result.target.id = `cem-command-target-${++sequence}`; } while (root.querySelectorAll(`[id="${result.target.id}"]`).length > 1);
        }
        attribute(button, 'commandfor', result.target.id);
        attribute(button, 'command', result.command ?? null);
        if (result.target.matches('dialog,[popover]')) {
            attribute(button, 'aria-controls', result.target.id);
            const expanded = result.target.hasAttribute('popover') ? result.target.matches(':popover-open') : result.target.hasAttribute('open');
            attribute(button, 'aria-expanded', host.getAttribute('aria-expanded') ?? host.getAttribute('expanded') ?? String(expanded));
        }
    } else {
        if (cemRoute || states.get(host)?.wired) restoreAria(host, button);
        if (state) state.wired = false;
        attribute(button, 'commandfor', host.getAttribute('commandfor'));
        attribute(button, 'command', host.getAttribute('command'));
    }
    return;
}
function observe(host: HTMLElement, root: Document | ShadowRoot): void {
    let group = roots.get(root);
    if (!group) {
        const hosts = new Set<HTMLElement>();
        const abort = new AbortController();
        const update = () => { for (const member of hosts) synchronize(member); };
        const observer = new MutationObserver(update);
        observer.observe(root, { childList: true, subtree: true, attributes: true,
            attributeFilter: ['id', 'data-cem-node-ref-command-target', 'data-cem-node-ref-interaction', 'interaction-scope', 'interaction-name', 'command-target', 'interaction', 'command', 'commandfor', 'popovertarget', 'popover', 'context-key', 'open', 'type', 'part', 'expanded', 'aria-expanded'] });
        root.addEventListener('toggle', update, { capture: true, signal: abort.signal });
        root.addEventListener('close', update, { capture: true, signal: abort.signal });
        group = { hosts, observer, abort };
        roots.set(root, group);
    }
    group.hosts.add(host);
}
/** Reusable invoker wiring; the target's capability owns its semantic lifecycle. */
export const CEM_ACTION_COMMAND_CAPABILITY: CemProducedElementBehavior = {
    connected(host) {
        if (states.has(host)) return;
        const root = host.getRootNode();
        if (!isDomInstance(root, 'Document') && !isDomInstance(root, 'ShadowRoot')) return;
        const abort = new AbortController();
        states.set(host, { root, abort });
        if (host.matches('[command-target],[interaction],[commandfor],[popovertarget]')) observe(host, root);
        host.addEventListener('click', event => {
            const button = control(host);
            if (!button || !button.contains(event.target as Node) || event.defaultPrevented || button.disabled) return;
            invocations.delete(button);
            const code = synchronize(host, false);
            if (code) {
                report(host, code, true);
                // Keep unrelated native submit/reset behavior; invalid commands cannot execute.
                if (button.type === 'button') event.preventDefault();
                return;
            }
            const result = resolve(host);
            invocations.set(button, Object.freeze({ ...captureCemSurfaceInvocation(button, event), source: button,
                command: result.command ?? null, contextKey: result.contextKey ?? null }));
        }, { capture: true, signal: abort.signal });
    },
    rendered(host) {
        const state = states.get(host);
        if (state && host.matches('[command-target],[interaction],[commandfor],[popovertarget]')) observe(host, state.root);
        synchronize(host);
    },
    preserveRenderedAttribute(host, current, _desired, attribute) {
        return current === control(host) && (host.hasAttribute('command-target') || host.hasAttribute('interaction')) &&
            ['commandfor', 'command', 'aria-controls', 'aria-expanded'].includes(attribute.name);
    },
    disconnected(host) {
        const state = states.get(host);
        if (!state) return;
        state.abort.abort();
        const button = control(host);
        if (button) invocations.delete(button);
        const group = roots.get(state.root);
        group?.hosts.delete(host);
        if (group && group.hosts.size === 0) {
            group.observer.disconnect(); group.abort.abort(); roots.delete(state.root);
        }
        states.delete(host);
    },
};

/** Read-only controller/route inspection. No synchronization, ID allocation or invocation. */
function checkpointAction(host: HTMLElement) {
    const state = states.get(host), button = control(host);
    if (!state || state.abort.signal.aborted || state.error || !host.isConnected || !button || state.root !== host.getRootNode()) return;
    if (host.hasAttribute('interestfor') || button.hasAttribute('interestfor')) return;
    const route = resolve(host);
    if (route.code || route.target && (!compatible(route.target, route.command) || button.type !== 'button')) return;
    if (route.target) {
        const unique = nativeReference(button, route.target.id);
        if (unique.code || unique.target !== route.target) return;
        if (button.hasAttribute('commandfor')) {
            if (button.getAttribute('commandfor') !== route.target.id || button.getAttribute('command') !== (route.command ?? null)
                || 'commandForElement' in button && (button as HTMLButtonElement & { commandForElement: Element | null }).commandForElement !== route.target) return;
        } else if (button.hasAttribute('popovertarget')) {
            if (button.getAttribute('popovertarget') !== route.target.id || button.popoverTargetElement !== route.target) return;
        } else return;
    } else if (button.hasAttribute('commandfor') || button.hasAttribute('popovertarget')
        || button.popoverTargetElement || 'commandForElement' in button && (button as HTMLButtonElement & { commandForElement: Element | null }).commandForElement) return;
    return { state, button, route };
}
export function cemActionCommandCheckpoint(host: HTMLElement, endpoint: (node: HTMLElement) => CheckpointEndpoint | undefined) {
    const inspected = checkpointAction(host); if (!inspected) return;
    const { button, route } = inspected, controlNode = button.getAttribute('data-cem-render-node-id');
    const target = route.target ? endpoint(route.target as HTMLElement) : null;
    const descriptor = route.descriptor ? endpoint(route.descriptor as HTMLElement) : null;
    if (!controlNode || route.target && !target || route.descriptor && !descriptor) return;
    return { kind: 'action-command' as const, controlNode, command: route.command ?? null, contextKey: route.contextKey ?? null,
        target: target ?? null, descriptor: descriptor ?? null, disposition: 'retain-without-invocation' as const };
}
/** Original browser objects, retained outside WASM; no authority inferred from IDs. */
export function prepareCemActionCommandGuard(host: HTMLElement) {
    const saved = checkpointAction(host); if (!saved) throw Error('cem.checkpoint.resume.action-stale');
    const invocation = invocations.get(saved.button), wired = saved.state.wired;
    const formState = () => { const button = saved.button, form = button.form; return JSON.stringify([form && [form.action, form.method, form.target, form.enctype, form.noValidate],
        button.formAction, button.formMethod, button.formTarget, button.formEnctype, button.formNoValidate]); };
    const form = saved.button.form, signature = formState();
    return () => { const value = checkpointAction(host); return !!value && value.state === saved.state && value.button === saved.button
        && value.route.target === saved.route.target && value.route.descriptor === saved.route.descriptor
        && value.route.command === saved.route.command && value.route.contextKey === saved.route.contextKey && value.state.wired === wired
        && invocations.get(saved.button) === invocation && saved.button.form === form && formState() === signature; };
}
