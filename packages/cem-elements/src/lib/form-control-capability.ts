import type { CemProducedElementBehavior, CemProducedElementBehaviorContext } from './cem-elements.js';

type Control = HTMLInputElement | HTMLTextAreaElement;
interface FormControlState {
    context: CemProducedElementBehaviorContext;
    authoredValue: string | null | undefined;
    formDisabled: boolean;
    customValidity: string;
    onInput?: EventListener;
    onKeyDown?: EventListener;
}
const states = new WeakMap<HTMLElement, FormControlState>();
const validityKeys = [
    'badInput', 'customError', 'patternMismatch', 'rangeOverflow', 'rangeUnderflow',
    'stepMismatch', 'tooLong', 'tooShort', 'typeMismatch', 'valueMissing',
] as const;

/** A single string-valued native control; the produced host owns submission. */
export const CEM_FORM_CONTROL_CAPABILITY: CemProducedElementBehavior = {
    formAssociated: true,
    constructed(instance, context) {
        const state: FormControlState = { context, authoredValue: undefined, formDisabled: false, customValidity: '' };
        states.set(instance, state);
        Object.defineProperties(instance, {
            value: { configurable: true, get: () => controlFor(instance)?.value ?? String(context.snapshot().slices.value ?? ''),
                set: (value: unknown) => context.setSlices({ value: String(value) }) },
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
        state.onInput = event => {
            if (event.target === controlFor(instance)) synchronize(instance, state);
        };
        instance.addEventListener('input', state.onInput);
        instance.addEventListener('change', state.onInput);
        state.onKeyDown = event => implicitSubmit(instance, state, event as KeyboardEvent);
        instance.addEventListener('keydown', state.onKeyDown);
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (state.onInput) {
            instance.removeEventListener('input', state.onInput);
            instance.removeEventListener('change', state.onInput);
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
        }
        context.setSlices(slices, { render: false });
    },
    rendered(instance) {
        synchronize(instance, stateFor(instance));
    },
    formDisabled(instance, disabled, context) {
        const state = stateFor(instance);
        state.formDisabled = disabled;
        context.requestRender();
    },
    formReset(instance, context) {
        context.setSlices({ value: instance.getAttribute('value') ?? '' });
    },
    formStateRestore(_instance, restored, _mode, context) {
        // This capability owns one string value, unlike file and multi-choice controls.
        if (typeof restored === 'string' || restored === null) {
            context.setSlices({ value: restored ?? '' });
        }
    },
};

function stateFor(instance: HTMLElement): FormControlState {
    const state = states.get(instance);
    if (!state) throw new Error('Form control capability was not constructed');
    return state;
}

function controlFor(instance: HTMLElement): Control | null {
    return instance.querySelector<Control>('input[part~="control"], textarea[part~="control"]');
}

function synchronize(instance: HTMLElement, state: FormControlState): void {
    const control = controlFor(instance);
    const internals = state.context.internals;
    if (!control || !internals) return;
    control.setCustomValidity(state.customValidity);
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
    if (!event.isTrusted || event.key !== 'Enter' || event.isComposing || event.defaultPrevented
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
