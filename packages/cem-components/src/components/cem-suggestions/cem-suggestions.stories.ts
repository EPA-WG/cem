import { expect, waitFor, userEvent } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemComponent, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-suggestions.xhtml?raw';

const meta = preview.meta({
    component: 'cem-suggestions', title: 'CEM Components/cem-suggestions',
    loaders: [async () => {
        await Promise.all([loadCemComponent('cem-field'), loadCemComponent('cem-text-field')]);
        storybookCemRuntime().setLocalSuggestionsEnabled(true);
        await loadCemDeclaration('cem-suggestions', declarationSource);
        return {};
    }],
});
async function ready(host: HTMLElement) {
    await whenCemRendered(host);
    const field = host.querySelector<HTMLElement>('[slot=editor]');
    if (!field) throw new Error('Missing projected field');
    await whenCemRendered(field);
    await waitFor(() => {
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
        expect(storybookCemRuntime().renderedSuggestionsFor(host)?.current()).toBe(true);
    });
    expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    const input = field.querySelector('input'); if (!input) throw new Error('Missing native editor');
    return { input, field, surface: host.querySelector<HTMLElement>('[part=surface]') as HTMLElement };
}
export const BothFieldProvidersKeepSubmissionOwnership = meta.story({
    render: () => `<form aria-label="Field"><cem-suggestions label="Choices"><template><cem-field slot="editor" name="choice" label="Choice"><span slot="help">Field help</span></cem-field><template slot="options"><option value="a">Alpha</option><option value="b">Beta</option></template></template></cem-suggestions></form>
    <form aria-label="Text field"><cem-suggestions label="Choices"><template><cem-text-field slot="editor" name="choice" label="Choice"><span slot="help">Field help</span></cem-text-field><template slot="options"><option value="a">Alpha</option><option value="b">Beta</option></template></template></cem-suggestions></form>`,
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            const { input, field, surface } = await ready(host), form = host.closest('form') as HTMLFormElement;
            expect(input).toHaveAccessibleName('Choice');
            expect(input.hasAttribute('name')).toBe(false);
            expect(host.hasAttribute('name')).toBe(false);
            expect(host.querySelectorAll('input:not([type=hidden])')).toHaveLength(1);
            expect(surface.getAttribute('aria-label')).toBe('Choices');
            expect(surface.getAttribute('popover')).toBe('manual');
            expect(host.querySelector('[part=option-value]')?.textContent).toBe('a');
            expect(host.querySelector('[part=option]')?.getAttribute('aria-label')).toBe('Alpha (a)');
            const help = field.querySelector('[slot=help]'); expect(help?.textContent).toBe('Field help');
            input.focus(); await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
            await userEvent.keyboard('[ArrowDown]');
            await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
            expect(input.getAttribute('aria-activedescendant')).toBe(host.querySelector('[part=option]')?.id);
            await userEvent.keyboard('[Enter]');
            await waitFor(() => expect(input.value).toBe('a'));
            expect(new FormData(form).getAll('choice')).toEqual(['a']);
            expect(field.querySelector('input')).toBe(input);
            expect(field.querySelector('[slot=help]')).toBe(help);
            await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('false'));
            form.reset(); await waitFor(() => expect(input.value).toBe(''));
        }
    },
});
export const GroupedCapturedLabels = meta.story({
    render: () => `<cem-suggestions label="Letters"><template><cem-text-field slot="editor" label="Letter"></cem-text-field>
    <template slot="options"><optgroup label="Letters"><option value="a" label="Alpha">Ignored text</option><option value="b" disabled>Beta</option></optgroup></template>
    <template slot="option" type="text/cem-ml" xmlns:s="http://www.w3.org/2000/svg">{strong | {$suggestion.dom:attribute("label").value}}{s:svg | {s:path @d="M0 0"}}</template>
    <template slot="group-label" type="text/cem-ml">{em | Group: {$group.dom:attribute("label").value}}</template></template></cem-suggestions>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-suggestions') as HTMLElement;
        const { input } = await ready(host);
        expect(host.querySelector('[part=group]')?.getAttribute('aria-label')).toBe('Letters');
        expect(host.querySelector('[part=group-label] em')?.textContent).toBe('Group: Letters');
        expect(host.querySelector('[part=option-label] strong')?.textContent).toBe('Alpha');
        expect(host.querySelector('[part=option-label] svg')?.namespaceURI).toBe('http://www.w3.org/2000/svg');
        expect(host.querySelector('[part=surface]')?.textContent).not.toContain('Ignored text');
        expect(host.querySelectorAll('[part=option]')[1].getAttribute('aria-disabled')).toBe('true');
        input.focus(); await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
        await userEvent.keyboard('[ArrowDown][ArrowDown]');
        await waitFor(() => expect(host.querySelector('[part=option]')?.getAttribute('aria-selected')).toBe('true'));
        await userEvent.keyboard('[Escape]'); expect(input.value).toBe('');
    },
});
export const LocalizedFeedbackAndExplicitInputConflict = meta.story({
    render: () => `<cem-suggestions pending-message="Please wait" failure-message="Not available" empty-message="No matches"><template><cem-field slot="editor" label="Choose"></cem-field><template slot="options"><option value="a">Alpha</option></template></template></cem-suggestions>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-suggestions') as HTMLElement;
        const { input, surface } = await ready(host);
        input.focus(); host.setAttribute('options-state', 'pending'); await whenCemRendered(host);
        await waitFor(() => expect(host.querySelector('[part=status]')?.textContent).toBe('Please wait'));
        expect(surface.getAttribute('aria-busy')).toBe('true');
        host.setAttribute('options-state', 'failed'); await whenCemRendered(host);
        await waitFor(() => expect(host.querySelector('[part=status]')?.textContent).toBe('Not available'));
        host.setAttribute('options-state', 'ready'); await ready(host);
        host.setAttribute('options', 'conflict'); await whenCemRendered(host);
        await waitFor(() => expect(input.hasAttribute('aria-controls')).toBe(false));
        host.removeAttribute('options'); await ready(host);
        expect(host.querySelector('input')).toBe(input);
    },
});
