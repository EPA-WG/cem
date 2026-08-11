import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

type TooltipPosition = 'above' | 'after' | 'before' | 'below';

interface TooltipState {
    connected: boolean;
    descriptionId: string;
    escapeDismissed: boolean;
    focused: boolean;
    hideTimer: number | null;
    onDocumentKeyDown?: (event: KeyboardEvent) => void;
    onSurfacePointerEnter?: (event: PointerEvent) => void;
    onSurfacePointerLeave?: (event: PointerEvent) => void;
    onTriggerBlur?: () => void;
    onTriggerFocus?: () => void;
    onTriggerKeyDown?: (event: KeyboardEvent) => void;
    onTriggerPointerCancel?: (event: PointerEvent) => void;
    onTriggerPointerDown?: (event: PointerEvent) => void;
    onTriggerPointerEnter?: (event: PointerEvent) => void;
    onTriggerPointerLeave?: (event: PointerEvent) => void;
    onTriggerPointerUp?: (event: PointerEvent) => void;
    pointerSurface: boolean;
    pointerTrigger: boolean;
    showTimer: number | null;
    surface: HTMLElement | null;
    surfaceId: string;
    touchSequence: boolean;
    trigger: HTMLElement | null;
}

const TOOLTIP_POSITIONS = new Set<TooltipPosition>(['above', 'after', 'before', 'below']);
const TOOLTIP_STATES = new WeakMap<HTMLElement, TooltipState>();
const NATIVE_TRIGGER_SELECTOR = 'a[href], button, input, select, textarea, summary';
let tooltipSequence = 0;

export const CEM_TOOLTIP_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.onDocumentKeyDown = (event) => handleDocumentKeyDown(instance, state, event);
        instance.ownerDocument.addEventListener('keydown', state.onDocumentKeyDown);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        const message = normalizedMessage(instance);
        context.setSlices({
            descriptionId: state.descriptionId,
            message,
            mode: validAuthoring(instance, message) ? 'valid' : 'invalid',
            position: normalizedPosition(instance),
            surfaceId: state.surfaceId,
        }, { render: false });
    },
    rendered(instance) {
        synchronizeTooltip(instance);
    },
    disconnected(instance) {
        const state = stateFor(instance);
        state.connected = false;
        cancelTimers(instance, state);
        hideNow(state);
        unbindTrigger(state);
        unbindSurface(state);
        if (state.onDocumentKeyDown) {
            instance.ownerDocument.removeEventListener('keydown', state.onDocumentKeyDown);
        }
    },
};

function stateFor(instance: HTMLElement): TooltipState {
    let state = TOOLTIP_STATES.get(instance);
    if (state) return state;
    tooltipSequence += 1;
    state = {
        connected: false,
        descriptionId: `cem-tooltip-${tooltipSequence}-description`,
        escapeDismissed: false,
        focused: false,
        hideTimer: null,
        pointerSurface: false,
        pointerTrigger: false,
        showTimer: null,
        surface: null,
        surfaceId: `cem-tooltip-${tooltipSequence}-surface`,
        touchSequence: false,
        trigger: null,
    };
    TOOLTIP_STATES.set(instance, state);
    return state;
}

function synchronizeTooltip(instance: HTMLElement): void {
    const state = stateFor(instance);
    const message = normalizedMessage(instance);
    const triggers = triggerCandidates(instance);
    const trigger = triggers.length === 1 && supportedTrigger(triggers[0]) ? triggers[0] : null;
    const owner = instance.querySelector<HTMLElement>(':scope > .cem-tooltip');
    const surface = owner?.querySelector<HTMLElement>(':scope > .cem-tooltip__surface') ?? null;
    const valid = Boolean(trigger && surface && message);

    if (state.trigger !== trigger) {
        unbindTrigger(state);
        if (trigger) bindTrigger(instance, state, trigger);
    }
    if (state.surface !== surface) {
        unbindSurface(state);
        if (surface) bindSurface(instance, state, surface);
    }

    if (owner) {
        owner.dataset.mode = valid ? 'valid' : 'invalid';
        owner.dataset.position = normalizedPosition(instance);
    }

    if (trigger && surface && message && !tooltipDisabled(instance, trigger)) {
        appendDescription(trigger, state.descriptionId);
    } else {
        removeDescription(state);
    }

    synchronizeVisibility(instance, state);
}

function bindTrigger(instance: HTMLElement, state: TooltipState, trigger: HTMLElement): void {
    state.trigger = trigger;
    state.onTriggerPointerEnter = (event) => {
        if (event.pointerType === 'touch' || tooltipDisabled(instance, trigger)) return;
        state.pointerTrigger = true;
        synchronizeVisibility(instance, state);
    };
    state.onTriggerPointerLeave = (event) => {
        if (event.pointerType === 'touch') return;
        state.pointerTrigger = false;
        releaseAutomaticDismissal(state);
        synchronizeVisibility(instance, state);
    };
    state.onTriggerPointerDown = (event) => {
        if (event.pointerType !== 'touch') return;
        state.touchSequence = true;
        state.pointerTrigger = false;
        synchronizeVisibility(instance, state);
    };
    state.onTriggerPointerUp = (event) => finishTouchSequence(instance, state, event);
    state.onTriggerPointerCancel = (event) => finishTouchSequence(instance, state, event);
    state.onTriggerFocus = () => {
        if (state.touchSequence || tooltipDisabled(instance, trigger)) return;
        state.focused = true;
        synchronizeVisibility(instance, state);
    };
    state.onTriggerBlur = () => {
        state.focused = false;
        state.pointerTrigger = trigger.matches(':hover');
        releaseAutomaticDismissal(state);
        synchronizeVisibility(instance, state);
    };
    state.onTriggerKeyDown = (event) => {
        if (event.key === 'Escape') {
            dismissForEscape(instance, state, event);
            return;
        }
        if (!tooltipDisabled(instance, trigger) && trigger.matches(':focus')) {
            state.focused = true;
            synchronizeVisibility(instance, state);
        }
    };

    trigger.addEventListener('pointerenter', state.onTriggerPointerEnter);
    trigger.addEventListener('pointerleave', state.onTriggerPointerLeave);
    trigger.addEventListener('pointerdown', state.onTriggerPointerDown);
    trigger.addEventListener('pointerup', state.onTriggerPointerUp);
    trigger.addEventListener('pointercancel', state.onTriggerPointerCancel);
    trigger.addEventListener('focus', state.onTriggerFocus);
    trigger.addEventListener('blur', state.onTriggerBlur);
    trigger.addEventListener('keydown', state.onTriggerKeyDown);
}

function unbindTrigger(state: TooltipState): void {
    const trigger = state.trigger;
    if (trigger) {
        removeDescription(state);
        if (state.onTriggerPointerEnter) trigger.removeEventListener('pointerenter', state.onTriggerPointerEnter);
        if (state.onTriggerPointerLeave) trigger.removeEventListener('pointerleave', state.onTriggerPointerLeave);
        if (state.onTriggerPointerDown) trigger.removeEventListener('pointerdown', state.onTriggerPointerDown);
        if (state.onTriggerPointerUp) trigger.removeEventListener('pointerup', state.onTriggerPointerUp);
        if (state.onTriggerPointerCancel) trigger.removeEventListener('pointercancel', state.onTriggerPointerCancel);
        if (state.onTriggerFocus) trigger.removeEventListener('focus', state.onTriggerFocus);
        if (state.onTriggerBlur) trigger.removeEventListener('blur', state.onTriggerBlur);
        if (state.onTriggerKeyDown) trigger.removeEventListener('keydown', state.onTriggerKeyDown);
    }
    state.trigger = null;
    state.focused = false;
    state.pointerTrigger = false;
    state.touchSequence = false;
}

function bindSurface(instance: HTMLElement, state: TooltipState, surface: HTMLElement): void {
    state.surface = surface;
    state.onSurfacePointerEnter = (event) => {
        if (event.pointerType === 'touch' || tooltipDisabled(instance, state.trigger)) return;
        state.pointerSurface = true;
        synchronizeVisibility(instance, state);
    };
    state.onSurfacePointerLeave = (event) => {
        if (event.pointerType === 'touch') return;
        state.pointerSurface = false;
        releaseAutomaticDismissal(state);
        synchronizeVisibility(instance, state);
    };
    surface.addEventListener('pointerenter', state.onSurfacePointerEnter);
    surface.addEventListener('pointerleave', state.onSurfacePointerLeave);
}

function unbindSurface(state: TooltipState): void {
    const surface = state.surface;
    if (surface) {
        if (state.onSurfacePointerEnter) surface.removeEventListener('pointerenter', state.onSurfacePointerEnter);
        if (state.onSurfacePointerLeave) surface.removeEventListener('pointerleave', state.onSurfacePointerLeave);
    }
    state.surface = null;
    state.pointerSurface = false;
}

function synchronizeVisibility(instance: HTMLElement, state: TooltipState): void {
    if (shouldShow(instance, state)) scheduleShow(instance, state, normalizedDelay(instance, 'show-delay'));
    else scheduleHide(instance, state, immediateDismissal(instance, state) ? 0 : normalizedDelay(instance, 'hide-delay'));
}

function shouldShow(instance: HTMLElement, state: TooltipState): boolean {
    if (!state.surface || !normalizedMessage(instance) || tooltipDisabled(instance, state.trigger)) return false;
    if (!validAuthoring(instance, normalizedMessage(instance))) return false;
    if (instance.hasAttribute('open')) return true;
    if (state.escapeDismissed || state.touchSequence) return false;
    return (
        state.focused
        || state.pointerTrigger
        || state.pointerSurface
        || state.trigger?.matches(':hover') === true
        || state.surface.matches(':hover')
    );
}

function immediateDismissal(instance: HTMLElement, state: TooltipState): boolean {
    return (
        !state.surface
        || !normalizedMessage(instance)
        || tooltipDisabled(instance, state.trigger)
        || state.escapeDismissed
    );
}

function scheduleShow(instance: HTMLElement, state: TooltipState, delay: number): void {
    clearHideTimer(instance, state);
    if (state.surface?.matches(':popover-open') || state.showTimer !== null) return;
    if (delay === 0) {
        showNow(instance, state);
        return;
    }
    state.showTimer = timerWindow(instance).setTimeout(() => {
        state.showTimer = null;
        if (shouldShow(instance, state)) showNow(instance, state);
    }, delay);
}

function scheduleHide(instance: HTMLElement, state: TooltipState, delay: number): void {
    clearShowTimer(instance, state);
    if (!state.surface?.matches(':popover-open') || state.hideTimer !== null) return;
    if (delay === 0) {
        hideNow(state);
        return;
    }
    state.hideTimer = timerWindow(instance).setTimeout(() => {
        state.hideTimer = null;
        if (!shouldShow(instance, state)) hideNow(state);
    }, delay);
}

function showNow(instance: HTMLElement, state: TooltipState): void {
    const surface = state.surface;
    if (!surface || surface.matches(':popover-open') || !shouldShow(instance, state)) return;
    surface.showPopover();
}

function hideNow(state: TooltipState): void {
    if (state.surface?.matches(':popover-open')) state.surface.hidePopover();
}

function handleDocumentKeyDown(instance: HTMLElement, state: TooltipState, event: KeyboardEvent): void {
    if (event.key !== 'Escape') return;
    if (!state.surface?.matches(':popover-open') && state.showTimer === null) return;
    dismissForEscape(instance, state, event);
}

function dismissForEscape(instance: HTMLElement, state: TooltipState, event: KeyboardEvent): void {
    if (!state.surface?.matches(':popover-open') && state.showTimer === null) return;
    event.preventDefault();
    state.escapeDismissed = true;
    if (instance.hasAttribute('open')) instance.removeAttribute('open');
    cancelTimers(instance, state);
    hideNow(state);
}

function finishTouchSequence(instance: HTMLElement, state: TooltipState, event: PointerEvent): void {
    if (event.pointerType !== 'touch') return;
    timerWindow(instance).setTimeout(() => {
        state.touchSequence = false;
        synchronizeVisibility(instance, state);
    }, 0);
}

function releaseAutomaticDismissal(state: TooltipState): void {
    if (!state.focused && !state.pointerTrigger && !state.pointerSurface) state.escapeDismissed = false;
}

function appendDescription(trigger: HTMLElement, descriptionId: string): void {
    const descriptions = descriptionTokens(trigger);
    if (!descriptions.includes(descriptionId)) descriptions.push(descriptionId);
    trigger.setAttribute('aria-describedby', descriptions.join(' '));
}

function removeDescription(state: TooltipState): void {
    const trigger = state.trigger;
    if (!trigger) return;
    const descriptions = descriptionTokens(trigger).filter((id) => id !== state.descriptionId);
    if (descriptions.length > 0) trigger.setAttribute('aria-describedby', descriptions.join(' '));
    else trigger.removeAttribute('aria-describedby');
}

function descriptionTokens(trigger: HTMLElement): string[] {
    return (trigger.getAttribute('aria-describedby') ?? '').trim().split(/\s+/).filter(Boolean);
}

function validAuthoring(instance: HTMLElement, message: string): boolean {
    const triggers = triggerCandidates(instance);
    return Boolean(message && triggers.length === 1 && supportedTrigger(triggers[0]));
}

function triggerCandidates(instance: HTMLElement): HTMLElement[] {
    return [...instance.querySelectorAll<HTMLElement>('[slot="trigger"]')]
        .filter((candidate) => candidate.closest('cem-tooltip') === instance);
}

function supportedTrigger(trigger?: HTMLElement): trigger is HTMLElement {
    return Boolean(trigger?.matches(NATIVE_TRIGGER_SELECTOR));
}

function tooltipDisabled(instance: HTMLElement, trigger: HTMLElement | null): boolean {
    return instance.hasAttribute('disabled') || trigger?.matches(':disabled, [aria-disabled="true"]') === true;
}

function normalizedMessage(instance: HTMLElement): string {
    return instance.getAttribute('message')?.trim() ?? '';
}

function normalizedPosition(instance: HTMLElement): TooltipPosition {
    const position = instance.getAttribute('position') as TooltipPosition | null;
    return position && TOOLTIP_POSITIONS.has(position) ? position : 'below';
}

function normalizedDelay(instance: HTMLElement, name: 'hide-delay' | 'show-delay'): number {
    const value = Number(instance.getAttribute(name));
    return Number.isFinite(value) && value >= 0 ? value : 0;
}

function timerWindow(instance: HTMLElement): Window {
    const view = instance.ownerDocument.defaultView;
    if (!view) throw new Error('cem-tooltip requires an attached browser document');
    return view;
}

function clearShowTimer(instance: HTMLElement, state: TooltipState): void {
    if (state.showTimer === null) return;
    timerWindow(instance).clearTimeout(state.showTimer);
    state.showTimer = null;
}

function clearHideTimer(instance: HTMLElement, state: TooltipState): void {
    if (state.hideTimer === null) return;
    timerWindow(instance).clearTimeout(state.hideTimer);
    state.hideTimer = null;
}

function cancelTimers(instance: HTMLElement, state: TooltipState): void {
    clearShowTimer(instance, state);
    clearHideTimer(instance, state);
}
