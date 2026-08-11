import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
    SerializedPayloadNode,
} from '@epa-wg/cem-elements';

export interface CemStepDetail {
    value: string;
    index: number;
    previousIndex: number;
}

interface NormalizedStep {
    value: string;
    label: string;
    children: SerializedPayloadNode[];
    completed: boolean;
    editable: boolean;
    optional: boolean;
    invalid: boolean;
    disabled: boolean;
}

interface RenderStep extends NormalizedStep {
    index: number;
    buttonId: string;
    panelId: string;
    current: boolean;
    unavailable: boolean;
    tabIndex: number;
    marker: string;
    markerState: string;
    status: string;
    connectorCompleted: boolean;
}

interface StepperState {
    connected: boolean;
    context?: CemProducedElementBehaviorContext;
    steps: NormalizedStep[];
    payloadSignature: string;
    warnedPayloadSignature: string;
    issue: string | null;
    ownerId: string;
    focusIndex: number;
    refocusIndex: number;
    onClickCapture?: EventListener;
    onKeyDown?: EventListener;
}

interface NormalizeStepsResult {
    steps: NormalizedStep[];
    issue: string | null;
}

const STEPPER_STATES = new WeakMap<HTMLElement, StepperState>();
let stepperSequence = 0;

export const CEM_STEPPER_BEHAVIOR: CemProducedElementBehavior = {
    constructed(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        installHostApi(instance, state);
    },
    connected(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        if (state.connected) return;
        state.connected = true;
        state.onClickCapture = (event) => handleClick(instance, state, event);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        instance.addEventListener('click', state.onClickCapture, true);
        instance.addEventListener('keydown', state.onKeyDown);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        synchronizeSteps(instance, state, context.snapshot().payload.nodes);
        context.setSlices(renderSlices(instance, state), { render: false });
    },
    rendered(instance) {
        const state = stateFor(instance);
        if (state.refocusIndex < 0) return;
        const index = state.refocusIndex;
        state.refocusIndex = -1;
        directHeader(instance, index)?.focus({ preventScroll: true });
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClickCapture) instance.removeEventListener('click', state.onClickCapture, true);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
    },
};

function stateFor(instance: HTMLElement): StepperState {
    let state = STEPPER_STATES.get(instance);
    if (state) return state;
    stepperSequence += 1;
    state = {
        connected: false,
        steps: [],
        payloadSignature: '',
        warnedPayloadSignature: '',
        issue: null,
        ownerId: `cem-stepper-${stepperSequence}`,
        focusIndex: -1,
        refocusIndex: -1,
    };
    STEPPER_STATES.set(instance, state);
    return state;
}

function synchronizeSteps(
    instance: HTMLElement,
    state: StepperState,
    nodes: readonly SerializedPayloadNode[],
): void {
    const signature = JSON.stringify(nodes);
    if (signature !== state.payloadSignature) {
        const normalized = normalizeSteps(nodes);
        state.steps = normalized.steps;
        state.issue = normalized.issue;
        state.payloadSignature = signature;
        if (normalized.issue && state.warnedPayloadSignature !== signature) {
            state.warnedPayloadSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(`[cem-stepper] ${normalized.issue}`);
        }
    }

    const selected = selectedIndex(instance, state);
    if (!focusableIndex(instance, state, state.focusIndex)) {
        state.focusIndex = focusableIndex(instance, state, selected) ? selected : firstFocusableIndex(instance, state);
    }
}

function normalizeSteps(nodes: readonly SerializedPayloadNode[]): NormalizeStepsResult {
    const elements = nodes.filter(
        (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> => node.kind === 'element',
    );
    const unexpected = nodes.some((node) => node.kind !== 'comment' && (node.kind !== 'element' || node.tag !== 'cem-step'));
    if (unexpected || elements.some((node) => node.tag !== 'cem-step')) {
        return { steps: [], issue: 'Author only direct cem-step element children.' };
    }
    if (elements.length === 0) {
        return { steps: [], issue: 'Author at least one direct cem-step.' };
    }

    const steps: NormalizedStep[] = [];
    const values = new Set<string>();
    for (const node of elements) {
        const value = node.attributes.value?.trim() ?? '';
        const label = node.attributes.label?.trim() ?? '';
        if (!value || !label) {
            return { steps: [], issue: 'Every cem-step requires non-empty value and label attributes.' };
        }
        if (values.has(value)) {
            return { steps: [], issue: `Duplicate cem-step value "${value}" is not allowed.` };
        }
        values.add(value);
        steps.push({
            value,
            label,
            children: node.children,
            completed: Object.hasOwn(node.attributes, 'completed'),
            editable: Object.hasOwn(node.attributes, 'editable'),
            optional: Object.hasOwn(node.attributes, 'optional'),
            invalid: Object.hasOwn(node.attributes, 'invalid'),
            disabled: Object.hasOwn(node.attributes, 'disabled'),
        });
    }
    return { steps, issue: null };
}

function renderSlices(instance: HTMLElement, state: StepperState): Record<string, unknown> {
    const selected = selectedIndex(instance, state);
    const hostDisabled = instance.hasAttribute('disabled');
    const steps: RenderStep[] = state.steps.map((step, index) => {
        const activationEligible = isActivationEligible(instance, state, index, selected);
        const disabled = hostDisabled || step.disabled;
        return {
            ...step,
            disabled,
            index,
            buttonId: `${state.ownerId}-step-${index}`,
            panelId: `${state.ownerId}-panel-${index}`,
            current: index === selected,
            unavailable: disabled || !activationEligible,
            tabIndex: !disabled && index === state.focusIndex ? 0 : -1,
            marker: step.invalid ? '!' : step.completed ? '✓' : String(index + 1),
            markerState: step.invalid ? 'invalid' : step.completed ? 'completed' : 'default',
            status: step.invalid ? 'Error' : step.completed ? 'Complete' : step.optional ? 'Optional' : '',
            connectorCompleted: step.completed && !step.invalid,
        };
    });
    return {
        authoringValid: state.issue === null,
        orientation: normalizedOrientation(instance),
        steps,
    };
}

function normalizedOrientation(instance: HTMLElement): 'horizontal' | 'vertical' {
    return instance.getAttribute('orientation') === 'vertical' ? 'vertical' : 'horizontal';
}

function selectedIndex(instance: HTMLElement, state: StepperState): number {
    if (state.steps.length === 0) return 0;
    const parsed = Number.parseInt(instance.getAttribute('selected-index') ?? '', 10);
    const requested = Number.isFinite(parsed) ? parsed : 0;
    return Math.min(Math.max(requested, 0), state.steps.length - 1);
}

function isActivationEligible(
    instance: HTMLElement,
    state: StepperState,
    index: number,
    selected = selectedIndex(instance, state),
): boolean {
    const step = state.steps[index];
    if (!step || instance.hasAttribute('disabled') || step.disabled) return false;
    if (index === selected) return true;
    if (index < selected && step.completed && !step.editable) return false;
    if (!instance.hasAttribute('linear') || index < selected) return true;
    return state.steps.slice(0, index).every((candidate) =>
        candidate.disabled || (!candidate.invalid && (candidate.completed || candidate.optional)),
    );
}

function focusableIndex(instance: HTMLElement, state: StepperState, index: number): boolean {
    return index >= 0
        && index < state.steps.length
        && !instance.hasAttribute('disabled')
        && !state.steps[index]?.disabled;
}

function firstFocusableIndex(instance: HTMLElement, state: StepperState): number {
    if (instance.hasAttribute('disabled')) return -1;
    return state.steps.findIndex((step) => !step.disabled);
}

function lastFocusableIndex(instance: HTMLElement, state: StepperState): number {
    if (instance.hasAttribute('disabled')) return -1;
    for (let index = state.steps.length - 1; index >= 0; index -= 1) {
        if (!state.steps[index]?.disabled) return index;
    }
    return -1;
}

function handleClick(instance: HTMLElement, state: StepperState, event: Event): void {
    if (event.defaultPrevented) return;
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button.cem-stepper__header') ?? null;
    const index = button ? headerIndex(instance, button) : -1;
    if (index < 0) return;

    const previousIndex = selectedIndex(instance, state);
    if (!isActivationEligible(instance, state, index, previousIndex) || index === previousIndex) {
        if (index !== previousIndex) {
            event.preventDefault();
            event.stopImmediatePropagation();
        }
        return;
    }

    state.focusIndex = index;
    state.refocusIndex = index;
    instance.setAttribute('selected-index', String(index));
    const detail: CemStepDetail = {
        value: state.steps[index]?.value ?? '',
        index,
        previousIndex,
    };
    instance.dispatchEvent(new CustomEvent<CemStepDetail>('cem-step', {
        bubbles: true,
        composed: true,
        detail,
    }));
}

function handleKeyDown(instance: HTMLElement, state: StepperState, event: KeyboardEvent): void {
    const target = event.target instanceof Element
        ? event.target.closest<HTMLButtonElement>('button.cem-stepper__header')
        : null;
    const current = target ? headerIndex(instance, target) : -1;
    if (current < 0 || instance.hasAttribute('disabled')) return;

    const orientation = normalizedOrientation(instance);
    let destination: number;
    if (event.key === 'Home') destination = firstFocusableIndex(instance, state);
    else if (event.key === 'End') destination = lastFocusableIndex(instance, state);
    else if ((orientation === 'horizontal' && event.key === 'ArrowRight')
        || (orientation === 'vertical' && event.key === 'ArrowDown')) {
        destination = adjacentFocusableIndex(instance, state, current, 1);
    } else if ((orientation === 'horizontal' && event.key === 'ArrowLeft')
        || (orientation === 'vertical' && event.key === 'ArrowUp')) {
        destination = adjacentFocusableIndex(instance, state, current, -1);
    } else {
        return;
    }

    if (destination < 0) return;
    event.preventDefault();
    state.focusIndex = destination;
    state.refocusIndex = destination;
    state.context?.setSlices(renderSlices(instance, state));
}

function adjacentFocusableIndex(
    instance: HTMLElement,
    state: StepperState,
    start: number,
    direction: 1 | -1,
): number {
    if (state.steps.length === 0) return -1;
    for (let offset = 1; offset <= state.steps.length; offset += 1) {
        const candidate = (start + direction * offset + state.steps.length) % state.steps.length;
        if (focusableIndex(instance, state, candidate)) return candidate;
    }
    return -1;
}

function headerIndex(instance: HTMLElement, button: HTMLButtonElement): number {
    if (!isDirectHeader(instance, button)) return -1;
    const index = Number.parseInt(button.dataset.stepIndex ?? '', 10);
    return Number.isFinite(index) ? index : -1;
}

function isDirectHeader(instance: HTMLElement, button: HTMLButtonElement): boolean {
    const item = button.parentElement;
    const list = item?.parentElement;
    const surface = list?.parentElement;
    return (
        item?.classList.contains('cem-stepper__item') === true
        && list?.classList.contains('cem-stepper__steps') === true
        && surface?.classList.contains('cem-stepper') === true
        && surface.parentElement === instance
    );
}

function directHeader(instance: HTMLElement, index: number): HTMLButtonElement | null {
    const candidate = instance.querySelector<HTMLButtonElement>(
        `.cem-stepper > .cem-stepper__steps > .cem-stepper__item > .cem-stepper__header[data-step-index="${index}"]`,
    );
    return candidate && isDirectHeader(instance, candidate) ? candidate : null;
}

function installHostApi(instance: HTMLElement, state: StepperState): void {
    if (Object.getOwnPropertyDescriptor(instance, 'selectedIndex')) return;
    Object.defineProperty(instance, 'selectedIndex', {
        configurable: true,
        enumerable: true,
        get: () => selectedIndex(instance, state),
        set: (value: unknown) => {
            const parsed = Number(value);
            const requested = Number.isFinite(parsed) ? Math.floor(parsed) : 0;
            const normalized = state.steps.length > 0
                ? Math.min(Math.max(requested, 0), state.steps.length - 1)
                : 0;
            instance.setAttribute('selected-index', String(normalized));
        },
    });
}
