import { expect, waitFor } from 'storybook/test';
import preview, { cemComponentDeclarationSource, createCemStoryRuntime } from '../../.storybook/preview.js';

type Editor = HTMLElement & { value: string };
const editor = (host: Element) => host.querySelector<HTMLElement>('[part="control"]')!;
const paintedRanges = (host: Element) => [...CSS.highlights.entries()]
    .filter(([name]) => name.startsWith('cem-source-')).flatMap(([, paint]) => [...paint])
    .filter(range => host.contains(range.startContainer));
const meta = preview.meta({ title: 'CEM Runtime/Source paint', component: 'cem-source-editor' });

export const StalePaintResults = meta.story({
    render: () => '<section aria-label="Asynchronous source paint"></section>',
    play: async ({ canvasElement }) => {
        const declarationSource = await cemComponentDeclarationSource('cem-source-editor');
        const { cemProcessingHostForScope } = await import('./internal/runtime-support/processing-host-runtime.js');
        const { nativeCheckpointFailureFence } = await import('./internal/runtime-support/checkpoint-domain.js');
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-delayed-editor-${crypto.randomUUID()}`; await declare(declarationSource, tag);
        const host = document.createElement(tag) as Editor;
        host.setAttribute('content-type', 'text/html'); canvasElement.append(host); await runtime.whenRenderSettled(host);
        const processing = cemProcessingHostForScope(scope, {}), highlight = processing.highlight!.bind(processing);
        const releases: (() => void)[] = [], sources: string[] = [];
        // Hold genuine native responses at the transport boundary, including a
        // reply that arrives after its editor request has been cancelled.
        processing.highlight = input => {
            const job = highlight(input); sources.push(input.source);
            let release!: () => void; const held = new Promise<void>(resolve => { release = resolve; });
            releases.push(release);
            return { ...job, result: job.result.then(async result => { await held; return result; }) };
        };
        try {
            host.value = '<i>Old</i>';
            await waitFor(() => expect(sources).toContain('<i>Old</i>'));
            host.setAttribute('source-id', 'next'); host.value = '<strong>Current</strong>'; await runtime.whenRenderSettled(host);
            await waitFor(() => expect(sources).toContain('<strong>Current</strong>'));
            releases[0](); await new Promise(resolve => setTimeout(resolve, 100));
            expect(paintedRanges(host)).toHaveLength(0);
            releases[1](); await waitFor(() => expect(paintedRanges(host).map(range => range.toString())).toContain('strong'));
            // An unreported property-only DOM change cannot admit stale paint.
            host.value = '<em>Pending</em>'; await waitFor(() => expect(sources).toContain('<em>Pending</em>'));
            editor(host).textContent = 'Property-only edit'; releases[2]();
            await new Promise(resolve => setTimeout(resolve, 100)); expect(paintedRanges(host)).toHaveLength(0);
            host.value = '<b>Final</b>'; await waitFor(() => expect(sources).toContain('<b>Final</b>'));
            releases[3](); await waitFor(() => expect(paintedRanges(host).length).toBeGreaterThan(0));
            nativeCheckpointFailureFence(processing.checkpointClient!)();
            expect(paintedRanges(host)).toHaveLength(0); expect(host.value).toBe('<b>Final</b>');
            expect(editor(host).textContent).toBe('<b>Final</b>');
        } finally { for (const release of releases) release(); processing.highlight = highlight; host.remove(); scope.dispose(); }
    },
});
