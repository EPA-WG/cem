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
            await Promise.all([one.ready, two.ready]);
            expect(first.contentDocument?.querySelector('p')?.textContent).toBe('Initial');
            expect(first.contentDocument?.querySelector('cem-live-view')?.shadowRoot).toBeNull();
            const input = first.contentDocument!.querySelector('input')!;
            input.value = 'Amsterdam café'; input.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(first.contentDocument?.querySelector('p')?.textContent).toBe('Amsterdam café'));
            expect(second.contentDocument?.querySelector('p')?.textContent).toBe('Initial');
            const replacement = mountCemLivePreview(first, { ...options, source: '{p | Saved edit}' });
            await replacement.ready;
            expect(first.contentDocument?.querySelector('p')?.textContent).toBe('Saved edit');
            replacement.dispose();
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
