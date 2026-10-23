/** Isolated browser acceptance; no production imports or application behavior. */
export async function isolatedOwnerReplacement(base: string, load: (url: string) => Promise<any>, scenario: string): Promise<string> {
    const support = new URL('lib/internal/runtime-support/', base).href;
    const binding = await load(`${support}native-runtime.js`), initial = binding.isolateCemFallbackGeneration();
    await initial.ready();
    const [{ CemElementRuntime }, { createCemDeclarationScope }, { CemCheckpointPublicationCoordinator }, { resumeCemCheckpointOwners }, { CemNativeGenerationGroup }] = await Promise.all([
        load(new URL('lib/cem-elements.js', base).href), load(new URL('lib/declaration-scope.js', base).href),
        load(new URL('lib/checkpoint-publication.js', base).href), load(new URL('lib/checkpoint-resume.js', base).href), load(`${support}native-generation.js`),
    ]);
    const fallback = scenario === 'fallback', peers: any[] = [];
    let heap = initial, embedding = initial.claimOwnership(), admission: any, combined: any, stage = 'mount', initialSeal = true;
    const progress = setInterval(() => parent.postMessage({ progress: `${scenario}/${stage}` }, '*'), 250);
    const check = (value: unknown, message: string) => { if (!value) throw Error(message); };
    const capture = async (coordinator: any) => {
        for (let n = 0; ; n++) try { await coordinator.capture(); return; }
        catch (error) { if (n >= 100) throw error; await new Promise(resolve => setTimeout(resolve, 10)); }
    };
    try {
        for (let i = 0; i < (fallback ? 2 : 1); i++) {
            const scope = createCemDeclarationScope({ document });
            const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
                policyStamp: 'replacement', allowOwner: () => true,
                seal: (candidate: string, receipt: string) => { check(initialSeal, 'original sealing route reused after retirement'); return binding.sealCommittedCheckpoint(candidate, receipt); } });
            const runtime = new CemElementRuntime({ declarationScope: scope, checkpointPublication: coordinator,
                declarationTag: `replacement-declaration-${crypto.randomUUID()}`, processingWorkerFactory: () => {
                    if (fallback) throw Error('owned fallback'); return new Worker(`${support}processing-worker.js`, { type: 'module' });
                } }); runtime.install(window);
            const declaration = document.createElement('div'), template = document.createElement('template'), tag = `replacement-${crypto.randomUUID()}`;
            declaration.setAttribute('tag', tag); template.setAttribute('type', 'text/cem-ml'); template.textContent = '{input @value="Original"}';
            declaration.append(template); document.head.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
            const element = document.createElement(tag); document.body.append(element); await runtime.whenRenderSettled(element);
            peers.push({ scope, coordinator, runtime, element, tag });
        }
        for (let cycle = 0; cycle < 3; cycle++) {
            stage = `capture ${cycle}`;
            if (fallback) for (let n = 0; ; n++) try { combined = CemCheckpointPublicationCoordinator.combine(peers.map(peer => peer.coordinator)); break; }
            catch (error) { if (n >= 100) throw error; await new Promise(resolve => setTimeout(resolve, 10)); }
            const coordinator = combined ?? peers[0].coordinator;
            await capture(coordinator);
            const group = new CemNativeGenerationGroup([coordinator.generationParticipant()]);
            const consent = fallback ? heap.retirementConsent([embedding]) : undefined;
            const proof = await coordinator.closeRenderingGeneration(5000, consent); await capture(coordinator);
            const ticket = coordinator.prepareResume(), retired = await coordinator.retireRenderingGeneration(proof, 5000, consent);
            stage = `allocate ${cycle}`; const previous = admission; admission = await group.allocate([retired]);
            const resumed = await resumeCemCheckpointOwners(admission, [ticket]);
            for (const owner of resumed.owners) await owner.owner.resume(); previous?.revoke(); initialSeal = false;
            for (const peer of peers) check(peer.element.querySelector('input')?.value === (cycle ? `Replacement ${cycle - 1} é😀` : 'Original'), 'replacement state lost');
            if (fallback) { heap = admission.generations[0].runtime; embedding = heap.claimOwnership(); combined.dispose(); combined = undefined; }
            stage = `replace all owners ${cycle}`;
            const previousIds = peers.flatMap(peer => peer.coordinator.inspectRecovery().owners.map((owner: any) => owner.instanceId));
            for (const peer of peers) {
                const next = document.createElement(peer.tag); next.setAttribute('value', `Replacement ${cycle} é😀`);
                peer.element.replaceWith(next); peer.element = next; await peer.runtime.whenRenderSettled(next);
                const input = next.querySelector('input')!; input.value = `Replacement ${cycle} é😀`; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await peer.runtime.whenRenderSettled(next);
            }
            await Promise.resolve();
            for (const peer of peers) {
                const owners = peer.coordinator.inspectRecovery().owners;
                check(owners.length === 1 && owners.every((owner: any) => !previousIds.includes(owner.instanceId)), 'resumed owner survived replacement');
                if (!fallback) await capture(peer.coordinator);
            }
        }
        stage = 'final replacement seal';
        if (fallback) {
            combined = CemCheckpointPublicationCoordinator.combine(peers.map(peer => peer.coordinator));
            await capture(combined); combined.dispose(); combined = undefined;
        } else await capture(peers[0].coordinator);
        stage = 'dispose';
        for (const peer of peers) { peer.element.remove(); peer.scope.dispose(); peer.coordinator.dispose(); }
        admission.revoke();
        for (const peer of peers) {
            check(peer.coordinator.current() === undefined && peer.coordinator.inspectRecovery().owners.length === 0, 'disposed scope retained checkpoint owners');
            let refused = false; try { await peer.coordinator.capture(); } catch { refused = true; }
            check(refused, 'disposed route admitted capture');
        }
        return `${scenario} media passed`;
    } catch (error) { throw Error(`${scenario}/${stage}: ${String(error)}`, { cause: error }); }
    finally { clearInterval(progress); combined?.dispose(); document.body.replaceChildren(); for (const peer of peers) { peer.scope.dispose(); peer.coordinator.dispose(); } admission?.revoke(); }
}
