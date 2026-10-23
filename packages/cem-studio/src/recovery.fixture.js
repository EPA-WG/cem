/** Isolated acceptance driver, never imported by the application. */
export async function studioRecovery(mode, progress, complete = false, privacy = false) {
    const check = (condition, message) => { if (!condition) throw Error(message); };
    const elements = new URL('.', new URL('../../cem-elements/dist/index.js', import.meta.url));
    const support = new URL('lib/internal/runtime-support/', elements);
    progress('native binding import');
    const binding = await import(/* @vite-ignore */ new URL('native-runtime.js', support).href);
    const initial = binding.isolateCemFallbackGeneration(); progress('native initialization'); await initial.ready(); progress('native ready');
    let heap = initial, embedding = initial.claimOwnership(), admission, combined;
    const { CemElementRuntime, createCemDeclarationScope, CemCheckpointPublicationCoordinator, CemNativeRecoveryScope, resumeCemCheckpointOwners } = await import(/* @vite-ignore */ elements.href + 'index.js');
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
        policyStamp: 'studio-acceptance', maxBytes: 4 * 1024 * 1024, allowOwner: () => true,
        seal: (candidate, receipt) => binding.sealCommittedCheckpoint(candidate, receipt) });
    const recoveryScope = new CemNativeRecoveryScope();
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ declarationScope: scope, declarationTag: 'cem-studio-component-declaration', checkpointPublication: coordinator, recoveryScope,
        processingWorkerFactory: () => { if (mode === 'fallback') throw Error('owned fallback acceptance');
            return new Worker(new URL('processing-worker.js', support), { type: 'module' }); } });
    const map = document.createElement('script'); map.type = 'importmap'; map.textContent = JSON.stringify({ imports: {
        '@epa-wg/cem-theme': new URL('../../cem-theme/dist/index.js', import.meta.url).href,
        '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href,
    } }); document.head.append(map);
    const { installCemStudioShellComponents, mountCemStudioApplicationShell } = await import('./shell.js');
    const { createCemStudioProjectRepository } = await import('./repository.js');
    const { parseCemStudioProjectBundle } = await import('./file-system-provider.js');
    const { createCemStudioBrowserValidator } = await import('./feature-tour.js');
    const { createCemStudioProjectSession } = await import('./project-session.js');
    const { createCemStudioNavigation } = await import('./navigation.js');
    const { createCemStudioProjectWorkbench, createCemStudioSessionWorkbench, mountCemStudioWorkbench } = await import('./workbench.js');
    const records = new Map([['unrelated-draft', 'preserved']]);
    let storageWrites = 0;
    const storage = { getItem: key => records.get(key) ?? null, setItem: (key, value) => { storageWrites++; records.set(key, value); } };
    const databaseName = `cem-studio-recovery-${crypto.randomUUID()}`;
    progress('command services');
    const validator = await createCemStudioBrowserValidator({ recoveryScope });
    progress('command services ready');
    const repository = createCemStudioProjectRepository({ databaseName, validateProject: validator.validateProject });
    const request = (operation, parameters = {}) => ({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation, parameters });
    let session, workbench, view, shell, stage = 'install';
    const setStage = value => { stage = value; progress(value); };
    try {
        setStage('install'); await installCemStudioShellComponents({ runtime });
        check((await installCemStudioShellComponents()).runtime === runtime, 'Studio did not retain the supplied runtime');
        let replacementRefused = false;
        try { await installCemStudioShellComponents({ runtime }); } catch { replacementRefused = true; }
        check(replacementRefused, 'Studio accepted a runtime after installation');
        setStage('installed');
        const response = await fetch(new URL('../generated/feature-tour/examples/login.cem-studio.json', import.meta.url));
        check(response.ok, 'missing Login sample');
        const bundle = parseCemStudioProjectBundle(new Uint8Array(await response.arrayBuffer()));
        session = createCemStudioProjectSession({ storage, openWorkbench: selection => createCemStudioProjectWorkbench({ repository, validator, selection }) });
        setStage('session'); const base = await createCemStudioSessionWorkbench({ repository, session });
        const browser = new EventTarget(); browser.location = new URL('https://studio.test/?sample=login&file=login-html');
        browser.history = { pushState: (_, __, url) => { browser.location = new URL(url); }, replaceState: (_, __, url) => { browser.location = new URL(url); } };
        workbench = await createCemStudioNavigation({ repository, validator, workbench: base, browser,
            examples: [{ id: 'login', name: 'Login', load: async () => bundle }] });
        const root = document.createElement('main'); document.body.append(root);
        setStage('shell'); shell = await mountCemStudioApplicationShell({ root, storage });
        setStage('workbench'); view = await mountCemStudioWorkbench({ root: shell.root, workbench }); await view.whenSettled();
        setStage('clone'); await workbench.openEntity('sample=login'); workbench.requestAction('clone');
        await workbench.submitAction({ name: ['Recovery acceptance'] }); await view.whenSettled();
        const project = workbench.snapshot().navigation.selection.project;
        let sourceId = 'login-html';
        await workbench.openEntity(`project=${project}&file=login-html&build=render-graph`); await view.whenSettled();
        let original = workbench.snapshot().draft;
        let assertLogin = () => undefined;
        let checkpointBytes = 0, groupBytes = 0;
        let edits = 0;
        const paint = editor => [...CSS.highlights.values()].flatMap(value => [...value]).filter(range => editor.contains(range.startContainer));
        const unreviewedPreviews = () => [...root.querySelectorAll('cem-media-preview')].filter(host => !['live', 'artifact-tree', 'document', 'text', 'media', 'message'].includes(host.getAttribute('mode')));
        const blockers = () => coordinator.inspectRecovery().owners.filter(owner => !owner.adapterAvailable || owner.tag === 'cem-media-preview' && unreviewedPreviews().length > 0);
        const selectUnreviewedPreview = async () => {
            const preview = root.querySelector('cem-media-preview'); check(preview, 'missing real preview');
            // Deliberately select this public mode to retain the unsupported-
            // group test after live/artifact controllers gained adapters.
            preview.setAttribute('mode', 'unreviewed-recovery-mode'); await runtime.whenRenderSettled(preview);
        };
        const themes = coordinator.inspectRecovery().owners.filter(owner => owner.tag === 'cem-theme-switch');
        check(themes.length > 0 && themes.every(owner => owner.adapterAvailable), 'shared theme storage adapter missing');
        const retained = coordinator.inspectRecovery().owners.filter(owner => ['cem-action', 'cem-expansion', 'cem-list'].includes(owner.tag));
        check(retained.some(owner => owner.tag === 'cem-action') && retained.every(owner => owner.adapterAvailable), 'shared retained capability adapter missing');
        // Live/artifact controllers have adapters. Unknown preview modes still
        // refuse publication; keep this actual incomplete-workspace lane.
        setStage('initial native preview'); await workbench.buildProjectPreview(); await view.whenSettled();
        check(workbench.snapshot().projectPreview?.attempt?.status === 'succeeded', `initial recovery preview failed: ${JSON.stringify(workbench.snapshot().projectPreview?.attempt)}`);
        if (!complete) await selectUnreviewedPreview();
        check(complete || unreviewedPreviews().length > 0 && blockers().some(owner => owner.tag === 'cem-media-preview'), 'refusal lane has no unknown preview mode');
        if (privacy) {
            setStage('Login credential privacy');
            await workbench.selectWorkbench(JSON.stringify({ projectId: project, entryId: 'live-view' }));
            await view.whenSettled();
            workbench.selectPreviewConfiguration('render-graph');
            await workbench.runPersistedOperation(); await view.whenSettled();
            let liveDocument;
            for (let n = 0; n < 100; n++) {
                liveDocument = root.querySelector('cem-media-preview[mode=live] iframe')?.contentDocument;
                if (liveDocument?.querySelector('input[type=password]')
                    && root.querySelector('cem-media-preview[mode=live] [role=status]')?.textContent === 'Live view ready') break;
                await new Promise(resolve => setTimeout(resolve, 10));
            }
            await view.whenSettled();
            check(root.querySelector('cem-media-preview[mode=live] [role=status]')?.textContent === 'Live view ready', 'Login controller has not published readiness');
            const password = liveDocument?.querySelector('input[type=password]');
            const email = liveDocument?.querySelector('input[type=email]');
            check(password && email, 'missing real Login credential controls');
            const liveHost = root.querySelector('cem-media-preview[mode=live]');
            const notice = () => liveDocument.querySelector('cem-alert')?.getAttribute('label');
            const initialNotice = notice(), credential = `private-${crypto.randomUUID()}`, identity = `identity-${crypto.randomUUID()}@example.test`;
            const editor = root.querySelector('cem-source-editor[data-cem-studio-editor]');
            for (let n = 0; !paint(editor).length; n++) {
                check(n < 100, 'Login source coloring did not settle'); await new Promise(resolve => setTimeout(resolve, 10));
            }
            for (const peer of [coordinator, ...coordinator.previewCoordinators()]) peer.current();
            if (mode === 'fallback') combined = CemCheckpointPublicationCoordinator.combine([coordinator, ...coordinator.previewCoordinators()]);
            const assertPrivacy = async () => {
                const peers = coordinator.previewCoordinators();
                check(peers.length > 0, 'Login child runtime was omitted');
                let blocked = false;
                for (const peer of combined ? [combined] : peers) {
                    try { await peer.capture(); } catch (error) { check(/cem.checkpoint.unavailable/.test(String(error)), 'unexpected Login capture failure'); blocked = true; }
                    const bytes = peer.lastCommitted()?.bytes;
                    check(!bytes || !new TextDecoder().decode(bytes).includes(credential), 'credential entered a child checkpoint');
                }
                check(blocked, 'Login child admitted credential-bearing checkpoint');
                let refused = false;
                try { await recoveryScope.prepare(); } catch (error) { refused = /unsupported|unavailable/.test(String(error)); }
                check(refused, 'Login admitted an incomplete recovery group');
                check(liveHost.isConnected && liveHost.querySelector('iframe')?.contentDocument === liveDocument, 'privacy refusal replaced Login');
                check(notice() === initialNotice, 'privacy refusal invoked the Login action');
                const exported = (await repository.query(request('export-project', { projectId: project }))).value;
                check(Object.values(exported.contents).every(bytes => !new TextDecoder().decode(bytes).includes(credential)), 'credential entered project resources');
                check(!JSON.stringify([...records]).includes(credential) && !JSON.stringify([...records]).includes(identity), 'credential entered draft/session storage');
            };
            password.value = credential; email.value = identity;
            await assertPrivacy(); // Password-manager/property-only edits cannot bypass the policy.
            password.dispatchEvent(new InputEvent('input', { bubbles: true }));
            email.dispatchEvent(new InputEvent('input', { bubbles: true }));
            await view.whenSettled(); await assertPrivacy();
            check(password.value === credential && email.value === identity, 'privacy refusal erased live credentials');
            password.value = '';
            password.dispatchEvent(new InputEvent('input', { bubbles: true }));
            email.dispatchEvent(new InputEvent('input', { bubbles: true }));
            await view.whenSettled();
            // The real Login view remains enrolled for all following recoveries.
            // Clearing is an explicit test-user action, never recovery behavior.
            setStage('Login credential cleared');
            const captureSettled = async c => {
                for (let n = 0; ; n++) {
                    try { await c.capture(); return; }
                    catch (error) {
                        if (n === 0) progress(`empty Login capture: ${JSON.stringify(c.inspectRecovery())}`);
                        if (n >= 100 || !/unavailable|stale/.test(String(error))) throw Error(`${error}: ${JSON.stringify(c.inspectRecovery())}`, { cause: error });
                        await new Promise(resolve => setTimeout(resolve, 10));
                    }
                }
            };
            const peers = [coordinator, ...coordinator.previewCoordinators()];
            // Entered/cleared input invalidates a combined membership witness.
            // Dispose the old alias and prepare a fresh one after publication.
            combined?.dispose(); combined = undefined;
            if (mode === 'fallback') {
                for (let n = 0; ; n++) {
                    for (const peer of peers) peer.current();
                    try { combined = CemCheckpointPublicationCoordinator.combine(peers); break; }
                    catch (error) {
                        if (n >= 100 || !/coverage/.test(String(error))) throw error;
                        await new Promise(resolve => setTimeout(resolve, 10));
                    }
                }
            }
            // Fallback peers share a physical heap: only their complete combined
            // coordinator has native inventory coverage, even with empty fields.
            for (const peer of mode === 'worker' ? peers.slice(1) : []) {
                await captureSettled(peer);
                check(!new TextDecoder().decode(peer.current().bytes).includes(credential), 'cleared Login checkpoint retained a credential');
            }
            for (const c of combined ? [combined] : peers) await captureSettled(c);
            assertLogin = () => check(liveHost.isConnected && liveHost.querySelector('iframe')?.contentDocument === liveDocument
                && password.value === '' && email.value === identity && notice() === initialNotice,
            'handoff changed Login edits, frame or action state');
            assertLogin();
            combined?.dispose(); combined = undefined;
            sourceId = 'login-live-view';
            original = workbench.snapshot().draft;

        }
        const enrolledKinds = () => recoveryScope.inspect().members.map(member => member.kind);
        if (privacy) check(enrolledKinds().includes('live-preview'), 'active Login runtime was omitted from membership');
        check(enrolledKinds().includes('rendering') && enrolledKinds().includes('command-service') && enrolledKinds().includes('command-invocation'), 'Studio omitted an active runtime from recovery membership');
        let groupRefused = false;
        try { if (!complete) await recoveryScope.prepare(); } catch (error) { groupRefused = /unsupported|unavailable/.test(String(error)); }
        check(complete || groupRefused, 'incomplete Studio runtime inventory prepared a partial group');

        for (let cycle = 0; cycle < 3; cycle++) {
            for (let edit = 0; edit < 4; edit++) {
                setStage(`edit ${cycle}/${edit}`);
                const control = root.querySelector('cem-source-editor[data-cem-studio-editor] [role=textbox]');
                control.focus(); control.value = privacy ? original.replace('Edit and save this view', `Edit and save this view recovery ${cycle}/${edit} é😀`) : `${original}\n<!-- recovery ${cycle}/${edit} é😀 -->\n`;
                control.setSelectionRange(2, 7, 'backward'); control.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await view.whenSettled(); edits++;
                check(workbench.snapshot().dirty && workbench.snapshot().draft === control.value, 'editor draft not accepted');
            }
            const colored = root.querySelector('cem-source-editor[data-cem-studio-editor]');
            for (let attempt = 0; !paint(colored).length; attempt++) {
                check(attempt < 100, 'editor coloring did not settle'); await new Promise(resolve => setTimeout(resolve, 10));
            }
            setStage(`${complete ? 'recovery' : 'refusal'} ${cycle}`); if (!complete) await selectUnreviewedPreview();
            const draft = workbench.snapshot().draft;
            const editor = root.querySelector('cem-source-editor[data-cem-studio-editor]'), control = editor.querySelector('[role=textbox]');
            const writes = storageWrites;
            const url = browser.location.href, savedRecords = JSON.stringify([...records]), styles = [...document.querySelectorAll('style')];
            const previews = [...root.querySelectorAll('cem-media-preview')].map(host => ({ host, mode: host.getAttribute('mode'), frames: [...host.querySelectorAll('iframe')].map(frame => ({ frame, document: frame.contentDocument })) }));
            const themeValues = [...root.querySelectorAll('cem-theme-switch')].map(host => ({ host, value: host.getAttribute('value') }));
            if (complete) {
                const capture = async c => {
                    for (let n = 0; ; n++) {
                        try { await c.capture(); return; } catch (error) {
                            if (n === 0) progress(`capture retry ${error}: ${JSON.stringify(c.inspectRecovery().owners.filter(owner => !owner.acknowledged || !owner.adapterAvailable))}`);
                            if (n >= 100 || !/unavailable|stale/.test(String(error))) throw Error(`${error.stack} ${JSON.stringify(c.inspectRecovery().owners.filter(owner => !owner.acknowledged || !owner.adapterAvailable))}`, { cause: error });
                            await new Promise(resolve => setTimeout(resolve, 10));
                        }
                    }
                };
                const peers = [coordinator, ...coordinator.previewCoordinators()];
                if (mode === 'fallback' && peers.length > 1) combined = CemCheckpointPublicationCoordinator.combine(peers);
                const coordinators = combined ? [combined] : peers;
                setStage(`capture ${cycle}`); for (const c of coordinators) await capture(c);
                const consent = mode === 'fallback' ? heap.retirementConsent([embedding]) : undefined;
                const proofs = []; for (const c of coordinators) proofs.push(await c.closeRenderingGeneration(5000, consent));
                for (const c of coordinators) await capture(c);
                const bytes = coordinators.reduce((total, c) => total + c.current().bytes.byteLength, 0);
                check(bytes <= 4 * 1024 * 1024, 'complete workspace exceeded existing aggregate byte budget');
                checkpointBytes = Math.max(checkpointBytes, bytes);
                const tickets = coordinators.map(c => c.prepareResume()), native = validator.prepareRecovery();
                const plan = await recoveryScope.prepare(), group = plan.createGenerationGroup();
                const commands = [native.invocation, native.commands], owners = commands.map(client => client.claimNativeEmbedding());
                const commandProofs = []; for (let i = 0; i < commands.length; i++) commandProofs.push(await commands[i].closeNativeAdmission([owners[i]]));
                setStage(`retire ${cycle}`); const retired = [];
                for (let i = 0; i < coordinators.length; i++) retired.push(await coordinators[i].retireRenderingGeneration(proofs[i], 5000, consent));
                for (let i = 0; i < commands.length; i++) retired.push(await commands[i].retireNativeGeneration(commandProofs[i], [owners[i]], {
                    current: plan.current, retain: () => new TextEncoder().encode('Studio host ledger; no pending commands or writes'),
                }));
                setStage(`allocate ${cycle}`); const previous = admission; admission = await group.allocate(retired);
                groupBytes = Math.max(groupBytes, admission.generations.reduce((total, generation) => total + generation.checkpoint.byteLength, 0));
                check(groupBytes <= 4 * 1024 * 1024, 'complete native group exceeded existing byte budget');
                check(admission.generations.length === retired.length && admission.generations.every(generation => generation.runtimeId !== generation.previousRuntimeId), 'partial or reused runtime generation admitted');
                setStage(`resume ${cycle}`); const resumed = await resumeCemCheckpointOwners(admission, tickets, 5000, native.bindings);
                for (const owner of resumed.owners) await owner.owner.resume(); previous?.revoke();
                if (mode === 'fallback') { heap = admission.generations[0].runtime; embedding = heap.claimOwnership(); }
                combined?.dispose(); combined = undefined;
            } else {
                const supportBefore = coordinator.inspectRecovery();
                let refused = false;
                try { coordinator.prepareResume(); } catch (error) { refused = /unavailable|unsupported/.test(String(error)); }
                check(refused, 'incomplete workspace granted a resume ticket');
                try { await coordinator.capture(); throw Error('incomplete workspace admitted capture'); }
                catch (error) { check(/cem.checkpoint.unavailable/.test(String(error)), String(error)); }
                check(coordinator.lastCommitted() === undefined, 'incomplete workspace published a partial checkpoint');
                check(JSON.stringify(coordinator.inspectRecovery()) === JSON.stringify(supportBefore), 'admission refusal changed owners or journal');
            }
            assertLogin();
            check(previews.every(({ host, mode, frames }) => host.isConnected && host.getAttribute('mode') === mode && frames.every(({ frame, document }) => host.contains(frame) && frame.contentDocument === document)), 'handoff replaced preview presentation');
            check(storageWrites === writes, 'handoff repeated a host storage write');
            check(themeValues.every(({ host, value }) => host.isConnected && host.getAttribute('value') === value), 'handoff changed theme values');
            check(root.querySelector('cem-source-editor[data-cem-studio-editor]') === editor && editor.querySelector('[role=textbox]') === control, 'handoff replaced source editor');
            check(document.activeElement === control && control.value === draft && control.selectionStart === 2 && control.selectionEnd === 7 && control.selectionDirection === 'backward', 'handoff lost focus, selection or draft');
            check(browser.location.href === url && JSON.stringify([...records]) === savedRecords && records.get('unrelated-draft') === 'preserved', 'handoff changed URL or storage');
            check(paint(editor).length > 0 && [...records.values()].some(value => value.includes(JSON.stringify(draft).slice(1, -1))), 'handoff lost paint or draft journal');
            check(styles.every(style => style.isConnected), 'handoff replaced stylesheet installation');
            setStage(`save ${cycle}`); await workbench.saveSource(); await view.whenSettled();
            const exported = (await repository.query(request('export-project', { projectId: project }))).value;
            check(new TextDecoder().decode(exported.contents[sourceId]) === draft, 'Save after handoff did not persist exact bytes');
            setStage(`build ${cycle}`);
            await workbench.buildProjectPreview(); await view.whenSettled();
            check(workbench.snapshot().projectPreview?.attempt?.status === 'succeeded', `native build after handoff failed: ${JSON.stringify(workbench.snapshot().projectPreview?.attempt)}`);
        }
        await selectUnreviewedPreview();
        check(unreviewedPreviews().length > 0 && blockers().some(owner => owner.tag === 'cem-media-preview'), 'unknown preview mode incorrectly published');
        setStage('unknown outcome refusal');
        const ownerId = coordinator.inspectRecovery().owners.find(owner => owner.tag === 'cem-studio-workbench-controls').instanceId;
        const operation = coordinator.operation(ownerId);
        check(coordinator.inspectRecovery().unknownOperations === 1, 'unknown operation was omitted');
        let refused = false; try { coordinator.prepareResume(); } catch { refused = true; }
        check(refused && coordinator.inspectRecovery().unknownOperations === 1, 'refusal consumed an unknown operation');
        operation.acknowledge();
        check(coordinator.inspectRecovery().unknownOperations === 0, 'explicit acknowledgment was lost');
        return { ...(privacy ? { privacyRefusals: 2, checkpointBytes, groupBytes } : {}), recoveries: complete ? 3 : 0, refusedRecoveries: complete ? 0 : 3, edits, unknownRefused: true, unavailable: [...new Set(blockers().map(owner => owner.tag))].sort() };
    } catch (error) { throw Error(`${mode}/${stage}: ${error.stack || error}`, { cause: error }); }
    finally {
        view?.dispose(); shell?.dispose(); workbench?.dispose(); session?.dispose(); await validator?.close(); repository.close();
        admission?.revoke(); combined?.dispose(); coordinator.dispose(); scope.dispose(); recoveryScope.dispose();
        await new Promise((resolve, reject) => { const deletion = indexedDB.deleteDatabase(databaseName); deletion.onsuccess = resolve; deletion.onerror = () => reject(deletion.error); });
    }
}


/** Focused complete rendering/preview/command handoff. Full workspace acceptance
 * remains separate; this uses ordinary Studio host ledgers without UI helpers. */
export async function combinedMediaRecovery(mode, progress) {
    // eslint-disable-next-line @nx/enforce-module-boundaries -- shared acceptance driver; not an application dependency.
    const { isolatedMediaResume } = await import('../../cem-elements/src/lib/retained-media.fixtures.js');
    const { createCemStudioBrowserValidator } = await import('./feature-tour.js');
    const source = await (await fetch(new URL('../../cem-components/src/components/cem-media-preview/cem-media-preview.xhtml', import.meta.url))).text();
    const base = new URL('../../cem-elements/dist/', import.meta.url).href;
    progress('combined media and command startup');
    // The shared driver reports progress to its isolated frame's parent.
    return isolatedMediaResume(base, url => import(/* @vite-ignore */ url), mode, source,
        recoveryScope => createCemStudioBrowserValidator({ recoveryScope }));
}
