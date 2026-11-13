// Benchmark-only frame sampling. No runtime methods, native calls or browser APIs
// are wrapped; original browser assertions must authorize final publication.
export function browserFixtureReadiness({ kind }) {
    if (!['selected', 'controls', 'data', 'editor'].includes(kind)) throw Error('unknown readiness fixture');
    let active = true, frame, previous = [], eligibleFrames = 0, firstEligibleMs = null;
    let readyMs = null, confirmationGapMs = null, lastSampleMs = null, frameCount = 0, invalidations = 0;
    let samplingWorkMs = 0, maxSampleMs = 0;
    const same = (a, b) => a.length === b.length && a.every((node, index) => node === b[index]);
    const visible = element => element?.isConnected && !element.closest('[hidden],[inert],[aria-hidden=true],[aria-busy=true]')
        && element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })
        && [...element.getClientRects()].some(rect => rect.width > 0 && rect.height > 0);
    const one = selector => { const nodes = document.querySelectorAll(selector); return nodes.length === 1 ? nodes[0] : null; };
    const nodes = () => {
        if (kind === 'selected') return globalThis.auditSelectedReady === true ? [document.documentElement] : [];
        if (kind === 'controls') {
            const button = one('cem-action button'), host = one('cem-text-field[name=item]'), input = host?.querySelector('input');
            const label = input?.labels?.length === 1 ? input.labels[0] : null;
            const form = host?.closest('form');
            return visible(button) && visible(input) && visible(label) && form && button.closest('form') === form
                && (button.getAttribute('aria-label') ?? button.textContent.trim()) === 'Apply'
                && button.type === 'button' && !button.disabled && !input.disabled && !input.readOnly
                && input.type === 'text' && input.value === 'Sample' && label.textContent.trim() === 'Item'
                ? [form, button, host, input, label] : [];
        }
        if (kind === 'data') {
            const host = one('cem-data-view'), root = one('cem-data-view > [part~=root]');
            const notice = root?.querySelector('[part~=notice]');
            return host?.getAttribute('format') === 'json' && host.getAttribute('value') === ''
                && root?.getAttribute('role') === 'group' && root.getAttribute('aria-label') === 'Inventory'
                && visible(root) && visible(notice) && notice.textContent.trim() === 'No data'
                ? [host, root, notice] : [];
        }
        const host = one('cem-source-editor'), control = one('cem-source-editor [role=textbox]');
        const label = host?.querySelector('[part~=label]');
        return visible(control) && visible(label) && label.textContent.trim() === 'Source'
            && control.getAttribute('aria-label') === 'Source' && control.getAttribute('aria-multiline') === 'true'
            && control.getAttribute('contenteditable') === 'plaintext-only' && control.getAttribute('aria-disabled') === 'false'
            && control.getAttribute('aria-readonly') === 'false' && control.textContent === ''
            ? [host, control, label] : [];
    };
    const reset = () => {
        if (previous.length) invalidations++;
        previous = []; eligibleFrames = 0; firstEligibleMs = null; readyMs = null; confirmationGapMs = null;
    };
    const tick = () => {
        if (!active) return;
        const started = performance.now(), current = nodes(), now = performance.now(); frameCount++;
        samplingWorkMs += now - started; maxSampleMs = Math.max(maxSampleMs, now - started);
        if (!current.length) reset();
        else {
            if (!same(current, previous)) { reset(); previous = current; firstEligibleMs = now; }
            eligibleFrames++;
            if (eligibleFrames === 2) { readyMs = now; confirmationGapMs = now - lastSampleMs; }
        }
        lastSampleMs = now; frame = requestAnimationFrame(tick);
    };
    const close = () => { active = false; cancelAnimationFrame(frame); previous = []; };
    globalThis.__cemFixtureReadiness = Object.freeze({
        inspect: () => ({ active, readyMs, eligibleFrames, invalidations, frameCount }),
        cancel: close,
        finish(assertionsPassed) {
            if (!active) throw Error('browser readiness recorder is closed');
            try {
                if (assertionsPassed !== true) throw Error('original browser assertions must pass');
                const current = nodes();
                if (readyMs === null || !current.length || !same(previous, current)) throw Error('fixture is hidden, incomplete or unsettled');
                return { contract: 'cem-browser-fixture-readiness/1', kind, timeOrigin: performance.timeOrigin,
                    originalAssertionsPassed: true,
                    readyMs, firstEligibleMs, confirmationGapMs, eligibleFrames, frameCount, invalidations, samplingWorkMs, maxSampleMs,
                    assertionConfirmedMs: performance.now(), sampling: 'two consecutive animation frames; current predicate rechecked after original assertions' };
            } finally { close(); }
        },
    });
    frame = requestAnimationFrame(tick);
}
