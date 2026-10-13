import { expect, waitFor } from 'storybook/test';
import preview, { cemComponentDeclarationSource, createCemStoryRuntime } from '../../.storybook/preview.js';
import { mountCemView, type CemHostCommandRequest } from './host-view.js';
import { connectCemNativeSurface } from './native-surface.js';

const meta = preview.meta({ title: 'Runtime/Declarative host-service views' });

async function contract(canvasElement: HTMLElement, fallback: boolean) {
    const { runtime, scope } = createCemStoryRuntime(canvasElement, fallback);
    const tag = `host-view-${fallback ? 'fallback' : 'worker'}`;
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.type = 'text/cem-ml';
    template.textContent = `
{input @aria-label=Draft @value="initial" @host-command=edit @host-command-event=input}
{button @type=button @host-command=save @disabled={if datadom.slices.busy { true } else { null }} | Save}
{button @type=button @host-command=missing | Unavailable}
{button @type=button @host-command=fail | Fail}
{button @type=button @host-command=reject | Reject}
{form @host-command=submit @host-command-event=submit |
    {input @name=mode @value=ast}
    {input @name=mode @value=events}
    {input @name=ignored @value=disabled @disabled}
    {input @name=id @value=selected-page}
    {input @name=__proto__ @value=safe}
    {button @type=submit @name=action @value=run | Submit}}
{form @host-command=missing @host-command-event=submit}
{output | {$datadom.slices.status ?? "Waiting"}}
{slot}`;
    declaration.append(template);
    expect(runtime.registerDeclaration(declaration)).toBe(true);
    await runtime.whenDeclarationSettled(declaration);
    const calls: unknown[] = [];
    const root = canvasElement.querySelector('section') as HTMLElement;
    const view = mountCemView(runtime, {
        root, tag, state: { status: 'Ready', busy: false },
        commands: {
            save: request => { calls.push(request); },
            edit: request => { calls.push(request); },
            fail: () => { throw new Error('Synchronous failure'); },
            reject: () => Promise.reject(new Error('Asynchronous failure')),
            submit: request => { calls.push(request); },
        },
    });
    try {
        await view.whenSettled();
        expect(view.element.querySelector('output')?.textContent).toBe('Ready');
        const input = view.element.querySelector('input') as HTMLInputElement;
        const save = view.element.querySelector('button') as HTMLButtonElement;
        input.focus(); input.value = 'edited'; input.setSelectionRange(1, 4);
        input.dispatchEvent(new Event('input', { bubbles: true }));
        expect(calls).toEqual([{ command: 'edit', eventType: 'input', value: 'edited', isTrusted: false }]);
        save.click();
        // Invocation happens on the native event stack, retaining browser gesture authority.
        expect(calls).toHaveLength(2);
        expect(calls[1]).toEqual({ command: 'save', eventType: 'click', value: null, isTrusted: false });
        view.update({ status: 'Changed', busy: true });
        await view.whenSettled();
        expect(view.element.querySelector('input')).toBe(input);
        expect(view.element.querySelector('button')).toBe(save);
        expect(input.value).toBe('edited');
        expect([input.selectionStart, input.selectionEnd]).toEqual([1, 4]);
        expect(document.activeElement).toBe(input);
        expect(save.disabled).toBe(true);
        save.click();
        view.element.querySelector<HTMLButtonElement>('[host-command="missing"]')?.click();
        expect(calls).toHaveLength(2);

        const slot = document.createElement('section');
        view.element.append(slot);
        const nestedCalls: unknown[] = [];
        const nested = mountCemView(runtime, { root: slot, tag, commands: { save: request => nestedCalls.push(request) } });
        await nested.whenSettled(); await view.whenSettled();
        nested.element.querySelector<HTMLButtonElement>('button')?.click();
        expect(nestedCalls).toHaveLength(1);
        expect(calls).toHaveLength(2);
        view.update({ status: 'Retain nested view' }); await view.whenSettled();
        expect(slot.contains(nested.element)).toBe(true);
        expect(view.element.contains(nested.element)).toBe(true);
        nested.dispose();

        view.element.querySelector<HTMLButtonElement>('[host-command="fail"]')?.click();
        view.element.querySelector<HTMLButtonElement>('[host-command="reject"]')?.click();
        await Promise.resolve();
        expect(view.errors.map(error => error.message)).toEqual(['Synchronous failure', 'Asynchronous failure']);
        const submit = new Event('submit', { bubbles: true, cancelable: true });
        view.element.querySelector('form')?.dispatchEvent(submit);
        expect(submit.defaultPrevented).toBe(true);
        expect(calls).toHaveLength(3);
        const form = view.element.querySelector('form') as HTMLFormElement;
        expect(calls[2]).toMatchObject({ fields: { mode: ['ast', 'events'], id: ['selected-page'], ['__proto__']: ['safe'] } });
        const fields = (calls[2] as { fields: Record<string, string[]> }).fields;
        expect(Object.keys(fields)).toEqual(['mode', 'id', '__proto__']);
        expect(Object.isFrozen(fields)).toBe(true);
        expect(Object.isFrozen(fields.mode)).toBe(true);
        form.querySelector<HTMLButtonElement>('button')?.click();
        expect(calls).toHaveLength(4);
        expect(calls[3]).toMatchObject({ fields: { action: ['run'] } });
        const foreign = document.createElement('input');
        form.id = `host-command-form-${fallback}`;
        foreign.setAttribute('form', form.getAttribute('id')!); foreign.name = 'foreign'; foreign.value = 'private';
        canvasElement.append(foreign);
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
        expect(calls).toHaveLength(4);
        expect(view.errors.at(-1)?.message).toMatch(/same view/);
        foreign.remove();
        const nestedRoot = document.createElement('section'); form.append(nestedRoot);
        const formChild = mountCemView(runtime, { root: nestedRoot, tag });
        await formChild.whenSettled();
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
        expect(calls).toHaveLength(4);
        expect(view.errors.at(-1)?.message).toMatch(/same view/);
        formChild.dispose(); nestedRoot.remove();
        const file = document.createElement('input'); file.type = 'file'; file.name = 'attachment'; form.append(file);
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
        expect(calls).toHaveLength(4);
        expect(view.errors.at(-1)?.message).toMatch(/only string values/);
        file.remove();
        const unknown = new Event('submit', { bubbles: true, cancelable: true });
        view.element.querySelectorAll('form')[1].dispatchEvent(unknown);
        expect(unknown.defaultPrevented).toBe(false);
        const revokedRoot = document.createElement('section'); canvasElement.append(revokedRoot);
        const revoked = mountCemView(runtime, { root: revokedRoot, tag, commands: { submit: request => calls.push(request) } });
        await revoked.whenSettled();
        const revokedForm = revoked.element.querySelector('form') as HTMLFormElement;
        revokedForm.addEventListener('formdata', () => revoked.dispose(), { once: true });
        revokedForm.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
        expect(revoked.element.isConnected).toBe(false);
        expect(calls).toHaveLength(4);
        revokedRoot.remove();
        scope.dispose();
        save.disabled = false; save.click();
        expect(calls).toHaveLength(4);
        view.dispose();
        save.disabled = false; save.click();
        expect(calls).toHaveLength(4);
        expect(view.update({ status: 'Disposed' })).toBe(false);
        expect(root.children).toHaveLength(0);
        expect(runtime.diagnosticsFor(declaration).filter(({ severity }) => severity === 'error' || severity === 'fatal')).toEqual([]);
    } finally {
        view.dispose(); scope.dispose();
    }
}

export const Worker = meta.story({ render: () => '<section></section>', play: async ({ canvasElement }) => contract(canvasElement, false) });
export const Fallback = meta.story({ render: () => '<section></section>', play: async ({ canvasElement }) => contract(canvasElement, true) });

async function surfaceContract(canvasElement: HTMLElement, fallback: boolean) {
    const { runtime, scope } = createCemStoryRuntime(canvasElement, fallback);
    const tag = `host-surface-${fallback ? 'fallback' : 'worker'}`;
    const declaration = document.createElement('div'); declaration.setAttribute('tag', tag);
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = `{dialog @mode=modal @aria-label=Task @host-command=dismiss @host-command-event=cem-close |
        {button | Inside}{dialog @aria-label=Nested | {button | Nested action}}}`;
    declaration.append(template);
    expect(runtime.registerDeclaration(declaration)).toBe(true);
    await runtime.whenDeclarationSettled(declaration);
    const calls: unknown[] = [];
    const view = mountCemView(runtime, { root: canvasElement.querySelector('section') as HTMLElement, tag,
        commands: { dismiss: request => calls.push(request) } });
    await view.whenSettled();
    const owner = view.element.querySelector('dialog') as HTMLDialogElement;
    const child = owner.querySelector('dialog') as HTMLDialogElement;
    const surface = connectCemNativeSurface(owner), nested = connectCemNativeSurface(child);
    const cancel = (event: Event) => event.preventDefault();
    try {
        expect(surface.open()).toBe(true);
        expect(owner.matches(':modal')).toBe(true);
        expect(nested.open()).toBe(true);
        expect(nested.requestClose()).toBe(true);
        expect(calls).toHaveLength(0); // Nested surface dismissal cannot cancel its parent task.
        owner.addEventListener('cem-before-close', cancel);
        expect(surface.requestClose()).toBe(false);
        expect(owner.open).toBe(true);
        expect(calls).toHaveLength(0);
        owner.removeEventListener('cem-before-close', cancel);
        owner.close();
        await waitFor(() => expect(calls).toHaveLength(1));
        expect(calls[0]).toEqual({ command: 'dismiss', eventType: 'cem-close', value: null, isTrusted: false });
        expect(Object.isFrozen(calls[0])).toBe(true);
        expect(surface.open()).toBe(true);
        view.update({ unrelated: true }); await view.whenSettled();
        expect(view.element.querySelector('dialog')).toBe(owner);
        expect(owner.matches(':modal')).toBe(true);
        view.dispose();
        surface.requestClose();
        owner.dispatchEvent(new CustomEvent('cem-close', { bubbles: true }));
        expect(calls).toHaveLength(1);
        expect(view.errors).toEqual([]);
    } finally { nested.disconnect(); surface.disconnect(); view.dispose(); scope.dispose(); }
}

export const SurfaceWorker = meta.story({ render: () => '<section></section>', play: async ({ canvasElement }) => surfaceContract(canvasElement, false) });
export const SurfaceFallback = meta.story({ render: () => '<section></section>', play: async ({ canvasElement }) => surfaceContract(canvasElement, true) });

async function treeContract(canvasElement: HTMLElement, fallback: boolean) {
    const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
    const treeTag = `host-tree-${crypto.randomUUID()}`;
    const treeSource = await declare(await cemComponentDeclarationSource('cem-tree'), treeTag);
    const tag = `tree-command-view-${crypto.randomUUID()}`;
    const declaration = document.createElement('div'); declaration.setAttribute('tag', tag);
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = `
{${treeTag} @label=Resources @selection=single @selected-values=current @expanded-values=group @host-command=open @host-command-event=cem-tree-activate |
    {cem-tree-item @value=group @label=Group |
        {cem-tree-item @value=source @label=Source}
        {cem-tree-item @value=blocked @label=Blocked @disabled}}
    {cem-tree-item @value=current @label=Current}}
{button @type=button @host-command=save | Save}
{section @host-command=open @host-command-event=cem-tree-activate |
    {${treeTag} @label=Unbound | {cem-tree-item @value=other @label=Other}}}
{slot}`;
    declaration.append(template);
    expect(runtime.registerDeclaration(declaration)).toBe(true);
    await runtime.whenDeclarationSettled(declaration);
    const calls: CemHostCommandRequest[] = [];
    const view = mountCemView(runtime, {
        root: canvasElement.querySelector('section') as HTMLElement, tag,
        commands: {
            open: request => { calls.push(request); },
            save: request => { calls.push(request); },
        },
    });
    const required = <T extends HTMLElement = HTMLElement>(root: ParentNode, selector: string): T => {
        const node = root.querySelector<T>(selector);
        if (!node) throw new Error(`Missing ${selector}`);
        return node;
    };
    const emit = (owner: Element, detail: unknown = { value: 'source' }, type = 'cem-tree-activate') => {
        const event = new CustomEvent(type, { bubbles: true, composed: true, cancelable: true, detail });
        owner.dispatchEvent(event);
        return event;
    };
    const native = import.meta.env.MODE === 'test' ? await import('vitest/browser') : undefined;
    const click = async (button: HTMLElement) => {
        if (native) await native.userEvent.click(button); else button.click();
        await view.whenSettled();
    };
    try {
        await view.whenSettled();
        const owner = required(view.element, `${treeTag}[label=Resources]`);
        const node = (value: string) => required<HTMLButtonElement>(owner, `[data-tree-value="${value}"]`);
        expect(required(owner, '[role=tree]')).toHaveAccessibleName('Resources');
        await click(node('source'));
        expect(calls).toEqual([{ command: 'open', eventType: 'cem-tree-activate', value: 'source', isTrusted: false }]);
        expect(Object.isFrozen(calls[0])).toBe(true);
        expect(structuredClone(calls[0])).toEqual(calls[0]);
        expect(owner.getAttribute('selected-values')).toBe('current');

        if (native) {
            node('source').focus();
            await native.userEvent.keyboard('{Enter}');
            await native.userEvent.keyboard(' ');
            expect(calls.map(request => request.value)).toEqual(['source', 'source', 'source']);
            await native.userEvent.keyboard('{ArrowDown}');
            await view.whenSettled();
            expect(document.activeElement).toBe(node('current'));
        }
        calls.length = 0;
        await click(node('group')); // Expansion is local tree behavior, not a host command.
        expect(owner.getAttribute('expanded-values')).toBe('');
        expect(calls).toEqual([]);
        await click(node('group'));
        node('blocked').click();
        node('current').focus();
        expect(calls).toEqual([]);
        expect(owner.getAttribute('selected-values')).toBe('current');
        view.update({ unrelated: true }); await view.whenSettled();
        expect(required(view.element, `${treeTag}[label=Resources]`)).toBe(owner);
        expect(document.activeElement).toBe(node('current'));

        // Only the bound owner can report its value; descendants and nested owners cannot.
        emit(node('source'));
        const unbound = required(view.element, `${treeTag}[label=Unbound]`);
        await click(required(unbound, 'button'));
        emit(owner, { value: 'source' }, 'cem-tree-toggle');
        owner.setAttribute('host-command-event', 'cem-unknown');
        emit(owner, { value: 'source' }, 'cem-unknown');
        owner.removeAttribute('host-command-event');
        emit(owner);
        owner.setAttribute('host-command-event', 'cem-tree-activate');
        owner.setAttribute('host-command', 'missing'); emit(owner);
        owner.setAttribute('host-command', 'open');
        owner.setAttribute('disabled', 'false'); emit(owner); owner.removeAttribute('disabled');
        owner.addEventListener('cem-tree-activate', event => event.preventDefault(), { once: true });
        expect(emit(owner).defaultPrevented).toBe(true);
        expect(calls).toEqual([]);
        expect(view.errors).toEqual([]);

        let reads = 0;
        const accessor = { get value() { reads += 1; return 'source'; } };
        const malformed = [null, 'source', 7, {}, { value: 7 }, { value: '' }, { value: 'two words' },
            { value: '\tsource' }, { value: document.body }, ['source'],
            Object.assign([], { value: 'source' }), Object.create({ value: 'source' }), accessor];
        for (const detail of malformed) emit(owner, detail);
        owner.dispatchEvent(new Event('cem-tree-activate', { bubbles: true }));
        expect(calls).toEqual([]);
        expect(reads).toBe(0);
        expect(view.errors).toHaveLength(malformed.length + 1);
        expect(view.errors.every(error => error instanceof TypeError && /tree.*value/i.test(error.message))).toBe(true);
        // Unrelated detail is never read or transported, even if it contains live objects.
        const detail = { value: 'source', node: owner, get unused() { reads += 1; return owner; } };
        emit(owner, detail); detail.value = 'changed';
        expect(calls).toEqual([{ command: 'open', eventType: 'cem-tree-activate', value: 'source', isTrusted: false }]);
        expect(reads).toBe(0);
        calls.length = 0;

        const button = required<HTMLButtonElement>(view.element, 'button[host-command=save]');
        button.click(); // The command must arrive before synchronous dispatch returns.
        expect(calls).toEqual([{ command: 'save', eventType: 'click', value: null, isTrusted: false }]);
        calls.length = 0;
        await click(button);
        expect(calls).toEqual([{ command: 'save', eventType: 'click', value: null, isTrusted: !!native }]);
        calls.length = 0;

        const nestedRoot = document.createElement('section'); view.element.append(nestedRoot);
        const nestedCalls: CemHostCommandRequest[] = [];
        const nested = mountCemView(runtime, { root: nestedRoot, tag, commands: { open: request => nestedCalls.push(request) } });
        try {
            await nested.whenSettled();
            const nestedTree = required(nested.element, `${treeTag}[label=Resources]`);
            emit(nestedTree);
            expect(nestedCalls).toHaveLength(1); expect(calls).toEqual([]);
            nestedTree.setAttribute('host-command', 'save'); emit(nestedTree);
            expect(nestedCalls).toHaveLength(1); expect(calls).toEqual([]);
            nestedTree.setAttribute('host-command', 'open');
            nested.dispose(); nestedRoot.append(nested.element);
            emit(nestedTree);
            expect(nestedCalls).toHaveLength(1); expect(calls).toEqual([]);
        } finally { nested.dispose(); nestedRoot.remove(); }

        // Projection can encounter a proxy trap; recheck authority before invoking a service.
        const revokedRoot = document.createElement('section'); view.element.append(revokedRoot);
        const revoked = mountCemView(runtime, { root: revokedRoot, tag, commands: { open: request => calls.push(request) } });
        try {
            await revoked.whenSettled();
            const revoke = new Proxy({ value: 'source' }, {
                getOwnPropertyDescriptor(target, key) { revoked.dispose(); return Reflect.getOwnPropertyDescriptor(target, key); },
            });
            emit(required(revoked.element, treeTag), revoke);
            expect(calls).toEqual([]);
            expect(revoked.element.isConnected).toBe(false);
        } finally { revoked.dispose(); revokedRoot.remove(); }
        scope.dispose(); emit(owner);
        expect(calls).toEqual([]);
        view.dispose(); emit(owner);
        expect(calls).toEqual([]);
        expect(view.update({ afterDisposal: true })).toBe(false);
    } finally { view.dispose(); scope.dispose(); treeSource.remove(); }
}

const treeParameters = { docs: { description: { story: 'Native pointer/Enter/Space and trust checks run in the browser test runner; ordinary Storybook uses programmatic activation.' } } };
export const TreeWorker = meta.story({ render: () => '<section></section>', parameters: treeParameters, play: async ({ canvasElement }) => treeContract(canvasElement, false) });
export const TreeFallback = meta.story({ render: () => '<section></section>', parameters: treeParameters, play: async ({ canvasElement }) => treeContract(canvasElement, true) });

async function authoredContract(canvasElement: HTMLElement, fallback: boolean) {
    const { runtime, scope } = createCemStoryRuntime(canvasElement, fallback);
    const tag = `authored-child-${fallback ? 'fallback' : 'worker'}`;
    const parentTag = `authored-parent-${fallback ? 'fallback' : 'worker'}`;
    const register = async (name: string, source: string) => {
        const declaration = document.createElement('div'); declaration.setAttribute('tag', name);
        const template = document.createElement('template'); template.type = 'text/cem-ml'; template.textContent = source;
        declaration.append(template); expect(runtime.registerDeclaration(declaration)).toBe(true);
        await runtime.whenDeclarationSettled(declaration);
    };
    await register(tag, '{input @value=initial @aria-label=Draft}{button @host-command=save | Save}{output | {$datadom.slices.status}}');
    await register(parentTag, `{section @host-command=pane @host-command-event=cem-tab @hidden={if datadom.slices.hidden { true } else { null }} | {${tag}}}`);
    const root = canvasElement.querySelector('section') as HTMLElement;
    const calls: string[] = [];
    const parent = mountCemView(runtime, { root, tag: parentTag, commands: { save: () => calls.push('parent'), pane: request => calls.push(`pane:${request.value}`) } });
    try {
        await parent.whenSettled();
        const element = parent.element.querySelector(tag) as HTMLElement;
        const container = element.parentElement as HTMLElement;
        const input = element.querySelector('input') as HTMLInputElement;
        container.dispatchEvent(new CustomEvent('cem-tab', { bubbles: true, detail: { value: 'result pane', index: 1, previousIndex: 0 } }));
        expect(calls.splice(0)).toEqual(['pane:result pane']);
        element.dispatchEvent(new CustomEvent('cem-tab', { bubbles: true, detail: { value: 'nested' } }));
        expect(calls).toEqual([]);
        container.dispatchEvent(new CustomEvent('cem-tab', { bubbles: true, detail: Object.create({ value: 'inherited' }) }));
        expect(parent.errors).toHaveLength(1);

        input.value = 'retained draft'; input.setSelectionRange(2, 6);
        expect(() => mountCemView(runtime, { root, tag, element })).toThrow(/direct child/);
        expect(() => mountCemView(runtime, { root: container, tag: parentTag, element })).toThrow(/requested tag/);
        const child = mountCemView(runtime, { root: container, tag, element, state: { status: 'Ready' }, commands: { save: () => calls.push('child') } });
        try {
            await child.whenSettled();
            expect(element.querySelector('output')?.textContent).toBe('Ready');
            expect(() => mountCemView(runtime, { root: container, tag, element })).toThrow(/unowned/);
            parent.update({ hidden: true }); await parent.whenSettled();
            parent.update({ hidden: false }); await parent.whenSettled();
            expect(parent.element.querySelector(tag)).toBe(element);
            expect(element.querySelector('input')).toBe(input);
            expect(input.value).toBe('retained draft');
            expect([input.selectionStart, input.selectionEnd]).toEqual([2, 6]);
            (element.querySelector('button') as HTMLButtonElement).click(); expect(calls).toEqual(['child']);
            child.dispose(); expect(element.isConnected).toBe(true);
            (element.querySelector('button') as HTMLButtonElement).click(); expect(calls).toEqual(['child']);
            expect(child.update({ status: 'Revoked' })).toBe(false);
        } finally { child.dispose(); }
    } finally { parent.dispose(); scope.dispose(); }
}
export const AuthoredWorker = meta.story({ render: () => '<section></section>', play: async ({ canvasElement }) => authoredContract(canvasElement, false) });
export const AuthoredFallback = meta.story({ render: () => '<section></section>', play: async ({ canvasElement }) => authoredContract(canvasElement, true) });
