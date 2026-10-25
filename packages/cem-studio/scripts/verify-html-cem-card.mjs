import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Ordinary project acceptance; all preview and interaction behavior is shared. */
export async function verifyHtmlCemCard({ page, context, url }) {
    const definition = JSON.parse(await readFile(new URL('../../../examples/projects/html-cem-card/example.json', import.meta.url), 'utf8'));
    const configuration = id => definition.configurations.find(item => item.id === id);
    const source = id => definition.sources.find(item => item.id === id);
    const fixture = id => readFile(new URL(`../../../${source(id).source}`, import.meta.url), 'utf8');
    const htmlId = configuration('inspect-html').source, cemId = configuration('inspect-cem').source;
    const viewId = configuration('live-view').source, componentId = configuration('inspect-component').source;
    const template = source(configuration('build-html').dependencies.find(id => source(id).role === 'template'));
    const repair = configuration('repair-html-policy');
    const expectedSemantics = JSON.parse(await fixture(configuration('semantics-html').expectedSemanticContract));
    const expectedHtml = await fixture(configuration('build-html').expectedGraphOutputs['generated/card.html']);
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => {
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle();
    };
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), preview = current.projectPreview;
        return { route: current.navigation.selection, status: current.status, dirty: current.dirty, text: current.projection?.output?.text,
            diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics,
            previewKind: current.projection?.preview?.kind,
            declarations: current.projection?.preview?.kind === 'live-cem' ? JSON.parse(current.projection.preview.resourceSet).declarations : undefined,
            buildStatus: preview.attempt?.status, entryArtifact: preview.entryArtifact, html: preview.projection?.output?.text,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            inputs: preview.projection?.buildRun.inputs.map(item => ({ resourceId: item.resourceId, path: item.path, uri: item.uri, sha256: item.sha256 })),
            origins: preview.projection?.buildRun.artifacts.flatMap(item => item.provenance.map(frame => frame.uri)) };
    });
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonlySource = page.locator('cem-source-editor[readonly] [role=textbox]');
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); return state(); };
    const run = async (project, id) => {
        await open({ project, configuration: id, build: 'build-html' });
        if (!await page.getByRole('button', { name: 'Run configuration', exact: true }).isVisible()) {
            await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        await click('Run configuration'); return state();
    };
    const validate = async project => {
        await open({ project, configuration: repair.id, build: 'build-html' });
        const button = page.getByRole('button', { name: 'Validate persisted revision', exact: true });
        if (!await button.isVisible()) {
            await page.locator('cem-expansion[label="Editor details"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        await button.click(); await settle(); return state();
    };
    const values = current => {
        const report = JSON.parse(current.text);
        return report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
    };
    const build = async project => {
        await open({ project, build: 'build-html' }); await click('Build project');
        const current = await state(); assert.equal(current.buildStatus, 'succeeded', JSON.stringify(current)); return current;
    };
    const artifactFrame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
    const artifactReady = async title => {
        await page.waitForFunction(title => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('h1')?.textContent === title, title);
        assert.equal(await artifactFrame.getAttribute('sandbox'), 'allow-same-origin');
    };
    const showResults = async () => {
        const header = page.locator('cem-expansion[label="Commands and results"] > [part~=root] > [part~=heading] > [part~=header]');
        if (await header.getAttribute('aria-expanded') === 'false') { await header.click(); await settle(); }
        await page.getByRole('tab', { name: 'Results', exact: true }).click(); await settle();
        await page.getByRole('tab', { name: 'Operation output', exact: true }).click(); await settle();
    };
    const liveFrame = page.locator('[data-cem-studio-operation-preview] cem-media-preview[mode=live] iframe[part~=live-document]');
    const liveStatus = '[data-cem-studio-operation-preview] cem-media-preview[mode=live] [role=status]';
    const liveReady = async () => {
        await showResults();
        await page.waitForFunction(selector => document.querySelector(selector)?.textContent === 'Live view ready', liveStatus);
        // Controller readiness precedes asynchronous project-resource settlement.
        await page.waitForFunction(() => document.querySelector('[data-cem-studio-operation-preview] iframe[part~=live-document]')
            ?.contentDocument?.querySelector('h1')?.textContent?.trim());
        assert.equal((await state()).previewKind, 'live-cem');
        assert.equal(await liveFrame.getAttribute('sandbox'), 'allow-same-origin');
        assert.equal(await liveFrame.getAttribute('allow'), '');
    };
    const live = () => liveFrame.evaluate(frame => {
        const document = frame.contentDocument, card = document.querySelector('cem-card'), section = document.querySelector('section[aria-label="Shared card example"]');
        const paint = frame.contentWindow.getComputedStyle(section);
        return { heading: document.querySelector('h1')?.textContent?.trim(), cards: document.querySelectorAll('cem-card').length,
            action: document.querySelector('cem-action button')?.textContent?.trim(), notice: document.querySelector('cem-alert')?.getAttribute('label'),
            detailShown: [...document.querySelectorAll('cem-card p')].some(paragraph => paragraph.textContent.trim() === 'The saved source belongs to this project.'),
            light: section.classList.contains('cem-theme-light'), shadow: card.shadowRoot !== null,
            foreground: paint.color, background: paint.backgroundColor, gap: parseFloat(paint.rowGap), inset: parseFloat(paint.paddingTop),
            location: frame.contentWindow.location.href, scripts: document.querySelectorAll('script').length };
    });
    const liveClick = async () => {
        const point = await liveFrame.evaluate(frame => {
            frame.scrollIntoView({ block: 'center' });
            const button = frame.contentDocument.querySelector('cem-action button'); button.scrollIntoView({ block: 'nearest' });
            const outer = frame.getBoundingClientRect(), inner = button.getBoundingClientRect();
            return { x: outer.x + frame.clientLeft + inner.x + inner.width / 2, y: outer.y + frame.clientTop + inner.y + inner.height / 2 };
        });
        const diagnostics = () => liveFrame.evaluate((frame, point) => {
            const document = frame.contentDocument, outer = frame.getBoundingClientRect(), button = document?.querySelector('cem-action button');
            const describe = node => node?.nodeType === 1 ? { tag: node.tagName,
                attributes: [...node.attributes].slice(0, 24).map(attribute => [attribute.name, attribute.value.slice(0, 240)]) } : { tag: node?.nodeName };
            const local = { x: point.x - outer.x - frame.clientLeft, y: point.y - outer.y - frame.clientTop };
            const owner = host => {
                const island = host?.querySelector(':scope > template[data-cem-island="instance"]');
                const text = island?.content?.textContent ?? '', index = text.indexOf('notice');
                return { host: describe(host), islandLength: text.length,
                    notice: text.slice(Math.max(0, index - 100), Math.max(0, index - 100) + 1400) };
            };
            const clipping = [];
            for (let node = frame.parentElement; node && clipping.length < 10; node = node.parentElement) {
                const style = frame.ownerDocument.defaultView.getComputedStyle(node);
                if ([style.overflowX, style.overflowY].some(value => value !== 'visible')) clipping.push({
                    ...describe(node), overflow: [style.overflowX, style.overflowY], rect: node.getBoundingClientRect().toJSON(),
                });
            }
            return { point, local, frame: outer.toJSON(), button: button?.getBoundingClientRect().toJSON(),
                parentHit: describe(frame.ownerDocument.elementFromPoint(point.x, point.y)), childHit: describe(document?.elementFromPoint(local.x, local.y)),
                clipping, events: frame.__cemFixtureClickEvents ?? [], status: frame.closest('cem-media-preview')?.querySelector('[role=status]')?.textContent,
                heading: document?.querySelector('h1')?.textContent, alerts: [...document?.querySelectorAll('cem-alert') ?? []].map(describe),
                owners: ['cem-live-view', 'cem-card[label="Primary shared card"]', 'cem-action'].map(selector => owner(document?.querySelector(selector))) };
        }, point);
        const before = await diagnostics();
        await liveFrame.evaluate(frame => {
            frame.__cemFixtureClickEvents = [];
            frame.contentDocument.addEventListener('click', event => {
                frame.__cemFixtureClickEvents.push({ trusted: event.isTrusted, target: event.target?.nodeName,
                    path: event.composedPath().slice(0, 12).map(node => node.nodeName ?? 'window') });
            }, { once: true, capture: true });
        });
        await page.mouse.click(point.x, point.y);
        try {
            await page.waitForFunction(() => document.querySelector('[data-cem-studio-operation-preview] iframe[part~=live-document]')?.contentDocument?.querySelector('cem-alert')?.getAttribute('label') === 'Shared CEM interaction received.');
        } catch (error) {
            console.error('[cem-studio:html-cem-card:live-click]', JSON.stringify({ before, after: await diagnostics() }));
            throw error;
        }
    };

    await page.goto(`${url}/?collection=content-types-documents-ui`); await ready();
    const record = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === 'html-cem-card'));
    assert.ok(record?.description);
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
    assert.equal((await state()).route.sample, 'html-cem-card');
    const originals = {};
    for (const id of [htmlId, cemId, viewId, componentId, repair.source]) {
        await open({ collection: 'content-types-documents-ui', sample: 'html-cem-card', file: id, build: 'build-html' });
        assert.equal(await readonlySource.getAttribute('aria-readonly'), 'true'); originals[id] = await readonlySource.evaluate(element => element.value);
    }
    await open({ collection: 'content-types-documents-ui', sample: 'html-cem-card', build: 'build-html' }); await click('Build project');
    const sample = await state(); assert.equal(sample.buildStatus, 'succeeded', JSON.stringify(sample));
    assert.equal(sample.entryArtifact, 'generated/card.html'); assert.equal(sample.html, expectedHtml); await artifactReady('Harbor field notes');
    await click('Clone selected');
    await page.getByRole('dialog').getByRole('textbox').fill('Navigator HTML and CEM card');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    const projectId = (await state()).route.project; assert.ok(projectId);
    for (const id of ['inspect-html', 'inspect-cem', 'inspect-component']) assert.equal((await run(projectId, id)).status, 'projected');
    assert.deepEqual(values(await run(projectId, 'semantics-html')), expectedSemantics);
    assert.deepEqual(values(await run(projectId, 'semantics-cem')), expectedSemantics);
    const invalid = await validate(projectId), expected = repair.expectedValidation;
    const code = expected.errorCode ?? expected.warningCode, diagnostic = invalid.diagnostics?.find(item => item.code === code);
    assert.ok(diagnostic, JSON.stringify(invalid)); assert.equal(diagnostic.uri, `studio://${projectId}/${source(repair.source).path}`);
    assert.equal(diagnostic.severity, expected.errorCode ? 'error' : 'warning');
    for (const [key, value] of Object.entries(expected.location ?? {})) assert.equal(diagnostic[key] ?? null, value);
    await open({ project: projectId, file: repair.source, build: 'build-html' });
    const repairedPolicy = originals[repair.source].replace(repair.repair.find, repair.repair.replace);
    assert.notEqual(repairedPolicy, originals[repair.source]); await save(repairedPolicy);
    assert.equal((await validate(projectId)).status, 'valid');
    console.log('[cem-studio:html-cem-card] catalog, static output, semantic configurations and native policy repair passed');
    const themeBefore = await page.evaluate(() => localStorage.getItem('cem-studio-theme'));
    await run(projectId, 'live-view'); await liveReady();
    const baselineLive = await live();
    assert.equal(baselineLive.heading, 'Harbor field notes'); assert.equal(baselineLive.action, 'Show details'); assert.equal(baselineLive.cards, 2);
    assert.equal(baselineLive.detailShown, false); assert.equal(baselineLive.notice, 'This view uses shared CEM components.');
    assert.equal(baselineLive.shadow, false); assert.equal(baselineLive.light, true); assert.equal(baselineLive.location, 'about:srcdoc'); assert.equal(baselineLive.scripts, 0);
    assert.ok(baselineLive.gap > 0 && baselineLive.inset > 0 && baselineLive.background !== 'rgba(0, 0, 0, 0)' && baselineLive.foreground !== baselineLive.background);
    await liveClick(); assert.equal((await live()).detailShown, true);
    assert.equal(await page.evaluate(() => localStorage.getItem('cem-studio-theme')), themeBefore);
    await page.emulateMedia({ forcedColors: 'active' });
    const forced = await live(); assert.notEqual(forced.foreground, forced.background); await page.emulateMedia({ forcedColors: 'none' });
    console.log('[cem-studio:html-cem-card] first genuine live interaction, conditional details and theme isolation passed');
    // A draft view does not replace persisted execution; saving explicitly changes it.
    const editedView = originals[viewId].replace('Show details', 'Reveal details'); assert.notEqual(editedView, originals[viewId]);
    await editor.fill(editedView); await settle(); await click('Run configuration'); await liveReady();
    assert.equal((await live()).action, 'Show details'); assert.equal((await state()).dirty, true);
    await click('Save and validate'); await click('Run configuration'); await liveReady(); assert.equal((await live()).action, 'Reveal details');
    await run(projectId, 'semantics-html');
    const editedHtml = originals[htmlId].replaceAll('Harbor field notes', 'Canal field notes'); assert.notEqual(editedHtml, originals[htmlId]);
    await editor.fill(editedHtml); await settle(); await click('Run configuration');
    assert.deepEqual(values(await state()), expectedSemantics); assert.equal((await state()).dirty, true);
    await click('Save and validate'); await click('Run configuration');
    assert.deepEqual(values(await state()), expectedSemantics.map(value => typeof value === 'string' ? value.replaceAll('Harbor field notes', 'Canal field notes') : value));
    await run(projectId, 'live-view'); await liveReady(); assert.equal((await live()).heading, 'Canal field notes');
    console.log('[cem-studio:html-cem-card] unsaved execution guards and saved HTML data/view reruns passed');
    // Inspection resources are editable project text, never declaration authority.
    const editedComponent = originals[componentId].replace('label | Card', 'label | Saved snapshot'); assert.notEqual(editedComponent, originals[componentId]);
    await open({ project: projectId, file: componentId, build: 'build-html' }); await save(editedComponent);
    await run(projectId, 'live-view'); await liveReady();
    assert.equal((await live()).cards, 2); assert.equal((await live()).heading, 'Canal field notes');
    assert.ok((await state()).declarations.some(item => item.tag === 'cem-card' && item.uri === 'cem-preview://shared/components/cem-card/cem-card.xhtml#cem-card'));
    // Retain the existing shared refusal for declarations emitted by preview content.
    await save(`${editedView}\n{cem-element @tag=cem-untrusted-card}`); await click('Run configuration'); await showResults();
    await page.waitForFunction(selector => document.querySelector(selector)?.textContent?.includes('cem.preview.element_denied: cem-element'), liveStatus);
    assert.equal(await liveFrame.evaluate(frame => Boolean(frame.contentDocument?.querySelector('cem-untrusted-card, cem-element[tag="cem-untrusted-card"]'))), false);
    await save(editedView); await click('Run configuration'); await liveReady(); assert.equal((await live()).heading, 'Canal field notes');
    console.log('[cem-studio:html-cem-card] component snapshot isolation, nested declaration refusal and repair passed');
    const editedBuild = await build(projectId);
    assert.equal(editedBuild.html, expectedHtml.replaceAll('Harbor field notes', 'Canal field notes')); await artifactReady('Canal field notes');
    assert.ok(editedBuild.origins.includes(`studio://${projectId}/${template.path}`));
    assert.ok(editedBuild.inputs.some(item => item.resourceId === htmlId && item.uri === `studio://${projectId}/${source(htmlId).path}`));
    assert.deepEqual(editedBuild.artifacts.map(item => item.path), ['generated/card.html']);
    await open({ collection: 'content-types-documents-ui', sample: 'html-cem-card', build: 'build-html' }); await click('Clone selected');
    await page.getByRole('dialog').getByRole('textbox').fill('Independent HTML and CEM card');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    const independentId = (await state()).route.project; assert.ok(independentId && independentId !== projectId);
    await run(independentId, 'live-view'); await liveReady();
    const independent = await live(); assert.equal(independent.heading, 'Harbor field notes'); assert.equal(independent.action, 'Show details');
    assert.equal(independent.notice, baselineLive.notice); assert.equal((await build(independentId)).html, expectedHtml);
    console.log('[cem-studio:html-cem-card] independent clone retains its original static and live views');
    await run(projectId, 'live-view'); await liveReady();
    await page.evaluate(async () => { await navigator.serviceWorker.ready; }); await context.setOffline(true);
    try {
        await page.reload(); await ready();
        assert.equal((await state()).route.project, projectId); assert.equal((await state()).route.configuration, 'live-view');
        await click('Run configuration'); await liveReady();
        const offlineLive = await live(); assert.equal(offlineLive.heading, 'Canal field notes'); assert.equal(offlineLive.action, 'Reveal details');
        assert.equal(offlineLive.notice, baselineLive.notice); await liveClick();
        const offline = await build(projectId); assert.deepEqual(offline.artifacts, editedBuild.artifacts);
        assert.deepEqual(offline.inputs, editedBuild.inputs); assert.deepEqual(offline.origins, editedBuild.origins); await artifactReady('Canal field notes');
        assert.equal((await validate(projectId)).status, 'valid');
        for (const [id, text] of [[htmlId, editedHtml], [viewId, editedView], [componentId, editedComponent], [repair.source, repairedPolicy]]) {
            await open({ project: projectId, file: id, build: 'build-html' }); assert.equal(await editor.evaluate(element => element.value), text);
        }
        for (const [id, text] of Object.entries(originals)) {
            await open({ collection: 'content-types-documents-ui', sample: 'html-cem-card', file: id, build: 'build-html' });
            assert.equal(await readonlySource.evaluate(element => element.value), text);
        }
    } finally { await context.setOffline(false); }
    assert.equal(await page.evaluate(() => localStorage.getItem('cem-studio-theme')), themeBefore);
    console.log('[cem-studio:html-cem-card] semantic equivalence, native policy repair, static/live saved views, declaration guards and offline independent copies passed');
    return { catalog: true, readonlyOriginals: true, semanticEquivalence: true, nativePolicyRepair: true, staticEntry: true,
        liveSharedComponents: true, genuineInteraction: true, unsavedSourceGuard: true, savedDataAndView: true,
        componentSnapshotHasNoAuthority: true, nestedDeclarationRefusal: true, themeIsolation: true, nativeProvenance: true, independentCopies: true, offline: true };
}
