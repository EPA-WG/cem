import type { CemProducedElementBehavior, CemProducedElementBehaviorContext } from './cem-elements.js';

interface EditorState {
    context: CemProducedElementBehaviorContext;
    value: string;
    authored?: string | null;
    control?: HTMLElement;
    abort?: AbortController;
    composing: boolean;
    deferred?: string;
    request?: string | null;
    selection: [number, number, string];
}
const states = new WeakMap<HTMLElement, EditorState>();
const controls = new WeakMap<EventTarget, EditorState>();

/** Only an admitted plaintext editor supplies a scalar value to declarative bindings. */
export function cemPlaintextEditorEventValue(target: EventTarget | null): string | undefined {
    return target ? controls.get(target)?.value : undefined;
}

/** Composition and deferred external values are not committed editor state. */
export function cemPlaintextEditorCheckpointReady(host: HTMLElement): boolean {
    const state = states.get(host);
    return !!state?.control && !state.composing && state.deferred === undefined;
}

/** Browser-owned plaintext editing; rendering and styling remain declarative. */
export const CEM_PLAINTEXT_EDITOR_CAPABILITY: CemProducedElementBehavior = {
    constructed(host, context) {
        const state: EditorState = { context, value: '', composing: false, selection: [0, 0, 'none'] };
        states.set(host, state);
        installValue(host, state);
    },
    connected(host) {
        const state = states.get(host)!;
        if (state.abort) return;
        const abort = new AbortController(); state.abort = abort;
        const options = { capture: true, signal: abort.signal };
        const owns = (event: Event) => event.target === state.control;
        const editable = () => !host.hasAttribute('disabled') && !host.hasAttribute('readonly');
        host.ownerDocument.addEventListener('selectionchange', () => { selection(state); }, { signal: abort.signal });
        host.addEventListener('beforeinput', event => { if (owns(event) && !editable()) event.preventDefault(); }, options);
        host.addEventListener('input', event => {
            if (!owns(event)) return;
            if (!editable()) { event.stopImmediatePropagation(); writeControl(state); return; }
            state.value = readControl(state.control!);
            state.context.setSlices({ value: state.value }, { render: false });
        }, options);
        host.addEventListener('compositionstart', event => { if (owns(event)) state.composing = true; }, options);
        host.addEventListener('compositionend', event => {
            if (!owns(event)) return;
            state.composing = false;
            // A final native input can follow compositionend. Admit a queued
            // external value only after that browser edit has completed.
            setTimeout(() => {
                if (abort.signal.aborted || state.composing || state.deferred === undefined) return;
                const value = state.deferred; state.deferred = undefined;
                replaceValue(state, value);
            });
        }, options);
    },
    beforeRender(host, context) {
        const state = states.get(host)!; state.context = context;
        const authored = host.getAttribute('value');
        if (state.authored !== authored) {
            const value = state.authored === undefined ? String(context.snapshot().slices.value ?? authored ?? '') : authored ?? '';
            state.authored = authored;
            if (state.composing && value !== state.value) state.deferred = value;
            else { state.deferred = undefined; state.value = value; }
        }
        context.setSlices({ value: state.value }, { render: false });
    },
    preserveRenderedChildren(host, current) { return current === states.get(host)?.control; },
    rendered(host) {
        const state = states.get(host)!;
        const control = host.querySelector<HTMLElement>(':scope > [part~="root"] > [part~="control"]');
        if (!control) return;
        if (state.control !== control) {
            if (state.control) controls.delete(state.control);
            state.control = control; controls.set(control, state); installValue(control, state);
        }
        if (!state.composing) writeControl(state);
        const request = host.getAttribute('selection-request');
        if (!request || request === state.request) return;
        state.request = request;
        const start = host.getAttribute('selection-start'), end = host.getAttribute('selection-end');
        const direction = host.getAttribute('selection-direction') ?? 'none';
        if (state.composing || host.hasAttribute('disabled') || start === null || end === null
            || !/^\d+$/.test(start) || !/^\d+$/.test(end) || !Number.isSafeInteger(Number(start))
            || !Number.isSafeInteger(Number(end)) || !['none', 'forward', 'backward'].includes(direction)) return;
        setSelection(state, Number(start), Number(end), direction, host.hasAttribute('selection-focus'));
    },
    disconnected(host) {
        const state = states.get(host)!;
        state.abort?.abort(); state.abort = undefined;
        state.composing = false; state.deferred = undefined;
        if (state.control) controls.delete(state.control);
        state.control = undefined;
    },
};

function installValue(element: HTMLElement, state: EditorState): void {
    Object.defineProperties(element, {
        value: { configurable: true, get: () => state.value, set: (value: unknown) => replaceValue(state, String(value)) },
        selectionStart: { configurable: true, get: () => selection(state)[0] },
        selectionEnd: { configurable: true, get: () => selection(state)[1] },
        selectionDirection: { configurable: true, get: () => selection(state)[2] },
        setSelectionRange: { configurable: true, value: (start: number, end: number, direction = 'none') => setSelection(state, start, end, direction, false) },
    });
}
function replaceValue(state: EditorState, value: string): void {
    if (state.composing) { state.deferred = value; return; }
    state.value = value;
    state.context.setSlices({ value }, { render: false });
    writeControl(state);
}
function writeControl(state: EditorState): void {
    const control = state.control;
    if (control && readControl(control) !== state.value) {
        // Source always enters as text, never authored HTML.
        control.textContent = state.value;
        state.selection = [0, 0, 'none'];
    }
}

function readControl(control: HTMLElement): string {
    const text = control.innerText;
    let last: ChildNode | null = control.lastChild;
    while (last?.lastChild) last = last.lastChild;
    // Enter uses a terminal BR; native paste can use a lone LF after a BR.
    // Both keep the caret on its final editable line. Preceding breaks and
    // literal text newlines written through value remain source data.
    const placeholder = last?.nodeName === 'BR'
        || last?.nodeType === 3 && last.nodeValue === '\n' && last.previousSibling?.nodeName === 'BR';
    return placeholder && text.endsWith('\n') ? text.slice(0, -1) : text;
}

interface TextSegment { node: Text; start: number; end: number }
interface Boundary { node: Node; offset: number; index: number }
/** Align native text/block/BR boundaries with admitted UTF-16 source offsets. */
function positions(state: EditorState): { segments: TextSegment[]; breaks: Boundary[] } {
    const control = state.control!, source = state.value;
    const segments: TextSegment[] = [], breaks: Boundary[] = [];
    const walker = control.ownerDocument.createTreeWalker(control, 5 /* elements and text */);
    let cursor = 0;
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        if (node.nodeType === 3) {
            const text = node as Text;
            if (!text.data) continue;
            const index = source.indexOf(text.data, cursor);
            if (index < 0) continue;
            segments.push({ node: text, start: index, end: index + text.length }); cursor = index + text.length;
        } else if ((node.nodeName === 'BR' || /^(DIV|P)$/.test(node.nodeName) && cursor > 0) && source[cursor] === '\n') {
            cursor++;
            breaks.push({ node: node.nodeName === 'BR' ? node.parentNode! : node,
                offset: node.nodeName === 'BR' ? Array.prototype.indexOf.call(node.parentNode!.childNodes, node) + 1 : 0, index: cursor });
        }
    }
    return { segments, breaks };
}
function selection(state: EditorState): [number, number, string] {
    const control = state.control, native = control?.ownerDocument.getSelection();
    if (!control || control.ownerDocument.activeElement !== control || !native?.anchorNode || !native.focusNode || !control.contains(native.anchorNode) || !control.contains(native.focusNode)) return state.selection;
    const { segments, breaks } = positions(state);
    const offset = (node: Node, index: number) => {
        const text = segments.find(segment => segment.node === node);
        if (text) return Math.min(text.end, text.start + index);
        const boundary = breaks.find(item => item.node === node && item.offset === index);
        if (boundary) return boundary.index;
        const range = control.ownerDocument.createRange(); range.setStart(node, index); range.collapse(true);
        return segments.find(segment => range.comparePoint(segment.node, 0) > 0)?.start ?? state.value.length;
    };
    const anchor = offset(native.anchorNode, native.anchorOffset), focus = offset(native.focusNode, native.focusOffset);
    return state.selection = [Math.min(anchor, focus), Math.max(anchor, focus), anchor > focus ? 'backward' : anchor === focus ? 'none' : 'forward'];
}
function setSelection(state: EditorState, start: number, end: number, direction: string, focus: boolean): void {
    const control = state.control;
    if (!control || !Number.isFinite(start) || !Number.isFinite(end)) return;
    end = Math.max(0, Math.min(Math.trunc(end), state.value.length));
    start = Math.max(0, Math.min(Math.trunc(start), end));
    state.selection = [start, end, direction];
    const { segments, breaks } = positions(state);
    const point = (index: number): [Node, number] => {
        const text = segments.find(segment => index >= segment.start && index <= segment.end);
        if (text) return [text.node, index - text.start];
        const boundary = breaks.find(item => item.index === index);
        if (boundary) return [boundary.node, boundary.offset];
        const next = segments.find(segment => segment.start > index);
        return next ? [next.node, 0] : [control, control.childNodes.length];
    };
    // A declarative range without selection-focus must never steal focus.
    if (!focus && control.ownerDocument.activeElement !== control) return;
    if (focus) control.focus();
    const first = point(start), last = point(end);
    control.ownerDocument.getSelection()?.setBaseAndExtent(...(direction === 'backward' ? last : first), ...(direction === 'backward' ? first : last));
}
