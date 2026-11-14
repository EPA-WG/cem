import assert from 'node:assert/strict';

/** Browser assessment of compiled shared adapters; this fixture supplies no UI behavior. */
export async function verifyDialogPresentations(browser, runtimeUrl, fixtureUrl) {
    const page = await browser.newPage({ viewport: { width: 900, height: 700 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const reports = [];
    try {
        for (const profile of ['modal', 'nonmodal', 'manual', 'auto']) {
            await page.goto(fixtureUrl);
            await page.evaluate(async ({ runtimeUrl, profile }) => {
                const { connectCemNativeSurface } = await import(runtimeUrl);
                document.body.innerHTML = '<button style="position:fixed;left:10px;top:10px">Launcher</button><button style="position:fixed;left:10px;bottom:10px">Outside</button><dialog aria-label="Assessment task" style="width:280px;height:180px"><input autofocus aria-label="Entry"><button>Last</button></dialog>';
                const [source, outside] = document.querySelectorAll('body > button'), owner = document.querySelector('dialog');
                if (['manual', 'auto'].includes(profile)) owner.setAttribute('popover', profile);
                else owner.setAttribute('mode', profile);
                const state = window.surfaceAssessment = { source, outside, owner, outsideClicks: 0, veto: false, guards: 0, hides: [], sharedReturns: 0 };
                outside.addEventListener('click', () => state.outsideClicks++);
                owner.addEventListener('cem-before-close', event => { state.guards++; if (state.veto) event.preventDefault(); });
                owner.addEventListener('beforetoggle', event => { if (event.newState === 'closed') state.hides.push(event.cancelable); });
                state.controller = connectCemNativeSurface(owner);
                source.focus();
                if (!state.controller.open({ source })) throw Error('Opening refused');
                source.focus = (...args) => { state.sharedReturns++; HTMLElement.prototype.focus.apply(source, args); };
            }, { runtimeUrl, profile });
            await page.waitForFunction(() => document.activeElement === document.querySelector('input'));
            const initial = await page.evaluate(() => {
                const { owner, outside } = window.surfaceAssessment;
                outside.focus();
                return { modal: owner.matches(':modal'), open: owner.open, popover: owner.matches(':popover-open'),
                    outsideFocused: document.activeElement === outside, reflectedInert: outside.inert,
                    backdrop: getComputedStyle(owner, '::backdrop').backgroundColor,
                    requestClose: typeof owner.requestClose === 'function', closedBy: 'closedBy' in owner };
            });
            assert.equal(initial.modal, profile === 'modal', `${profile}: modality`);
            assert.equal(initial.open, ['modal', 'nonmodal'].includes(profile), `${profile}: native open flag`);
            assert.equal(initial.popover, ['manual', 'auto'].includes(profile), `${profile}: popover visibility`);
            assert.equal(initial.outsideFocused, profile !== 'modal', `${profile}: effective inertness`);
            assert.equal(initial.reflectedInert, false, 'Browser modality does not author inert attributes');
            await page.mouse.click(30, 680);
            assert.equal(await page.evaluate(() => window.surfaceAssessment.outsideClicks), profile === 'modal' ? 0 : 1, `${profile}: outside hit testing`);
            let outsideCloseFocus;
            if (profile === 'auto') {
                await page.waitForFunction(() => !window.surfaceAssessment.owner.matches(':popover-open'));
                assert.equal(await page.evaluate(() => document.activeElement === window.surfaceAssessment.outside), true, 'Light dismissal preserves the outside destination');
                // A native auto-popover hide cannot be vetoed by a CEM request guard.
                await page.evaluate(() => { const s = window.surfaceAssessment; s.veto = true; s.controller.open({ source: s.source }); });
                await page.mouse.click(30, 680);
                await page.waitForFunction(() => !window.surfaceAssessment.owner.matches(':popover-open'));
                assert.equal(await page.evaluate(() => window.surfaceAssessment.guards), 0);
            } else {
                assert.equal(await page.evaluate(() => { const s = window.surfaceAssessment; return s.owner.open || s.owner.matches(':popover-open'); }), true, `${profile}: outside persistence`);
                if (profile !== 'modal') {
                    await page.evaluate(() => window.surfaceAssessment.controller.requestClose());
                    outsideCloseFocus = await page.evaluate(() => { const s = window.surfaceAssessment; return { destination: document.activeElement === s.outside ? 'outside' : document.activeElement === s.source ? 'launcher' : document.activeElement === document.body ? 'body' : 'other', sharedReturns: s.sharedReturns }; });
                    assert.equal(outsideCloseFocus.sharedReturns, 0, `${profile}: shared restoration stays suppressed after outside navigation`);
                    await page.evaluate(() => { const s = window.surfaceAssessment; s.controller.open({ source: s.source }); });
                }
                if (profile === 'modal') {
                    await page.getByRole('button', { name: 'Last', exact: true }).focus();
                    await page.keyboard.press('Tab');
                    // The native sequential-focus boundary may include browser chrome.
                    assert.equal(await page.evaluate(() => document.activeElement === window.surfaceAssessment.outside), false, 'Modal Tab cannot enter background controls');
                    await page.getByRole('textbox', { name: 'Entry' }).focus();
                }
                const beforeGuards = await page.evaluate(() => { const s = window.surfaceAssessment; s.veto = true; return s.guards; });
                await page.keyboard.press('Escape');
                assert.equal(await page.evaluate(() => { const s = window.surfaceAssessment; return s.owner.open || s.owner.matches(':popover-open'); }), true, `${profile}: close guard`);
                assert.equal(await page.evaluate(() => window.surfaceAssessment.guards), beforeGuards + 1, `${profile}: one guard per request`);
                await page.evaluate(() => { window.surfaceAssessment.veto = false; });
                await page.keyboard.press('Escape');
                await page.waitForFunction(() => { const s = window.surfaceAssessment; return !s.owner.open && !s.owner.matches(':popover-open'); });
                assert.equal(await page.evaluate(() => document.activeElement === window.surfaceAssessment.source), true, `${profile}: Escape returns to launcher`);
            }
            const hiding = await page.evaluate(() => window.surfaceAssessment.hides);
            assert(hiding.every(cancelable => !cancelable), `${profile}: closing beforetoggle is not a request guard`);
            reports.push({ profile, ...initial, ...(outsideCloseFocus ? { outsideCloseFocus } : {}) });
            await page.evaluate(() => window.surfaceAssessment.controller.disconnect());
        }
        // Observe policy gaps without turning current deviations into acceptance assertions.
        const policy = {};
        for (const adapted of [false, true]) {
            await page.goto(fixtureUrl);
            await page.evaluate(async ({ runtimeUrl, adapted }) => {
                document.body.innerHTML = '<button>Launcher</button><button>Outside</button><dialog closedby="none" aria-label="Policy probe"><input autofocus aria-label="Entry"></dialog>';
                const [source, outside] = document.querySelectorAll('body > button'), owner = document.querySelector('dialog');
                const s = window.surfaceAssessment = { source, outside, owner };
                source.focus();
                if (adapted) { const { connectCemNativeSurface } = await import(runtimeUrl); s.controller = connectCemNativeSurface(owner); s.controller.open({ source }); }
                else owner.show();
            }, { runtimeUrl, adapted });
            await page.keyboard.press('Escape');
            const outcome = await page.evaluate(() => { const s = window.surfaceAssessment; return { closedBySupported: 'closedBy' in s.owner, escapeClosed: !s.owner.open }; });
            if (!adapted && outcome.closedBySupported) assert.equal(outcome.escapeClosed, false, 'Native closedby=none ignores Escape');
            if (!adapted) outcome.outsideCloseDestination = await page.evaluate(() => {
                const s = window.surfaceAssessment; s.outside.focus(); s.owner.requestClose();
                return document.activeElement === s.outside ? 'outside' : document.activeElement === s.source ? 'launcher' : 'other';
            });
            policy[adapted ? 'adapterNonmodalNone' : 'nativeNonmodalNone'] = outcome;
            await page.evaluate(() => { const s = window.surfaceAssessment; if (s.controller) s.controller.disconnect(); else s.owner.close(); });
        }
        await page.goto(fixtureUrl);
        const lightDismissSupported = await page.evaluate(async runtimeUrl => {
            const { connectCemNativeSurface } = await import(runtimeUrl);
            document.body.innerHTML = '<button>Launcher</button><dialog mode="modal" closedby="any" aria-label="Dismissal probe"><input autofocus></dialog>';
            const source = document.querySelector('button'), owner = document.querySelector('dialog');
            const s = window.surfaceAssessment = { source, owner, veto: true, guards: 0 };
            owner.addEventListener('cem-before-close', event => { s.guards++; if (s.veto) event.preventDefault(); });
            s.controller = connectCemNativeSurface(owner); source.focus(); s.controller.open({ source });
            return 'closedBy' in owner;
        }, runtimeUrl);
        if (lightDismissSupported) {
            await page.mouse.click(880, 680);
            assert.deepEqual(await page.evaluate(() => { const s = window.surfaceAssessment; return [s.owner.open, s.guards]; }), [true, 1], 'Modal outside close request can be vetoed');
            await page.evaluate(() => { window.surfaceAssessment.veto = false; });
            await page.mouse.click(880, 680);
            await page.waitForFunction(() => !window.surfaceAssessment.owner.open);
        } else await page.evaluate(() => { const s = window.surfaceAssessment; s.veto = false; s.controller.requestClose(); });
        await page.evaluate(() => { const s = window.surfaceAssessment; s.veto = true; s.controller.open({ source: s.source }); });
        const beforeForce = await page.evaluate(() => window.surfaceAssessment.guards);
        await page.evaluate(() => window.surfaceAssessment.owner.close('forced'));
        assert.deepEqual(await page.evaluate(() => { const s = window.surfaceAssessment; return [s.owner.open, s.owner.returnValue, s.guards]; }), [false, 'forced', beforeForce], 'Forced native close is distinct from a guarded request');
        await page.evaluate(() => window.surfaceAssessment.controller.disconnect());
        await page.goto(fixtureUrl);
        await page.evaluate(async runtimeUrl => {
            const { connectCemNativeSurface } = await import(runtimeUrl);
            document.body.innerHTML = '<button>Launcher</button><dialog mode="modal" aria-label="Parent"><button autofocus>Child launcher</button><dialog popover="manual" aria-label="Child"><button autofocus>Child entry</button></dialog><span role="tooltip" popover="manual">Help</span></dialog>';
            const source = document.querySelector('body > button'), owner = document.querySelector('dialog'), child = owner.querySelector('dialog'), tip = owner.querySelector('[role=tooltip]');
            window.surfaceAssessment = { source, owner, child, tip, controller: connectCemNativeSurface(owner), nested: connectCemNativeSurface(child), tooltip: connectCemNativeSurface(tip) };
            const s = window.surfaceAssessment; source.focus(); s.controller.open({ source }); s.nested.open({ source: owner.querySelector('button') });
        }, runtimeUrl);
        await page.keyboard.press('Escape');
        assert.deepEqual(await page.evaluate(() => { const s = window.surfaceAssessment; return [s.owner.matches(':modal'), s.child.matches(':popover-open'), document.activeElement === s.owner.querySelector('button')]; }), [true, false, true], 'Escape dismisses the child before the modal parent');
        await page.evaluate(() => { const s = window.surfaceAssessment; s.tooltip.open({ source: s.owner.querySelector('button') }); });
        await page.keyboard.press('Escape');
        assert.deepEqual(await page.evaluate(() => { const s = window.surfaceAssessment; return [s.owner.matches(':modal'), s.tip.matches(':popover-open')]; }), [true, false], 'Tooltip Escape preserves the modal task');
        await page.evaluate(() => { const s = window.surfaceAssessment; s.nested.open({ source: s.owner.querySelector('button') }); s.controller.requestClose(); });
        assert.deepEqual(await page.evaluate(() => { const s = window.surfaceAssessment; return [s.owner.open, s.child.matches(':popover-open'), document.activeElement === s.source]; }), [false, false, true], 'Parent closure tears down descendants without child focus restoration');
        await page.evaluate(() => { const s = window.surfaceAssessment; s.tooltip.disconnect(); s.nested.disconnect(); s.controller.disconnect(); });
        assert.deepEqual(errors, []);
        console.log(`Dialog presentation assessment ${browser.browserType().name()} ${browser.version()}: ${JSON.stringify(reports)}; policy observations: ${JSON.stringify(policy)}; modal light dismissal supported: ${lightDismissSupported}; forced close, nested dismissal and focus passed (${runtimeUrl}).`);
    } finally { await page.close(); }
}
