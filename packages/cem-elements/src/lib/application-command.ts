import { type CemCheckpointJournalAuthority, type CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { isCemParameterEvent, openCommandParameters, readCommandResource } from './command-parameters.js';

export interface CemCommandDocument { readonly id: string; readonly revision: string; readonly content: string }
export interface CemDocumentWrite {
    readonly operationId: string;
    readonly document: CemCommandDocument;
    readonly parameters: Readonly<Record<string, string>>;
}
export interface CemWriteReceipt { readonly operationId: string; readonly outcome: 'acknowledged' | 'not-applied' }
export interface CemApplicationCommandResult {
    readonly invocationId: string;
    readonly status: 'succeeded' | 'failed' | 'cancelled' | 'unavailable';
}
export interface CemDocumentCommandOwner {
    /** The trusted host registers this root and the corresponding checkpoint owner. */
    root: HTMLElement;
    instanceId: string;
    pinDocument(): CemCommandDocument;
    write(request: CemDocumentWrite): Promise<CemWriteReceipt>;
    result?(result: CemApplicationCommandResult): void;
    /** Explicit store grants. Only a declared, nonsensitive format enum can cross this boundary. */
    preference?: { parameter: string; read?(): string | null; write?(value: string): void };
    menuLabels?: Readonly<Record<string, string>>;
}
export interface CemApplicationCommandOptions {
    manifest: Element;
    commandId: string;
    shortcutProfile: string;
    menuButton: HTMLButtonElement;
    menuItem: HTMLButtonElement;
    journal: CemCheckpointPublicationCoordinator;
    journalAuthority(): CemCheckpointJournalAuthority;
}

const appRoots = new WeakMap<HTMLElement, number>();
type FormSession = ReturnType<typeof openCommandParameters>;
interface Mount {
    owner: CemDocumentCommandOwner;
    live(): boolean;
    allowed: boolean;
    epoch: number;
    invocation?: string;
    form?: FormSession;
    cancel?: () => void;
}

/** One desktop document command, its granted editor endpoints and host menu projection. */
export function connectCemApplicationCommand(options: CemApplicationCommandOptions) {
    const { menuButton, menuItem, journal } = options, hostDocument = menuButton.ownerDocument;
    const resource = readCommandResource(options.manifest, options.commandId, options.shortcutProfile);
    if (menuItem.getAttribute('command-id') !== resource.id || menuItem.ownerDocument !== hostDocument)
        throw Error('cem.command.menu-binding');
    let locale = resource.defaultLocale, generation = 0, publishing = false, disposed = false;
    const mounts = new Map<HTMLElement, () => void>();
    const owners = new Set<Mount>();
    let focused: Mount | undefined, menuTarget: Mount | undefined;
    function eligible(mount?: Mount): mount is Mount {
        return !!mount && !disposed && !publishing && mount.allowed && mount.live();
    }
    function projectMenu() {
        const mount = menuTarget ?? focused;
        menuItem.textContent = mount?.owner.menuLabels?.[locale] ?? resource.text(locale, resource.menu);
        menuItem.lang = locale;
        menuItem.title = resource.text(locale, resource.description);
        const buttonKey = menuButton.getAttribute('text-key');
        if (buttonKey) { menuButton.textContent = resource.text(locale, buttonKey); menuButton.lang = locale; }
        menuItem.disabled = !eligible(mount) || !!mount.invocation;
        if (!menuItem.disabled) menuItem.setAttribute('aria-keyshortcuts', resource.shortcut);
        else menuItem.removeAttribute('aria-keyshortcuts');
    }
    function invoke(mount?: Mount): boolean {
        if (!eligible(mount) || mount.invocation) return false;
        const { owner, epoch } = mount;
        const invocationId = crypto.randomUUID();
        const admitted = () => mount.live() && mount.allowed && mount.epoch === epoch && !disposed;
        const finish = (status: CemApplicationCommandResult['status']) => {
            if (mount.invocation !== invocationId) return;
            const form = mount.form;
            mount.invocation = undefined;
            mount.cancel = undefined;
            mount.form = undefined;
            form?.close();
            projectMenu();
            if (admitted()) owner.result?.(Object.freeze({ invocationId, status }));
        };
        mount.invocation = invocationId;
        mount.cancel = () => finish('cancelled');
        projectMenu();
        try {
            const { id, revision, content } = owner.pinDocument();
            if (!id || !revision || typeof content !== 'string' || !admitted()) { finish('unavailable'); return false; }
            const document = Object.freeze({ id, revision, content });
            const pref = owner.preference, field = pref && resource.parameters.get(pref.parameter);
            const safePref = field?.type === 'enum' && !field.sensitive ? pref : undefined;
            let seed: string | null | undefined;
            try { seed = safePref?.read?.(); } catch { /* Optional store access. */ }
            const accept = (parameters: Readonly<Record<string, string>>) => {
                if (!admitted() || publishing || mount.invocation !== invocationId || !mount.form) return;
                // Release the private form before crossing the endpoint. No duplicate submit can dispatch.
                const form = mount.form;
                mount.form = undefined;
                mount.cancel = undefined;
                form.close();
                let operationId: string;
                try { operationId = journal.operation(owner.instanceId).operationId; }
                catch { finish('unavailable'); return; }
                const preference = safePref && parameters[safePref.parameter];
                function receive(receipt: CemWriteReceipt) {
                    try {
                        if (receipt.operationId !== operationId) { finish('failed'); return; }
                        journal.reconcileOperation(operationId, receipt.outcome, options.journalAuthority());
                    } catch { finish('failed'); return; }
                    if (receipt.outcome === 'acknowledged' && admitted() && preference) {
                        try { safePref?.write?.(preference); } catch { /* A committed write survives store denial. */ }
                    }
                    finish(receipt.outcome === 'acknowledged' ? 'succeeded' : 'failed');
                }
                try {
                    if (!admitted()) { journal.reconcileOperation(operationId, 'not-applied', options.journalAuthority()); finish('unavailable'); return; }
                    void Promise.resolve(owner.write({ operationId, document, parameters })).then(receive, () => finish('failed'));
                } catch { finish('failed'); }
            };
            if (!admitted() || publishing) { finish('unavailable'); return false; }
            const form = openCommandParameters(owner.root, resource, locale, accept, mount.cancel,
                safePref && seed && field?.choices.has(seed) ? [safePref.parameter, seed] : undefined);
            // Native entry can synchronously move focus and withdraw this mount.
            if (!admitted() || mount.invocation !== invocationId) { form.close(); return false; }
            mount.form = form;
            return true;
        } catch { finish('unavailable'); return false; }
    }
    function openMenu() { menuTarget = focused; projectMenu(); }
    function select() {
        const target = menuTarget;
        menuTarget = undefined;
        if (popup?.matches(':popover-open')) (popup as HTMLElement).hidePopover();
        if (target && menuItem.getAttribute('command-id') === resource.id) invoke(target);
        projectMenu();
    }
    const popup = menuButton.popoverTargetElement;
    function closed() { if (!popup?.matches(':popover-open')) { menuTarget = undefined; projectMenu(); } }
    menuButton.addEventListener('click', openMenu);
    menuItem.addEventListener('click', select);
    popup?.addEventListener('toggle', closed);
    projectMenu();
    return {
        register(owner: CemDocumentCommandOwner) {
            const { root } = owner, doc = root.ownerDocument;
            if (disposed || mounts.has(root) || [...owners].some(m => m.owner.instanceId === owner.instanceId))
                throw Error('cem.command.duplicate-owner');
            // Pin each browsing-context edge. isConnected alone remains true in retired iframe documents.
            const frames: { frame: HTMLIFrameElement; document: Document }[] = [];
            for (let current = doc; current !== hostDocument;) {
                const frame = current.defaultView?.frameElement;
                if (!frame) throw Error('cem.command.unreachable-owner');
                if (frame.localName !== 'iframe') throw Error('cem.command.unsupported-frame');
                frames.push({ frame: frame as HTMLIFrameElement, document: current }); current = frame.ownerDocument;
            }
            let active = true;
            const mount: Mount = { owner, epoch: 0, allowed: false,
                live: () => active && root.isConnected && root.ownerDocument === doc && frames.every(edge =>
                    edge.frame.isConnected && edge.frame.contentDocument === edge.document) };
            const owns = (event: Event) => event.composedPath().find(node => appRoots.has(node as HTMLElement)) === root;
            const focus = (event: Event) => { if (owns(event) && mount.live()) { focused = mount; projectMenu(); } };
            const key = (event: KeyboardEvent) => {
                if (!isCemParameterEvent(event) && owns(event) && resource.matches(event) && invoke(mount)) event.preventDefault();
            };
            owners.add(mount);
            appRoots.set(root, (appRoots.get(root) ?? 0) + 1);
            root.addEventListener('focusin', focus);
            root.addEventListener('keydown', key);
            function close() {
                if (!active) return;
                active = false; mount.epoch++; mount.allowed = false;
                mount.cancel?.();
                owners.delete(mount); mounts.delete(root);
                const count = (appRoots.get(root) ?? 1) - 1;
                if (count) appRoots.set(root, count); else appRoots.delete(root);
                root.removeEventListener('focusin', focus); root.removeEventListener('keydown', key);
                if (focused === mount) focused = undefined;
                // Keep a retired menu target pinned until the menu closes. It cannot fall through to another editor.
                projectMenu();
            }
            mounts.set(root, close);
            return { close, grant(allowed: boolean) {
                if (!active || mount.allowed === allowed) return;
                mount.allowed = allowed; mount.epoch++;
                if (!allowed) mount.cancel?.();
                projectMenu();
            } };
        },
        /** Fence old callbacks now. Only the latest complete controls + shortcut publication can lift the fence. */
        async setLocale(next: string, publication: { ready?: Promise<void>; acknowledge?(): Promise<void> } = {}) {
            const token = ++generation;
            publishing = true;
            for (const owner of owners) owner.form?.fence(true);
            projectMenu();
            await publication.ready;
            if (disposed || token !== generation) return false;
            locale = next; projectMenu();
            for (const owner of owners) owner.form?.localize(locale);
            await publication.acknowledge?.();
            if (disposed || token !== generation) return false;
            publishing = false;
            for (const owner of owners) owner.form?.fence(false);
            projectMenu();
            return true;
        },
        dispose() {
            disposed = true; generation++;
            for (const close of mounts.values()) close();
            menuButton.removeEventListener('click', openMenu); menuItem.removeEventListener('click', select);
            popup?.removeEventListener('toggle', closed); menuTarget = undefined; focused = undefined; projectMenu();
        },
    };
}
