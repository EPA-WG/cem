import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, within } from 'storybook/test';
import { CemElementRuntime, type CemCompiledDeclarationRegistration } from './cem-elements.js';
import { cemMlTemplateArtifactPayloadKey } from './internal/runtime-support/cem-ql-render.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Compiled Declaration Registration', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const assets = import.meta.glob<string>('../../../../dist/artifacts/cem_ql/declarations/*', { eager: true, query: '?url', import: 'default' });
async function fixture(mode: string, name: string): Promise<CemCompiledDeclarationRegistration> {
    const prefix = `../../../../dist/artifacts/cem_ql/declarations/${mode}-${name}`;
    const load = async (suffix: string) => {
        const url = assets[`${prefix}.${suffix}`];
        if (!url) throw Error('Build native declaration fixtures before this story');
        const response = await fetch(url);
        if (!response.ok) throw Error('Build native declaration fixtures before this story');
        return response;
    };
    return { metadataJson: await (await load('json')).text(), metadataHash: (await (await load('hash')).text()).trim(),
        templateBytes: await (await load('bin')).arrayBuffer() };
}
function scenario(fallback: boolean): Story {
    return { render: () => '<section aria-label="Compiled component registration"></section>', play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section'); if (!root) throw Error('Missing fixture');
        const mode = fallback ? 'fallback' : 'worker';
        const scope = createCemDeclarationScope({ document });
        const compileKinds: string[] = []; let workerSuccess = 0, sourceLoads = 0;
        const runtime = new CemElementRuntime({ declarationTag: `cem-aot-declarations-${mode}`, declarationScope: scope,
            retainedStylesheets: {}, loadSrcDocument: async () => { sourceLoads++; throw Error('Source downloads are forbidden in this fixture'); },
            processingWorkerFactory: request => {
                if (fallback) throw Error('fixture fallback');
                const worker = new Worker(request.scriptUrl, { type: request.type, name: request.name });
                const send = worker.postMessage.bind(worker);
                worker.postMessage = (message: unknown) => {
                    const packet = message as { operation?: string; payload?: { language?: string; source?: { kind: string } } };
                    if (packet.operation === 'compile' && packet.payload?.language === 'cem-ml') compileKinds.push(packet.payload.source?.kind ?? 'missing');
                    send(message);
                };
                worker.addEventListener('message', ({ data }) => { if (data.direction === 'response' && data.outcome === 'success') workerSuccess++; });
                return worker;
            } });
        runtime.install(window);
        const declarations: HTMLElement[] = [];
        try {
            for (const name of ['field', 'action', 'card']) {
                const tag = `cem-aot-${name}-${mode}`, input = await fixture(mode, name);
                const declaration = document.createElement(runtime.declarationTag); declaration.setAttribute('tag', tag);
                if (name === 'field') {
                    declaration.setAttribute('capability', 'action-command');
                    await expect(runtime.registerCompiledDeclaration(declaration, input)).rejects.toThrow('override capability');
                    expect(customElements.get(tag)).toBeUndefined(); declaration.removeAttribute('capability');
                    const retiredScope = createCemDeclarationScope({ document });
                    const retiredRuntime = new CemElementRuntime({ declarationScope: retiredScope });
                    const retiredDeclaration = document.createElement('div'); retiredDeclaration.setAttribute('tag', tag);
                    const retiredPending = retiredRuntime.registerCompiledDeclaration(retiredDeclaration, input);
                    retiredScope.dispose();
                    await expect(retiredPending).rejects.toThrow(); expect(customElements.get(tag)).toBeUndefined();
                    const legacyClaim = JSON.parse(input.metadataJson) as { templateFormat: string };
                    legacyClaim.templateFormat = 'cem-template-artifact/1';
                    const metadataJson = JSON.stringify(legacyClaim);
                    const pin = await cemMlTemplateArtifactPayloadKey(metadataJson, 'prod');
                    await expect(runtime.registerCompiledDeclaration(declaration, { ...input, metadataJson, metadataHash: pin.sourceHash })).rejects.toThrow('template format mismatch');
                    expect(customElements.get(tag)).toBeUndefined();
                    const corrupt = { ...input, metadataJson: `${input.metadataJson} ` };
                    await expect(runtime.registerCompiledDeclaration(declaration, corrupt)).rejects.toThrow('metadata');
                    expect(customElements.get(tag)).toBeUndefined();
                    const broken = { ...input, templateBytes: input.templateBytes.slice(0) };
                    new Uint8Array(broken.templateBytes)[broken.templateBytes.byteLength - 1] ^= 1;
                    await expect(runtime.registerCompiledDeclaration(declaration, broken)).rejects.toThrow();
                    expect(customElements.get(tag)).toBeUndefined();
                }
                const pending = runtime.registerCompiledDeclaration(declaration, input);
                // Async admission owns a snapshot of the caller's deployment inputs.
                input.metadataJson = 'changed after dispatch'; new Uint8Array(input.templateBytes).fill(0);
                // Connecting while native admission is pending cannot invoke a source path.
                root.append(declaration); declarations.push(declaration);
                await pending; expect(declaration.childNodes.length).toBe(0);
                expect(customElements.get(tag)).toBeDefined();
            }
            const card = document.createElement(`cem-aot-card-${mode}`);
            let form = document.createElement('form');
            let field = document.createElement(`cem-aot-field-${mode}`), action = document.createElement(`cem-aot-action-${mode}`);
            field.setAttribute('name', 'item'); field.setAttribute('value', 'Original');
            action.setAttribute('label', 'Apply'); action.setAttribute('type', 'button');
            form.append(field, action); card.append(form); root.append(card);
            await runtime.whenRenderSettled(card);
            // Initial slot projection materializes the live payload. Test its
            // controls, rather than the inert source held in the data island.
            const liveForm = card.querySelector('form'), liveField = card.querySelector<HTMLElement>(`cem-aot-field-${mode}`), liveAction = card.querySelector<HTMLElement>(`cem-aot-action-${mode}`);
            if (!liveForm || !liveField || !liveAction) throw Error('Missing projected form controls');
            form = liveForm; field = liveField; action = liveAction;
            await runtime.whenRenderSettled(field); await runtime.whenRenderSettled(action);
            const control = within(field).getByRole('textbox', { name: 'Text field' });
            field.setAttribute('label', 'Item'); await runtime.whenRenderSettled(field);
            expect(within(field).getByRole('textbox', { name: 'Item' })).toBe(control);
            expect(new FormData(form).get('item')).toBe('Original');
            await userEvent.clear(control); await userEvent.type(control, 'Edited');
            await runtime.whenRenderSettled(field); expect(new FormData(form).get('item')).toBe('Edited');
            control.focus(); (control as HTMLInputElement).setSelectionRange(1, 4);
            field.setAttribute('label', 'Renamed'); await runtime.whenRenderSettled(field);
            expect(within(field).getByRole('textbox', { name: 'Renamed' })).toBe(control);
            expect((control as HTMLInputElement).value).toBe('Edited'); expect(document.activeElement).toBe(control);
            expect([(control as HTMLInputElement).selectionStart, (control as HTMLInputElement).selectionEnd]).toEqual([1, 4]);
            let clicks = 0; action.addEventListener('click', () => clicks++);
            await userEvent.click(within(action).getByRole('button', { name: 'Apply' })); expect(clicks).toBe(1);
            expect([...document.querySelectorAll('style')].some(style => style.textContent?.includes(`cem-aot-field-${mode}`))).toBe(true);
            const conflicting = document.createElement(runtime.declarationTag);
            conflicting.setAttribute('tag', `cem-aot-field-${mode}`);
            await expect(runtime.registerCompiledDeclaration(conflicting, await fixture(mode, 'conflict'))).rejects.toThrow();
            expect(new FormData(form).get('item')).toBe('Edited');
            const declaration = declarations[0]; declaration.remove(); root.prepend(declaration);
            await runtime.whenDeclarationSettled(declaration); await runtime.whenRenderSettled(field);
            expect(new FormData(form).get('item')).toBe('Edited'); expect(sourceLoads).toBe(0);
            if (!fallback) { expect(workerSuccess).toBeGreaterThan(0); expect(compileKinds.length).toBeGreaterThan(0);
                expect(compileKinds.every(kind => kind === 'pinned-template-artifact-v1')).toBe(true); }
        } finally { root.replaceChildren(); scope.dispose(); }
    } };
}
export const WorkerComponents: Story = scenario(false);
export const FallbackComponents: Story = scenario(true);
