// Passed directly to Playwright addInitScript. This diagnostic never runs in a
// deployed runtime; no source, native handles or request bodies are retained.
export function browserStartupPhases({ kind, instrumentCalls = true }) {
    const events = [], restore = [];
    let active = true, overflow = false, visibleAt = null;
    const limit = 5000;
    const visible = element => !!element && getComputedStyle(element).visibility !== 'hidden'
        && [...element.getClientRects()].some(rect => rect.width > 0 && rect.height > 0);
    const check = () => {
        if (visibleAt !== null) return;
        const ready = kind === 'controls'
            ? [...document.querySelectorAll('cem-action button')].some(button => button.textContent.trim() === 'Apply' && visible(button))
                && visible(document.querySelector('cem-text-field[name=item] input'))
            : kind === 'data' ? visible(document.querySelector('cem-data-view > [part~=root]'))
                : visible(document.querySelector('cem-source-editor [role=textbox][aria-label=Source]'));
        if (ready) visibleAt = performance.now();
    };
    const observer = new MutationObserver(check);
    observer.observe(document, { subtree: true, childList: true, attributes: true, characterData: true });
    addEventListener('load', check, true);
    const wrap = (owner, name, label, sizeArgument) => {
        const original = owner[name];
        if (typeof original !== 'function') throw Error(`missing startup diagnostic endpoint: ${label}`);
        function measured(...args) {
            if (!active) return Reflect.apply(original, this, args);
            if (events.length >= limit) { overflow = true; return Reflect.apply(original, this, args); }
            const event = { label, start: performance.now(), end: null, status: 'pending' };
            if (sizeArgument !== undefined) event.inputBytes = args[sizeArgument]?.byteLength ?? null;
            events.push(event);
            const complete = status => { if (active) { event.end = performance.now(); event.status = status; } };
            try {
                const result = Reflect.apply(original, this, args);
                if (result instanceof Promise) void result.then(() => complete('fulfilled'), () => complete('rejected'));
                else complete('returned');
                return result;
            } catch (error) { complete('threw'); throw error; }
        }
        owner[name] = measured;
        restore.push(() => { if (owner[name] === measured) owner[name] = original; });
    };
    if (instrumentCalls) {
        wrap(globalThis, 'fetch', 'fetch-headers');
        wrap(crypto.subtle, 'digest', 'digest', 1);
        for (const name of ['compile', 'compileStreaming', 'instantiate', 'instantiateStreaming'])
            wrap(WebAssembly, name, `wasm.${name}`, name === 'compile' ? 0 : undefined);
    }
    globalThis.__cemStartupPhases = {
        instrumentRuntime(prototype) {
            if (!instrumentCalls) return;
            for (const name of ['registerDeclaration', 'registerResolvedDeclaration', 'surfaceDeclarationDiagnostics',
                'prepareRetainedStylesheets', 'renderInstance']) wrap(prototype, name, `runtime.${name}`);
        },
        stop() {
            check(); active = false; observer.disconnect(); removeEventListener('load', check, true);
            for (const reset of restore.reverse()) reset();
            return { contract: 'cem-browser-startup-phases/1', instrumentCalls, overflow, visibleAt,
                timeOrigin: performance.timeOrigin, events,
                resources: performance.getEntriesByType('resource').map(entry => ({ path: new URL(entry.name).pathname,
                    start: entry.startTime, responseStart: entry.responseStart, responseEnd: entry.responseEnd })) };
        },
    };
}
