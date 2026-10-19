import type { CemElementRuntime } from './cem-elements.js';
import { cemPlaintextEditorEventValue } from './plaintext-editor-capability.js';

export interface CemHostCommandRequest {
    readonly command: string;
    readonly eventType: string;
    /** Native/admitted plaintext value or validated tree/tab/artifact identity; never arbitrary event detail. */
    readonly value: string | null;
    readonly isTrusted: boolean;
    /** Native successful string controls, captured only for a form submit command. */
    readonly fields?: Readonly<Record<string, readonly string[]>>;
}

export interface CemHostViewOptions {
    readonly root: Element;
    readonly tag: string;
    /** Bind an authored direct child instead of replacing the root contents. */
    readonly element?: HTMLElement;
    readonly state?: Readonly<Record<string, unknown>>;
    readonly commands?: Readonly<Record<string, (request: CemHostCommandRequest) => unknown>>;
}

export interface CemHostView {
    readonly element: HTMLElement;
    readonly errors: readonly Error[];
    update(values: Readonly<Record<string, unknown>>): boolean;
    whenSettled(): Promise<void>;
    dispose(): void;
}

const boundaries = new WeakSet<Element>();
const mounts = new WeakMap<Element, CemHostView>();
const events = ['click', 'input', 'change', 'submit', 'cem-close', 'cem-tree-activate', 'cem-tab', 'cem-preview-navigate'] as const;

/** Mount declarative UI with an explicit, revocable set of non-UI host services. */
export function mountCemView(runtime: CemElementRuntime, options: CemHostViewOptions): CemHostView {
    const { root, tag } = options;
    const document = root.ownerDocument;
    if (!tag.includes('-') || !document.defaultView?.customElements.get(tag)) {
        throw new TypeError(`Register the declarative view before mounting ${tag}`);
    }
    const element = options.element ?? document.createElement(tag);
    if (options.element && (element.ownerDocument !== document || element.parentElement !== root || element.localName !== tag || boundaries.has(element))) {
        throw new TypeError('An authored view must be an unowned direct child with the requested tag');
    }
    if (!runtime.hasDeclarationFor(element)) throw new TypeError(`No declaration in this runtime for ${tag}`);
    const commands = new Map(Object.entries(options.commands ?? {}));
    const errors: Error[] = [];
    let active = true;
    const report = (error: unknown) => {
        if (active) errors.push(error instanceof Error ? error : new Error(String(error)));
    };

    const invoke = (event: Event) => {
        if (!active || !element.isConnected || !runtime.hasDeclarationFor(element) || event.defaultPrevented) return;
        let control: Element | undefined;
        for (const target of event.composedPath()) {
            if (target === element) break;
            if (!(target instanceof Element)) continue;
            // Never route a nested view's command through its parent, even when
            // the nested command name is not available to that child.
            if (boundaries.has(target)) return;
            control ??= target.hasAttribute('host-command') ? target : undefined;
        }
        if (!control || !element.contains(control)) return;
        // Component events report their bound owner's state. A nested owner
        // must never invoke an ancestor's command through a bubbling event.
        if ((event.type === 'cem-close' || event.type === 'cem-tree-activate' || event.type === 'cem-tab' || event.type === 'cem-preview-navigate') && event.target !== control) return;
        const command = control.getAttribute('host-command') ?? '';
        if ((control.getAttribute('host-command-event') ?? 'click') !== event.type) return;
        const handler = commands.get(command);
        if (!handler || control.hasAttribute('disabled')) return;
        if (event.type === 'submit') event.preventDefault();
        try {
            const fields = event.type === 'submit' ? formFields(event, control, element) : undefined;
            const value = event.type === 'cem-tree-activate' || event.type === 'cem-tab' || event.type === 'cem-preview-navigate' ? activationValue(event) : controlValue(event.target);
            // Input capture (FormData listeners, custom control getters or
            // payload proxy traps) can revoke authority before delivery.
            if (!active || !element.isConnected || !runtime.hasDeclarationFor(element) || !element.contains(control)
                || event.defaultPrevented && event.type !== 'submit'
                || control.hasAttribute('disabled') || commands.get(command) !== handler
                || control.getAttribute('host-command') !== command
                || (control.getAttribute('host-command-event') ?? 'click') !== event.type) return;
            const request = Object.freeze({ command, eventType: event.type, value, isTrusted: event.isTrusted,
                ...(fields ? { fields } : {}) });
            // Do not queue this call: file pickers/install prompts need the
            // same trusted event stack as the native control activation.
            const result = handler(request);
            if (result && typeof (result as PromiseLike<unknown>).then === 'function') {
                void Promise.resolve(result).catch(report);
            }
        } catch (error) {
            report(error);
        }
    };

    const view: CemHostView = Object.freeze({
        element,
        get errors() { return Object.freeze([...errors]); },
        update(values: Readonly<Record<string, unknown>>) {
            return active && runtime.setInstanceSlices(element, values);
        },
        async whenSettled() {
            if (!active) return;
            await runtime.whenRenderSettled(element);
            const visited = new Set<HTMLElement>();
            for (;;) {
                const child = [...element.querySelectorAll<HTMLElement>('*')].find(child =>
                    !visited.has(child) && runtime.hasDeclarationFor(child));
                if (!child || !active) return;
                visited.add(child);
                // An ancestor can publish a new payload to an existing child.
                // Settle in DOM order, then discover its current descendants;
                // parallel waits can observe a child before that update arrives.
                await runtime.whenRenderSettled(child);
            }
        },
        dispose() {
            if (!active) return;
            active = false;
            events.forEach(name => element.removeEventListener(name, invoke));
            commands.clear();
            // Preserve the boundary on a retained detached node so it cannot
            // acquire a parent's command authority if reinserted after disposal.
            if (!options.element) element.remove();
            if (mounts.get(root) === view) mounts.delete(root);
        },
    });
    runtime.setInstanceSlices(element, options.state ?? {}, { render: Boolean(options.element) });
    mounts.get(root)?.dispose();
    boundaries.add(element);
    events.forEach(name => element.addEventListener(name, invoke));
    mounts.set(root, view);
    if (!options.element) root.replaceChildren(element);
    return view;
}

function activationValue(event: Event): string {
    const detail: unknown = event instanceof CustomEvent ? event.detail : undefined;
    // Read only an own data property: never evaluate accessors, stringify live
    // objects or forward arbitrary event detail across the host-service boundary.
    const value: unknown = detail && typeof detail === 'object' && !Array.isArray(detail)
        ? Object.getOwnPropertyDescriptor(detail, 'value')?.value : undefined;
    if (typeof value !== 'string' || !value || (event.type === 'cem-tree-activate' && /\s/u.test(value))) {
        throw new TypeError('Activation commands require an own non-empty string value; tree values cannot contain whitespace');
    }
    return value;
}

function formFields(event: Event, control: Element, view: Element): Readonly<Record<string, readonly string[]>> {
    if (!(control instanceof HTMLFormElement) || event.target !== control) {
        throw new TypeError('A host submit command must be declared on its native form');
    }
    // A form can acquire external controls through `form=id`. Do not silently
    // widen this view's scalar input boundary to a sibling or nested host view.
    for (const field of control.elements) {
        let owner: Element | null = field;
        while (owner && !boundaries.has(owner)) owner = owner.parentElement;
        if (owner !== view) throw new TypeError('Host form controls must belong to the same view');
    }
    const submitter = event instanceof SubmitEvent ? event.submitter : null;
    const fields: Record<string, string[]> = Object.create(null);
    for (const [name, value] of new FormData(control, submitter)) {
        if (typeof value !== 'string') throw new TypeError('Host form commands accept only string values');
        (fields[name] ??= []).push(value);
    }
    Object.values(fields).forEach(Object.freeze);
    return Object.freeze(fields);
}

function controlValue(target: EventTarget | null): string | null {
    const plaintext = cemPlaintextEditorEventValue(target);
    if (plaintext !== undefined) return plaintext;
    if (target instanceof HTMLButtonElement || target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return target.value;
    if (target instanceof HTMLElement && (target.constructor as typeof HTMLElement & { formAssociated?: boolean }).formAssociated) {
        const value = (target as HTMLElement & { value?: unknown }).value;
        if (typeof value === 'string') return value;
    }
    return null;
}
