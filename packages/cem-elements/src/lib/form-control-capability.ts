import type { CemProducedElementBehavior, CemProducedElementBehaviorContext } from './cem-elements.js';
import { createCemEditorAttributeClaim, type CemEditorAttributeClaim } from './editor-attributes.js';

type Control = HTMLInputElement | HTMLTextAreaElement;
export type CemEditorChangeCause = 'input' | 'programmatic' | 'reset' | 'restore' | 'authored' | 'commit' | 'rebind' | 'availability' | 'composition-start' | 'composition-end' | 'claims' | 'attribute-claims';
export interface CemEditorUpdate { cause: CemEditorChangeCause; revision: number; control: Control | null }
export interface CemEditorCommit {
    revision: number;
    /** Lifecycle-prepared candidate/placement checks; synchronous and side-effect free. */
    current?(): boolean;
    /** Internal consumer provenance update, before validity and public notifications. */
    applied?(revision: number): void;
}
export interface CemEditorLease {
    readonly valid: boolean;
    readonly attributes: CemEditorAttributeClaim;
    handlePress(event: KeyboardEvent): boolean;
    commit(value: string, request: CemEditorCommit): boolean;
    release(): void;
}
export interface CemEditorProvider {
    readonly control: Control | null;
    readonly revision: number;
    readonly composing: boolean;
    readonly editable: boolean;
    compositionOwned(event: KeyboardEvent): boolean;
    subscribe(listener: (update: CemEditorUpdate) => void): () => void;
    lease(owner: object): CemEditorLease;
    validity(owner: object): { set(message: string): boolean; release(): void };
}
interface FormControlState {
    context: CemProducedElementBehaviorContext;
    authoredValue: string | null | undefined;
    formDisabled: boolean;
    customValidity: string;
    onInput?: EventListener;
    onKeyDown?: EventListener;
    abort?: AbortController;
    provider: CemEditorProvider;
    control: Control | null;
    revision: number;
    composing: boolean;
    imePresses: Set<string>;
    handledPresses: Map<string, object>;
    listeners: Set<(update: CemEditorUpdate) => void>;
    leases: Set<object>;
    attributeClaims: Map<object, CemEditorAttributeClaim>;
    validityClaims: Map<object, string>;
    generation: number;
    checkpoint?: { control: Control; revision: number; value: string };
}
const states = new WeakMap<HTMLElement, FormControlState>();
const providerEvents = new WeakSet<Event>();
// Source and packaged runtimes can coexist. Share only transient provider/event
// identities in this realm; never serialize them into an island or native input.
const EDITOR_PROVIDERS = Symbol.for('cem.editor-providers.v2');
const COMPOSITION_KEYS = Symbol.for('cem.editor-composition-keys.v1');
const EDITOR_LEASES = Symbol.for('cem.editor-leases.v1');
const environment = globalThis as typeof globalThis & {
    [EDITOR_PROVIDERS]?: WeakMap<HTMLElement, CemEditorProvider>;
    [COMPOSITION_KEYS]?: WeakSet<KeyboardEvent>;
    [EDITOR_LEASES]?: WeakMap<CemEditorLease, CemEditorProvider>;
};
const providers = environment[EDITOR_PROVIDERS] ??= new WeakMap();
const compositionKeys = environment[COMPOSITION_KEYS] ??= new WeakSet();
const editorLeases = environment[EDITOR_LEASES] ??= new WeakMap();
/** Exact transient provider ownership; a copied record cannot stand in for a lease. */
export function isCemEditorLeaseFor(lease: CemEditorLease, provider: CemEditorProvider): boolean { return editorLeases.get(lease) === provider; }
/** Captured before compositionend; native surface routes must keep this ownership. */
export function isCemEditorCompositionKey(event: KeyboardEvent): boolean { return event.isComposing || compositionKeys.has(event); }
/** Provider identity is established by the shared form capability, never a DOM selector. */
export function getCemEditorProvider(host: HTMLElement): CemEditorProvider | undefined { return providers.get(host); }
const validityKeys = [
    'badInput', 'customError', 'patternMismatch', 'rangeOverflow', 'rangeUnderflow',
    'stepMismatch', 'tooLong', 'tooShort', 'typeMismatch', 'valueMissing',
] as const;

/** A single string-valued native control; the produced host owns submission. */
export const CEM_FORM_CONTROL_CAPABILITY: CemProducedElementBehavior = {
    formAssociated: true,
    constructed(instance, context) {
        const state: FormControlState = { context, authoredValue: undefined, formDisabled: false, customValidity: '',
            provider: undefined as unknown as CemEditorProvider, control: null, revision: 0, composing: false,
            imePresses: new Set(), handledPresses: new Map(), listeners: new Set(), leases: new Set(), attributeClaims: new Map(), validityClaims: new Map(), generation: 0 };
        states.set(instance, state);
        state.provider = editorProvider(instance, state);
        providers.set(instance, state.provider);
        Object.defineProperties(instance, {
            value: { configurable: true, get: () => controlFor(instance)?.value ?? String(context.snapshot().slices.value ?? ''),
                set: (value: unknown) => replaceValue(instance, state, String(value), 'programmatic') },
            defaultValue: { configurable: true, get: () => instance.getAttribute('value') ?? '',
                set: (value: unknown) => instance.setAttribute('value', String(value)) },
            form: { configurable: true, get: () => context.internals?.form ?? null },
            validity: { configurable: true, get: () => context.internals?.validity },
            validationMessage: { configurable: true, get: () => context.internals?.validationMessage ?? '' },
            willValidate: { configurable: true, get: () => context.internals?.willValidate ?? false },
            labels: { configurable: true, get: () => context.internals?.labels },
            checkValidity: { configurable: true, value: () => { synchronize(instance, state); return context.internals?.checkValidity() ?? true; } },
            reportValidity: { configurable: true, value: () => { synchronize(instance, state); return context.internals?.reportValidity() ?? true; } },
            setCustomValidity: { configurable: true, value: (message: string) => {
                state.customValidity = String(message);
                synchronize(instance, state);
            } },
        });
    },
    connected(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        if (state.onInput) return;
        const abort = new AbortController(); state.abort = abort;
        const options = { signal: abort.signal, capture: true };
        state.onInput = event => {
            if (event.target !== controlFor(instance) || providerEvents.has(event)) return;
            if (event.type === 'change') {
                const saved = state.checkpoint, control = controlFor(instance);
                if (event.isTrusted && saved && saved.control === control && saved.revision === state.revision && saved.value === control?.value) {
                    event.stopImmediatePropagation(); state.checkpoint = undefined; return;
                }
            } else {
                state.checkpoint = undefined;
                state.context.setSlices({ value: controlFor(instance)?.value ?? '' }, { render: false });
                publish(instance, state, 'input', true);
            }
            synchronize(instance, state);
        };
        instance.addEventListener('input', state.onInput, options);
        instance.addEventListener('change', state.onInput, options);
        instance.addEventListener('compositionstart', event => {
            if (event.target !== controlFor(instance)) return;
            state.composing = true; publish(instance, state, 'composition-start');
        }, options);
        instance.addEventListener('compositionend', event => {
            if (event.target !== controlFor(instance)) return;
            state.composing = false;
            const control = controlFor(instance), revision = state.revision;
            setTimeout(() => {
                if (state.abort !== abort || abort.signal.aborted || state.composing || control !== controlFor(instance)) return;
                // Final input may follow compositionend. Its own edit revision wins.
                if (revision === state.revision) synchronize(instance, state);
                publish(instance, state, 'composition-end');
            });
        }, options);
        instance.addEventListener('keydown', event => {
            if (event.target !== controlFor(instance)) return;
            const key = event as KeyboardEvent;
            const press = key.code || key.key;
            if (state.composing || key.isComposing || key.key === 'Process' || key.keyCode === 229 || state.imePresses.has(press)) {
                state.imePresses.add(press); compositionKeys.add(key);
            } else if (state.handledPresses.has(press)) {
                key.preventDefault(); key.stopPropagation();
            }
        }, options);
        instance.addEventListener('keyup', event => {
            const key = event as KeyboardEvent, press = key.code || key.key;
            state.imePresses.delete(press); state.handledPresses.delete(press);
        }, options);
        instance.addEventListener('focusout', event => {
            if (event.target !== controlFor(instance)) return;
            state.composing = false; state.imePresses.clear(); state.handledPresses.clear();
        }, options);
        state.onKeyDown = event => implicitSubmit(instance, state, event as KeyboardEvent);
        instance.addEventListener('keydown', state.onKeyDown);
    },
    disconnected(instance) {
        const state = stateFor(instance);
        state.abort?.abort(); state.abort = undefined;
        state.composing = false; state.imePresses.clear(); state.handledPresses.clear(); state.checkpoint = undefined;
        state.generation++; state.leases.clear(); state.validityClaims.clear();
        for (const claim of state.attributeClaims.values()) claim.dispose(); state.attributeClaims.clear();
        publish(instance, state, 'availability');
        if (state.onInput) {
            instance.removeEventListener('input', state.onInput, true);
            instance.removeEventListener('change', state.onInput, true);
            state.onInput = undefined;
        }
        if (state.onKeyDown) {
            instance.removeEventListener('keydown', state.onKeyDown);
            state.onKeyDown = undefined;
        }
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        const authored = instance.getAttribute('value');
        const slices: Record<string, unknown> = { formDisabled: state.formDisabled || instance.hasAttribute('disabled') };
        if (state.authoredValue !== authored) {
            // On resume, a retained live slice wins over initial host attributes.
            if (state.authoredValue !== undefined || context.snapshot().slices.value === undefined) {
                slices.value = authored ?? '';
            }
            state.authoredValue = authored;
            publish(instance, state, 'authored', true);
        }
        context.setSlices(slices, { render: false });
    },
    rendered(instance) {
        const state = stateFor(instance), control = controlFor(instance);
        if (state.control !== control) {
            state.control = control; state.generation++; state.leases.clear(); state.validityClaims.clear();
            for (const claim of state.attributeClaims.values()) claim.dispose(); state.attributeClaims.clear();
            publish(instance, state, 'rebind', true);
        }
        synchronize(instance, state);
        publish(instance, state, 'availability');
    },
    preserveRenderedAttribute(instance, current, desired, attribute) {
        const state = stateFor(instance);
        if (current !== controlFor(instance) || desired.hasAttribute(attribute.name)
            && desired.getAttribute(attribute.name) !== current.getAttribute(attribute.name)) return false;
        return [...state.attributeClaims.values()].some(claim => claim.preserves(attribute.name));
    },
    formDisabled(instance, disabled, context) {
        const state = stateFor(instance);
        state.formDisabled = disabled;
        publish(instance, state, 'availability');
        context.requestRender();
    },
    formReset(instance) {
        replaceValue(instance, stateFor(instance), instance.getAttribute('value') ?? '', 'reset');
    },
    formStateRestore(instance, restored) {
        // This capability owns one string value, unlike file and multi-choice controls.
        if (typeof restored === 'string' || restored === null) {
            replaceValue(instance, stateFor(instance), restored ?? '', 'restore');
        }
    },
};

function stateFor(instance: HTMLElement): FormControlState {
    const state = states.get(instance);
    if (!state) throw new Error('Form control capability was not constructed');
    return state;
}

function controlFor(instance: HTMLElement): Control | null {
    const candidates = [...instance.querySelectorAll<Control>('input[part~="control"], textarea[part~="control"]')].filter(control => {
        for (let parent = control.parentElement; parent && parent !== instance; parent = parent.parentElement) {
            if (states.has(parent) || parent.localName.includes('-')) return false;
        }
        return true;
    });
    return candidates.length === 1 ? candidates[0] : null;
}

function publish(instance: HTMLElement, state: FormControlState, cause: CemEditorChangeCause, edit = false): void {
    if (edit) { state.revision++; state.checkpoint = undefined; }
    for (const claim of state.attributeClaims.values()) { if (edit) claim.clear(); else claim.refresh(); }
    const update = { cause, revision: state.revision, control: controlFor(instance) };
    for (const listener of [...state.listeners]) listener(update);
}

function replaceValue(instance: HTMLElement, state: FormControlState, value: string, cause: CemEditorChangeCause): void {
    const control = controlFor(instance);
    if (control) control.value = value;
    state.context.setSlices({ value: control?.value ?? value }, { render: false });
    publish(instance, state, cause, true);
    synchronize(instance, state);
    state.context.requestRender();
}

function editable(instance: HTMLElement, state: FormControlState): boolean {
    const control = controlFor(instance);
    return !!control && instance.isConnected && control.isConnected && !state.formDisabled
        && !instance.hasAttribute('disabled') && !instance.hasAttribute('readonly')
        && !control.matches(':disabled') && !control.readOnly;
}

function editorProvider(instance: HTMLElement, state: FormControlState): CemEditorProvider {
    return {
        get control() { return controlFor(instance); },
        get revision() { return state.revision; },
        get composing() { return state.composing; },
        get editable() { return editable(instance, state); },
        compositionOwned(event) { return state.composing || isCemEditorCompositionKey(event) || state.imePresses.has(event.code || event.key); },
        subscribe(listener) { state.listeners.add(listener); return () => state.listeners.delete(listener); },
        validity(owner) {
            const token = { owner }, generation = state.generation; let released = false;
            return {
                set(message) {
                    if (released || generation !== state.generation || !instance.isConnected || !state.abort) return false;
                    if (message) state.validityClaims.set(token, message); else state.validityClaims.delete(token);
                    synchronize(instance, state); return true;
                },
                release() {
                    if (released) return;
                    released = true; state.validityClaims.delete(token); synchronize(instance, state);
                },
            };
        },
        lease(owner) {
            // A token is per claim, so repeated claims by one owner still conflict.
            const token = { owner }, admitted = controlFor(instance);
            state.leases.add(token); publish(instance, state, 'claims');
            let released = false;
            const valid = () => !released && state.leases.has(token) && instance.isConnected && !!state.abort
                && state.leases.size === 1 && !!admitted && controlFor(instance) === admitted;
            const attributes = admitted ? createCemEditorAttributeClaim(admitted, () => valid() && editable(instance, state),
                () => state.revision, () => publish(instance, state, 'attribute-claims')) : {
                    valid: false, set: () => false, refresh: () => undefined, clear: () => undefined, dispose: () => undefined, preserves: () => false,
                };
            state.attributeClaims.set(token, attributes);
            const lease: CemEditorLease = {
                get valid() { return valid(); },
                attributes,
                handlePress(event) {
                    if (!valid() || event.defaultPrevented || event.target !== controlFor(instance)
                        || !['Enter', 'Escape'].includes(event.key) || state.provider.compositionOwned(event)) return false;
                    state.handledPresses.set(event.code || event.key, token);
                    event.preventDefault();
                    if (event.key === 'Escape') event.stopPropagation();
                    return true;
                },
                commit(value, request) {
                    const control = controlFor(instance);
                    const current = () => valid() && editable(instance, state) && !state.composing
                        && control === controlFor(instance) && state.revision === request.revision
                        && state.authoredValue === instance.getAttribute('value') && (request.current?.() ?? true);
                    if (!control || !current() || control instanceof HTMLInputElement && control.type === 'file') return false;
                    const probe = control.cloneNode(false) as Control; probe.value = value;
                    if (probe.value !== value) return false;
                    const before = new InputEvent('beforeinput', { bubbles: true, composed: true, cancelable: true,
                        inputType: 'insertReplacementText', data: value, isComposing: false });
                    if (!control.dispatchEvent(before) || !current()) return false;
                    control.value = value;
                    state.context.setSlices({ value }, { render: false });
                    const revision = ++state.revision; state.checkpoint = undefined;
                    request.applied?.(revision);
                    synchronize(instance, state);
                    publish(instance, state, 'commit');
                    state.context.requestRender();
                    if (state.revision !== revision || control !== controlFor(instance) || !valid()) return true;
                    const input = new InputEvent('input', { bubbles: true, composed: true,
                        inputType: 'insertReplacementText', data: value, isComposing: false });
                    providerEvents.add(input); control.dispatchEvent(input);
                    if (state.revision !== revision || control !== controlFor(instance) || !valid()) return true;
                    const change = new Event('change', { bubbles: true }); providerEvents.add(change);
                    state.checkpoint = { control, revision, value };
                    control.dispatchEvent(change);
                    return true;
                },
                release() {
                    if (released) return;
                    released = true; state.leases.delete(token);
                    attributes.dispose(); state.attributeClaims.delete(token); editorLeases.delete(lease);
                    for (const [press, claim] of state.handledPresses) if (claim === token) state.handledPresses.delete(press);
                    synchronize(instance, state); publish(instance, state, 'claims');
                },
            };
            editorLeases.set(lease, state.provider); return lease;
        },
    };
}

function synchronize(instance: HTMLElement, state: FormControlState): void {
    const control = controlFor(instance);
    const internals = state.context.internals;
    if (!control || !internals) return;
    control.setCustomValidity('');
    const nativeMessage = control.validationMessage, nativeInvalid = !control.validity.valid;
    const selection = state.validityClaims.values().next().value ?? '';
    control.setCustomValidity(state.customValidity || (selection && nativeInvalid ? nativeMessage : selection));
    const disabled = state.formDisabled || instance.hasAttribute('disabled');
    internals.setFormValue(disabled || !instance.getAttribute('name') ? null : control.value, control.value);
    if (disabled || control.readOnly || !control.willValidate) {
        internals.setValidity({});
        return;
    }
    const flags: ValidityStateFlags = {};
    for (const key of validityKeys) if (control.validity[key]) flags[key] = true;
    internals.setValidity(flags, control.validationMessage, control);
}

const implicitSubmissionTypes = new Set([
    'text', 'search', 'tel', 'url', 'email', 'password', 'date', 'month', 'week',
    'time', 'datetime-local', 'number',
]);

function implicitSubmit(instance: HTMLElement, state: FormControlState, event: KeyboardEvent): void {
    const control = controlFor(instance);
    const imeOwned = state.composing || event.isComposing || state.imePresses.has(event.code || event.key);
    if (!event.isTrusted || event.key !== 'Enter' || imeOwned || event.defaultPrevented
        || event.target !== control || !(control instanceof HTMLInputElement)
        || !implicitSubmissionTypes.has(control.type) || control.form) return;
    // A task (not a microtask checkpoint between native listeners) lets outer
    // key handlers cancel first. The detached native input has no
    // default form action, so only this capability submits its host's form.
    setTimeout(() => {
        const form = state.context.internals?.form;
        if (event.defaultPrevented || !instance.isConnected || !form || state.formDisabled
            || instance.hasAttribute('disabled') || control.matches(':disabled')) return;
        synchronize(instance, state);
        const root = form.getRootNode() as Document | ShadowRoot;
        const submitter = [...root.querySelectorAll<HTMLInputElement | HTMLButtonElement>('input, button')]
            .find(node => node.form === form && (node.type === 'submit' || node.type === 'image'));
        if (submitter) {
            if (!submitter.matches(':disabled')) submitter.click();
            return;
        }
        // Count native text controls and the equivalent form-control hosts.
        const blockers = [...form.elements].filter(node => {
            const input = node instanceof HTMLElement && states.has(node) ? controlFor(node) : node;
            return input instanceof HTMLInputElement && implicitSubmissionTypes.has(input.type);
        });
        if (blockers.length <= 1) form.requestSubmit();
    });
}
