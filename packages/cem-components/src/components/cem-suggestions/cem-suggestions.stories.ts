import { expect, waitFor, userEvent } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemComponent, whenCemRendered, storybookCemRuntime, nativeTap, nativeTouchGesture, nativePenDrag, nativeWheel } from '../../../../cem-elements/.storybook/preview.js';
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

const acceptanceFields = () => ['cem-field', 'cem-text-field'].map(tag => `<form aria-label="${tag}"><cem-suggestions filter="none" require-selection><template><${tag} slot="editor" name="choice" label="Choice"><span slot="help">Keep this help</span></${tag}><template slot="options"><optgroup label="Letters"><option value="a" label="Alpha">Ignored</option><option value="x" disabled>Disabled</option><option value="b">Beta</option></optgroup></template></template></cem-suggestions><button type="button">Outside</button><button type="submit">Submit</button></form>`).join('');
export const NativePointerTapsKeepBothEditorsFocused = meta.story({
    render: acceptanceFields,
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return; // Chromium native-input evidence is browser-runner-only.
        const { cdp } = await import('vitest/browser');
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            const { input } = await ready(host), form = host.closest('form') as HTMLFormElement;
            for (const pointerType of ['mouse', 'touch', 'pen'] as const) {
                form.querySelector<HTMLButtonElement>('button')?.focus(); form.reset(); await ready(host); input.focus();
                await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
                const row = host.querySelector<HTMLElement>('[part=option]') as HTMLElement;
                const events: string[] = [], pointers: PointerEvent[] = [];
                const changes = (event: Event) => events.push(event.type);
                const down = (event: PointerEvent) => pointers.push(event);
                input.addEventListener('input', changes); input.addEventListener('change', changes); row.addEventListener('pointerdown', down);
                try {
                    await nativeTap(cdp(), row, pointerType);
                    await waitFor(() => expect(pointers.map(event => ({ type: event.pointerType, trusted: event.isTrusted, cancelled: event.defaultPrevented })))
                        .toEqual([{ type: pointerType, trusted: true, cancelled: pointerType === 'mouse' }]));
                    await waitFor(() => expect(input.value).toBe('a'));
                    expect(input.ownerDocument.activeElement).toBe(input);
                    expect(events).toEqual(['input', 'change']);
                    expect(new FormData(form).getAll('choice')).toEqual(['a']);
                    expect(host.querySelector('input')).toBe(input);
                    await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('false'));
                } finally { input.removeEventListener('input', changes); input.removeEventListener('change', changes); row.removeEventListener('pointerdown', down); }
            }
        }
    },
});

export const IndependentFilteringAvailabilityAndSourceReplacement = meta.story({
    render: acceptanceFields,
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')];
        const prepared = await Promise.all(hosts.map(ready));
        for (let i = 0; i < hosts.length; i++) {
            const host = hosts[i], { input, field, surface } = prepared[i];
            const other = prepared[1 - i].input, form = host.closest('form') as HTMLFormElement;
            const help = field.querySelector('[slot=help]'), description = input.getAttribute('aria-describedby');
            host.setAttribute('filter', 'prefix'); await ready(host);
            input.focus(); await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
            await userEvent.type(input, 'Be'); await ready(host);
            await waitFor(() => expect(host.querySelectorAll('[part=option]:not([hidden])')).toHaveLength(1));
            expect(host.querySelector('[part=option]:not([hidden])')?.getAttribute('aria-label')).toBe('Beta (b)');
            expect(other.value).toBe(''); expect(other.hasAttribute('aria-activedescendant')).toBe(false);
            expect(field.querySelector('input')).toBe(input); expect(field.querySelector('[slot=help]')).toBe(help);
            expect(input.getAttribute('aria-describedby')).toBe(description);
            await userEvent.keyboard('[ArrowDown]');
            field.setAttribute('readonly', ''); await whenCemRendered(field);
            await waitFor(() => expect(surface.matches(':popover-open')).toBe(false));
            expect(input.hasAttribute('aria-activedescendant')).toBe(false); expect(input.value).toBe('Be');
            field.removeAttribute('readonly'); await whenCemRendered(field); await ready(host);
            expect(surface.matches(':popover-open')).toBe(false);
            await userEvent.keyboard('[ArrowDown]'); await waitFor(() => expect(input.hasAttribute('aria-activedescendant')).toBe(true));
            const oldRow = host.querySelector('[part=option]:not([hidden])') as HTMLElement;
            const sources = storybookCemRuntime().localSuggestionsEnvironmentFor(host)?.optionsSources;
            const source = sources?.[0] as HTMLTemplateElement;
            source.innerHTML = '<option value="c">Beatrice</option>'; await ready(host);
            expect(input.hasAttribute('aria-activedescendant')).toBe(false);
            oldRow.click(); expect(input.value).toBe('Be'); expect(new FormData(form).getAll('choice')).toEqual(['Be']);
            expect(surface.matches(':popover-open')).toBe(false);
            await userEvent.keyboard('[ArrowDown]'); await waitFor(() => expect(surface.matches(':popover-open')).toBe(true));
            field.setAttribute('disabled', ''); await whenCemRendered(field);
            await waitFor(() => expect(surface.matches(':popover-open')).toBe(false));
            expect(input.disabled).toBe(true); expect(new FormData(form).has('choice')).toBe(false);
            field.removeAttribute('disabled'); await whenCemRendered(field); await ready(host);
            expect(input.disabled).toBe(false); expect(surface.matches(':popover-open')).toBe(false);
            form.reset(); await ready(host);
            expect(input.value).toBe(''); expect(field.querySelector('input')).toBe(input);
        }
    },
});

export const CommitVetoAndRawPublicEventCoherence = meta.story({
    render: acceptanceFields,
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            const { input, field } = await ready(host), form = host.closest('form') as HTMLFormElement;
            const raw: string[] = [], published: string[] = [], values: string[] = [];
            const capture = (event: Event) => { if (event.target === input) raw.push(event.type); };
            const bubbling = (event: Event) => {
                published.push(event.type);
                values.push(`${(field as HTMLElement & { value: string }).value}/${new FormData(form).get('choice')}`);
            };
            input.addEventListener('input', capture, true); input.addEventListener('change', capture, true);
            field.addEventListener('input', bubbling); field.addEventListener('change', bubbling);
            const veto = (event: InputEvent) => { expect(event.inputType).toBe('insertReplacementText'); event.preventDefault(); };
            try {
                input.focus(); await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
                await userEvent.keyboard('[ArrowDown]'); input.addEventListener('beforeinput', veto, { once: true });
                await userEvent.keyboard('[Enter]'); expect(input.value).toBe('');
                expect(raw).toEqual([]); expect(published).toEqual([]);
                await userEvent.keyboard('[ArrowDown][Enter]'); await waitFor(() => expect(input.value).toBe('b'));
                expect(raw).toEqual(['input', 'change']); expect(published).toEqual(['input', 'change']); expect(values).toEqual(['b/b', 'b/b']);
                expect(form.checkValidity()).toBe(true);
                await ready(host); await userEvent.keyboard('[ArrowDown][Escape]'); expect(input.value).toBe('b');
                await userEvent.keyboard('[ArrowDown][Tab]'); expect(input.value).toBe('b');
                expect(raw).toEqual(['input', 'change']); expect(published).toEqual(['input', 'change']);
                expect(field.querySelector('input')).toBe(input);
            } finally {
                input.removeEventListener('input', capture, true); input.removeEventListener('change', capture, true);
                field.removeEventListener('input', bubbling); field.removeEventListener('change', bubbling);
            }
        }
    },
});

export const BrowserCompositionSettlesWithoutTerminalCommitOrSubmit = meta.story({
    render: acceptanceFields,
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return; // Protocol-driven Chromium IME evidence; physical IME remains a release check.
        const { cdp, userEvent: nativeUserEvent } = await import('vitest/browser');
        const driver = cdp();
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            const { input, surface } = await ready(host), form = host.closest('form') as HTMLFormElement;
            const composition: { type: string; trusted: boolean }[] = [], submissions: Event[] = [];
            const record = (event: Event) => composition.push({ type: event.type, trusted: event.isTrusted });
            const submit = (event: Event) => { event.preventDefault(); submissions.push(event); };
            for (const type of ['compositionstart', 'compositionupdate', 'compositionend']) input.addEventListener(type, record);
            form.addEventListener('submit', submit);
            try {
                input.focus(); await waitFor(() => expect(input.getAttribute('aria-expanded')).toBe('true'));
                await nativeUserEvent.keyboard('[ArrowDown]');
                await driver.send('Input.imeSetComposition', { text: 'あ', selectionStart: 1, selectionEnd: 1 });
                await waitFor(() => expect(input.value).toBe('あ'));
                expect(surface.matches(':popover-open')).toBe(false); expect(input.hasAttribute('aria-activedescendant')).toBe(false);
                await driver.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
                await driver.send('Input.insertText', { text: 'あ' });
                await driver.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
                await ready(host);
                expect(input.value).toBe('あ'); expect(new FormData(form).getAll('choice')).toEqual(['あ']); expect(submissions).toEqual([]);
                expect(composition.map(event => event.type)).toContain('compositionstart');
                await waitFor(() => expect(composition.map(event => event.type)).toContain('compositionend'));
                expect(input.hasAttribute('aria-activedescendant')).toBe(false);
                await nativeUserEvent.keyboard('[ArrowDown][Enter]'); await waitFor(() => expect(input.value).toBe('a'));
                expect(submissions).toEqual([]); expect(input.ownerDocument.activeElement).toBe(input);
            } finally {
                for (const type of ['compositionstart', 'compositionupdate', 'compositionend']) input.removeEventListener(type, record);
                form.removeEventListener('submit', submit);
            }
        }
    },
});

export const NativePanCancellationDragAndOutsideFocus = meta.story({
    render: () => ['cem-field', 'cem-text-field'].map(tag => `<form aria-label="${tag}"><cem-suggestions filter="none"><template><${tag} slot="editor" name="choice" label="Choice"></${tag}><template slot="options">${Array.from({ length: 10 }, (_, i) => `<option value="${i}">Option ${i}</option>`).join('')}</template></template></cem-suggestions><button type="button">Outside</button></form>`).join(''),
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return; // Native Chromium pointer/scroll evidence; physical devices remain separate.
        const { cdp } = await import('vitest/browser');
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            const { input, surface } = await ready(host), form = host.closest('form') as HTMLFormElement;
            const changes: string[] = [], downs: PointerEvent[] = [];
            const changed = (event: Event) => changes.push(event.type), down = (event: PointerEvent) => downs.push(event);
            input.addEventListener('input', changed); input.addEventListener('change', changed); surface.addEventListener('pointerdown', down);
            try {
                surface.style.blockSize = '100px'; // Constrain this geometry fixture to require scrolling.
                input.focus(); await waitFor(() => expect(surface.matches(':popover-open')).toBe(true));
                expect(surface.scrollHeight).toBeGreaterThan(surface.clientHeight);
                await nativeTouchGesture(cdp(), surface, 'pan');
                expect(downs.at(-1)?.pointerType).toBe('touch'); expect(downs.at(-1)?.defaultPrevented).toBe(false);
                await nativeWheel(cdp(), surface);
                await waitFor(() => expect(surface.scrollTop).toBeGreaterThan(0));
                expect(input.ownerDocument.activeElement).toBe(input); expect(input.value).toBe(''); expect(changes).toEqual([]);
                surface.scrollTop = 0;
                const row = host.querySelector('[part=option]') as HTMLElement;
                await nativeTouchGesture(cdp(), row, 'cancel'); await nativePenDrag(cdp(), row);
                expect(input.value).toBe(''); expect(changes).toEqual([]); expect(input.ownerDocument.activeElement).toBe(input);
                const outside = form.querySelector('button') as HTMLButtonElement;
                await nativeTap(cdp(), outside, 'mouse');
                await waitFor(() => expect(input.ownerDocument.activeElement).toBe(outside));
                expect(surface.matches(':popover-open')).toBe(false); expect(input.hasAttribute('aria-activedescendant')).toBe(false);
                row.click(); expect(input.value).toBe(''); expect(changes).toEqual([]); expect(input.ownerDocument.activeElement).toBe(outside);
            } finally { input.removeEventListener('input', changed); input.removeEventListener('change', changed); surface.removeEventListener('pointerdown', down); }
        }
    },
});

export const UnsupportedEditorsAndInputConflictsSuspendThenRecover = meta.story({
    render: acceptanceFields,
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            const { input, field, surface } = await ready(host);
            for (const [name, value] of [['list', 'author-list'], ['type', 'search']] as const) {
                input.focus(); await userEvent.keyboard('[ArrowDown]');
                field.setAttribute(name, value); await whenCemRendered(field);
                await waitFor(() => expect(input.hasAttribute('aria-controls')).toBe(false));
                expect(surface.matches(':popover-open')).toBe(false); expect(input.hasAttribute('aria-activedescendant')).toBe(false);
                expect(field.querySelector('input')).toBe(input); expect(input.value).toBe('');
                field.removeAttribute(name); await whenCemRendered(field); await ready(host);
                expect(surface.matches(':popover-open')).toBe(false);
            }
            host.setAttribute('options', 'explicit-conflict'); await whenCemRendered(host);
            await waitFor(() => expect(input.hasAttribute('aria-controls')).toBe(false));
            expect(surface.matches(':popover-open')).toBe(false); expect(input.value).toBe('');
            host.removeAttribute('options'); await ready(host);
            expect(surface.matches(':popover-open')).toBe(false);
            input.focus(); await userEvent.keyboard('[ArrowDown][Enter]'); await waitFor(() => expect(input.value).toBe('a'));
        }
    },
});

export const NativeDatalistKeepsOriginalFieldsAndValues = meta.story({
    render: () => ['cem-field', 'cem-text-field'].map(tag => `<form><cem-suggestions profile="native-datalist"><template><${tag} slot="editor" type="number" name="choice" label="Number" min="0" max="3" step="1"></${tag}><template slot="options"><option value="1" label="One"></option><option value="2" label="Two"></option><option value="" label="Empty"></option></template></template></cem-suggestions></form>`).join(''),
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            await whenCemRendered(host);
            const field = host.querySelector<HTMLElement>('[slot=editor]') as HTMLElement; await whenCemRendered(field);
            const input = field.querySelector('input') as HTMLInputElement, form = host.closest('form') as HTMLFormElement;
            await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(2));
            expect(input.type).toBe('number'); expect(input).toHaveAccessibleName('Number');
            expect(input.list?.options[0].value).toBe('1'); expect(input.list?.options[0].label).toBe('One');
            const ids = [...canvasElement.querySelectorAll('datalist[id]')].map(node => node.id);
            expect(new Set(ids).size).toBe(ids.length);
            expect(host.querySelector('[part=surface]')).toBeNull(); expect(input.hasAttribute('role')).toBe(false);
            expect(input.hasAttribute('aria-expanded')).toBe(false); expect(input.hasAttribute('aria-activedescendant')).toBe(false);
            input.focus(); await userEvent.type(input, '1');
            await waitFor(() => expect(new FormData(form).get('choice')).toBe('1'));
            expect(field.querySelector('input')).toBe(input); expect(document.activeElement).toBe(input);
            await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(2));
            host.setAttribute('options-state', 'pending'); await whenCemRendered(host);
            await waitFor(() => expect(input.hasAttribute('list')).toBe(false));
            expect(input.value).toBe('1');
            host.setAttribute('options-state', 'ready'); await whenCemRendered(host);
            await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(2));
            host.setAttribute('filter', 'none'); await whenCemRendered(host);
            await waitFor(() => expect(input.hasAttribute('list')).toBe(false));
            host.removeAttribute('filter'); await whenCemRendered(host);
            await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(2));
            for (const name of ['require-selection', 'selection-message', 'filter-by', 'options-query-revision']) {
                host.setAttribute(name, ''); await whenCemRendered(host); await waitFor(() => expect(input.hasAttribute('list')).toBe(false));
                host.removeAttribute(name); await whenCemRendered(host); await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(2));
            }
            form.reset(); await waitFor(() => expect(input.value).toBe(''));
            await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(2));
        }
    },
});

export const NativeProfileChangesReleaseThePreviousRoute = meta.story({
    render: () => `<cem-suggestions profile="native-datalist"><template><cem-text-field slot="editor" label="Choice"></cem-text-field><template slot="options"><option value="a" label="Alpha"></option></template></template></cem-suggestions>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-suggestions') as HTMLElement;
        await whenCemRendered(host); const field = host.querySelector<HTMLElement>('[slot=editor]') as HTMLElement; await whenCemRendered(field);
        const input = field.querySelector('input') as HTMLInputElement;
        await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(1));
        host.setAttribute('profile', 'listbox'); await whenCemRendered(host);
        await waitFor(() => expect(input.getAttribute('role')).toBe('combobox'));
        expect(input.hasAttribute('list')).toBe(false); expect(host.querySelector('datalist')).toBeNull();
        host.setAttribute('profile', 'native-datalist'); await whenCemRendered(host);
        await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(1));
        expect(input.hasAttribute('role')).toBe(false); expect(host.querySelector('[part=surface]')).toBeNull();
        host.setAttribute('profile', 'unknown'); await whenCemRendered(host);
        await waitFor(() => expect(input.hasAttribute('list')).toBe(false)); expect(input.hasAttribute('role')).toBe(false);
        host.setAttribute('profile', 'native-datalist'); await whenCemRendered(host);
        await waitFor(() => expect(input.list?.options.length, JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toBe(1));
        expect(field.querySelector('input')).toBe(input);
    },
});

export const NativeDatalistBrowserEditingEvidence = meta.story({
    render: () => ['number', 'email', 'url'].map(type => `<form aria-label="${type}"><cem-suggestions profile="native-datalist"><template><cem-field slot="editor" type="${type}" name="value" label="${type}" min="0" max="3" step="1"></cem-field><template slot="options"><option value="${type === 'number' ? '1' : type === 'email' ? 'alice@example.com' : 'https://example.com/'}" label="${type === 'number' ? 'One' : 'Example'}"></option></template></template></cem-suggestions></form>`).join(''),
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return; // Native input attempt is browser-runner-only, not device/AT acceptance.
        const { userEvent: nativeUserEvent } = await import('vitest/browser');
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-suggestions')) {
            await whenCemRendered(host); const field = host.querySelector<HTMLElement>('[slot=editor]') as HTMLElement & { value: string };
            await whenCemRendered(field); const input = field.querySelector('input') as HTMLInputElement, form = host.closest('form') as HTMLFormElement;
            await waitFor(() => expect(input.list?.options.length, JSON.stringify({type: input.type, diagnostics: storybookCemRuntime().diagnosticsFor(host), attributes: [...host.attributes].map(a => [a.name, a.value])})).toBe(1));
            const submissions: Event[] = [], events: { type: string; trusted: boolean; value: string }[] = [];
            const submit = (event: Event) => { event.preventDefault(); submissions.push(event); };
            const record = (event: Event) => events.push({ type: event.type, trusted: event.isTrusted, value: input.value });
            form.addEventListener('submit', submit); input.addEventListener('input', record); input.addEventListener('change', record);
            try {
                if (input.type === 'number') {
                    await nativeUserEvent.click(input); await nativeUserEvent.keyboard('[ArrowDown][Enter]');
                    console.info('Native datalist numeric picker attempt', JSON.stringify({ browser: navigator.userAgent, value: input.value, events }));
                    // A stepped/typed value alone is not proof that the browser chose an option.
                    expect(input.type).toBe('number'); expect(input.value).not.toBe('One');
                    await nativeUserEvent.keyboard('{Control>}a{/Control}1');
                    await waitFor(() => expect(new FormData(form).get('value')).toBe('1'));
                    expect(events.filter(event => event.type === 'input').every(event => event.trusted)).toBe(true);
                    field.value = '1.5'; expect(input.validity.stepMismatch).toBe(true);
                    field.value = '8'; expect(input.validity.rangeOverflow).toBe(true);
                    expect(new FormData(form).get('value')).toBe('8');
                } else {
                    field.value = input.type === 'email' ? '  alice@example.com  ' : '  https://example.com/  ';
                    expect(input.value).toBe(input.type === 'email' ? 'alice@example.com' : 'https://example.com/');
                    expect(new FormData(form).get('value')).toBe(input.value);
                    field.value = 'invalid'; expect(input.validity.typeMismatch).toBe(true);
                }
                expect(host.querySelector('[part=surface]')).toBeNull(); expect(input.hasAttribute('aria-activedescendant')).toBe(false);
            } finally { form.removeEventListener('submit', submit); input.removeEventListener('input', record); input.removeEventListener('change', record); }
        }
    },
});
