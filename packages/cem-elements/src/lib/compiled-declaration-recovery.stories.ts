import type { CemNativeGenerationAdmission } from './internal/runtime-support/native-generation.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';

export default { title: 'CEM Elements/Compiled Application Recovery', tags: ['test'] } satisfies Meta;
const generated = import.meta.glob<string>('../../../../dist/artifacts/cem_ql/compiled-form/*', { eager: true, query: '?url', import: 'default' });

/** Test-only host driver in its own JS realm and physical WASM generations. */
async function recoverCompiledForm(base: string, htmlUrl: string, load: (url: string) => Promise<any>, fallback: boolean, scenario: string): Promise<string> {
    const support = new URL('lib/internal/runtime-support/', base).href;
    const binding = await load(`${support}native-runtime.js`);
    let heap = binding.isolateCemFallbackGeneration(); await heap.ready(); let embedding = heap.claimOwnership();
    const [{ CemElementRuntime }, { createCemDeclarationScope }, { CemCheckpointPublicationCoordinator }, { resumeCemCheckpointOwners }, { CemNativeGenerationGroup }] = await Promise.all([
        load(new URL('lib/cem-elements.js', base).href), load(new URL('lib/declaration-scope.js', base).href),
        load(new URL('lib/checkpoint-publication.js', base).href), load(new URL('lib/checkpoint-resume.js', base).href),
        load(`${support}native-generation.js`),
    ]);
    const check = (value: unknown, message: string) => { if (!value) throw Error(message); };
    const html = await (await fetch(htmlUrl)).text();
    const elementBase = document.createElement('base'); elementBase.href = htmlUrl; document.head.append(elementBase);
    const scope = createCemDeclarationScope({ document });
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
        policyStamp: 'compiled-form-recovery', maxBytes: 4 * 1024 * 1024, allowOwner: () => true,
        seal: (candidate: string, receipt: string) => binding.sealCommittedCheckpoint(candidate, receipt) });
    let sourceLoads = 0, workerReplies = 0;
    const kinds: string[] = [];
    const runtime = new CemElementRuntime({ declarationScope: scope, checkpointPublication: coordinator, retainedStylesheets: {},
        loadSrcDocument: async () => { sourceLoads++; throw Error('Original component source is unavailable'); },
        processingWorkerFactory: (request: { scriptUrl: string; type: WorkerType; name?: string }) => {
            if (fallback) throw Error('Owned fallback fixture');
            const worker = new Worker(request.scriptUrl, { type: request.type, name: request.name });
            const post = worker.postMessage.bind(worker);
            worker.postMessage = (message: unknown) => {
                const packet = message as { operation?: string; payload?: { language?: string; source?: { kind: string } } };
                if (packet.operation === 'compile' && packet.payload?.language === 'cem-ml') kinds.push(packet.payload.source?.kind ?? 'missing');
                post(message);
            };
            worker.addEventListener('message', ({ data }) => { if (data.direction === 'response' && data.outcome === 'success') workerReplies++; });
            return worker;
        } });
    const fetcher = globalThis.fetch, downloads: string[] = [];
    globalThis.fetch = async (input, init) => { downloads.push(String(input)); return fetcher(input, init); };
    let admission: CemNativeGenerationAdmission | undefined, stage = 'load';
    const capture = async () => {
        for (let n = 0; ; n++) {
            try { await coordinator.capture(); return; }
            catch (error) { if (n >= 100 || !/unavailable|stale/.test(String(error))) throw error; await new Promise(resolve => setTimeout(resolve, 10)); }
        }
    };
    try {
        document.body.innerHTML = html; runtime.install(window);
        await Promise.all([...document.querySelectorAll('cem-element')].map(element => runtime.whenDeclarationSettled(element)));
        const app = document.createElement('cem-compiled-form-app'); document.body.append(app); await runtime.whenRenderSettled(app);
        const card = app.querySelector('cem-card'); check(card, 'missing compiled card'); await runtime.whenRenderSettled(card);
        for (const element of app.querySelectorAll('cem-text-field, cem-action, cem-text, cem-alert')) await runtime.whenRenderSettled(element);
        const field = app.querySelector('cem-text-field') as HTMLElement, form = app.querySelector('form');
        const input = field?.querySelector('input'); check(input && form, 'missing compiled form control');
        if (!input || !form) throw Error('Missing compiled form');
        const value = async (text: string) => { input.value = text; input.dispatchEvent(new InputEvent('input', { bubbles: true })); await runtime.whenRenderSettled(field); };
        await capture(); check(coordinator.current().receipt.owners.length === 6, 'incomplete application checkpoint');
        if (scenario === 'success') {
            stage = 'credential privacy';
            for (const [type, autocomplete] of [['password', 'off'], ['text', 'one-time-code']]) {
                stage = `empty ${type} credential`;
                await value(''); field.setAttribute('value', ''); field.setAttribute('type', type); field.setAttribute('autocomplete', autocomplete); await runtime.whenRenderSettled(field);
                await capture();
                const safe = new Uint8Array(coordinator.current().bytes);
                const secret = `private-${crypto.randomUUID()}`; input.value = secret;
                const privacy = async () => {
                    let refused = false; try { await coordinator.capture(); } catch (error) { refused = /unavailable/.test(String(error)); }
                    const held = coordinator.lastCommitted()?.bytes;
                    check(refused && (!held || held.length === safe.length && held.every((byte: number, index: number) => byte === safe[index]))
                        && input.value === secret, 'credential was exported or live input was erased');
                    refused = false; try { coordinator.prepareResume(); } catch (error) { refused = /unavailable|unsupported/.test(String(error)); }
                    check(refused, 'entered credential granted a recovery ticket');
                };
                await privacy();
                input.dispatchEvent(new InputEvent('input', { bubbles: true })); await runtime.whenRenderSettled(field);
                await privacy();
                stage = `cleared ${type} credential`;
                await value(''); await capture();
            }
            field.setAttribute('type', 'text'); field.removeAttribute('autocomplete'); await runtime.whenRenderSettled(field);
        }
        await value('Unsaved compiled draft é😀'); input.focus(); input.setSelectionRange(2, 9, 'backward'); await capture();
        const styles = [...document.querySelectorAll('style')], acquired = downloads.length;
        const declaration = document.querySelector('cem-element[tag="cem-compiled-form-app"]'); check(declaration, 'missing pinned declaration');
        for (let cycle = 0; cycle < (scenario === 'success' ? 2 : 1); cycle++) {
            stage = `close ${cycle}`;
            const group = new CemNativeGenerationGroup([coordinator.generationParticipant()]);
            const consent = fallback ? heap.retirementConsent([embedding]) : undefined;
            const proof = await coordinator.closeRenderingGeneration(5000, consent); await capture();
            const before = coordinator.current(), ticket = coordinator.prepareResume();
            check(before.bytes.byteLength <= 4 * 1024 * 1024, 'application exceeded checkpoint budget');
            const replies = workerReplies;
            stage = `retire ${cycle}`; const retired = await coordinator.retireRenderingGeneration(proof, 5000, consent);
            stage = `allocate ${cycle}`; const previous = admission; admission = await group.allocate([retired]);
            check(admission.generations.length === 1 && admission.generations[0].runtimeId !== admission.generations[0].previousRuntimeId, 'reused or incomplete generation');
            check(admission.generations[0].checkpoint.byteLength <= 4 * 1024 * 1024, 'generation exceeded checkpoint budget');
            if (scenario === 'newer-input') input.value = 'Newer property-only draft';
            if (scenario === 'changed-pin') declaration?.setAttribute('compiled-hash', `cem-bin/1+blake3:${'0'.repeat(64)}`);
            stage = `resume ${cycle}`;
            if (scenario !== 'success') {
                const focus = document.activeElement, start = input.selectionStart, end = input.selectionEnd, direction = input.selectionDirection;
                let refused = false, failure = ''; try { await resumeCemCheckpointOwners(admission, [ticket]); } catch (error) { failure = String(error); refused = /stale|coverage/.test(failure); }
                check(refused && field.querySelector('input') === input && input.value === (scenario === 'newer-input' ? 'Newer property-only draft' : 'Unsaved compiled draft é😀'), `stale guard: error=${failure} same=${field.querySelector('input') === input} value=${input.value === (scenario === 'newer-input' ? 'Newer property-only draft' : 'Unsaved compiled draft é😀')}`);
                check(document.activeElement === focus && input.selectionStart === start && input.selectionEnd === end && input.selectionDirection === direction, 'stale refusal changed selection or focus');
                const last = coordinator.lastCommitted().bytes;
                check(last.length === before.bytes.length && last.every((byte: number, index: number) => byte === before.bytes[index]), 'refusal changed the committed checkpoint');
                return `${scenario} passed`;
            }
            const resumed = await resumeCemCheckpointOwners(admission, [ticket]);
            for (const owner of resumed.owners) await owner.owner.resume(); previous?.revoke();
            if (fallback) { heap = admission.generations[0].runtime; embedding = heap.claimOwnership(); }
            await capture();
            check(field.querySelector('input') === input && document.activeElement === input
                && input.value === (cycle ? 'Accepted after 0' : 'Unsaved compiled draft é😀')
                && input.selectionStart === 2 && input.selectionEnd === 9 && input.selectionDirection === 'backward', 'recovery replaced the edit/focus/selection');
            check(new FormData(form).get('item') === input.value, 'recovery lost native form association');
            check(styles.every(style => style.isConnected), 'recovery replaced installed styles');
            check(coordinator.current().receipt.owners.every((owner: { instanceId: string; acceptedEventSequence: number }) =>
                owner.acceptedEventSequence === before.receipt.owners.find((prior: { instanceId: string }) => prior.instanceId === owner.instanceId)?.acceptedEventSequence), 'recovery manufactured accepted input');
            input.value = `Property-only ${cycle}`; input.setSelectionRange(2, 9, 'backward');
            field.setAttribute('label', `Fresh binding ${cycle}`); await runtime.whenRenderSettled(field); await capture();
            check(field.textContent?.includes(`Fresh binding ${cycle}`) && field.querySelector('input') === input && input.value === `Property-only ${cycle}`, 'fresh evaluation lost binding or property-only edit');
            await value(`Accepted after ${cycle}`); input.focus(); input.setSelectionRange(2, 9, 'backward'); await capture();
            check(new FormData(form).get('item') === input.value && coordinator.current().receipt.owners.some((owner: { instanceId: string; acceptedEventSequence: number }) =>
                owner.acceptedEventSequence > before.receipt.owners.find((prior: { instanceId: string }) => prior.instanceId === owner.instanceId)?.acceptedEventSequence), 'fresh accepted input was not published');
            check(sourceLoads === 0 && downloads.slice(acquired).every(url => !/\.(json|bin|xhtml)(?:\?|$)/.test(url)), 'recovery refetched a component asset');
            if (!fallback) check(workerReplies > replies && kinds.length > 0 && kinds.every(kind => kind === 'pinned-template-artifact-v1'), 'fresh worker did not execute pinned IR');
        }
        return 'success passed';
    } catch (error) { throw Error(`${fallback ? 'fallback' : 'worker'}/${stage}: ${String(error)}`, { cause: error }); }
    finally { globalThis.fetch = fetcher; admission?.revoke(); coordinator.dispose(); scope.dispose(); document.body.replaceChildren(); }
}

function scenario(fallback: boolean, kind: string): StoryObj {
    return { render: () => '<section aria-label="Compiled application generation recovery"></section>', play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section'); if (!root) throw Error('Missing fixture');
        const html = generated['../../../../dist/artifacts/cem_ql/compiled-form/declarations.html']; if (!html) throw Error('Build native declaration fixtures first');
        const frame = document.createElement('iframe'), id = crypto.randomUUID(); root.append(frame);
        let stop = () => undefined;
        try {
            const completed = new Promise<string>((resolve, reject) => {
                const timer = setTimeout(() => reject(Error('Compiled form recovery timed out')), 20000);
                const receive = (event: MessageEvent) => { if (event.source !== frame.contentWindow || event.data?.id !== id) return;
                    clearTimeout(timer); if (event.data.error) reject(Error(event.data.error)); else resolve(event.data.result); };
                window.addEventListener('message', receive); stop = () => { clearTimeout(timer); window.removeEventListener('message', receive); };
            });
            const hook = import.meta.env.MODE === 'test' ? 'globalThis.__vitest_browser_runner__ = { wrapDynamicImport: f => f() };' : '';
            frame.srcdoc = `<script type="module">${hook}(${recoverCompiledForm.toString()})(${JSON.stringify(new URL('../../dist/', import.meta.url).href)}, ${JSON.stringify(new URL(html, location.href).href)}, url => import(url), ${fallback}, ${JSON.stringify(kind)})
                .then(result => parent.postMessage({id:${JSON.stringify(id)},result}, '*'), error => parent.postMessage({id:${JSON.stringify(id)},error:error.stack || String(error)}, '*'));</script>`;
            expect(await completed).toBe(`${kind} passed`);
        } finally { stop(); frame.remove(); }
    } };
}
export const WorkerRecovery = scenario(false, 'success');
export const FallbackRecovery = scenario(true, 'success');
export const WorkerNewerInput = scenario(false, 'newer-input');
export const FallbackNewerInput = scenario(true, 'newer-input');
export const WorkerChangedPin = scenario(false, 'changed-pin');
export const FallbackChangedPin = scenario(true, 'changed-pin');
