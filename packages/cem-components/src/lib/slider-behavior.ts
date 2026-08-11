import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

type SliderMode = 'invalid' | 'range' | 'single';
type SliderThumb = 'end' | 'single' | 'start';

interface SliderBounds {
    max: number;
    min: number;
    step: number;
}

interface SliderInputs {
    end?: HTMLInputElement;
    mode: SliderMode;
    single?: HTMLInputElement;
    start?: HTMLInputElement;
}

interface SliderState {
    connected: boolean;
    onChangeCapture?: EventListener;
    onInputCapture?: EventListener;
    warnedSignature: string;
}

const DEFAULT_MIN = 0;
const DEFAULT_RANGE = 100;
const DEFAULT_STEP = 1;
const SLIDER_STATES = new WeakMap<HTMLElement, SliderState>();

export const CEM_SLIDER_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.onInputCapture = (event) => handleNativeValueEvent(instance, event);
        state.onChangeCapture = (event) => handleNativeValueEvent(instance, event);
        instance.addEventListener('input', state.onInputCapture, true);
        instance.addEventListener('change', state.onChangeCapture, true);
    },
    beforeRender(instance, context) {
        const inputs = sliderInputs(instance);
        context.setSlices({ mode: inputs.mode }, { render: false });
    },
    rendered(instance) {
        synchronizeSlider(instance);
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onInputCapture) instance.removeEventListener('input', state.onInputCapture, true);
        if (state.onChangeCapture) instance.removeEventListener('change', state.onChangeCapture, true);
    },
};

function stateFor(instance: HTMLElement): SliderState {
    let state = SLIDER_STATES.get(instance);
    if (state) return state;
    state = { connected: false, warnedSignature: '' };
    SLIDER_STATES.set(instance, state);
    return state;
}

function sliderInputs(instance: HTMLElement): SliderInputs {
    const inputs = [...instance.querySelectorAll<HTMLInputElement>('input[type="range"][data-cem-slider-thumb]')]
        .filter((input) => input.closest('cem-slider') === instance);
    const byThumb = (thumb: SliderThumb) => inputs.filter((input) => input.dataset.cemSliderThumb === thumb);
    const singles = byThumb('single');
    const starts = byThumb('start');
    const ends = byThumb('end');

    if (inputs.length === 1 && singles.length === 1) return { mode: 'single', single: singles[0] };
    if (inputs.length === 2 && starts.length === 1 && ends.length === 1) {
        return { end: ends[0], mode: 'range', start: starts[0] };
    }
    return { mode: 'invalid' };
}

function normalizedBounds(instance: HTMLElement): SliderBounds {
    const authoredMin = Number(instance.getAttribute('min'));
    const min = Number.isFinite(authoredMin) ? authoredMin : DEFAULT_MIN;
    const authoredMax = Number(instance.getAttribute('max'));
    const max = Number.isFinite(authoredMax) && authoredMax > min ? authoredMax : min + DEFAULT_RANGE;
    const authoredStep = Number(instance.getAttribute('step'));
    const step = Number.isFinite(authoredStep) && authoredStep > 0 ? authoredStep : DEFAULT_STEP;
    return { max, min, step };
}

function synchronizeSlider(instance: HTMLElement, changing?: HTMLInputElement): void {
    const inputs = sliderInputs(instance);
    const state = stateFor(instance);
    const signature = [...instance.querySelectorAll<HTMLInputElement>('input[type="range"]')]
        .filter((input) => input.closest('cem-slider') === instance)
        .map((input) => input.dataset.cemSliderThumb ?? '(unmarked)')
        .join('|');
    if (inputs.mode === 'invalid') {
        if (signature !== state.warnedSignature) {
            state.warnedSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(
                '[cem-slider] Author exactly one single range input or one start/end range-input pair.',
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

function constrainRange(inputs: SliderInputs, changing?: HTMLInputElement): void {
    const start = requiredInput(inputs.start);
    const end = requiredInput(inputs.end);
    if (start.valueAsNumber <= end.valueAsNumber) return;
    if (changing === end) end.value = start.value;
    else start.value = end.value;
}

function setMode(instance: HTMLElement, mode: SliderMode): void {
    const owner = instance.querySelector<HTMLElement>(':scope > .cem-slider');
    if (owner && owner.dataset.mode !== mode) owner.dataset.mode = mode;
}

function updateVisuals(instance: HTMLElement, inputs: SliderInputs, bounds: SliderBounds): void {
    const owner = instance.querySelector<HTMLElement>(':scope > .cem-slider');
    if (!owner) return;
    const startInput = inputs.mode === 'single' ? requiredInput(inputs.single) : requiredInput(inputs.start);
    const endInput = inputs.mode === 'single' ? startInput : requiredInput(inputs.end);
    const startPosition = inputs.mode === 'single' ? 0 : percentage(startInput.valueAsNumber, bounds);
    const endPosition = percentage(endInput.valueAsNumber, bounds);
    owner.style.setProperty('--_cem-slider-start-position', `${startPosition}%`);
    owner.style.setProperty('--_cem-slider-end-position', `${endPosition}%`);
    owner.style.setProperty('--_cem-slider-tick-spacing', `${tickSpacing(bounds)}%`);

    updateValueLabel(owner, 'single', inputs.single);
    updateValueLabel(owner, 'start', inputs.start);
    updateValueLabel(owner, 'end', inputs.end);
}

function updateValueLabel(owner: HTMLElement, thumb: SliderThumb, input?: HTMLInputElement): void {
    const label = owner.querySelector<HTMLElement>(`.cem-slider__value[data-cem-slider-value="${thumb}"]`);
    if (!label) return;
    label.textContent = input?.getAttribute('aria-valuetext')?.trim() || input?.value || '';
}

function tickSpacing(bounds: SliderBounds): number {
    const intervals = (bounds.max - bounds.min) / bounds.step;
    if (!Number.isFinite(intervals) || intervals <= 1) return 100;
    return Math.max(0.1, 100 / Math.min(intervals, 1000));
}

function percentage(value: number, bounds: SliderBounds): number {
    if (!Number.isFinite(value)) return 0;
    return Math.min(100, Math.max(0, ((value - bounds.min) / (bounds.max - bounds.min)) * 100));
}

function requiredInput(input?: HTMLInputElement): HTMLInputElement {
    if (!input) throw new Error('cem-slider input invariant failed');
    return input;
}

function handleNativeValueEvent(instance: HTMLElement, event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement) || target.type !== 'range') return;
    if (target.closest('cem-slider') !== instance) return;
    synchronizeSlider(instance, target);
}
