import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

interface FeedbackDialogState {
    connected: boolean;
    owner: HTMLDialogElement | null;
    previousFocus: HTMLElement | null;
    focusWasWithin: boolean;
    cancelCloses: WeakSet<HTMLDialogElement>;
    suppressedCloses: WeakSet<HTMLDialogElement>;
    onCancel: ((event: Event) => void) | null;
    onClose: (() => void) | null;
    onFocusIn: (() => void) | null;
    onFocusOut: (() => void) | null;
}

const FEEDBACK_DIALOG_STATES = new WeakMap<HTMLElement, FeedbackDialogState>();

export const CEM_FEEDBACK_DIALOG_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        stateFor(instance).connected = true;
    },
    beforeRender(instance) {
        const state = stateFor(instance);
        const owner = state.owner;
        if (!owner?.open) return;
        if (instance.hasAttribute('transient') && instance.hasAttribute('expanded')) return;
        closeWithoutDismissal(state, owner);
    },
    preserveRenderedAttribute(instance, current, desired, attribute) {
        return (
            attribute.name === 'open'
            && FEEDBACK_DIALOG_STATES.get(instance)?.owner === current
            && desired.localName === 'dialog'
            && instance.hasAttribute('transient')
            && instance.hasAttribute('expanded')
        );
    },
    rendered(instance) {
        const state = stateFor(instance);
        const owner = directDialogOwner(instance);
        if (state.owner !== owner) {
            unbindOwner(state);
            if (owner) bindOwner(instance, state, owner);
        }

        if (!owner || !instance.hasAttribute('transient') || !instance.hasAttribute('expanded') || owner.open) {
            return;
        }

        state.previousFocus = activeHtmlElement(instance.ownerDocument);
        state.focusWasWithin = false;
        owner.showModal();
    },
    disconnected(instance) {
        const state = stateFor(instance);
        state.connected = false;
        const owner = state.owner;
        const previousFocus = state.focusWasWithin ? state.previousFocus : null;

        if (owner?.open) closeWithoutDismissal(state, owner);
        unbindOwner(state);

        if (previousFocus?.isConnected) previousFocus.focus();
        state.previousFocus = null;
        state.focusWasWithin = false;
    },
};

function stateFor(instance: HTMLElement): FeedbackDialogState {
    let state = FEEDBACK_DIALOG_STATES.get(instance);
    if (state) return state;
    state = {
        connected: false,
        owner: null,
        previousFocus: null,
        focusWasWithin: false,
        cancelCloses: new WeakSet<HTMLDialogElement>(),
        suppressedCloses: new WeakSet<HTMLDialogElement>(),
        onCancel: null,
        onClose: null,
        onFocusIn: null,
        onFocusOut: null,
    };
    FEEDBACK_DIALOG_STATES.set(instance, state);
    return state;
}

function directDialogOwner(instance: HTMLElement): HTMLDialogElement | null {
    const owner = Array.from(instance.children).find((child) => child.localName === 'dialog');
    return owner ? owner as HTMLDialogElement : null;
}

function bindOwner(instance: HTMLElement, state: FeedbackDialogState, owner: HTMLDialogElement): void {
    state.owner = owner;
    state.onCancel = (event) => {
        queueMicrotask(() => {
            if (!event.defaultPrevented && !state.suppressedCloses.has(owner)) {
                state.cancelCloses.add(owner);
            }
        });
    };
    state.onClose = () => {
        const suppressed = state.suppressedCloses.delete(owner);
        const canceled = state.cancelCloses.delete(owner);
        state.focusWasWithin = false;
        state.previousFocus = null;
        if (suppressed || !state.connected || !instance.hasAttribute('transient')) return;

        instance.removeAttribute('expanded');
        instance.dispatchEvent(new CustomEvent('cem-dismiss', {
            bubbles: true,
            composed: true,
            detail: {
                reason: canceled ? 'cancel' : 'close',
                returnValue: owner.returnValue,
            },
        }));
    };
    state.onFocusIn = () => {
        state.focusWasWithin = true;
    };
    state.onFocusOut = () => {
        queueMicrotask(() => {
            if (state.connected && state.owner === owner && !owner.contains(instance.ownerDocument.activeElement)) {
                state.focusWasWithin = false;
            }
        });
    };

    owner.addEventListener('cancel', state.onCancel);
    owner.addEventListener('close', state.onClose);
    owner.addEventListener('focusin', state.onFocusIn);
    owner.addEventListener('focusout', state.onFocusOut);
}

function unbindOwner(state: FeedbackDialogState): void {
    const owner = state.owner;
    if (owner) {
        if (state.onCancel) owner.removeEventListener('cancel', state.onCancel);
        if (state.onClose) owner.removeEventListener('close', state.onClose);
        if (state.onFocusIn) owner.removeEventListener('focusin', state.onFocusIn);
        if (state.onFocusOut) owner.removeEventListener('focusout', state.onFocusOut);
    }
    state.owner = null;
    state.onCancel = null;
    state.onClose = null;
    state.onFocusIn = null;
    state.onFocusOut = null;
}

function closeWithoutDismissal(state: FeedbackDialogState, owner: HTMLDialogElement): void {
    state.suppressedCloses.add(owner);
    owner.close();
}

function activeHtmlElement(document: Document): HTMLElement | null {
    const active = document.activeElement;
    const HTMLElementConstructor = document.defaultView?.HTMLElement;
    return HTMLElementConstructor && active instanceof HTMLElementConstructor ? active : null;
}
