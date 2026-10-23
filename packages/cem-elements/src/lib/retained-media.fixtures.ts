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
    const eventTrace: string[] = [];
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
    const nativeSrcdoc = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'srcdoc')!;
    let previousLoad: Promise<void> | undefined, releaseNavigation: (() => void) | undefined;
    if (scenario.endsWith('-load')) Object.defineProperty(HTMLIFrameElement.prototype, 'srcdoc', { ...nativeSrcdoc,
        set(this: HTMLIFrameElement, value: string) {
            if (!previousLoad && this.getAttribute('part')?.split(' ').includes('live-document')) {
                previousLoad = new Promise<void>(resolve => {
                    const loaded = () => {
                        if (this.contentDocument?.querySelector('meta[name="cem-live-preview-owner"]')?.getAttribute('content') !== 'previous') return;
                        this.removeEventListener('load', loaded); resolve();
                    };
                    this.addEventListener('load', loaded);
                });
                releaseNavigation = () => nativeSrcdoc.set!.call(this, value);
                nativeSrcdoc.set!.call(this, '<!doctype html><meta name="cem-live-preview-owner" content="previous"><body>Prior document</body>');
            } else nativeSrcdoc.set!.call(this, value);
        },
    });
    const heldSelect: (() => void)[] = [];
    let selectReleased = false, selectObserved: (() => void) | undefined;
    const selectReady = new Promise<void>(resolve => { selectObserved = resolve; });
    const restoreSelect: (() => void)[] = [];
    if (scenario.includes('-select')) Object.defineProperty(HTMLIFrameElement.prototype, 'srcdoc', { ...nativeSrcdoc,
        set(this: HTMLIFrameElement, value: string) {
            if (this.getAttribute('part')?.split(' ').includes('live-document')) {
                const prepare = () => {
                    const view = this.contentWindow;
                    if (!view || !this.contentDocument?.querySelector('meta[name="cem-live-preview-owner"]')) return;
                    this.removeEventListener('load', prepare, true);
                    const prototype = (view as any).EventTarget.prototype, add = prototype.addEventListener, remove = prototype.removeEventListener;
                    const wrappers = new WeakMap<object, EventListener>();
                    prototype.addEventListener = function(type: string, listener: EventListener, options: any) {
                        if (type === 'select' && options === true && listener?.name === 'input' && this.localName === 'cem-live-view') {
                            const wrapped = (event: Event) => {
                                if (selectReleased || !event.isTrusted) listener.call(this, event);
                                else { heldSelect.push(() => listener.call(this, event)); selectObserved?.(); }
                            };
                            wrappers.set(listener, wrapped); return add.call(this, type, wrapped, options);
                        }
                        return add.call(this, type, listener, options);
                    };
                    prototype.removeEventListener = function(type: string, listener: EventListener, options: any) {
                        return remove.call(this, type, type === 'select' ? wrappers.get(listener) ?? listener : listener, options);
                    };
                    restoreSelect.push(() => { prototype.addEventListener = add; prototype.removeEventListener = remove; });
                };
                this.addEventListener('load', prepare, true);
            }
            nativeSrcdoc.set!.call(this, value);
        },
    });
    const capture = async (c: any) => { for (let n = 0; ; n++) { try { await c.capture(); return; } catch (error) {
        if (n >= 100) throw Error(`${String(error)} ${JSON.stringify(c.inspectRecovery())} publication=${JSON.stringify([...document.querySelectorAll('cem-media-preview')].map(host => ({ mode: host.getAttribute('mode'), token: runtime.renderTokens.get(host), published: runtime.checkpointPublishedTokens.get(host), revision: runtime.dataRevisions.get(host), queued: runtime.queuedRenders.has(host), processing: runtime.processingRenderJobs.has(host), interacting: runtime.checkpointInteracting.has(host), frameChildren: host.querySelector('iframe')?.contentDocument?.body.childElementCount, diagnostics: runtime.diagnosticsFor(host) })))}`, { cause: error }); await new Promise(resolve => setTimeout(resolve, 10));
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
        if (scenario.includes('http')) {
            live.setAttribute('value', '{http-request @slice=data @url="data.json"}{input @value="Original é😀"}{p | {$datadom.slices.data.state}}');
            live.setAttribute('resource-set', JSON.stringify({ resources: [{ uri: 'studio://media/data.json', contentType: 'application/json', text: '{"original":true}' }] }));
        }
        artifact.setAttribute('mode', 'artifact-tree'); artifact.setAttribute('resource-set', tree.resourceSet); artifact.setAttribute('artifact-path', 'index.html');
        document.body.append(live, artifact); await runtime.whenRenderSettled(live); await runtime.whenRenderSettled(artifact);
        if (scenario.endsWith('-load')) {
            check(previousLoad && releaseNavigation, 'live srcdoc navigation was not intercepted');
            await previousLoad; await Promise.resolve(); await Promise.resolve();
            const previewMembers = recoveryScope.inspect().members.filter((member: any) => member.kind === 'live-preview');
            check(previewMembers.length === 1 && previewMembers[0].id.startsWith('cem-preview-loading:'), 'prior document initialized preview owners');
            check(!live.querySelector('iframe')!.contentDocument!.querySelector('cem-element'), 'prior document received CEM declarations');
            releaseNavigation!();
        }
        for (let n = 0; ; n++) {
            if (live.textContent!.includes('Live view ready') && artifact.textContent!.includes('Artifact view ready')) break;
            if (n > 200) throw Error(`not ready: ${live.textContent} ${artifact.textContent}`); await new Promise(resolve => setTimeout(resolve, 10));
        }
        const frame = live.querySelector('iframe')!, artifactFrame = artifact.querySelector('iframe')!, liveDocument = frame.contentDocument!, artifactDocument = artifactFrame.contentDocument!;
        for (const type of ['select', 'focusin', 'focusout', 'input', 'change']) liveDocument.addEventListener(type, event => eventTrace.push(`${stage}:event:${event.type}:${(event.target as Element).localName}:${event.isTrusted}`), true);
        const input = liveDocument.querySelector('input')!, artifactInput = artifactDocument.querySelector('input')!;
        input.value = 'Edited café 😀'; input.focus(); input.setSelectionRange(2, 7, 'backward'); artifactInput.value = 'Edited artifact';
        const statics: { host: HTMLElement; root: Element; frame?: HTMLIFrameElement; control?: HTMLInputElement; audio?: HTMLAudioElement; loads(): number }[] = [];
        if (scenario.includes('static')) for (const mode of ['text', 'document', 'media', 'message']) {
            const host = document.createElement('cem-media-preview'); host.setAttribute('mode', mode);
            host.setAttribute('value', mode === 'document' ? '<input value="Opaque document">' : 'Static café 😀\n'.repeat(mode === 'text' ? 100 : 1));
            if (mode === 'media') {
                const control = document.createElement('input'); control.value = 'Projected'; host.append(control);
                const audio = document.createElement('audio'), bytes = new Uint8Array(8044), view = new DataView(bytes.buffer);
                for (const [offset, value] of [[0, 'RIFF'], [8, 'WAVE'], [12, 'fmt '], [36, 'data']] as const)
                    for (let n = 0; n < value.length; n++) bytes[offset + n] = value.charCodeAt(n);
                view.setUint32(4, 8036, true); view.setUint32(16, 16, true); view.setUint16(20, 1, true); view.setUint16(22, 1, true);
                view.setUint32(24, 8000, true); view.setUint32(28, 8000, true); view.setUint16(32, 1, true); view.setUint16(34, 8, true); view.setUint32(40, 8000, true);
                bytes.fill(128, 44); audio.src = `data:audio/wav;base64,${btoa(String.fromCharCode(...bytes))}`; audio.preload = 'auto'; host.append(audio);
            }
            document.body.append(host); await runtime.whenRenderSettled(host);
            const root = host.querySelector('[part~=root]')!, frame = host.querySelector('iframe') ?? undefined;
            // The sandbox stays opaque; test code must not widen permissions.
            if (frame) { frame.loading = 'eager'; await new Promise<void>(resolve => frame.addEventListener('load', () => resolve(), { once: true })); check(frame.contentDocument === null, 'static sandbox gained document access'); }
            const control = host.querySelector('input') ?? undefined; if (control) control.value = 'Edited projected é😀';
            const audio = host.querySelector('audio') ?? undefined;
            if (audio) {
                if (audio.readyState < 1) await new Promise<void>((resolve, reject) => { audio.addEventListener('loadedmetadata', () => resolve(), { once: true }); audio.addEventListener('error', () => reject(Error('fixture media failed')), { once: true }); });
                const seeked = new Promise<void>(resolve => audio.addEventListener('seeked', () => resolve(), { once: true }));
                audio.currentTime = 0.25; audio.volume = 0.3; await seeked;
            }
            let loads = 0; frame?.addEventListener('load', () => { loads++; });
            statics.push({ host, root, frame, control, audio, loads: () => loads });
        }
        const text = statics.find(value => value.host.getAttribute('mode') === 'text')?.host.querySelector('pre');
        const textNode = text ? [...text.childNodes].find(node => node.nodeType === 3 && node.nodeValue!.length >= 7)! : undefined;
        if (text) { text.style.maxHeight = '20px'; text.scrollTop = 30; window.getSelection()!.setBaseAndExtent(textNode!, 2, textNode!, 7); }
        if (scenario.includes('-select')) await selectReady;
        const child = coordinator.previewCoordinators()[0]; check(child, 'preview coordinator missing');
        if (fallback) {
            let refused = false; try { await coordinator.capture(); } catch { refused = true; } check(refused, 'partial parent fallback coverage admitted');
        }
        await runCommand();
        let navigations = 0; artifact.addEventListener('cem-preview-navigate', () => { navigations++; });
        for (let cycle = 0; cycle < (scenario.endsWith('refuse') || scenario.endsWith('release') || scenario.endsWith('frame') || scenario.endsWith('static-change') ? 1 : 2); cycle++) {
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
            stage = `allocate ${cycle}`; const previous = admission;
            const allocating = group.allocate(retired);
            if (scenario.includes('-select') && cycle === 0) {
                check(heldSelect.length > 0, 'trusted native select notification was not intercepted');
                selectReleased = true;
                if (scenario.endsWith('-select-synthetic')) input.dispatchEvent(new Event('select', { bubbles: true }));
                if (scenario.endsWith('-select-change')) input.setSelectionRange(0, 1, 'forward');
                for (const notify of heldSelect.splice(0)) notify();
                if (scenario.endsWith('-select-change') || scenario.endsWith('-select-synthetic')) {
                    let refused = false; try { await allocating; } catch (error) { refused = /enrollment-stale/.test(String(error)); }
                    check(refused && input.value === 'Edited café 😀' && input.selectionStart === (scenario.endsWith('-select-change') ? 0 : 2) && input.selectionEnd === (scenario.endsWith('-select-change') ? 1 : 7), 'changed or synthetic selection admitted or lost');
                    return `${scenario} media passed`;
                }
            }
            admission = await allocating;
            if (scenario.endsWith('release')) tree.dispose();
            if (scenario === 'worker-frame') artifactFrame.srcdoc = '<p>Changed document</p>';
            if (scenario.endsWith('static-change')) statics[0].root.append(document.createTextNode('changed'));
            if (scenario.endsWith('static-frame')) statics[1].frame!.srcdoc = '<p>Changed opaque document</p>';
            if (scenario.endsWith('refuse')) live.setAttribute('resource-set', '{"resources":[]}');
            if (scenario.endsWith('refuse') || scenario.endsWith('release') || scenario.endsWith('frame') || scenario.endsWith('static-change')) {
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
            check(liveDocument.querySelector('p')!.textContent === (scenario.includes('http') ? 'loaded' : 'Original resource map'), 'original resource map lost');
            if (statics.length) await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
            for (const value of statics) {
                check(value.host.querySelector('[part~=root]') === value.root, 'static presentation replaced');
                if (value.frame) check(value.host.querySelector('iframe') === value.frame && value.frame.contentDocument === null && value.frame.getAttribute('sandbox') === '' && value.loads() === 0, 'opaque document reloaded/replaced or permissions widened');
                if (value.audio) check(value.host.querySelector('audio') === value.audio && value.audio.paused && value.audio.currentTime === 0.25 && value.audio.volume === 0.3, 'browser media state lost or replayed');
                if (value.control) check(value.host.querySelector('input') === value.control && value.control.value === 'Edited projected é😀', 'projected control state lost');
            }
            if (text) check(text.scrollTop === 30 && window.getSelection()!.anchorNode === textNode
                && window.getSelection()!.anchorOffset === 2 && window.getSelection()!.focusOffset === 7, 'static text scroll/selection lost');
            await runCommand();
            if (fallback) { heap = admission.generations[0].runtime; embedding = heap.claimOwnership(); combined.dispose(); combined = undefined; }
        }
        stage = 'fresh artifact callback';
        artifactDocument.querySelector('a')!.click(); await runtime.whenRenderSettled(artifact);
        check(navigations === 1 && artifact.textContent!.includes('Artifact view ready') && artifactFrame.contentDocument === artifactDocument, 'fresh callback lost or remounted document');
        check(!services || commandRuns === 3, 'command work replayed during handoff');
        return `${scenario} media passed`;
    } catch (error) { throw Error(`${scenario}/${stage}: ${error instanceof Error ? error.stack : String(error)} events=${JSON.stringify(eventTrace.slice(-16))} peers=${JSON.stringify([coordinator, ...coordinator.previewCoordinators()].map(c => c.inspectRecovery()))}`, { cause: error }); }
    finally { Object.defineProperty(HTMLIFrameElement.prototype, 'srcdoc', nativeSrcdoc); clearInterval(progress); await services?.close(); admission?.revoke(); combined?.dispose(); document.body.replaceChildren(); scope.dispose(); coordinator.dispose(); tree.dispose(); recoveryScope.dispose(); for (const restore of restoreSelect) restore(); }
}
