/** Isolated acceptance driver, never imported by the application. */
export async function studioRecovery(mode, progress) {
    const check = (condition, message) => { if (!condition) throw Error(message); };
    const elements = new URL('.', new URL('../../cem-elements/dist/index.js', import.meta.url));
    const support = new URL('lib/internal/runtime-support/', elements);
    const binding = await import(/* @vite-ignore */ new URL('native-runtime.js', support).href);
    const initial = binding.isolateCemFallbackGeneration(); await initial.ready();
    initial.claimOwnership();
    const { CemElementRuntime, createCemDeclarationScope, CemCheckpointPublicationCoordinator, CemNativeRecoveryScope } = await import(/* @vite-ignore */ elements.href + 'index.js');
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
        policyStamp: 'studio-acceptance', allowOwner: () => true,
        seal: (candidate, receipt) => binding.sealCommittedCheckpoint(candidate, receipt) });
    const recoveryScope = new CemNativeRecoveryScope();
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ declarationScope: scope, declarationTag: 'cem-studio-component-declaration', checkpointPublication: coordinator, recoveryScope,
        processingWorkerFactory: () => { if (mode === 'fallback') throw Error('owned fallback acceptance');
            return new Worker(new URL('processing-worker.js', support), { type: 'module' }); } });
    const map = document.createElement('script'); map.type = 'importmap'; map.textContent = JSON.stringify({ imports: {
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
    const storage = { getItem: key => records.get(key) ?? null, setItem: (key, value) => records.set(key, value) };
    const databaseName = `cem-studio-recovery-${crypto.randomUUID()}`;
    const validator = await createCemStudioBrowserValidator({ recoveryScope });
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
        await workbench.openEntity(`project=${project}&file=login-html&build=render-graph`); await view.whenSettled();
        const original = workbench.snapshot().draft;
        let edits = 0;
        const paint = editor => [...CSS.highlights.values()].flatMap(value => [...value]).filter(range => editor.contains(range.startContainer));
        const blockers = () => coordinator.inspectRecovery().owners.filter(owner => !owner.adapterAvailable);
        const themes = coordinator.inspectRecovery().owners.filter(owner => owner.tag === 'cem-theme-switch');
        check(themes.length > 0 && themes.every(owner => owner.adapterAvailable), 'shared theme storage adapter missing');
        const retained = coordinator.inspectRecovery().owners.filter(owner => ['cem-action', 'cem-expansion', 'cem-list'].includes(owner.tag));
        check(retained.some(owner => owner.tag === 'cem-action') && retained.every(owner => owner.adapterAvailable), 'shared retained capability adapter missing');
        // The complete theme composition is supported. Mount a real preview so
        // this refusal lane proves an actual unsupported capability.
        setStage('initial native preview'); await workbench.buildProjectPreview(); await view.whenSettled();
        check(workbench.snapshot().projectPreview?.attempt?.status === 'succeeded', 'initial recovery preview failed');
        check(blockers().some(owner => owner.tag === 'cem-media-preview'), 'preview refusal lane has no preview owner');
        const enrolledKinds = () => recoveryScope.inspect().members.map(member => member.kind);
        check(enrolledKinds().includes('rendering') && enrolledKinds().includes('command-service') && enrolledKinds().includes('command-invocation'), 'Studio omitted an active runtime from recovery membership');
        let groupRefused = false;
        try { await recoveryScope.prepare(); } catch (error) { groupRefused = /unsupported|unavailable/.test(String(error)); }
        check(groupRefused, 'incomplete Studio runtime inventory prepared a partial group');

        for (let cycle = 0; cycle < 3; cycle++) {
            for (let edit = 0; edit < 4; edit++) {
                setStage(`edit ${cycle}/${edit}`);
                const control = root.querySelector('cem-source-editor[data-cem-studio-editor] [role=textbox]');
                control.focus(); control.value = `${original}\n<!-- recovery ${cycle}/${edit} é😀 -->\n`;
                control.setSelectionRange(2, 7, 'backward'); control.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await view.whenSettled(); edits++;
                check(workbench.snapshot().dirty && workbench.snapshot().draft === control.value, 'editor draft not accepted');
            }
            const colored = root.querySelector('cem-source-editor[data-cem-studio-editor]');
            for (let attempt = 0; !paint(colored).length; attempt++) {
                check(attempt < 100, 'editor coloring did not settle'); await new Promise(resolve => setTimeout(resolve, 10));
            }
            setStage(`refusal ${cycle}`);
            const draft = workbench.snapshot().draft;
            const editor = root.querySelector('cem-source-editor[data-cem-studio-editor]'), control = editor.querySelector('[role=textbox]');
            const url = browser.location.href, savedRecords = JSON.stringify([...records]), styles = [...document.querySelectorAll('style')];
            const supportBefore = coordinator.inspectRecovery();
            let refused = false;
            try { coordinator.prepareResume(); } catch (error) { refused = /unavailable|unsupported/.test(String(error)); }
            check(refused, 'incomplete workspace granted a resume ticket');
            try { await coordinator.capture(); throw Error('incomplete workspace admitted capture'); }
            catch (error) { check(/cem.checkpoint.unavailable/.test(String(error)), String(error)); }
            check(coordinator.lastCommitted() === undefined, 'incomplete workspace published a partial checkpoint');
            check(JSON.stringify(coordinator.inspectRecovery()) === JSON.stringify(supportBefore), 'admission refusal changed owners or journal');
            check(root.querySelector('cem-source-editor[data-cem-studio-editor]') === editor && editor.querySelector('[role=textbox]') === control, 'refusal replaced source editor');
            check(document.activeElement === control && control.value === draft && control.selectionStart === 2 && control.selectionEnd === 7, 'refusal lost focus, selection or draft');
            check(browser.location.href === url && JSON.stringify([...records]) === savedRecords && records.get('unrelated-draft') === 'preserved', 'refusal changed URL or storage');
            check(paint(editor).length > 0 && [...records.values()].some(value => value.includes(JSON.stringify(draft).slice(1, -1))), 'refusal lost paint or draft journal');
            check(styles.every(style => style.isConnected), 'refusal replaced stylesheet installation');
            setStage(`save ${cycle}`); await workbench.saveSource(); await view.whenSettled();
            const exported = (await repository.query(request('export-project', { projectId: project }))).value;
            check(new TextDecoder().decode(exported.contents['login-html']) === draft, 'Save after refusal did not persist exact bytes');
            setStage(`build ${cycle}`);
            await workbench.buildProjectPreview(); await view.whenSettled();
            check(workbench.snapshot().projectPreview?.attempt?.status === 'succeeded', `native build after refusal failed: ${JSON.stringify(workbench.snapshot().projectPreview?.attempt)}`);
        }
        check(blockers().some(owner => owner.tag === 'cem-media-preview'), 'preview capability incorrectly admitted');
        setStage('unknown outcome refusal');
        const ownerId = coordinator.inspectRecovery().owners.find(owner => owner.tag === 'cem-studio-workbench-controls').instanceId;
        const operation = coordinator.operation(ownerId);
        check(coordinator.inspectRecovery().unknownOperations === 1, 'unknown operation was omitted');
        let refused = false; try { coordinator.prepareResume(); } catch { refused = true; }
        check(refused && coordinator.inspectRecovery().unknownOperations === 1, 'refusal consumed an unknown operation');
        operation.acknowledge();
        check(coordinator.inspectRecovery().unknownOperations === 0, 'explicit acknowledgment was lost');
        return { recoveries: 0, refusedRecoveries: 3, edits, unknownRefused: true, unsupported: [...new Set(blockers().map(owner => owner.tag))].sort() };
    } catch (error) { throw Error(`${mode}/${stage}: ${error.stack || error}`, { cause: error }); }
    finally {
        view?.dispose(); shell?.dispose(); workbench?.dispose(); session?.dispose(); await validator?.close(); repository.close();
        coordinator.dispose(); scope.dispose(); recoveryScope.dispose();
        await new Promise((resolve, reject) => { const deletion = indexedDB.deleteDatabase(databaseName); deletion.onsuccess = resolve; deletion.onerror = () => reject(deletion.error); });
    }
}
