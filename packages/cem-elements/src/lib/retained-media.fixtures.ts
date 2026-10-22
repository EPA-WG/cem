/** Isolated browser acceptance driver; never included by the runtime. */
export async function isolatedMediaResume(base: string, load: (url: string) => Promise<any>, scenario: string, source: string, createServices?: (scope: any) => Promise<any>): Promise<string> {
    const support = new URL('lib/internal/runtime-support/', base).href;
    const binding = await load(`${support}native-runtime.js`), initial = binding.isolateCemFallbackGeneration(); await initial.ready();
    const [{ CemElementRuntime }, { createCemDeclarationScope }, { CemCheckpointPublicationCoordinator }, { resumeCemCheckpointOwners }, { CemNativeRecoveryScope }, artifacts] = await Promise.all([
        load(new URL('lib/cem-elements.js', base).href), load(new URL('lib/declaration-scope.js', base).href),
        load(new URL('lib/checkpoint-publication.js', base).href), load(new URL('lib/checkpoint-resume.js', base).href), load(`${support}native-generation.js`),
        load(new URL('lib/artifact-preview.js', base).href),
    ]);
    const check = (value: unknown, message: string) => { if (!value) throw Error(message); };
    const fallback = scenario.startsWith('fallback'), scope = createCemDeclarationScope({ document }), recoveryScope = new CemNativeRecoveryScope();
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1, policyStamp: 'media',
        allowOwner: () => true, seal: (candidate: string, receipt: string) => binding.sealCommittedCheckpoint(candidate, receipt) });
    const runtime = new CemElementRuntime({ declarationScope: scope, checkpointPublication: coordinator, recoveryScope,
        declarationTag: `media-declaration-${crypto.randomUUID()}`, processingWorkerFactory: () => {
            if (fallback) throw Error('owned fallback'); return new Worker(`${support}processing-worker.js`, { type: 'module' });
        } });
    runtime.install(window);
    const services = await createServices?.(recoveryScope);
    let commandRuns = 0;
    const runCommand = async () => {
        if (!services) return;
        const output = await services.parseResource({ bytes: new TextEncoder().encode('<root>Original é😀</root>'),
            uri: 'studio://media/input.xml', resourceScheme: 'studio', contentType: 'application/xml', schema: 'https://cem.dev/ns/data/xml/1', projection: 'ast' });
        check(output.output?.text.includes('Original é😀'), 'fresh command lost original host ledger/output'); commandRuns++;
        const graph = services.authorGraph({ source: '{run | {import @id=data @src=data.xml | {export @id=result @out=result.xml}}}' });
        check(graph.nodes?.length > 0, 'fresh synchronous graph authoring failed');
    };
    const tree = artifacts.registerCemArtifactTree([{ path: 'index.html', contentType: 'text/html', bytes: new TextEncoder().encode('<input value="Artifact"><a href="#section">Next</a><h2 id="section">Section</h2>') }]);
    let heap = initial, embedding = initial.claimOwnership(), admission: any, combined: any, stage = 'mount';
    const progress = setInterval(() => parent.postMessage({ progress: stage }, '*'), 250);
    const capture = async (c: any) => { for (let n = 0; ; n++) { try { await c.capture(); return; } catch (error) {
        if (n >= 100) throw Error(`${String(error)} ${JSON.stringify(c.inspectRecovery())}`, { cause: error }); await new Promise(resolve => setTimeout(resolve, 10));
    } } };
    try {
        const declaration = document.createElement('div');
        // Register authored XHTML through the ordinary declaration boundary.
        declaration.setAttribute('tag', 'cem-media-preview'); declaration.setAttribute('src', 'studio://media/preview.xhtml#cem-media-preview');
        const parsed = new DOMParser().parseFromString(source, 'application/xhtml+xml');
        declaration.removeAttribute('src'); declaration.append(document.importNode(parsed.querySelector('template')!, true));
        document.body.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
        const live = document.createElement('cem-media-preview'), artifact = document.createElement('cem-media-preview');
        live.setAttribute('mode', 'live'); live.setAttribute('source-uri', 'studio://media/view.cem');
        live.setAttribute('value', '{input @value="Original é😀"}{preview-note}');
        live.setAttribute('resource-set', JSON.stringify({ resources: [{ uri: 'studio://media/note.xhtml', contentType: 'application/xhtml+xml', text: '<cem-element xmlns="http://www.w3.org/1999/xhtml" tag="preview-note"><template id="preview-note" type="text/cem-ml">{p | Original resource map}</template></cem-element>' }], declarations: [{ tag: 'preview-note', uri: 'studio://media/note.xhtml#preview-note' }] }));
        artifact.setAttribute('mode', 'artifact-tree'); artifact.setAttribute('resource-set', tree.resourceSet); artifact.setAttribute('artifact-path', 'index.html');
        document.body.append(live, artifact); await runtime.whenRenderSettled(live); await runtime.whenRenderSettled(artifact);
        for (let n = 0; ; n++) {
            if (live.textContent!.includes('Live view ready') && artifact.textContent!.includes('Artifact view ready')) break;
            if (n > 200) throw Error(`not ready: ${live.textContent} ${artifact.textContent}`); await new Promise(resolve => setTimeout(resolve, 10));
        }
        const frame = live.querySelector('iframe')!, artifactFrame = artifact.querySelector('iframe')!, liveDocument = frame.contentDocument!, artifactDocument = artifactFrame.contentDocument!;
        const input = liveDocument.querySelector('input')!, artifactInput = artifactDocument.querySelector('input')!;
        input.value = 'Edited café 😀'; input.focus(); input.setSelectionRange(2, 7, 'backward'); artifactInput.value = 'Edited artifact';
        const child = coordinator.previewCoordinators()[0]; check(child, 'preview coordinator missing');
        if (fallback) {
            let refused = false; try { await coordinator.capture(); } catch { refused = true; } check(refused, 'partial parent fallback coverage admitted');
        }
        await runCommand();
        let navigations = 0; artifact.addEventListener('cem-preview-navigate', () => { navigations++; });
        for (let cycle = 0; cycle < (scenario.endsWith('refuse') || scenario.endsWith('release') || scenario.endsWith('frame') ? 1 : 2); cycle++) {
            stage = `capture ${cycle}`;
            if (fallback) for (let n = 0; ; n++) {
                try { combined = CemCheckpointPublicationCoordinator.combine([coordinator, child]); break; }
                catch (error) { if (n >= 100) throw error; await new Promise(resolve => setTimeout(resolve, 10)); }
            }
            const coordinators = fallback ? [combined] : [coordinator, child];
            for (const c of coordinators) await capture(c);
            const consents = coordinators.map(() => fallback ? heap.retirementConsent([embedding]) : undefined);
            const proofs = []; for (let i = 0; i < coordinators.length; i++) proofs.push(await coordinators[i].closeRenderingGeneration(5000, consents[i]));
            for (const c of coordinators) await capture(c);
            const tickets = coordinators.map(c => c.prepareResume()), native = services?.prepareRecovery(), plan = await recoveryScope.prepare(), group = plan.createGenerationGroup();
            const commands = native ? [native.invocation, native.commands] : [], commandOwners = commands.map(client => client.claimNativeEmbedding());
            const commandProofs = []; for (let i = 0; i < commands.length; i++) commandProofs.push(await commands[i].closeNativeAdmission([commandOwners[i]]));
            const retired = []; for (let i = 0; i < coordinators.length; i++) retired.push(await coordinators[i].retireRenderingGeneration(proofs[i], 5000, consents[i]));
            for (let i = 0; i < commands.length; i++) retired.push(await commands[i].retireNativeGeneration(commandProofs[i], [commandOwners[i]], {
                current: plan.current, retain: () => new TextEncoder().encode('terminal command ledger; no outstanding handles or effects'),
            }));
            stage = `allocate ${cycle}`; const previous = admission; admission = await group.allocate(retired);
            if (scenario.endsWith('release')) tree.dispose();
            if (scenario.endsWith('frame')) artifactFrame.srcdoc = '<p>Changed document</p>';
            if (scenario.endsWith('refuse')) live.setAttribute('resource-set', '{"resources":[]}');
            if (scenario.endsWith('refuse') || scenario.endsWith('release') || scenario.endsWith('frame')) {
                let refused = false; try { await resumeCemCheckpointOwners(admission, tickets, 5000, native?.bindings ?? []); } catch { refused = true; }
                check(refused, 'stale preview controller admitted'); return `${scenario} media passed`;
            }
            stage = `resume ${cycle}`;
            const resumed = await resumeCemCheckpointOwners(admission, tickets, 5000, native?.bindings ?? []);
            for (const owner of resumed.owners) await owner.owner.resume(); previous?.revoke();
            check(live.querySelector('iframe') === frame && artifact.querySelector('iframe') === artifactFrame, 'preview frame replaced');
            check(frame.contentDocument === liveDocument && artifactFrame.contentDocument === artifactDocument, 'preview document remounted');
            check(liveDocument.querySelector('input') === input && input.value === 'Edited café 😀' && input.selectionStart === 2 && input.selectionEnd === 7
                && input.selectionDirection === 'backward' && liveDocument.activeElement === input, 'live edits/focus/selection lost');
            check(artifactInput.value === 'Edited artifact' && navigations === 0, 'artifact edited state lost or navigation replayed');
            check(liveDocument.querySelector('p')!.textContent === 'Original resource map', 'original resource map lost');
            await runCommand();
            if (fallback) { heap = admission.generations[0].runtime; embedding = heap.claimOwnership(); combined.dispose(); combined = undefined; }
        }
        stage = 'fresh artifact callback';
        artifactDocument.querySelector('a')!.click(); await runtime.whenRenderSettled(artifact);
        check(navigations === 1 && artifact.textContent!.includes('Artifact view ready') && artifactFrame.contentDocument === artifactDocument, 'fresh callback lost or remounted document');
        check(!services || commandRuns === 3, 'command work replayed during handoff');
        return `${scenario} media passed`;
    } catch (error) { throw Error(`${scenario}/${stage}: ${error instanceof Error ? error.stack : String(error)}`, { cause: error }); }
    finally { clearInterval(progress); await services?.close(); admission?.revoke(); combined?.dispose(); document.body.replaceChildren(); scope.dispose(); coordinator.dispose(); tree.dispose(); recoveryScope.dispose(); }
}
