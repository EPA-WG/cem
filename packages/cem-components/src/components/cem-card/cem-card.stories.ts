import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemStoryRuntime, createCemStoryRuntime, loadCemWorkflowDependencies, assertAccessibleName, assertAriaReferenceIntegrity, assertFocusVisible, assertLightDomRendered, nextRenderFrame } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-card.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-card', title: 'CEM Components/cem-card', loaders: [async () => { await loadCemDeclaration('cem-card', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

export const BusyContent = meta.story({
 name: "marks explicit busy cards without making nested content primitives loading owners",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-card label=\"Ordinary card\">\n                    <span slot=\"title\">Ordinary card</span>\n                    <p>Ready</p>\n                </cem-card>\n                <cem-card label=\"Initial assets\" busy>\n                    <span slot=\"title\">Initial assets</span>\n                    <p class=\"loading-message\">Loading assets\u2026</p>\n                    <cem-skeleton label=\"Asset rows\"></cem-skeleton>\n                    <cem-skeleton label=\"Asset preview\"></cem-skeleton>\n                </cem-card>\n                <cem-card label=\"Profile\" busy=\"false\">\n                    <span slot=\"title\">Profile</span>\n                    <p>Grace Hopper</p>\n                    <button type=\"button\">Edit profile</button>\n                </cem-card>\n                <cem-card label=\"Empty assets\" busy>\n                    <span slot=\"title\">Empty assets</span>\n                    <ul aria-label=\"Assets\"><li>No items</li></ul>\n                </cem-card>\n                <cem-table id=\"busy-table\" label=\"Standalone table\" busy></cem-table>\n                <cem-media-preview id=\"busy-preview\" label=\"Standalone preview\" busy></cem-media-preview>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
        const authoredFallback = document.createElement('cem-card');
        authoredFallback.setAttribute('label', 'Assets');
        authoredFallback.setAttribute('busy', '');
        authoredFallback.innerHTML = `
            <span slot="title">Assets</span>
            <p>Loading assets…</p>
            <cem-skeleton label="Asset rows"></cem-skeleton>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('Loading assets…');
        expect(authoredFallback.querySelector('cem-skeleton')?.getAttribute('label')).toBe('Asset rows');


        await waitForStateSelector(root, '#busy-preview > .cem-media-preview');

        const ordinary = harness.query<HTMLElement>('cem-card[label="Ordinary card"] > section');
        const initialHost = harness.query<HTMLElement>('cem-card[label="Initial assets"]');
        const initial = harness.query<HTMLElement>('cem-card[label="Initial assets"] > section');
        const initialHeader = harness.query<HTMLElement>('cem-card[label="Initial assets"] > section > header');
        const initialBody = harness.query<HTMLElement>('cem-card[label="Initial assets"] .cem-card__body');
        const initialMessage = harness.query<HTMLParagraphElement>('cem-card[label="Initial assets"] .loading-message');
        const skeletons = Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-card[label="Initial assets"] cem-skeleton .cem-skeleton'),
        );
        const refreshHost = harness.query<HTMLElement>('cem-card[label="Profile"]');
        const refreshCard = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const refreshHeader = harness.query<HTMLElement>('cem-card[label="Profile"] > section > header');
        const refreshBody = harness.query<HTMLElement>('cem-card[label="Profile"] .cem-card__body');
        const refreshButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const emptyHost = harness.query<HTMLElement>('cem-card[label="Empty assets"]');
        const emptyCard = harness.query<HTMLElement>('cem-card[label="Empty assets"] > section');
        const standaloneTable = harness.query<HTMLElement>('#busy-table > [role="table"]');
        const standalonePreview = harness.query<HTMLElement>('#busy-preview > .cem-media-preview');

        assertStateHostsRendered(
            harness.root,
            'cem-card, #busy-table, #busy-preview, cem-card cem-skeleton',
        );
        expect(ordinary.className).toBe('cem-card');
        expect(ordinary.getAttribute('aria-label')).toBe('Ordinary card');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.hasAttribute('aria-busy')).toBe(false);
        expect(ordinary.querySelector('.cem-card__header')?.textContent?.trim()).toBe('Ordinary card');
        expect(ordinary.querySelector('.cem-card__body')?.textContent?.trim()).toBe('Ready');

        expect(assertAccessibleName(initial, 'Initial assets')).toBe('Initial assets');
        expect(initial.getAttribute('data-state')).toBe('loading');
        expect(initial.getAttribute('aria-busy')).toBe('true');
        expect(initialHeader.className).toBe('cem-card__header');
        expect(initialBody.className).toBe('cem-card__body');
        expect(initialMessage.textContent?.trim()).toBe('Loading assets…');
        expect(initialMessage.getAttribute('role')).toBeNull();
        expect(initialMessage.getAttribute('aria-live')).toBeNull();
        expect(skeletons).toHaveLength(2);
        expect(skeletons.every((skeleton) => skeleton.getAttribute('aria-hidden') === 'true')).toBe(true);
        expect(initial.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(initial.hasAttribute('inert')).toBe(false);

        expect(refreshCard.getAttribute('data-state')).toBe('loading');
        expect(refreshCard.getAttribute('aria-busy')).toBe('true');
        expect(refreshCard.textContent).toContain('Grace Hopper');
        expect(refreshButton.disabled).toBe(false);
        expect(refreshButton.tabIndex).toBe(0);
        expect(emptyCard.getAttribute('data-state')).toBe('loading');

        for (const candidate of [standaloneTable, standalonePreview]) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }

        const refreshRect = refreshCard.getBoundingClientRect();
        expect(refreshRect.width).toBeGreaterThan(0);
        expect(refreshRect.height).toBeGreaterThan(0);
        const lifecycleEvents: string[] = [];
        for (const name of ['cem-loaded', 'cem-error', 'cem-cancel']) {
            refreshHost.addEventListener(name, () => lifecycleEvents.push(name));
        }

        await assertFocusVisible(refreshButton);
        refreshHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const settledCard = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const settledHeader = harness.query<HTMLElement>('cem-card[label="Profile"] > section > header');
        const settledBody = harness.query<HTMLElement>('cem-card[label="Profile"] .cem-card__body');
        const settledButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const settledRect = settledCard.getBoundingClientRect();
        expect(settledCard).toBe(refreshCard);
        expect(settledHeader).toBe(refreshHeader);
        expect(settledBody).toBe(refreshBody);
        expect(settledButton).toBe(refreshButton);
        expect(settledCard.hasAttribute('data-state')).toBe(false);
        expect(settledCard.hasAttribute('aria-busy')).toBe(false);
        expect(settledRect.width).toBe(refreshRect.width);
        expect(settledRect.height).toBe(refreshRect.height);
        expect(document.activeElement).toBe(settledButton);

        refreshHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const pendingAgain = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const pendingButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const pendingRect = pendingAgain.getBoundingClientRect();
        expect(pendingAgain).toBe(refreshCard);
        expect(pendingButton).toBe(refreshButton);
        expect(pendingAgain.getAttribute('data-state')).toBe('loading');
        expect(pendingAgain.getAttribute('aria-busy')).toBe('true');
        expect(pendingRect.width).toBe(refreshRect.width);
        expect(pendingRect.height).toBe(refreshRect.height);
        expect(document.activeElement).toBe(pendingButton);

        emptyHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const settledEmptyCard = harness.query<HTMLElement>('cem-card[label="Empty assets"] > section');
        expect(settledEmptyCard).toBe(emptyCard);
        expect(settledEmptyCard.hasAttribute('data-state')).toBe(false);
        expect(settledEmptyCard.hasAttribute('aria-busy')).toBe(false);

        const initialSnapshot = runtime.snapshotInstance(initialHost);
        const refreshSnapshot = runtime.snapshotInstance(refreshHost);
        expect(initialSnapshot.slices).not.toHaveProperty('busy');
        expect(initialSnapshot.slices).not.toHaveProperty('loading');
        expect(initialSnapshot.eventPayloads).not.toHaveProperty('busy');
        expect(initialSnapshot.eventPayloads).not.toHaveProperty('loading');
        expect(refreshSnapshot.slices).not.toHaveProperty('busy');
        expect(refreshSnapshot.eventPayloads).not.toHaveProperty('busy');
        expect(lifecycleEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

async function waitForStateSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        const element = root.querySelector(selector);

        if (element) {
            return element;
        }

        await nextRenderFrame();
    }

    throw new Error(`Expected state render output matching ${selector}`);
}

function assertStateHostsRendered(root: ParentNode, selector: string): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>(selector))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
}

export const WorkerRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-card-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<strong slot=\"title\">Authored title</strong><label>Retained field <input value=\"Initial\"></label>";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
        const authored = Array.from(owner.querySelectorAll('[slot], label')), input = host.querySelector('input');
        expect(owner).not.toBeNull();
        expect(owner.tabIndex).toBe(-1);
        expect(owner.hasAttribute('inert')).toBe(false);
        expect(owner.hasAttribute('aria-live')).toBe(false);
        expect(host.shadowRoot).toBeNull();
        if (input) { input.value = 'Edited'; input.focus(); }
        for (const value of ['', 'false']) {
            host.setAttribute('busy', value); host.setAttribute('empty', value); host.setAttribute('decorative', value);
            host.setAttribute('gap', 'lg'); host.setAttribute('columns', '2'); host.setAttribute('orientation', 'vertical');
            host.setAttribute('label', 'Updated label'); host.setAttribute('variant', 'caption');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner);
            expect(owner.getAttribute('data-state')).toBe('loading');
            expect(owner.getAttribute('aria-busy')).toBe('true');
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);

            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.hasAttribute('aria-busy')).toBe(false);
            host.removeAttribute('empty'); host.removeAttribute('decorative');
            await runtime.whenRenderSettled(host);
            expect(owner.hasAttribute('data-state')).toBe(false);
        }
        host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); }
        const snapshot = runtime.snapshotInstance(host);
        for (const state of ['busy', 'empty', 'decorative']) {
            expect(snapshot.slices).not.toHaveProperty(state); expect(snapshot.eventPayloads).not.toHaveProperty(state);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.remove();

 },
});

export const FallbackRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-card-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<strong slot=\"title\">Authored title</strong><label>Retained field <input value=\"Initial\"></label>";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
        const authored = Array.from(owner.querySelectorAll('[slot], label')), input = host.querySelector('input');
        expect(owner).not.toBeNull();
        expect(owner.tabIndex).toBe(-1);
        expect(owner.hasAttribute('inert')).toBe(false);
        expect(owner.hasAttribute('aria-live')).toBe(false);
        expect(host.shadowRoot).toBeNull();
        if (input) { input.value = 'Edited'; input.focus(); }
        for (const value of ['', 'false']) {
            host.setAttribute('busy', value); host.setAttribute('empty', value); host.setAttribute('decorative', value);
            host.setAttribute('gap', 'lg'); host.setAttribute('columns', '2'); host.setAttribute('orientation', 'vertical');
            host.setAttribute('label', 'Updated label'); host.setAttribute('variant', 'caption');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner);
            expect(owner.getAttribute('data-state')).toBe('loading');
            expect(owner.getAttribute('aria-busy')).toBe('true');
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);

            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.hasAttribute('aria-busy')).toBe(false);
            host.removeAttribute('empty'); host.removeAttribute('decorative');
            await runtime.whenRenderSettled(host);
            expect(owner.hasAttribute('data-state')).toBe(false);
        }
        host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); }
        const snapshot = runtime.snapshotInstance(host);
        for (const state of ['busy', 'empty', 'decorative']) {
            expect(snapshot.slices).not.toHaveProperty(state); expect(snapshot.eventPayloads).not.toHaveProperty(state);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.remove();

 },
});

export const ThemeInheritance = meta.story({
 render: () => '<div class="theme-audit"><cem-card>123.45</cem-card></div>',
 play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector<HTMLElement>('.theme-audit');
        const host = container?.querySelector<HTMLElement>('cem-card');
        if (!container || !host) throw new Error('Missing theme audit fixture');
        await whenCemRendered(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw new Error('Missing component root');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-palette-comfort', 'rgb(12, 24, 36)');
            container.style.setProperty('--cem-palette-comfort-text', 'rgb(240, 244, 248)');
            container.style.setProperty('--cem-inset-container', '23px');
            container.style.setProperty('--cem-bend-container', '7px');
            container.style.setProperty('--cem-layer-work', 'rgb(1, 2, 3) 0px 0px 0px 2px');
            container.style.setProperty('--cem-stroke-boundary', '3px');
            container.style.setProperty('--cem-typography-feature-optical-sizing', 'none');
            const style = getComputedStyle(owner);
            expect(style.backgroundColor).toBe('rgb(12, 24, 36)'); expect(style.color).toBe('rgb(240, 244, 248)');
            expect(style.boxShadow).toBe('rgb(1, 2, 3) 0px 0px 0px 2px');
            expect(style.paddingInlineStart).toBe('23px'); expect(style.borderStartStartRadius).toBe('7px'); expect(style.borderBlockStartWidth).toBe('3px');
            expect(style.fontOpticalSizing).toBe('none');
        }

 },
});

const authFormWorkflowMarkup = `<cem-card label="Sign in">
  <span slot="title">Sign in</span>
  <form>
    <cem-text-field name="email" placeholder="name@example.com">
      <span slot="label">Email</span>
      <span slot="help">Use your work address.</span>
    </cem-text-field>

    <cem-text-field name="password" type="password">
      <span slot="label">Password</span>
    </cem-text-field>

    <cem-checkbox name="remember" value="yes">Remember this device</cem-checkbox>

    <cem-action variant="primary">Continue</cem-action>
  </form>
</cem-card>
`;
export const AuthFormWorkflow = meta.story({
    name: 'renders the auth form workflow without app JavaScript',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${authFormWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(authFormWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(authFormWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(authFormWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-action button');

        const card = workflowQuery<HTMLElement>(canvasElement, 'cem-card section');
        const form = workflowQuery<HTMLFormElement>(canvasElement, 'form');
        const email = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="email"] input');
        const password = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="password"] input');
        const remember = workflowQuery<HTMLInputElement>(canvasElement, 'cem-checkbox input[name="remember"]');
        const submit = workflowQuery<HTMLButtonElement>(canvasElement, 'cem-action button');

        expect(assertAccessibleName(card, 'Sign in')).toBe('Sign in');
        expect(form).toBeInstanceOf(HTMLFormElement);
        expect(email.getAttribute('placeholder')).toBe('name@example.com');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(password.type).toBe('password');
        expect(assertAccessibleName(password, 'Password')).toBe('Password');
        expect(remember.type).toBe('checkbox');
        expect(remember.getAttribute('value')).toBe('yes');
        expect(assertAccessibleName(remember, 'Remember this device')).toBe('Remember this device');
        expect(submit.type).toBe('button');
        expect(assertAccessibleName(submit, 'Continue')).toBe('Continue');
        assertWorkflowIntegrity(canvasElement);
    },
});

const WORKFLOW_RENDER_TIMEOUT_MS = 3000;
function workflowQuery<T extends Element = Element>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected workflow fixture to contain ${selector}`);
    return element;
}
async function waitForWorkflowSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + WORKFLOW_RENDER_TIMEOUT_MS;
    while (Date.now() < deadline) {
        const found = root.querySelector(selector);
        if (found) {
            return found;
        }
        await nextRenderFrame();
    }
    throw new Error(`Expected workflow render output matching ${selector}`);
}

async function waitForWorkflowText(root: ParentNode, selector: string, text: string): Promise<Element> {
    const deadline = Date.now() + WORKFLOW_RENDER_TIMEOUT_MS;
    while (Date.now() < deadline) {
        const found = root.querySelector(selector);
        if (found?.textContent?.includes(text)) {
            return found;
        }
        await nextRenderFrame();
    }
    throw new Error(`Expected workflow render output matching ${selector} to contain ${text}`);
}

function assertWorkflowIntegrity(root: ParentNode): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>('cem-card, cem-list, cem-action, cem-text-field, cem-textarea, cem-checkbox, cem-radio, cem-switch, cem-app-bar, cem-tabs, cem-table, cem-media-preview, cem-alert, cem-toast, cem-progress, cem-skeleton, cem-dialog, cem-sheet, cem-avatar, cem-icon-button, cem-chip, cem-badge'))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
    expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
}

const registrationWorkflowMarkup = `<cem-card label="Create account">
  <span slot="title">Create account</span>
  <form>
    <cem-text-field name="full-name" required>
      <span slot="label">Full name</span>
      <span id="registration-name-help" slot="help">Use your legal name.</span>
    </cem-text-field>

    <cem-text-field
      name="email"
      placeholder="name@example.com"
      required
      invalid="true"
      describedby="registration-email-help"
      error="registration-email-error"
    >
      <span slot="label">Email</span>
      <span id="registration-email-help" slot="help">Use your work address.</span>
    </cem-text-field>
    <p id="registration-email-error">Email is required.</p>

    <cem-text-field name="password" type="password" required readonly value="temporary-pass">
      <span slot="label">Password</span>
      <span id="registration-password-help" slot="help">Use at least 12 characters.</span>
    </cem-text-field>

    <cem-checkbox
      name="terms"
      value="accepted"
      required
      invalid="true"
    >Accept terms</cem-checkbox>

    <cem-alert tone="warning" role="alert">Complete required fields before continuing.</cem-alert>
    <cem-progress label="Registration progress" value="65" max="100"></cem-progress>
    <cem-action variant="primary" pending="true" disabled>Create account</cem-action>
  </form>
</cem-card>
`;
export const RegistrationWorkflow = meta.story({
    name: 'renders the registration auth workflow with required, invalid, and loading states',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${registrationWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(registrationWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(registrationWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(registrationWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-progress progress');

        const card = workflowQuery<HTMLElement>(canvasElement, 'cem-card section');
        const fullName = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="full-name"] input');
        const email = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="email"] input');
        const password = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="password"] input');
        const terms = workflowQuery<HTMLInputElement>(canvasElement, 'cem-checkbox input[name="terms"]');
        const alert = workflowQuery<HTMLElement>(canvasElement, 'cem-alert [role="alert"]');
        const progress = workflowQuery<HTMLProgressElement>(canvasElement, 'cem-progress progress');
        const submit = workflowQuery<HTMLButtonElement>(canvasElement, 'cem-action button');

        expect(assertAccessibleName(card, 'Create account')).toBe('Create account');
        expect(fullName.required).toBe(true);
        expect(assertAccessibleName(fullName, 'Full name')).toBe('Full name');
        expect(email.required).toBe(true);
        expect(email.getAttribute('aria-invalid')).toBe('true');
        expect(email.getAttribute('aria-describedby')).toBe('registration-email-help');
        expect(email.getAttribute('aria-errormessage')).toBe('registration-email-error');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(password.type).toBe('password');
        expect(password.required).toBe(true);
        expect(password.readOnly).toBe(true);
        expect(assertAccessibleName(password, 'Password')).toBe('Password');
        expect(terms.required).toBe(true);
        expect(terms.getAttribute('aria-invalid')).toBe('true');
        expect(assertAccessibleName(terms, 'Accept terms')).toBe('Accept terms');
        expect(alert.textContent?.trim()).toBe('Complete required fields before continuing.');
        expect(assertAccessibleName(progress, 'Registration progress')).toBe('Registration progress');
        expect(progress.getAttribute('value')).toBe('65');
        expect(submit.disabled).toBe(true);
        expect(submit.getAttribute('aria-busy')).toBe('true');
        expect(assertAccessibleName(submit, 'Create account')).toBe('Create account');
        assertWorkflowIntegrity(canvasElement);
    },
});

const passwordResetWorkflowMarkup = `<cem-card label="Reset password">
  <span slot="title">Reset password</span>
  <form>
    <cem-text-field
      name="email"
      placeholder="name@example.com"
      required
      invalid="true"
      describedby="password-reset-email-help"
      error="password-reset-email-error"
    >
      <span slot="label">Email</span>
      <span id="password-reset-email-help" slot="help">Enter the email used for your account.</span>
    </cem-text-field>
    <p id="password-reset-email-error">Account email is required.</p>

    <cem-alert tone="info" role="status">We will send a secure reset link.</cem-alert>
    <cem-progress label="Reset request progress" value="25" max="100"></cem-progress>
    <cem-action variant="primary" pending="true" disabled>Send reset link</cem-action>
  </form>
</cem-card>
`;
export const PasswordResetWorkflow = meta.story({
    name: 'renders the password reset workflow with help, error, and loading feedback',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${passwordResetWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(passwordResetWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(passwordResetWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(passwordResetWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-progress progress');

        const card = workflowQuery<HTMLElement>(canvasElement, 'cem-card section');
        const email = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="email"] input');
        const alert = workflowQuery<HTMLElement>(canvasElement, 'cem-alert [role="status"]');
        const progress = workflowQuery<HTMLProgressElement>(canvasElement, 'cem-progress progress');
        const submit = workflowQuery<HTMLButtonElement>(canvasElement, 'cem-action button');

        expect(assertAccessibleName(card, 'Reset password')).toBe('Reset password');
        expect(email.required).toBe(true);
        expect(email.getAttribute('placeholder')).toBe('name@example.com');
        expect(email.getAttribute('aria-invalid')).toBe('true');
        expect(email.getAttribute('aria-describedby')).toBe('password-reset-email-help');
        expect(email.getAttribute('aria-errormessage')).toBe('password-reset-email-error');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(alert.textContent?.trim()).toBe('We will send a secure reset link.');
        expect(assertAccessibleName(progress, 'Reset request progress')).toBe('Reset request progress');
        expect(progress.getAttribute('value')).toBe('25');
        expect(submit.disabled).toBe(true);
        expect(submit.getAttribute('aria-busy')).toBe('true');
        expect(assertAccessibleName(submit, 'Send reset link')).toBe('Send reset link');
        assertWorkflowIntegrity(canvasElement);
    },
});

const discussionThreadWorkflowMarkup = `<cem-card label="Discussion thread">
  <span slot="title">Discussion thread</span>
  <cem-list label="Messages">
    <li>
      <cem-card label="Ada message">
        <span slot="title">Ada</span>
        <p>Can we publish the package today?</p>
        <cem-badge tone="info">New</cem-badge>
      </cem-card>
    </li>
    <li>
      <cem-card label="Grace message">
        <span slot="title">Grace</span>
        <p>The verification gate is green.</p>
      </cem-card>
    </li>
  </cem-list>
  <cem-textarea name="reply">
    <span slot="label">Reply</span>
  </cem-textarea>
  <cem-toast>Draft saved</cem-toast>
  <cem-alert tone="warning" role="alert">Resolve mentions before posting.</cem-alert>
  <cem-action variant="primary">Post reply</cem-action>
</cem-card>
`;
export const DiscussionThreadWorkflow = meta.story({
    name: 'renders the discussion thread workflow without app JavaScript',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${discussionThreadWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(discussionThreadWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(discussionThreadWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(discussionThreadWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-action button');
        await waitForWorkflowText(canvasElement, 'cem-list', 'The verification gate is green.');

        const thread = workflowQuery<HTMLElement>(canvasElement, 'cem-card section');
        const messages = workflowQuery<HTMLUListElement>(canvasElement, 'cem-list ul');
        const reply = workflowQuery<HTMLTextAreaElement>(canvasElement, 'cem-textarea[name="reply"] textarea');
        const toast = workflowQuery<HTMLElement>(canvasElement, 'cem-toast [role="status"]');
        const alert = workflowQuery<HTMLElement>(canvasElement, 'cem-alert [role="alert"]');
        const post = workflowQuery<HTMLButtonElement>(canvasElement, 'cem-action button');

        expect(assertAccessibleName(thread, 'Discussion thread')).toBe('Discussion thread');
        expect(assertAccessibleName(messages, 'Messages')).toBe('Messages');
        expect(messages.querySelectorAll('li')).toHaveLength(2);
        expect(messages.textContent).toContain('The verification gate is green.');
        expect(assertAccessibleName(reply, 'Reply')).toBe('Reply');
        expect(toast.textContent?.trim()).toBe('Draft saved');
        expect(alert.textContent?.trim()).toBe('Resolve mentions before posting.');
        expect(assertAccessibleName(post, 'Post reply')).toBe('Post reply');
        assertWorkflowIntegrity(canvasElement);
    },
});
