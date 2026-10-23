import { expect, userEvent } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-recovery-status.xhtml?raw';

const meta = preview.meta({ component: 'cem-recovery-status', title: 'CEM Components/cem-recovery-status',
    loaders: [async () => { await loadCemDeclaration('cem-recovery-status', declarationSource); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw Error(`Missing ${selector}`); return node;
}
export const StatusAndDisclosure = meta.story({
    render: () => '<cem-recovery-status></cem-recovery-status>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-recovery-status'); await whenCemRendered(host);
        const details = required<HTMLDetailsElement>(host, 'details'), summary = required<HTMLElement>(host, 'summary');
        const status = required(host, '[role=status]');
        expect(status.getAttribute('aria-live')).toBe('polite'); expect(status.getAttribute('aria-atomic')).toBe('true');
        expect(summary.textContent).toContain('Recovery · Not configured'); expect(details.open).toBe(false);
        await userEvent.click(summary); await new Promise(resolve => setTimeout(resolve, 0)); await whenCemRendered(host); expect(details.open).toBe(true);
        expect(details.textContent).toContain('Entered passwords or one-time codes block recovery');
        expect(details.textContent).toContain('Automatic recovery is off');
        for (const [state, label] of [['observing', 'Status observed'], ['busy', 'Work in progress'], ['blocked', 'Blocked'], ['unavailable', 'Unavailable'], ['ready-password=secret', 'Unavailable']]) {
            host.setAttribute('state', state); host.setAttribute('reason', 'private-error-secret'); await whenCemRendered(host);
            expect(summary.textContent).toContain(label); expect(details.open).toBe(true);
            expect(host.querySelector('details')).toBe(details); expect(host.querySelector('summary')).toBe(summary);
            expect(details.textContent).not.toContain('private-error-secret'); expect(summary.textContent).not.toContain('ready-password=secret');
        }
        host.setAttribute('label', 'Workspace recovery'); host.setAttribute('aria-label', 'Recovery diagnostics'); await whenCemRendered(host);
        expect(summary.textContent).toContain('Workspace recovery'); expect(summary.getAttribute('aria-label')).toBe('Recovery diagnostics');
        expect(host.querySelector('button, input, textarea, a')).toBeNull(); expect(host.shadowRoot).toBeNull();
    },
});
export const ReasonsAndCounts = meta.story({
    render: () => '<cem-recovery-status state="blocked" runtime-count="3" owner-count="5" acknowledged-count="4" admission-count="2" unknown-operation-count="1" unsupported-owner-count="1"><strong>Session</strong></cem-recovery-status>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-recovery-status'); await whenCemRendered(host);
        expect(required(host, 'summary strong').textContent).toBe('Session');
        expect(host.textContent).toContain('Runtime registrations: 3. Owner publications: 4 / 5. Active admissions: 2. Unknown operations: 1. Owners without adapters: 1.');
        for (const [reason, text] of [['observed', 'eligibility has not been checked'], ['preparing', 'capture is in progress'], ['admissions', 'has not drained'], ['unknown-operations', 'requires reconciliation'], ['owner-adapters', 'no resume adapter'], ['owner-publications', 'Waiting for all owner publications'], ['scope-unavailable', 'inspection is unavailable']]) {
            host.setAttribute('reason', reason); await whenCemRendered(host); expect(host.textContent).toContain(text);
        }
    },
});
async function retention(canvasElement: HTMLElement, fallback: boolean) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `cem-recovery-status-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
            const host = document.createElement(tag); canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const summary = required<HTMLElement>(host, 'summary'), details = required<HTMLDetailsElement>(host, 'details');
                await userEvent.click(summary); await new Promise(resolve => setTimeout(resolve, 0)); await runtime.whenRenderSettled(host); summary.focus();
                host.setAttribute('state', 'busy'); host.setAttribute('reason', 'admissions'); host.setAttribute('admission-count', '1');
                await runtime.whenRenderSettled(host);
                expect(host.querySelector('summary')).toBe(summary); expect(details.open).toBe(true); expect(document.activeElement).toBe(summary);
                expect(summary.textContent).toContain('Work in progress');
                for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
                    host.className = `cem-theme-${mode}`;
                    host.style.setProperty('--cem-typography-ui-font-size', '19px');
                    host.style.setProperty('--cem-palette-comfort-text', 'rgb(24, 36, 48)');
                    host.style.setProperty('--cem-coupling-zone-min', '57px');
                    expect(getComputedStyle(summary).fontSize).toBe('19px');
                    expect(getComputedStyle(summary).color).toBe('rgb(24, 36, 48)');
                    expect(getComputedStyle(summary).minBlockSize).toBe('57px');
                }
                host.setAttribute('dir', 'rtl'); await runtime.whenRenderSettled(host); expect(getComputedStyle(summary).direction).toBe('rtl');
                host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
}
export const WorkerRetention = meta.story({ render: () => '<div></div>', play: async ({ canvasElement }) => retention(canvasElement, false) });
export const FallbackRetention = meta.story({ render: () => '<div></div>', play: async ({ canvasElement }) => retention(canvasElement, true) });
