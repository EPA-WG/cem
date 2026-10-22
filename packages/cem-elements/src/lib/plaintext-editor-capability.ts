import type { CemSourceHighlightResult } from './internal/runtime-support/processing-host.js';
import type { CemProducedElementBehavior, CemProducedElementBehaviorContext } from './cem-elements.js';
import type { CemCheckpointControlState, CemCheckpointRuntimeState } from './checkpoint-controls.js';

interface EditorState {
    host: HTMLElement;
    context: CemProducedElementBehaviorContext;
    value: string;
    authored?: string | null;
    control?: HTMLElement;
    abort?: AbortController;
    composing: boolean;
    deferred?: string;
    request?: string | null;
    selection: [number, number, string];
    paintKey?: { source: string; type: string; identity: string };
    paintAbort?: AbortController;
    paintTimer?: ReturnType<typeof setTimeout>;
    paint?: { registry: HighlightRegistry; name: string; highlight: Highlight; range: Range }[];
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
    return !!state?.control?.isConnected && host.isConnected && !state.composing && state.deferred === undefined
        && readControl(state.control) === state.value;
}

export function cemPlaintextEditorCheckpoint(host: HTMLElement) {
    const state = states.get(host);
    return cemPlaintextEditorCheckpointReady(host) && state
        ? { kind: 'plaintext-editor' as const, authored: state.authored ?? null, selectionRequest: state.request ?? null } : undefined;
}

/** Native-validated text/selection metadata admits the retained browser owner.
 * Never replay a saved value or selection over live edits, or serialize paint. */
export function prepareCemPlaintextEditorResume(host: HTMLElement, saved: CemCheckpointRuntimeState['capability'], controls: CemCheckpointControlState) {
    const state = states.get(host), control = state?.control, captured = controls.controls[0];
    const current = () => {
        if (!state || !control || states.get(host) !== state || state.control !== control || !cemPlaintextEditorCheckpointReady(host)
            || saved?.kind !== 'plaintext-editor' || controls.controls.length !== 1 || !captured || captured.kind !== 'text'
            || captured.inputType !== 'plaintext' || !captured.selection || captured.id !== control.getAttribute('data-cem-render-node-id')
            || captured.value !== state.value || saved.authored !== (state.authored ?? null) || saved.selectionRequest !== (state.request ?? null)
            || captured.focused !== (control.ownerDocument.activeElement === control)
            || captured.disabled !== (control.getAttribute('aria-disabled') === 'true')
            || captured.readOnly !== (control.getAttribute('aria-readonly') === 'true')) return false;
        const [start, end, direction] = selection(state);
        return captured.selection.start === start && captured.selection.end === end && captured.selection.direction === direction;
    };
    if (!current()) throw new Error('cem.checkpoint.resume.editor');
    return { current, activate() {
        if (!current()) throw new Error('cem.checkpoint.resume.editor-stale');
        // Even a pending/failed old request must not suppress an identical new
        // source on the fresh heap. Rendering will schedule disposable paint.
        cancelPaint(state!);
    } };
}

/** Browser-owned plaintext editing; rendering and styling remain declarative. */
export const CEM_PLAINTEXT_EDITOR_CAPABILITY: CemProducedElementBehavior = {
    constructed(host, context) {
        const state: EditorState = { host, context, value: '', composing: false, selection: [0, 0, 'none'] };
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
        host.addEventListener('beforeinput', event => {
            if (!owns(event)) return;
            if (!editable()) event.preventDefault(); else cancelPaint(state);
        }, options);
        host.addEventListener('input', event => {
            if (!owns(event)) return;
            if (!editable()) { event.stopImmediatePropagation(); writeControl(state); return; }
            state.value = readControl(state.control!);
            state.context.setSlices({ value: state.value }, { render: false });
            schedulePaint(host, state);
        }, options);
        host.addEventListener('compositionstart', event => { if (owns(event)) { state.composing = true; cancelPaint(state); } }, options);
        host.addEventListener('compositionend', event => {
            if (!owns(event)) return;
            state.composing = false;
            // A final native input can follow compositionend. Admit a queued
            // external value only after that browser edit has completed.
            setTimeout(() => {
                if (abort.signal.aborted || state.composing) return;
                if (state.deferred !== undefined) { const value = state.deferred; state.deferred = undefined; replaceValue(state, value); }
                schedulePaint(host, state);
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
        schedulePaint(host, state);
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
        cancelPaint(state);
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
    cancelPaint(state);
    state.value = value;
    state.context.setSlices({ value }, { render: false });
    writeControl(state);
    schedulePaint(state.host, state);
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
    if ([...control.childNodes].every(node => node.nodeType === 3)) return control.textContent ?? '';
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

// Paint is disposable browser presentation. Native text/undo and the canonical
// value never contain color markup. Names are shared per document, ranges per owner.
const paints = new WeakMap<Document, Map<string, Highlight>>();
function clearPaint(state: EditorState): void {
    for (const { registry, name, highlight, range } of state.paint ?? []) {
        highlight.delete(range);
        if (!highlight.size && registry.get(name) === highlight) registry.delete(name);
    }
    state.paint = undefined;
}
function cancelPaint(state: EditorState): void {
    clearTimeout(state.paintTimer); state.paintTimer = undefined;
    state.paintAbort?.abort(); state.paintAbort = undefined; state.paintKey = undefined; clearPaint(state);
}
function schedulePaint(host: HTMLElement, state: EditorState): void {
    const type = host.getAttribute('content-type') ?? 'text/plain', identity = host.getAttribute('source-id') ?? '';
    const control = state.control, view = control?.ownerDocument.defaultView as (Window & typeof globalThis) | null;
    if (!control || !host.isConnected || state.composing || host.getAttribute('highlight') === 'off'
        || type === 'text/plain' || !view?.CSS?.highlights || !view.Highlight || !state.context.highlightSource
        || state.value.length > 262144) { cancelPaint(state); return; }
    if (state.paintKey?.source === state.value && state.paintKey.type === type && state.paintKey.identity === identity) return;
    cancelPaint(state);
    const key = { source: state.value, type, identity }; state.paintKey = key;
    const abort = new AbortController(); state.paintAbort = abort;
    state.paintTimer = setTimeout(() => {
        state.paintTimer = undefined;
        if (abort.signal.aborted || state.composing) return;
        void state.context.highlightSource!(key.source, type, abort.signal).then(result => {
            if (abort.signal.aborted || result.signal.aborted || state.paintKey !== key || state.value !== key.source
                || state.control !== control || !host.isConnected || state.composing || readControl(control) !== key.source
                || (host.getAttribute('content-type') ?? 'text/plain') !== key.type
                || (host.getAttribute('source-id') ?? '') !== key.identity || host.getAttribute('highlight') === 'off') {
                abort.abort(); return;
            }
            if (result.status !== 'painted') { abort.abort(); return; }
            const revoke = () => {
                if (state.paintKey !== key) return;
                clearPaint(state); state.paintKey = undefined; state.paintAbort = undefined; abort.abort();
            };
            result.signal.addEventListener('abort', revoke, { once: true });
            abort.signal.addEventListener('abort', () => result.signal.removeEventListener('abort', revoke), { once: true });
            applyPaint(state, result, view);
        }).catch(() => { if (state.paintKey === key) { clearPaint(state); abort.abort(); } });
    }, 80);
}
function applyPaint(state: EditorState, result: CemSourceHighlightResult, view: Window & typeof globalThis): void {
    const control = state.control!, registry = view.CSS.highlights;
    let owned = paints.get(control.ownerDocument);
    if (!owned) { owned = new Map(); paints.set(control.ownerDocument, owned); }
    const { segments, breaks } = positions(state), boundaries = new Map(breaks.map(item => [item.index, item]));
    let cursor = 0;
    const point = (offset: number): [Node, number] => {
        while (cursor < segments.length && segments[cursor].end < offset) cursor++;
        const text = segments[cursor];
        if (text && offset >= text.start) return [text.node, Math.min(text.node.length, offset - text.start)];
        const boundary = boundaries.get(offset);
        if (boundary) return [boundary.node, boundary.offset];
        return text ? [text.node, 0] : [control, control.childNodes.length];
    };
    state.paint = [];
    for (const token of result.ranges) {
        if (['syntax.raw', 'syntax.text', 'syntax.punctuation'].includes(token.role)) continue;
        const name = `cem-source-${token.role.slice(7)}`;
        let highlight = owned.get(name);
        if (!highlight) { highlight = new view.Highlight(); owned.set(name, highlight); }
        if (registry.has(name) && registry.get(name) !== highlight) continue;
        const range = control.ownerDocument.createRange();
        range.setStart(...point(token.start)); range.setEnd(...point(token.end));
        highlight.add(range); registry.set(name, highlight);
        state.paint.push({ registry, name, highlight, range });
    }
}
