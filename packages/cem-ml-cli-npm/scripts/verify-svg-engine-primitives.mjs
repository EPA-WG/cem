import assert from 'node:assert/strict';

async function deadline(promise, label) {
    let timer;
    try {
        return await Promise.race([promise, new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error(`${label} timed out`)), 5000);
        })]);
    } finally { clearTimeout(timer); }
}

/** Engine primitive feasibility only. Neither isolated evaluation nor a debugger
 * pause grants native origin authority or a production mutation interval. */
export async function verifySvgEnginePrimitives(context, page, origin) {
    await page.goto(origin);
    const controller = await context.newCDPSession(page);
    const competitor = await context.newCDPSession(page);
    const cases = [];
    let paused = false;
    let evaluation;
    try {
        await controller.send('Page.enable');
        await controller.send('Runtime.enable');
        await controller.send('DOM.enable');
        await controller.send('Debugger.enable');
        await competitor.send('DOM.enable');
        await competitor.send('CSS.enable');
        const { frameTree } = await controller.send('Page.getFrameTree');
        const { executionContextId } = await controller.send('Page.createIsolatedWorld', {
            frameId: frameTree.frame.id, worldName: 'cem-origin-media-fixture',
        });
        const isolatedMedia = async () => {
            const result = await controller.send('Runtime.evaluate', {
                contextId: executionContextId,
                expression: "matchMedia('(forced-colors: active)').matches", returnByValue: true,
            });
            assert.ok(!result.exceptionDetails);
            assert.equal(result.result.type, 'boolean');
            return result.result.value;
        };
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await isolatedMedia(), true);
        await page.evaluate(() => { globalThis.matchMedia = () => ({ matches: false }); });
        assert.equal(await page.evaluate(() => matchMedia('(forced-colors: active)').matches), false);
        assert.equal(await isolatedMedia(), true);
        cases.push({ name: 'isolated-media-resists-main-world-spoof', isolated: true, mainWorld: false });
        await page.emulateMedia({ forcedColors: 'none' });
        assert.equal(await isolatedMedia(), false);
        cases.push({ name: 'isolated-media-tracks-restoration', authenticatedEnvironmentLease: false });

        const { root } = await competitor.send('DOM.getDocument');
        const { nodeId } = await competitor.send('DOM.querySelector', { nodeId: root.nodeId, selector: 'rect' });
        const { styleSheetId } = await competitor.send('CSS.createStyleSheet', { frameId: frameTree.frame.id });
        const pausedEvent = new Promise(resolve => controller.once('Debugger.paused', () => { paused = true; resolve(); }));
        controller.on('Debugger.resumed', () => { paused = false; });
        evaluation = controller.send('Runtime.evaluate', { expression: 'debugger;' });
        // Observe failures even if a pause timeout takes the cleanup path.
        evaluation.catch(() => undefined);
        await deadline(pausedEvent, 'debugger pause');
        const alias = `url("${origin}/alias.svg#p")`;
        const source = `url("${origin}/source.svg#p")`;
        await deadline(competitor.send('DOM.setAttributeValue', { nodeId, name: 'fill', value: alias }), 'competing DOM write');
        assert.equal(paused, true, 'DOM write unexpectedly resumed debugger');
        const { attributes } = await competitor.send('DOM.getAttributes', { nodeId });
        assert.equal(attributes[attributes.indexOf('fill') + 1], alias);
        cases.push({ name: 'paused-debugger-allows-independent-dom-write', intervalEstablished: false });
        await deadline(competitor.send('CSS.setStyleSheetText', { styleSheetId, text: `rect { fill: ${source} !important; }` }), 'competing CSS write');
        const { computedStyle } = await deadline(competitor.send('CSS.getComputedStyleForNode', { nodeId }), 'paused computed observation');
        assert.equal(computedStyle.find(p => p.name === 'fill').value, source);
        assert.equal(paused, true, 'CSS write unexpectedly resumed debugger');
        cases.push({ name: 'paused-debugger-allows-independent-css-write', intervalEstablished: false });
        await controller.send('Debugger.resume');
        await deadline(evaluation, 'debugger evaluation completion');
        evaluation = undefined;
        await page.goto(origin + '/new-document');
        await assert.rejects(isolatedMedia);
        cases.push({ name: 'navigation-revokes-isolated-context' });
        assert.equal(cases.length, 5);
        console.log(`Verified ${cases.length} SVG origin engine primitive cases; debugger pause does not establish mutation exclusion.`);
        return { cases, executionSeal: false, productionAdmission: false,
            environmentLease: false, mutationInterval: false };
    } finally {
        try {
            if (paused) await controller.send('Debugger.resume');
            if (evaluation) await deadline(evaluation, 'cleanup evaluation').catch(() => undefined);
        } finally {
            await Promise.allSettled([controller.detach(), competitor.detach()]);
        }
    }
}
