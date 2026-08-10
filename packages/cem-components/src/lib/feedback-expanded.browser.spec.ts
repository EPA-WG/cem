import '@epa-wg/cem-theme/styles.css';
import '../styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import feedbackExpandedFixture from '../../tests/feedback/expanded.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    expectComponentEvent,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

interface CemDialogDismissDetail {
    reason: 'cancel' | 'close';
    returnValue: string;
}

describe('feedback expanded acceptance fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-feedback-expanded-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        closeOpenDialogs();
        harness?.cleanup();
    });

    it('is declarative markup and preserves passive output when expanded changes', async () => {
        expect(feedbackExpandedFixture).not.toMatch(/<script\b/i);
        expect(feedbackExpandedFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const cases = [
            { family: 'dialog', owner: '.cem-dialog', tag: 'div' },
            { family: 'dialog-shell', owner: '.cem-dialog-shell', tag: 'div' },
            { family: 'sheet', owner: '.cem-sheet', tag: 'aside' },
        ] as const;

        for (const passiveCase of cases) {
            const baselineHost = requiredElement<HTMLElement>(
                root,
                `[data-passive-pair="${passiveCase.family}"] [data-passive="baseline"]`,
            );
            const expandedHost = requiredElement<HTMLElement>(
                root,
                `[data-passive-pair="${passiveCase.family}"] [data-passive="expanded"]`,
            );
            const baselineOwner = requiredElement<HTMLElement>(baselineHost, passiveCase.owner);
            const expandedOwner = requiredElement<HTMLElement>(expandedHost, passiveCase.owner);
            const baselineInput = requiredElement<HTMLInputElement>(baselineOwner, 'input');
            const expandedInput = requiredElement<HTMLInputElement>(expandedOwner, 'input');

            expect(baselineOwner.localName).toBe(passiveCase.tag);
            expect(semanticHtml(expandedOwner)).toBe(semanticHtml(baselineOwner));
            expect(baselineOwner.hidden).toBe(false);
            expect(expandedOwner.hidden).toBe(false);
            expect(baselineInput.value).toBe('unchanged');
            expect(expandedInput.value).toBe('unchanged');

            const ownerIdentity = baselineOwner;
            baselineHost.toggleAttribute('expanded');
            await nextRenderFrame();
            await nextRenderFrame();
            expect(requiredElement(baselineHost, passiveCase.owner)).toBe(ownerIdentity);
            expect(semanticHtml(requiredElement(baselineHost, passiveCase.owner))).toBe(semanticHtml(expandedOwner));
        }

        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('uses native owners for transient initialization and live state transitions', async () => {
        const root = await renderFixture();
        const dialogCases = [
            {
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog"]'),
            },
            {
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog-shell'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog-shell"]'),
            },
        ];

        for (const dialogCase of dialogCases) {
            const owner = requiredElement(dialogCase.host, 'dialog');
            expect(owner).toBeInstanceOf(HTMLDialogElement);
            const dialog = owner as HTMLDialogElement;
            expect(dialog.open).toBe(false);
            expect(dialog.matches(':modal')).toBe(false);
            expect(dialog.hasAttribute('role')).toBe(false);
            expect(dialog.hasAttribute('aria-modal')).toBe(false);
            expect(dialogCase.host.hasAttribute('aria-expanded')).toBe(false);
            expect(dialog.hasAttribute('aria-expanded')).toBe(false);

            setExternalExpanded(dialogCase.opener, dialogCase.host, true);
            await waitFor(() => dialog.open && dialog.matches(':modal'), 'transient dialog opens modally');
            expect(dialogCase.opener.getAttribute('aria-expanded')).toBe('true');
            setExternalExpanded(dialogCase.opener, dialogCase.host, false);
            await waitFor(() => !dialog.open && !dialog.matches(':modal'), 'transient dialog closes natively');
        }

        const initial = document.createElement('cem-dialog-shell');
        initial.setAttribute('label', 'Initially expanded shell');
        initial.setAttribute('transient', '');
        initial.setAttribute('expanded', '');
        initial.innerHTML = '<button type="button" autofocus>Initial focus</button>';
        root.append(initial);
        const initialOwner = await waitForElement<HTMLDialogElement>(initial, 'dialog');
        await waitFor(() => initialOwner.open && initialOwner.matches(':modal'), 'initial expanded state opens modally');
        initial.removeAttribute('expanded');
        await waitFor(() => !initialOwner.open, 'initial expanded dialog can transition closed');

        const sheetHost = requiredElement<HTMLElement>(root, '#feedback-transient-sheet');
        const sheet = requiredElement<HTMLElement>(sheetHost, 'aside');
        expect(sheet.hidden).toBe(true);
        sheetHost.setAttribute('expanded', '');
        await waitFor(() => !sheet.hidden, 'expanded sheet becomes visible');
        sheetHost.removeAttribute('expanded');
        await waitFor(() => sheet.hidden === true, 'collapsed sheet becomes hidden');
    });

    it('delegates modal focus, keyboard dismissal, return value, and restoration to the native dialog', async () => {
        const root = await renderFixture();
        const host = requiredElement<HTMLElement>(root, '#feedback-transient-dialog');
        const opener = requiredElement<HTMLButtonElement>(root, '[data-opener="dialog"]');
        const before = requiredElement<HTMLButtonElement>(root, '[data-outside="before"]');
        const after = requiredElement<HTMLButtonElement>(root, '[data-outside="after"]');
        const dialog = requiredElement<HTMLDialogElement>(host, 'dialog');
        const autofocus = requiredElement<HTMLButtonElement>(dialog, '[data-focus="autofocus"]');
        const dismissals: CemDialogDismissDetail[] = [];
        host.addEventListener('cem-dismiss', (event) => {
            dismissals.push((event as CustomEvent<CemDialogDismissDetail>).detail);
            opener.setAttribute('aria-expanded', 'false');
        });

        expect(dialog).toBeInstanceOf(HTMLDialogElement);
        opener.focus();
        setExternalExpanded(opener, host, true);
        await waitFor(() => dialog.open && dialog.matches(':modal'), 'dialog reaches native modal state');
        expect(document.activeElement).toBe(autofocus);
        expect(assertAccessibleName(dialog, 'Transient dialog')).toBe('Transient dialog');
        expect(opener.getAttribute('aria-controls')).toBe(host.id);
        expect(opener.getAttribute('aria-expanded')).toBe('true');

        for (let index = 0; index < 6; index += 1) {
            await userEvent.tab();
            expect([before, after]).not.toContain(document.activeElement);
            expect(document.activeElement === document.body || dialog.contains(document.activeElement)).toBe(true);
        }
        for (let index = 0; index < 6; index += 1) {
            await userEvent.tab({ shift: true });
            expect([before, after]).not.toContain(document.activeElement);
            expect(document.activeElement === document.body || dialog.contains(document.activeElement)).toBe(true);
        }

        dialog.addEventListener('cancel', (event) => event.preventDefault(), { once: true });
        await userEvent.keyboard('{Escape}');
        await nextRenderFrame();
        expect(dialog.open).toBe(true);
        expect(host.hasAttribute('expanded')).toBe(true);
        expect(dismissals).toEqual([]);

        const cancelEvent = await expectComponentEvent<CemDialogDismissDetail>(
            host,
            'cem-dismiss',
            () => userEvent.keyboard('{Escape}'),
            { detail: { reason: 'cancel', returnValue: '' } },
        );
        expect(cancelEvent.cancelable).toBe(false);
        await waitFor(() => !dialog.open && !host.hasAttribute('expanded'), 'Escape synchronizes closed host state');
        expect(document.activeElement).toBe(opener);
        expect(opener.getAttribute('aria-expanded')).toBe('false');

        opener.focus();
        setExternalExpanded(opener, host, true);
        await waitFor(() => dialog.open, 'dialog reopens for native form close');
        await expectComponentEvent<CemDialogDismissDetail>(
            host,
            'cem-dismiss',
            () => userEvent.click(requiredElement<HTMLButtonElement>(dialog, 'button[value="confirm"]')),
            { detail: { reason: 'close', returnValue: 'confirm' } },
        );
        await waitFor(() => !dialog.open && !host.hasAttribute('expanded'), 'native form close synchronizes host state');
        expect(dialog.returnValue).toBe('confirm');
        expect(document.activeElement).toBe(opener);
        expect(dismissals).toEqual([
            { reason: 'cancel', returnValue: '' },
            { reason: 'close', returnValue: 'confirm' },
        ]);
    });

    it('keeps focus-visible ownership on native dialog fallbacks and authored descendants', async () => {
        const root = await renderFixture();
        const authoredDialogCases = [
            {
                family: 'dialog',
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog"]'),
                owner: '.cem-dialog',
            },
            {
                family: 'dialog-shell',
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog-shell'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog-shell"]'),
                owner: '.cem-dialog-shell',
            },
        ];

        for (const dialogCase of authoredDialogCases) {
            const dialog = requiredElement<HTMLDialogElement>(dialogCase.host, 'dialog');
            const authoredTarget = requiredElement<HTMLElement>(dialog, '[autofocus]');
            const staticHost = requiredElement<HTMLElement>(
                root,
                `[data-passive-pair="${dialogCase.family}"] [data-passive="baseline"]`,
            );
            const staticOwner = requiredElement<HTMLElement>(staticHost, dialogCase.owner);

            await enterKeyboardModality(dialogCase.opener);
            setExternalExpanded(dialogCase.opener, dialogCase.host, true);
            await waitFor(
                () => dialog.open && document.activeElement === authoredTarget,
                `${dialogCase.family} focuses its authored target`,
            );
            expect(authoredTarget.matches(':focus-visible')).toBe(true);
            expect(dialog.matches(':focus-visible')).toBe(false);
            expect(dialogCase.host.matches(':focus-visible')).toBe(false);
            expect(staticOwner.matches(':focus-visible')).toBe(false);
            expect(staticHost.matches(':focus-visible')).toBe(false);
            expect(staticOwner.hasAttribute('tabindex')).toBe(false);
            expect(staticHost.hasAttribute('tabindex')).toBe(false);

            setExternalExpanded(dialogCase.opener, dialogCase.host, false);
            await waitFor(
                () => !dialog.open && document.activeElement === dialogCase.opener,
                `${dialogCase.family} restores its opener`,
            );
        }

        const sheetHost = requiredElement<HTMLElement>(root, '#feedback-transient-sheet');
        const sheetOpener = requiredElement<HTMLButtonElement>(root, '[data-opener="sheet"]');
        const sheet = requiredElement<HTMLElement>(sheetHost, 'aside');
        const sheetTarget = requiredElement<HTMLInputElement>(sheet, '[data-state="sheet"]');
        const sheetEvents: string[] = [];
        const sheetDismissals: unknown[] = [];
        for (const eventName of ['click', 'input', 'change']) {
            sheetHost.addEventListener(eventName, () => sheetEvents.push(eventName));
        }
        sheetHost.addEventListener('cem-dismiss', (event) => {
            sheetDismissals.push((event as CustomEvent).detail);
        });

        await enterKeyboardModality(sheetOpener);
        setExternalExpanded(sheetOpener, sheetHost, true);
        await waitFor(() => !sheet.hidden, 'sheet opens before descendant focus');
        expect(document.activeElement).toBe(sheetOpener);
        const sheetGeometry = rectSnapshot(sheet);
        const sheetHtml = sheet.outerHTML;
        const sheetRuntime = feedbackRuntimeState(runtime, sheetHost);

        await userEvent.tab();
        expect(document.activeElement).toBe(sheetTarget);
        expect(sheetTarget.matches(':focus-visible')).toBe(true);
        expect(sheet.matches(':focus-visible')).toBe(false);
        expect(sheetHost.matches(':focus-visible')).toBe(false);
        expect(sheet.hasAttribute('tabindex')).toBe(false);
        expect(sheetHost.hasAttribute('tabindex')).toBe(false);

        await userEvent.keyboard('{Escape}');
        await nextRenderFrame();
        expect(document.activeElement).toBe(sheetTarget);
        expect(sheetTarget.matches(':focus-visible')).toBe(true);
        expect(sheet.hidden).toBe(false);
        expect(sheetHost.hasAttribute('expanded')).toBe(true);
        expect(rectSnapshot(sheet)).toEqual(sheetGeometry);
        expect(sheet.outerHTML).toBe(sheetHtml);
        expect(feedbackRuntimeState(runtime, sheetHost)).toBe(sheetRuntime);
        expect(sheetEvents).toEqual([]);
        expect(sheetDismissals).toEqual([]);

        sheetOpener.focus();
        setExternalExpanded(sheetOpener, sheetHost, false);
        await waitFor(() => sheet.hidden === true, 'sheet closes after focus ownership assertions');
    });

    it('preserves native fallback focus, modal boundaries, and state through cancel and close', async () => {
        const root = await renderFixture();
        const before = requiredElement<HTMLButtonElement>(root, '[data-outside="before"]');
        const after = requiredElement<HTMLButtonElement>(root, '[data-outside="after"]');
        const fallbackCases = [
            {
                family: 'dialog',
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog-fallback'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog-fallback"]'),
            },
            {
                family: 'dialog-shell',
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog-shell-fallback'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog-shell-fallback"]'),
            },
        ];

        for (const fallbackCase of fallbackCases) {
            const dialog = requiredElement<HTMLDialogElement>(fallbackCase.host, 'dialog');
            const disabled = requiredElement<HTMLButtonElement>(dialog, '[data-focus="disabled"]');
            const dismissals: CemDialogDismissDetail[] = [];
            const mutationEvents: string[] = [];
            for (const eventName of ['click', 'input', 'change']) {
                fallbackCase.host.addEventListener(eventName, () => mutationEvents.push(eventName));
            }
            fallbackCase.host.addEventListener('cem-dismiss', (event) => {
                dismissals.push((event as CustomEvent<CemDialogDismissDetail>).detail);
                fallbackCase.opener.setAttribute('aria-expanded', 'false');
            });

            await enterKeyboardModality(fallbackCase.opener);
            setExternalExpanded(fallbackCase.opener, fallbackCase.host, true);
            await waitFor(
                () => dialog.open && dialog.matches(':modal') && document.activeElement === dialog,
                `${fallbackCase.family} focuses its native fallback owner`,
            );
            expect(dialog.matches(':focus-visible')).toBe(true);
            expect(fallbackCase.host.matches(':focus-visible')).toBe(false);
            expect(disabled.disabled).toBe(true);
            expect(document.activeElement).not.toBe(disabled);
            expect(dialog.hasAttribute('tabindex')).toBe(false);
            expect(fallbackCase.host.hasAttribute('tabindex')).toBe(false);

            for (let index = 0; index < 3; index += 1) {
                await userEvent.tab();
                expect([before, after, disabled]).not.toContain(document.activeElement);
                expect(
                    document.activeElement === document.body
                        || document.activeElement === dialog
                        || dialog.contains(document.activeElement),
                ).toBe(true);
            }
            for (let index = 0; index < 3; index += 1) {
                await userEvent.tab({ shift: true });
                expect([before, after, disabled]).not.toContain(document.activeElement);
                expect(
                    document.activeElement === document.body
                        || document.activeElement === dialog
                        || dialog.contains(document.activeElement),
                ).toBe(true);
            }

            dialog.focus();
            await nextRenderFrame();
            expect(document.activeElement).toBe(dialog);
            expect(dialog.matches(':focus-visible')).toBe(true);
            const dialogGeometry = rectSnapshot(dialog);
            const hostGeometry = rectSnapshot(fallbackCase.host);
            const dialogHtml = dialog.outerHTML;
            const hostHtml = fallbackCase.host.outerHTML;
            const runtimeState = feedbackRuntimeState(runtime, fallbackCase.host);
            const mutations: MutationRecord[] = [];
            const observer = new MutationObserver((records) => mutations.push(...records));
            observer.observe(fallbackCase.host, { attributes: true, subtree: true });

            dialog.addEventListener('cancel', (event) => event.preventDefault(), { once: true });
            await userEvent.keyboard('{Escape}');
            await nextRenderFrame();
            mutations.push(...observer.takeRecords());
            observer.disconnect();
            expect(document.activeElement).toBe(dialog);
            expect(dialog.matches(':focus-visible')).toBe(true);
            expect(dialog.open).toBe(true);
            expect(fallbackCase.host.hasAttribute('expanded')).toBe(true);
            expect(rectSnapshot(dialog)).toEqual(dialogGeometry);
            expect(rectSnapshot(fallbackCase.host)).toEqual(hostGeometry);
            expect(dialog.outerHTML).toBe(dialogHtml);
            expect(fallbackCase.host.outerHTML).toBe(hostHtml);
            expect(feedbackRuntimeState(runtime, fallbackCase.host)).toBe(runtimeState);
            expect(mutations).toEqual([]);
            expect(mutationEvents).toEqual([]);
            expect(dismissals).toEqual([]);

            await expectComponentEvent<CemDialogDismissDetail>(
                fallbackCase.host,
                'cem-dismiss',
                () => userEvent.keyboard('{Escape}'),
                { detail: { reason: 'cancel', returnValue: '' } },
            );
            await waitFor(
                () => !dialog.open && document.activeElement === fallbackCase.opener,
                `${fallbackCase.family} restores its opener after Escape`,
            );
            expect(dialog.matches(':focus-visible')).toBe(false);
            expect(fallbackCase.host.hasAttribute('expanded')).toBe(false);
            expect(fallbackCase.opener.getAttribute('aria-expanded')).toBe('false');
            expect(mutationEvents).toEqual([]);
            expect(dismissals).toEqual([{ reason: 'cancel', returnValue: '' }]);
        }
    });

    it.fails('paints only focused native dialog fallbacks with the D5 zebra outline', async () => {
        const root = await renderFixture();
        const fallbackCases = [
            {
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog-fallback'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog-fallback"]'),
            },
            {
                host: requiredElement<HTMLElement>(root, '#feedback-transient-dialog-shell-fallback'),
                opener: requiredElement<HTMLButtonElement>(root, '[data-opener="dialog-shell-fallback"]'),
            },
        ];
        const actualTreatments: FocusTreatment[] = [];
        const expectedTreatments: FocusTreatment[] = [];

        for (const fallbackCase of fallbackCases) {
            const dialog = requiredElement<HTMLDialogElement>(fallbackCase.host, 'dialog');
            await enterKeyboardModality(fallbackCase.opener);
            setExternalExpanded(fallbackCase.opener, fallbackCase.host, true);
            await waitFor(
                () => document.activeElement === dialog && dialog.matches(':focus-visible'),
                'native dialog fallback becomes keyboard-focus-visible',
            );
            actualTreatments.push(captureFocusTreatment(dialog));
            expectedTreatments.push(expectedFeedbackFocusTreatment(dialog));
            expect(fallbackCase.host.matches(':focus-visible')).toBe(false);

            setExternalExpanded(fallbackCase.opener, fallbackCase.host, false);
            await waitFor(
                () => !dialog.open && document.activeElement === fallbackCase.opener,
                'dialog closes after focus paint capture',
            );
            expect(dialog.matches(':focus-visible')).toBe(false);
        }

        expect(actualTreatments).toEqual(expectedTreatments);
    });

    it('keeps a transient sheet non-modal, focus-neutral, and application-controlled', async () => {
        const root = await renderFixture();
        const host = requiredElement<HTMLElement>(root, '#feedback-transient-sheet');
        const opener = requiredElement<HTMLButtonElement>(root, '[data-opener="sheet"]');
        const sheet = requiredElement<HTMLElement>(host, 'aside');
        const input = requiredElement<HTMLInputElement>(sheet, '[data-state="sheet"]');
        const dismissals: unknown[] = [];
        host.addEventListener('cem-dismiss', (event) => dismissals.push((event as CustomEvent).detail));

        expect(sheet.hidden).toBe(true);
        expect(sheet.getAttribute('role')).toBe('region');
        expect(sheet.hasAttribute('aria-modal')).toBe(false);
        opener.focus();
        setExternalExpanded(opener, host, true);
        await waitFor(() => !sheet.hidden, 'sheet expands through native visibility');
        expect(document.activeElement).toBe(opener);
        expect(opener.getAttribute('aria-expanded')).toBe('true');

        input.value = 'browser-owned draft';
        const ownerIdentity = sheet;
        const inputIdentity = input;
        const geometry = rectSnapshot(sheet);
        host.setAttribute('label', 'Renamed transient sheet');
        await waitFor(() => sheet.getAttribute('aria-label') === 'Renamed transient sheet', 'sheet label rerenders');
        expect(requiredElement(host, 'aside')).toBe(ownerIdentity);
        expect(requiredElement(sheet, '[data-state="sheet"]')).toBe(inputIdentity);
        expect(input.value).toBe('browser-owned draft');
        expect(rectSnapshot(sheet)).toEqual(geometry);

        await userEvent.keyboard('{Escape}');
        await nextRenderFrame();
        expect(sheet.hidden).toBe(false);
        expect(host.hasAttribute('expanded')).toBe(true);
        expect(document.activeElement).toBe(opener);
        expect(dismissals).toEqual([]);

        setExternalExpanded(opener, host, false);
        await waitFor(() => sheet.hidden === true, 'sheet collapses through native visibility');
        expect(document.activeElement).toBe(opener);
        expect(dismissals).toEqual([]);
    });

    it('preserves open-dialog identity and state while cleaning up close, replacement, and reconnect paths', async () => {
        const root = await renderFixture();
        const host = requiredElement<HTMLElement>(root, '#feedback-transient-dialog');
        const opener = requiredElement<HTMLButtonElement>(root, '[data-opener="dialog"]');
        const dialog = requiredElement<HTMLDialogElement>(host, 'dialog');
        const input = requiredElement<HTMLInputElement>(dialog, '[data-state="dialog"]');
        const dismissals: unknown[] = [];
        host.addEventListener('cem-dismiss', (event) => dismissals.push((event as CustomEvent).detail));

        expect(dialog).toBeInstanceOf(HTMLDialogElement);
        opener.focus();
        setExternalExpanded(opener, host, true);
        await waitFor(() => dialog.open && dialog.matches(':modal'), 'dialog opens before stability assertions');
        input.value = 'browser-owned dialog draft';
        input.setSelectionRange(2, 8);
        const geometry = rectSnapshot(dialog);
        const mutations: MutationRecord[] = [];
        const observer = new MutationObserver((records) => mutations.push(...records));
        observer.observe(dialog, { attributes: true });

        host.setAttribute('label', 'Renamed transient dialog');
        await waitFor(() => dialog.getAttribute('aria-label') === 'Renamed transient dialog', 'dialog label rerenders');
        await nextRenderFrame();
        mutations.push(...observer.takeRecords());
        observer.disconnect();
        expect(requiredElement(host, 'dialog')).toBe(dialog);
        expect(requiredElement(dialog, '[data-state="dialog"]')).toBe(input);
        expect(dialog.open).toBe(true);
        expect(dialog.matches(':modal')).toBe(true);
        expect(input.value).toBe('browser-owned dialog draft');
        expect(input.selectionStart).toBe(2);
        expect(input.selectionEnd).toBe(8);
        expect(rectSnapshot(dialog)).toEqual(geometry);
        expect(mutations.filter((record) => record.attributeName === 'open')).toHaveLength(0);
        expect(dismissals).toEqual([]);

        setExternalExpanded(opener, host, false);
        await waitFor(() => !dialog.open && document.activeElement === opener, 'application close restores focus');
        expect(dismissals).toEqual([]);

        setExternalExpanded(opener, host, true);
        await waitFor(() => dialog.open && dialog.matches(':modal'), 'dialog reopens before owner replacement');
        opener.setAttribute('aria-expanded', 'false');
        host.removeAttribute('transient');
        await waitFor(
            () => !dialog.open && !dialog.isConnected && host.querySelector('.cem-dialog') !== null,
            'leaving transient mode closes before replacing the native owner',
        );
        const passiveReplacement = requiredElement<HTMLElement>(host, '.cem-dialog');
        expect(passiveReplacement).not.toBeInstanceOf(HTMLDialogElement);
        expect(dismissals).toEqual([]);

        host.removeAttribute('expanded');
        host.setAttribute('transient', '');
        const replacementDialog = await waitForElement<HTMLDialogElement>(host, 'dialog');
        expect(replacementDialog).not.toBe(dialog);
        expect(replacementDialog.open).toBe(false);

        setExternalExpanded(opener, host, true);
        await waitFor(
            () => replacementDialog.open && replacementDialog.matches(':modal'),
            'replacement dialog opens before disconnect',
        );
        host.remove();
        await nextRenderFrame();
        expect(replacementDialog.open).toBe(false);
        expect(replacementDialog.matches(':modal')).toBe(false);
        expect(document.activeElement).toBe(opener);
        expect(dismissals).toEqual([]);

        root.append(host);
        await waitFor(
            () => replacementDialog.open && replacementDialog.matches(':modal'),
            'reconnect reads expanded state and reopens',
        );
        expect(requiredElement(host, 'dialog')).toBe(replacementDialog);
        host.removeAttribute('expanded');
        await waitFor(() => !replacementDialog.open, 'reconnected dialog closes cleanly');
        expect(dismissals).toEqual([]);
    });

    function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness();
        return harness.render(feedbackExpandedFixture).then(async (root) => {
            await waitForElement(root, '#feedback-transient-sheet aside');
            return root;
        });
    }
});

function setExternalExpanded(opener: HTMLButtonElement, host: HTMLElement, expanded: boolean): void {
    opener.setAttribute('aria-expanded', String(expanded));
    host.toggleAttribute('expanded', expanded);
}

function semanticHtml(element: Element): string {
    const clone = element.cloneNode(true) as Element;
    for (const candidate of [clone, ...Array.from(clone.querySelectorAll('*'))]) {
        for (const attribute of Array.from(candidate.attributes)) {
            if (attribute.name.startsWith('data-cem-')) {
                candidate.removeAttribute(attribute.name);
            }
        }
    }
    const walker = document.createTreeWalker(clone, NodeFilter.SHOW_COMMENT);
    const comments: Comment[] = [];
    while (walker.nextNode()) comments.push(walker.currentNode as Comment);
    for (const comment of comments) comment.remove();
    return clone.outerHTML.replace(/\s+/g, ' ').replace(/> </g, '><').trim();
}

function rectSnapshot(element: Element): { height: number; width: number; x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return {
        height: Math.round(rect.height * 100) / 100,
        width: Math.round(rect.width * 100) / 100,
        x: Math.round(rect.x * 100) / 100,
        y: Math.round(rect.y * 100) / 100,
    };
}

function requiredElement<T extends Element = Element>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected fixture to contain ${selector}`);
    return element;
}

async function waitForElement<T extends Element = Element>(root: ParentNode, selector: string): Promise<T> {
    await waitFor(() => root.querySelector(selector) !== null, `${selector} renders`);
    return requiredElement<T>(root, selector);
}

async function waitFor(predicate: () => boolean, label: string, frames = 120): Promise<void> {
    for (let frame = 0; frame < frames; frame += 1) {
        if (predicate()) return;
        await nextRenderFrame();
    }
    throw new Error(`${label} within ${frames} frames`);
}

function closeOpenDialogs(): void {
    for (const dialog of Array.from(document.querySelectorAll<HTMLDialogElement>('dialog[open]'))) {
        dialog.close();
    }
}

interface FocusTreatment {
    boxShadow: string;
    forcedColorAdjust: string;
    outlineColor: string;
    outlineOffset: string;
    outlineStyle: string;
    outlineWidth: string;
}

async function enterKeyboardModality(opener: HTMLButtonElement): Promise<void> {
    opener.focus();
    await userEvent.keyboard('{ArrowDown}');
    expect(document.activeElement).toBe(opener);
}

function captureFocusTreatment(element: Element): FocusTreatment {
    const styles = getComputedStyle(element);
    return {
        boxShadow: styles.boxShadow,
        forcedColorAdjust: styles.forcedColorAdjust,
        outlineColor: styles.outlineColor,
        outlineOffset: styles.outlineOffset,
        outlineStyle: styles.outlineStyle,
        outlineWidth: styles.outlineWidth,
    };
}

function expectedFeedbackFocusTreatment(element: Element): FocusTreatment {
    return {
        boxShadow: 'none',
        forcedColorAdjust: 'auto',
        outlineColor: resolveTokenColor(element, '--cem-zebra-color-1'),
        outlineOffset: resolveTokenLength(element, '--cem-stroke-indicator-offset'),
        outlineStyle: 'solid',
        outlineWidth: resolveTokenLength(element, '--cem-stroke-focus'),
    };
}

function resolveTokenColor(element: Element, tokenName: string): string {
    const styles = getComputedStyle(element);
    const tokenValue = styles.getPropertyValue(tokenName).trim();
    if (!tokenValue) throw new Error(`Expected generated color token ${tokenName}`);
    const probe = document.createElement('span');
    probe.hidden = true;
    probe.style.colorScheme = styles.colorScheme;
    probe.style.setProperty(tokenName, tokenValue);
    probe.style.color = `var(${tokenName})`;
    document.body.append(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    if (!color) throw new Error(`Expected generated color token ${tokenName} to resolve`);
    return color;
}

function resolveTokenLength(element: Element, tokenName: string): string {
    const value = getComputedStyle(element).getPropertyValue(tokenName).trim();
    if (!/^-?\d*\.?\d+px$/.test(value)) {
        throw new Error(`Expected generated length token ${tokenName}, received ${value || '<empty>'}`);
    }
    return value;
}

function feedbackRuntimeState(runtime: CemElementRuntime, host: HTMLElement): string {
    const snapshot = runtime.snapshotInstance(host);
    return JSON.stringify({
        eventPayloads: snapshot.eventPayloads,
        formData: snapshot.formData,
        payload: snapshot.payload,
        slices: snapshot.slices,
        validationState: snapshot.validationState,
    });
}
