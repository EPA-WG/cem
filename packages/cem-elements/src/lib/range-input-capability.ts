import type { CemProducedElementBehavior, SerializedPayloadNode } from './cem-elements.js';

type RangeMode = 'invalid' | 'range' | 'single';
type RangeThumb = 'end' | 'single' | 'start';

interface RangeBounds {
    max: number;
    min: number;
    step: number;
}

interface RangeInputs {
    end?: HTMLInputElement;
    mode: RangeMode;
    single?: HTMLInputElement;
    start?: HTMLInputElement;
}

interface RangeState {
    connected: boolean;
    valid: boolean;
    generation: number;
    payloadSignature: string;
    onReset?: EventListener;
    onChangeCapture?: EventListener;
    onInputCapture?: EventListener;
    warnedSignature: string;
}

const DEFAULT_MIN = 0;
const DEFAULT_RANGE = 100;
const DEFAULT_STEP = 1;
const RANGE_STATES = new WeakMap<HTMLElement, RangeState>();

export const CEM_RANGE_INPUT_CAPABILITY: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.generation++;
        state.onReset = event => {
            const inputs = rangeInputs(instance);
            if (![inputs.single, inputs.start, inputs.end].some(input => input?.form === event.target)) return;
            const generation = state.generation;
            instance.ownerDocument.defaultView?.setTimeout(() => {
                if (!event.defaultPrevented && state.connected && generation === state.generation) synchronizeRange(instance);
            }, 0);
        };
        instance.ownerDocument.addEventListener('reset', state.onReset, true);
        state.onInputCapture = (event) => handleNativeValueEvent(instance, event);
        state.onChangeCapture = (event) => handleNativeValueEvent(instance, event);
        instance.addEventListener('input', state.onInputCapture, true);
        instance.addEventListener('change', state.onChangeCapture, true);
    },
    beforeRender(instance, context) {
        const nodes = context.snapshot().payload.nodes;
        stateFor(instance).valid = validPayload(nodes);
        stateFor(instance).payloadSignature = JSON.stringify(nodes);
        const thumbSlots = [...new Set(nodes.flatMap(node => node.kind === 'element' ? [node.attributes.slot || ''] : []))];
        const mode: RangeMode = stateFor(instance).valid
            ? nodes.filter(node => node.kind === 'element').length === 1 ? 'single' : 'range'
            : 'invalid';
        context.setSlices({ mode, thumbSlots }, { render: false });
    },
    rendered(instance) {
        synchronizeRange(instance);
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        state.generation++;
        if (state.onReset) instance.ownerDocument.removeEventListener('reset', state.onReset, true);
        if (state.onInputCapture) instance.removeEventListener('input', state.onInputCapture, true);
        if (state.onChangeCapture) instance.removeEventListener('change', state.onChangeCapture, true);
    },
};

function stateFor(instance: HTMLElement): RangeState {
    let state = RANGE_STATES.get(instance);
    if (state) return state;
    state = { connected: false, valid: false, generation: 0, payloadSignature: '', warnedSignature: '' };
    RANGE_STATES.set(instance, state);
    return state;
}

function rangeInputs(instance: HTMLElement): RangeInputs {
    if (!stateFor(instance).valid) return { mode: 'invalid' };
    const inputs = [...instance.querySelectorAll<HTMLInputElement>(':scope > [part~="root"] > [part~="inputs"] > input[type="range"]')];
    const byThumb = (thumb: RangeThumb) => inputs.filter(input => thumbFor(input) === thumb);
    const singles = byThumb('single');
    const starts = byThumb('start');
    const ends = byThumb('end');

    if (inputs.length === 1 && singles.length === 1) return { mode: 'single', single: singles[0] };
    if (inputs.length === 2 && starts.length === 1 && ends.length === 1) {
        return { end: ends[0], mode: 'range', start: starts[0] };
    }
    return { mode: 'invalid' };
}

function normalizedBounds(instance: HTMLElement): RangeBounds {
    const authoredMin = Number(instance.getAttribute('min'));
    const min = Number.isFinite(authoredMin) ? authoredMin : DEFAULT_MIN;
    const authoredMax = Number(instance.getAttribute('max') ?? Number.NaN);
    const max = Number.isFinite(authoredMax) && authoredMax > min ? authoredMax : min + DEFAULT_RANGE;
    const authoredStep = Number(instance.getAttribute('step'));
    const step = Number.isFinite(authoredStep) && authoredStep > 0 ? authoredStep : DEFAULT_STEP;
    return { max, min, step };
}

function synchronizeRange(instance: HTMLElement, changing?: HTMLInputElement): void {
    const inputs = rangeInputs(instance);
    const state = stateFor(instance);
    const signature = state.payloadSignature;
    if (inputs.mode === 'invalid') {
        if (signature !== state.warnedSignature) {
            state.warnedSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(
                '[range-input] Author exactly one single range input or one start/end range-input pair.',
            );
        }
        setMode(instance, 'invalid');
        return;
    }

    state.warnedSignature = '';
    const bounds = normalizedBounds(instance);
    const disabled = instance.hasAttribute('disabled');
    const orderedInputs = inputs.mode === 'single'
        ? [requiredInput(inputs.single)]
        : [requiredInput(inputs.start), requiredInput(inputs.end)];
    for (const input of orderedInputs) {
        input.min = String(bounds.min);
        input.max = String(bounds.max);
        input.step = String(bounds.step);
        input.disabled = disabled;
    }

    if (inputs.mode === 'range') constrainRange(inputs, changing);
    setMode(instance, inputs.mode);
    updateVisuals(instance, inputs, bounds);
}

function constrainRange(inputs: RangeInputs, changing?: HTMLInputElement): void {
    const start = requiredInput(inputs.start);
    const end = requiredInput(inputs.end);
    if (start.valueAsNumber <= end.valueAsNumber) return;
    if (changing === end) end.value = start.value;
    else start.value = end.value;
}

function setMode(instance: HTMLElement, mode: RangeMode): void {
    const owner = instance.querySelector<HTMLElement>(':scope > [part~="root"]');
    if (owner && owner.dataset.mode !== mode) owner.dataset.mode = mode;
}

function updateVisuals(instance: HTMLElement, inputs: RangeInputs, bounds: RangeBounds): void {
    const owner = instance.querySelector<HTMLElement>(':scope > [part~="root"]');
    if (!owner) return;
    const startInput = inputs.mode === 'single' ? requiredInput(inputs.single) : requiredInput(inputs.start);
    const endInput = inputs.mode === 'single' ? startInput : requiredInput(inputs.end);
    const startPosition = inputs.mode === 'single' ? 0 : percentage(startInput.valueAsNumber, bounds);
    const endPosition = percentage(endInput.valueAsNumber, bounds);
    owner.style.setProperty('--_cem-range-start-position', `${startPosition}%`);
    owner.style.setProperty('--_cem-range-end-position', `${endPosition}%`);
    owner.style.setProperty('--_cem-range-tick-spacing', `${tickSpacing(bounds)}%`);

    updateValueLabel(owner, 'single', inputs.single);
    updateValueLabel(owner, 'start', inputs.start);
    updateValueLabel(owner, 'end', inputs.end);
}

function updateValueLabel(owner: HTMLElement, thumb: RangeThumb, input?: HTMLInputElement): void {
    const label = owner.querySelector<HTMLElement>(`[part~="${thumb}-value"]`);
    if (!label) return;
    label.textContent = input?.getAttribute('aria-valuetext')?.trim() || input?.value || '';
}

function tickSpacing(bounds: RangeBounds): number {
    const intervals = (bounds.max - bounds.min) / bounds.step;
    if (!Number.isFinite(intervals) || intervals <= 1) return 100;
    return Math.max(0.1, 100 / Math.min(intervals, 1000));
}

function percentage(value: number, bounds: RangeBounds): number {
    if (!Number.isFinite(value)) return 0;
    return Math.min(100, Math.max(0, ((value - bounds.min) / (bounds.max - bounds.min)) * 100));
}

function requiredInput(input?: HTMLInputElement): HTMLInputElement {
    if (!input) throw new Error('range-input invariant failed');
    return input;
}

function handleNativeValueEvent(instance: HTMLElement, event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement) || target.type !== 'range') return;
    if (target.parentElement !== instance.querySelector(':scope > [part~=root] > [part~=inputs]')) return;
    synchronizeRange(instance, target);
}

function thumbFor(input: HTMLInputElement): string | null {
    return input.getAttribute('slot') || input.getAttribute('data-cem-slider-thumb');
}

function validPayload(nodes: readonly SerializedPayloadNode[]): boolean {
    const elements = nodes.filter((node): node is Extract<SerializedPayloadNode, { kind: 'element' }> => node.kind === 'element');
    if (elements.some(node => node.tag !== 'input' || node.attributes.type?.toLowerCase() !== 'range')) return false;
    const thumbs = elements.map(node => {
        const slot = node.attributes.slot, legacy = node.attributes['data-cem-slider-thumb'];
        return slot && legacy && slot !== legacy ? null : slot || legacy;
    });
    return (thumbs.length === 1 && thumbs[0] === 'single') ||
        (thumbs.length === 2 && thumbs.includes('start') && thumbs.includes('end'));
}
