import { expect, waitFor } from 'storybook/test';
import preview from '../../.storybook/preview.js';
import { mountCemLivePreview } from './live-preview.js';
const meta = preview.meta({ title: 'Runtime/Live CEM preview' });
export const Lifecycle = meta.story({
    render: () => '<iframe title="First preview"></iframe><iframe title="Independent preview"></iframe>',
    play: async ({ canvasElement }) => {
        const [first, second] = [...canvasElement.querySelectorAll('iframe')];
        const options = { sourceUri: 'studio://one/view.cem', resources: [], source: '{slice @name=name | Initial}{input @aria-label=Name @slice=name @slice-event=input @slice-value="$target.value"}{p | {$datadom.slices.name}}' };
        const one = mountCemLivePreview(first, options), two = mountCemLivePreview(second, options);
        try {
            expect(one.checkpoint()).toBeUndefined();
            await Promise.all([one.ready, two.ready]);
            const original = one.checkpoint()!; expect(original()).toBe(true);
            expect(first.contentDocument?.querySelector('p')?.textContent).toBe('Initial');
            expect(first.contentDocument?.querySelector('cem-live-view')?.shadowRoot).toBeNull();
            const input = first.contentDocument!.querySelector('input')!;
            input.value = 'Amsterdam café'; input.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(first.contentDocument?.querySelector('p')?.textContent).toBe('Amsterdam café'));
            expect(second.contentDocument?.querySelector('p')?.textContent).toBe('Initial');
            const replacement = mountCemLivePreview(first, { ...options, source: '{p | Saved edit}' });
            expect(original()).toBe(false);
            await replacement.ready;
            const retained = replacement.checkpoint()!; expect(retained()).toBe(true);
            expect(first.contentDocument?.querySelector('p')?.textContent).toBe('Saved edit');
            replacement.dispose(); expect(retained()).toBe(false);
            expect(first.hasAttribute('srcdoc')).toBe(false);
        } finally { one.dispose(); two.dispose(); }
    },
});
export const DeniedOutput = meta.story({
    render: () => '<iframe title="Restricted preview"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = canvasElement.querySelector('iframe')!;
        for (const source of ['{script | alert(1)}', '{meta @http-equiv=refresh @content="0;url=https://example.com"}', '{a @href="https://example.com" | Link}', '{iframe @srcdoc=unsafe}', '{button @onclick="alert(1)" | Run}']) {
            const controller = mountCemLivePreview(frame, { source, sourceUri: 'studio://one/view.cem', resources: [] });
            try { await expect(controller.ready).rejects.toThrow('cem.preview.'); }
            finally { controller.dispose(); }
        }
    },
});

export const ProjectResources = meta.story({
    render: () => '<iframe title="Project preview"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = canvasElement.querySelector('iframe')!;
        const source = '{http-request @slice=data @url="./data.json" @content-type=application/json}{p | {$datadom.slices.data.state}}';
        const controller = mountCemLivePreview(frame, { source, sourceUri: 'studio://one/view.cem', resources: [{ uri: 'studio://one/data.json', contentType: 'application/json', text: '{"title":"café"}' }] });
        try { await controller.ready; await waitFor(() => expect(frame.contentDocument?.querySelector('p')?.textContent).toBe('loaded')); }
        finally { controller.dispose(); }
        const denied = mountCemLivePreview(frame, { source, sourceUri: 'studio://two/view.cem', resources: [{ uri: 'studio://one/data.json', contentType: 'application/json', text: '{}' }] });
        try { await expect(denied.ready).rejects.toThrow('resource_denied'); }
        finally { denied.dispose(); }
        localStorage.setItem('cem-preview-test-isolation', 'parent');
        const storage = mountCemLivePreview(frame, { sourceUri: 'studio://one/view.cem', resources: [], source: '{local-storage @slice=theme @key=cem-preview-test-isolation @value=child}{location-element @slice=location @method=replace @url="https://example.com"}{p | Independent}' });
        try { await storage.ready; expect(localStorage.getItem('cem-preview-test-isolation')).toBe('parent'); expect(frame.contentWindow?.location.href).toBe('about:srcdoc'); }
        finally { storage.dispose(); localStorage.removeItem('cem-preview-test-isolation'); }
    },
});

export const IncrementalAdmissionAndCancellation = meta.story({
    render: () => '<iframe title="Changing preview"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = canvasElement.querySelector('iframe')!;
        const errors: string[] = [];
        const options = { sourceUri: 'preview://one/view.cem', resources: [], source: '{slice @name=go | false}{button @type=button @slice=go @slice-event=click @slice-value="true" | Change}{a @href={if datadom.slices.go { "https://example.com" } else { null }} | Local text}' };
        const controller = mountCemLivePreview(frame, { ...options, onError: message => errors.push(message) });
        try {
            await controller.ready;
            const link = frame.contentDocument!.querySelector('a')!;
            frame.contentDocument!.querySelector('button')!.click();
            await waitFor(() => expect(errors.some(message => message.includes('attribute_denied'))).toBe(true));
            expect(link.hasAttribute('href')).toBe(false);
            expect(frame.contentWindow?.location.href).toBe('about:srcdoc');
        } finally { controller.dispose(); }
        const cancelled = mountCemLivePreview(frame, options);
        const replacement = mountCemLivePreview(frame, { ...options, source: '{p | Replacement}' });
        try {
            await expect(cancelled.ready).rejects.toThrow('disposed');
            await replacement.ready;
            expect(frame.contentDocument!.querySelector('p')!.textContent).toBe('Replacement');
        } finally { replacement.dispose(); }
    },
});

export const RecoveryEnrollment = meta.story({
    render: () => '<iframe title="Enrolled live preview"></iframe>',
    play: async ({ canvasElement }) => {
        const { CemNativeRecoveryScope } = await import('./internal/runtime-support/native-generation.js');
        const { CemCheckpointPublicationCoordinator } = await import('./checkpoint-publication.js');
        const { resumeCemCheckpointOwners } = await import('./checkpoint-resume.js');
        const binding = await import('./internal/runtime-support/native-runtime.js');
        const scope = new CemNativeRecoveryScope();
        const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
            policyStamp: 'preview-enrollment', allowOwner: () => true, seal: (candidate, receipt) => binding.sealCommittedCheckpoint(candidate, receipt) });
        const frame = canvasElement.querySelector('iframe')!;
        const options = { sourceUri: 'studio://enrolled/view.cem', resources: [],
            source: '{input @value="café 😀"}{p | Original source}', runtimeOptions: { checkpointPublication: coordinator, recoveryScope: scope } };
        const controller = mountCemLivePreview(frame, options);
        options.source = '{p | Mutated caller}';
        let admission: import('./internal/runtime-support/native-generation.js').CemNativeGenerationAdmission | undefined;
        try {
            await expect(scope.prepare()).rejects.toThrow('cem.preview.recovery.pending');
            await controller.ready;
            expect(scope.inspect().members.map(member => member.kind)).toEqual(['live-preview']);
            expect(frame.contentDocument!.querySelector('p')!.textContent).toBe('Original source');
            const input = frame.contentDocument!.querySelector('input')!; input.value = 'edited é😀'; input.focus(); input.setSelectionRange(2, 5, 'backward');
            const capture = async () => { await waitFor(async () => { await coordinator.capture(); expect(coordinator.current()).toBeDefined(); }); };
            await capture();
            const proof = await coordinator.closeRenderingGeneration(); await capture();
            const ticket = coordinator.prepareResume();
            const plan = await scope.prepare(), group = plan.createGenerationGroup();
            const retired = await coordinator.retireRenderingGeneration(proof);
            admission = await group.allocate([retired]);
            const resumed = await resumeCemCheckpointOwners(admission, [ticket]);
            for (const owner of resumed.owners) await (owner.owner as { resume(): Promise<void> }).resume();
            await capture();
            expect(frame.contentDocument!.querySelector('input')).toBe(input);
            expect(input.value).toBe('edited é😀'); expect(input.selectionStart).toBe(2); expect(input.selectionEnd).toBe(5);
            expect(frame.contentDocument!.activeElement).toBe(input);
            expect(frame.contentDocument!.querySelector('p')!.textContent).toBe('Original source');
        } finally { admission?.revoke(); controller.dispose(); coordinator.dispose(); expect(scope.inspect().members).toEqual([]); scope.dispose(); }
    },
});

export const RecoveryStartupCancellation = meta.story({
    render: () => '<iframe title="Cancelled enrolled preview"></iframe>',
    play: async ({ canvasElement }) => {
        const { CemNativeRecoveryScope } = await import('./internal/runtime-support/native-generation.js');
        const recoveryScope = new CemNativeRecoveryScope(); let disposals = 0;
        const controller = mountCemLivePreview(canvasElement.querySelector('iframe')!, {
            sourceUri: 'studio://cancel/view.cem', source: '{p | Not mounted}', resources: [],
            runtimeOptions: { recoveryScope, dispose: () => { disposals++; } },
        });
        expect(recoveryScope.inspect().members.map(member => member.kind)).toEqual(['live-preview']);
        controller.dispose(); controller.dispose();
        await expect(controller.ready).rejects.toThrow('disposed');
        expect(recoveryScope.inspect().members).toEqual([]); expect(disposals).toBe(1);
        expect(() => mountCemLivePreview(canvasElement.querySelector('iframe')!, {
            sourceUri: 'invalid-url', source: '{p | Invalid setup}', resources: [],
            runtimeOptions: { recoveryScope, dispose: () => { disposals++; } },
        })).toThrow();
        expect(recoveryScope.inspect().members).toEqual([]); expect(disposals).toBe(2); recoveryScope.dispose();
    },
});
