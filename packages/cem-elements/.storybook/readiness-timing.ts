import type { CemElementRuntime } from '../src/index.js';
import { startProcessingTiming } from './tree-processing-timing.js';

const enabled = import.meta.env.STORYBOOK_CEM_TREE_TRACE === '1';
let stop: (() => void) | undefined;
let active: { root: HTMLElement; emit: (event: string, detail?: object) => void; snapshot: () => object } | undefined;

/** Read-only lifecycle observations; no runtime snapshots, source data or new revisions. */
export function startReadinessTiming(root: HTMLElement, story: string, runtime?: CemElementRuntime): void {
    if (!enabled) return;
    stop?.();
    startProcessingTiming(root, story);
    const started = performance.now();
    const emit = (event: string, detail: object = {}): void => {
        const now = performance.now();
        console.warn('[cem-readiness]', JSON.stringify({
            story, event, at: new Date(performance.timeOrigin + now).toISOString(),
            elapsedMs: Math.round((now - started) * 10) / 10, ...detail,
        }));
    };
    const diagnostics = (target: HTMLElement) => runtime?.diagnosticsFor(target).map(item => ({
        code: item.code, severity: item.severity,
    })) ?? [];
    const declarations = () => Array.from(root.querySelectorAll<HTMLElement>(
        `cem-element, ${CSS.escape(runtime?.declarationTag ?? 'cem-element')}`));
    const renders = new WeakMap<HTMLElement, 'awaiting-definition' | 'pending' | 'settled' | 'rejected'>();
    const instances = () => {
        const owners = declarations();
        const declared = new Set(owners.map(element => element.getAttribute('tag')));
        return Array.from(root.querySelectorAll<HTMLElement>('*')).filter(element =>
            element.localName.includes('-') && !element.localName.startsWith('cem-element')
            && !owners.includes(element)
            && element.localName !== 'cem-demo-element'
            && (declared.has(element.localName) || customElements.get(element.localName)));
    };
    const snapshot = () => ({
        connected: root.isConnected,
        cards: root.querySelectorAll('cem-demo-element[legend]').length,
        declarations: declarations().map(element => ({
            tag: element.getAttribute('tag'), src: element.getAttribute('src'),
            registered: !!customElements.get(element.getAttribute('tag') ?? ''),
            diagnostics: diagnostics(element),
        })),
        instances: instances().map(element => ({
            tag: element.localName, registered: !!customElements.get(element.localName),
            initialRender: renders.get(element) ?? 'unobserved',
            children: element.childElementCount, buttons: element.querySelectorAll('button').length,
            definitions: element.querySelectorAll('dd').length,
            tables: element.querySelectorAll('table').length,
            selects: Array.from(element.querySelectorAll<HTMLSelectElement>('select'), select => ({
                options: select.options.length, value: select.value,
            })),
            diagnostics: diagnostics(element),
        })),
    });
    active = { root, emit, snapshot };
    const observed = new WeakSet<HTMLElement>();
    let mounted = false;
    let alive = true;
    let previous = '';
    const observe = () => {
        mounted ||= root.isConnected;
        if (mounted && !root.isConnected) {
            emit('detached', snapshot());
            stop?.();
            return;
        }
        if (!root.isConnected) return;
        for (const element of declarations()) {
            if (observed.has(element)) continue;
            observed.add(element);
            const tag = element.getAttribute('tag');
            void runtime?.whenDeclarationSettled(element).then(() => {
                if (alive) emit('declaration-settled', { tag, src: element.getAttribute('src'), diagnostics: diagnostics(element) });
            }).catch(error => {
                if (alive) emit('declaration-rejected', { tag, error: String(error) });
            });
        }
        for (const instance of instances()) {
            if (observed.has(instance)) continue;
            observed.add(instance);
            renders.set(instance, 'awaiting-definition');
            void customElements.whenDefined(instance.localName).then(async () => {
                if (!alive) return;
                renders.set(instance, 'pending');
                emit('defined', { tag: instance.localName });
                await runtime?.whenRenderSettled(instance);
                renders.set(instance, 'settled');
                if (alive) emit('render-settled', { tag: instance.localName, diagnostics: diagnostics(instance) });
            }).catch(error => {
                renders.set(instance, 'rejected');
                if (alive) emit('render-rejected', { tag: instance.localName, error: String(error) });
            });
        }
        const state = snapshot();
        const serialized = JSON.stringify(state);
        if (serialized !== previous) {
            previous = serialized;
            emit('dom-state', state);
        }
    };
    const observer = new MutationObserver(observe);
    observer.observe(root.ownerDocument, { subtree: true, childList: true, attributes: true, characterData: true });
    stop = () => {
        alive = false;
        observer.disconnect();
        active = undefined;
        stop = undefined;
    };
    emit('start');
    observe();
}

/** Capture an explicit attempt/assertion boundary without waiting or changing it. */
export function readinessCheckpoint(event: string, detail: object = {}): void {
    if (active?.root.isConnected) active.emit(event, { ...detail, ...active.snapshot() });
}

/** Surround the existing frame loop without changing its predicate or deadline. */
export function readinessWait(message: string, frames: number): (event: 'ready' | 'timeout', attempts: number) => void {
    const trace = active;
    if (!trace?.root.isConnected) return () => undefined;
    const started = performance.now();
    // Some existing assertion errors append rendered markup. Keep that in the
    // assertion itself, not in this control-only timing stream.
    const label = message.split('; rendered=')[0];
    trace.emit('wait-start', { message: label, frames });
    return (event, attempts) => trace.emit(`wait-${event}`, {
        message: label, frames, attempts, waitMs: Math.round((performance.now() - started) * 10) / 10,
        ...(event === 'timeout' ? trace.snapshot() : {}),
    });
}
